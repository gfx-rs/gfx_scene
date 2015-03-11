extern crate draw_queue;

use std::cmp::Ordering;
use gfx;
use mem::Memory;

pub type FlushError = gfx::DrawError<gfx::batch::OutOfBounds>;

/// An abstract phase. Needs to be object-safe as phases should be
/// allowed to be stored in boxed form in containers.
pub trait AbstractPhase<D: gfx::Device, E, Z> {
    /// Check if it makes sense to draw this entity
    fn test(&self, &E) -> bool;
    /// Add an entity to the queue
    fn enqueue(&mut self, &E, Z, &mut gfx::batch::Context<D::Resources>)
               -> Result<(), gfx::batch::Error>;
    /// Flush the queue into a given renderer
    fn flush(&mut self, &gfx::Frame<D::Resources>,
             &gfx::batch::Context<D::Resources>,
             &mut gfx::Renderer<D::Resources, D::CommandBuffer>)
             -> Result<(), FlushError>;
}

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

pub trait ToDepth {
    type Depth: PartialOrd;
    fn to_depth(&self) -> Self::Depth;
}

/// Phase is doing draw call accumulating and sorting,
/// based a given technique.
pub struct Phase<
    R: gfx::Resources,
    M: ::Material,
    Z: ToDepth,
    T: ::Technique<R, M, Z>,
    Y,
>{
    pub name: String,
    pub technique: T,
    memory: Y,
    pub sort: Vec<Sort>,
    queue: draw_queue::Queue<Object<Z::Depth, T::Params>>,
}

impl<
    R: gfx::Resources,
    M: ::Material,
    Z: ToDepth,
    T: ::Technique<R, M, Z>,
> Phase<R, M, Z, T, ()> {
    pub fn new(name: &str, tech: T) -> Phase<R, M, Z, T, ()> {
        Phase {
            name: name.to_string(),
            technique: tech,
            memory: (),
            sort: Vec::new(),
            queue: draw_queue::Queue::new(),
        }
    }
}

impl<
    D: gfx::Device,
    M: ::Material,
    Z: ToDepth + Copy,
    E: ::Entity<D::Resources, M>,
    T: ::Technique<D::Resources, M, Z>,
    Y: Memory<T::Essense, Object<Z::Depth, T::Params>>,
>AbstractPhase<D, E, Z> for Phase<D::Resources, M, Z, T, Y> where
    Z::Depth: Copy,
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Copy,    
{
    fn test(&self, entity: &E) -> bool {
        self.technique.test(entity.get_mesh().0, entity.get_material())
                      .is_some()
    }

    fn enqueue(&mut self, entity: &E, data: Z,
               context: &mut gfx::batch::Context<D::Resources>)
               -> Result<(), gfx::batch::Error> {
        let essense = self.technique.test(
            entity.get_mesh().0, entity.get_material())
            .unwrap(); //TODO?
        let (orig_mesh, slice) = entity.get_mesh();
        // Try recalling from memory
        match self.memory.lookup(essense) {
            Some(Ok(mut o)) => {
                o.slice = slice.clone();
                self.technique.fix_params(entity.get_material(),
                                          &data, &mut o.params);
                self.queue.objects.push(o);
                return Ok(())
            },
            Some(Err(e)) => return Err(e),
            None => ()
        }
        // Compile with the technique
        let depth = data.to_depth();
        let (program, params, inst_mesh, state) =
            self.technique.compile(essense, data);
        // this would be useful, but requires a ton of new constraints on Params
        //debug_assert_eq!({
        //    let mut p2 = params;
        //    self.technique.fix_params(entity.get_material(), &data, &mut p2);
        //    p2}, params);
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
        self.memory.store(essense, object.clone());
        match object {
            Ok(o) => Ok(self.queue.objects.push(o)),
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self, frame: &gfx::Frame<D::Resources>,
             context: &gfx::batch::Context<D::Resources>,
             renderer: &mut gfx::Renderer<D::Resources, D::CommandBuffer>)
             -> Result<(), FlushError> {
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
