extern crate draw_queue;

use std::cmp::Ordering;
use std::collections::HashMap;
use gfx;
use mem;

pub type FlushError = gfx::DrawError<gfx::batch::OutOfBounds>;

/// An aspect of the phase to allow flushing into a Renderer
pub trait FlushPhase<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    /// Flush the queue into a given renderer
    fn flush(&mut self, &gfx::Frame<R>, &gfx::batch::Context<R>,
             &mut gfx::Renderer<R, C>) -> Result<(), FlushError>;
}

/// An aspect of the phase that allows queuing entities for rendering
pub trait QueuePhase<R: gfx::Resources, E, V: ::ToDepth> {
    /// Check if it makes sense to draw this entity
    fn test(&self, &E) -> bool;
    /// Add an entity to the queue
    fn enqueue(&mut self, &E, V, &mut gfx::batch::Context<R>)
               -> Result<(), gfx::batch::Error>;
}

/// An abstract phase. Needs to be object-safe as phases should be
/// allowed to be stored in boxed form in containers.
pub trait AbstractPhase<D: gfx::Device, E, V: ::ToDepth>:
    QueuePhase<D::Resources, E, V> +
    FlushPhase<D::Resources, D::CommandBuffer>
{}

struct Object<S, P: gfx::shade::ShaderParam> {
    batch: gfx::batch::CoreBatch<P>,
    params: P,
    slice: gfx::Slice<P::Resources>,
    depth: S,
}

impl<S: Copy, P: gfx::shade::ShaderParam + Clone> Clone
for Object<S, P> where P::Link: Copy
{
    fn clone(&self) -> Object<S, P> {
        Object {
            batch: self.batch,
            params: self.params.clone(),
            slice: self.slice.clone(),
            depth: self.depth,
        }
    }
}

impl<S: PartialOrd, P: gfx::shade::ShaderParam> Object<S, P> {
    fn cmp_depth(&self, other: &Object<S, P>) -> Ordering {
        self.depth.partial_cmp(&other.depth)
            .unwrap_or(Ordering::Equal)
    }
}

pub enum Sort {
    FrontToBack,
    BackToFront,
    Program,
    Mesh,
    DrawState,
}

/// Phase is doing draw call accumulating and sorting,
/// based a given technique.
pub struct Phase<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
    Y,  // Memory
>{
    pub name: String,
    pub technique: T,
    memory: Y,
    pub sort: Vec<Sort>,
    queue: draw_queue::Queue<Object<V::Depth, T::Params>>,
}

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> Phase<R, M, V, T, ()> {
    pub fn new(name: &str, tech: T) -> Phase<R, M, V, T, ()> {
        Phase {
            name: name.to_string(),
            technique: tech,
            memory: (),
            sort: Vec::new(),
            queue: draw_queue::Queue::new(),
        }
    }
}

pub type CacheMap<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> = HashMap<T::Kernel, mem::MemResult<Object<V::Depth, T::Params>>>;

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> Phase<R, M, V, T, CacheMap<R, M, V, T>> {
    pub fn new_cached(name: &str, tech: T) -> Phase<R, M, V, T, CacheMap<R, M, V, T>> {
        Phase {
            name: name.to_string(),
            technique: tech,
            memory: HashMap::new(),
            sort: Vec::new(),
            queue: draw_queue::Queue::new(),
        }
    }
}

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth + Copy,
    E: ::Entity<R, M>,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<T::Kernel, Object<V::Depth, T::Params>>,
>QueuePhase<R, E, V> for Phase<R, M, V, T, Y> where
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Copy,    
{
    fn test(&self, entity: &E) -> bool {
        self.technique.test(entity.get_mesh().0, entity.get_material())
                      .is_some()
    }

    fn enqueue(&mut self, entity: &E, view_info: V,
               context: &mut gfx::batch::Context<R>)
               -> Result<(), gfx::batch::Error> {
        let kernel = self.technique.test(
            entity.get_mesh().0, entity.get_material())
            .unwrap(); //TODO?
        let (orig_mesh, slice) = entity.get_mesh();
        // Try recalling from memory
        match self.memory.lookup(kernel) {
            Some(Ok(mut o)) => {
                o.slice = slice.clone();
                self.technique.fix_params(entity.get_material(),
                                          &view_info, &mut o.params);
                self.queue.objects.push(o);
                return Ok(())
            },
            Some(Err(e)) => return Err(e),
            None => ()
        }
        // Compile with the technique
        let depth = view_info.to_depth();
        let (program, mut params, inst_mesh, state) =
            self.technique.compile(kernel, view_info);
        self.technique.fix_params(entity.get_material(),
                                  &view_info, &mut params);
        let mut temp_mesh = gfx::Mesh::new(orig_mesh.num_vertices);
        let mesh = match inst_mesh {
            Some(m) => {
                temp_mesh.attributes.extend(orig_mesh.attributes.iter()
                    .chain(m.attributes.iter()).map(|a| a.clone()));
                &temp_mesh
            },
            None => orig_mesh,
        };
        // Create queue object
        let object = context.make_core(program, mesh, state)
                            .map(|b| Object {
                                batch: b,
                                params: params,
                                slice: slice.clone(),
                                depth: depth,
                            });
        // Remember and return
        self.memory.store(kernel, object.clone());
        match object {
            Ok(o) => Ok(self.queue.objects.push(o)),
            Err(e) => Err(e),
        }
    }
}

impl<
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    M: ::Material,
    V: ::ToDepth + Copy,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<T::Kernel, Object<V::Depth, T::Params>>,
>FlushPhase<R, C> for Phase<R, M, V, T, Y> {
    fn flush(&mut self, frame: &gfx::Frame<R>, context: &gfx::batch::Context<R>,
             renderer: &mut gfx::Renderer<R, C>) -> Result<(), FlushError> {
        // sort the queue
        match self.sort.first() {
            Some(&Sort::FrontToBack) =>
                self.queue.sort(|a, b| a.cmp_depth(&b)),
            Some(&Sort::BackToFront) =>
                self.queue.sort(|a, b| b.cmp_depth(&a)),
            Some(&Sort::Program) =>
                self.queue.sort(|a, b| a.batch.cmp_program(&b.batch)),
            Some(&Sort::Mesh) =>
                self.queue.sort(|a, b| a.batch.cmp_mesh(&b.batch)),
            Some(&Sort::DrawState) =>
                self.queue.sort(|a, b| a.batch.cmp_state(&b.batch)),
            None => (),
        }
        // call the draws
        for o in self.queue.iter() {
            match renderer.draw(&context.bind(&o.batch, &o.slice, &o.params), frame) {
                Ok(_) => (),
                e => return e,
            }
        }
        // done
        self.queue.objects.clear();
        Ok(())
    }
}

impl<
    D: gfx::Device,
    M: ::Material,
    V: ::ToDepth + Copy,
    E: ::Entity<D::Resources, M>,
    T: ::Technique<D::Resources, M, V>,
    Y: mem::Memory<T::Kernel, Object<V::Depth, T::Params>>,
>AbstractPhase<D, E, V> for Phase<D::Resources, M, V, T, Y> where
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Copy,
{}
