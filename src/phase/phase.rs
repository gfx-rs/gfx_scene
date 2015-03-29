//! Phase infrastructure for Gfx.

extern crate draw_queue;

use std::cmp::Ordering;
use std::collections::HashMap;
use gfx;
use mem;

/// Type of phase sorting.
pub enum Sort {
    /// Sort by depth, front-to-back. Useful for opaque objects that updates
    /// the depth buffer. The front stuff will occlude more pixels, leaving
    /// less work to be done for the farther objects.
    FrontToBack,
    /// Sort by depth, back-to-front. Useful for transparent objects, since
    /// blending should take into account everything that lies behind.
    BackToFront,
    /// Sort by shader program. Switching a program is one of the heaviest
    /// state changes, so this variant is useful when the order is not important.
    Program,
    /// Sort by mesh. Allows minimizing the vertex format changes.
    Mesh,
    /// Sort by draw state.
    DrawState,
}

/// Potential error occuring during rendering.
pub type FlushError = gfx::DrawError<gfx::batch::OutOfBounds>;

/// An aspect of the phase to allow flushing into a Renderer.
pub trait FlushPhase<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    /// Flush the queue into a given renderer.
    fn flush(&mut self, &gfx::Frame<R>, &gfx::batch::Context<R>,
             &mut gfx::Renderer<R, C>) -> Result<(), FlushError>;
}

/// An aspect of the phase that allows queuing entities for rendering.
pub trait QueuePhase<R: gfx::Resources, E, V: ::ToDepth> {
    /// Check if it makes sense to draw this entity.
    fn test(&self, &E) -> bool;
    /// Add an entity to the queue.
    fn enqueue(&mut self, &E, V, &mut gfx::batch::Context<R>)
               -> Result<(), gfx::batch::Error>;
    /// Sort by given criterias.
    fn sort(&mut self, order: &[Sort]);
}

/// An abstract phase. Needs to be object-safe as phases should be
/// allowed to be stored in boxed form in containers.
pub trait AbstractPhase<R: gfx::Resources, C: gfx::CommandBuffer<R>, E, V: ::ToDepth>:
    QueuePhase<R, E, V> +
    FlushPhase<R, C>
{}

/// A rendering object, encapsulating the batch and additional info
/// needed for sorting. It is only exposed for this matter and
/// accessed by immutable references by the user.
#[allow(missing_docs)]
pub struct Object<S, K, P: gfx::shade::ShaderParam> {
    pub batch: gfx::batch::CoreBatch<P>,
    pub params: P,
    pub slice: gfx::Slice<P::Resources>,
    pub depth: S,
    pub kernel: K,
}

impl<S: Copy, K: Copy, P: gfx::shade::ShaderParam + Clone> Clone
for Object<S, K, P> where P::Link: Copy
{
    fn clone(&self) -> Object<S, K, P> {
        Object {
            batch: self.batch,
            params: self.params.clone(),
            slice: self.slice.clone(),
            depth: self.depth,
            kernel: self.kernel,
        }
    }
}

impl<S: PartialOrd, K, P: gfx::shade::ShaderParam> Object<S, K, P> {
    /// A helper method to compare the depth, which is only partially ordered.
    pub fn cmp_depth(&self, other: &Object<S, K, P>) -> Ordering {
        self.depth.partial_cmp(&other.depth)
            .unwrap_or(Ordering::Equal)
    }

    /// Order by depth, front-to-back. Useful for opaque objects that updates
    /// the depth buffer. The front stuff will occlude more pixels, leaving
    /// less work to be done for the farther objects.
    pub fn front_to_back(a: &Object<S, K, P>, b: &Object<S, K, P>) -> Ordering {
        a.cmp_depth(b)
    }

    /// Order by depth, back-to-front. Useful for transparent objects, since
    /// blending should take into account everything that lies behind.
    pub fn back_to_front(a: &Object<S, K, P>, b: &Object<S, K, P>) -> Ordering {
        b.cmp_depth(a)
    }
}

/// Phase is doing batch construction, accumulation, and memorization,
/// based on a given technique.
pub struct Phase<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
    Y,  // Memory
>{
    /// Phase name.
    pub name: String,
    /// Contained technique.
    pub technique: T,
    /// Phase memory.
    memory: Y,
    /// Sorted draw queue.
    pub queue: draw_queue::Queue<Object<V::Depth, T::Kernel, T::Params>>,
}

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> Phase<R, M, V, T, ()> {
    /// Create a new phase from a given technique.
    pub fn new(name: &str, tech: T) -> Phase<R, M, V, T, ()> {
        Phase {
            name: name.to_string(),
            technique: tech,
            memory: (),
            queue: draw_queue::Queue::new(),
        }
    }
}

/// Memory typedef using a `HashMap`.
pub type CacheMap<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> = HashMap<T::Kernel, mem::MemResult<Object<V::Depth, T::Kernel, T::Params>>>;

/// A render phase that caches created render objects.
pub type CachedPhase<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> = Phase<R, M, V, T, CacheMap<R, M, V, T>>;

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> Phase<R, M, V, T, CacheMap<R, M, V, T>> {
    /// Create a new phase that caches created objects.
    pub fn new_cached(name: &str, tech: T) -> CachedPhase<R, M, V, T> {
        Phase {
            name: name.to_string(),
            technique: tech,
            memory: HashMap::new(),
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
    Y: mem::Memory<T::Kernel, Object<V::Depth, T::Kernel, T::Params>>,
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
        let depth = view_info.to_depth();
        // Try recalling from memory
        match self.memory.lookup(kernel) {
            Some(Ok(mut o)) => {
                o.slice = slice.clone();
                o.depth = depth;
                assert_eq!(o.kernel, kernel);
                self.technique.fix_params(entity.get_material(),
                                          &view_info, &mut o.params);
                self.queue.objects.push(o);
                return Ok(())
            },
            Some(Err(e)) => return Err(e),
            None => ()
        }
        // Compile with the technique
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
                                kernel: kernel,
                            });
        // Remember and return
        self.memory.store(kernel, object.clone());
        match object {
            Ok(o) => Ok(self.queue.objects.push(o)),
            Err(e) => Err(e),
        }
    }

    fn sort(&mut self, sort: &[Sort]) {
        //TODO: multiple criterias
        match sort.first() {
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
    }
}

impl<
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    M: ::Material,
    V: ::ToDepth + Copy,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<T::Kernel, Object<V::Depth, T::Kernel, T::Params>>,
>FlushPhase<R, C> for Phase<R, M, V, T, Y> {
    fn flush(&mut self, frame: &gfx::Frame<R>, context: &gfx::batch::Context<R>,
             renderer: &mut gfx::Renderer<R, C>) -> Result<(), FlushError> {
        self.queue.update();
        // accumulate the draws into the renderer
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
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    M: ::Material,
    V: ::ToDepth + Copy,
    E: ::Entity<R, M>,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<T::Kernel, Object<V::Depth, T::Kernel, T::Params>>,
>AbstractPhase<R, C, E, V> for Phase<R, M, V, T, Y> where
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Copy,
{}
