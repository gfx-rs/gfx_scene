//! Phase infrastructure for Gfx.

use std::cmp::Ordering;
use std::collections::HashMap;
use draw_queue;
use gfx;
use mem;

/// Potential error occuring during rendering.
pub type FlushError = gfx::DrawError<gfx::batch::OutOfBounds>;

/// An aspect of the phase to allow flushing into a Renderer.
pub trait FlushPhase<R: gfx::Resources> {
    /// Flush the queue into a given renderer.
    fn flush<O: gfx::Output<R>, C: gfx::CommandBuffer<R>>(&mut self,
             &O, &mut gfx::Renderer<R, C>) -> Result<(), FlushError>;
}

/// An aspect of the phase that allows queuing entities for rendering.
pub trait QueuePhase<E, V: ::ToDepth> {
    /// Check if it makes sense to draw this entity.
    fn test(&self, &E) -> bool;
    /// Add an entity to the queue.
    fn enqueue(&mut self, &E, V) -> Result<(), gfx::batch::Error>;
}

/// An abstract phase. Needs to be object-safe as phases should be
/// allowed to be stored in boxed form in containers.
pub trait AbstractPhase<R: gfx::Resources, E, V: ::ToDepth>:
    QueuePhase<E, V> + FlushPhase<R>
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
}

/// A container for the standard sorting methods.
pub mod sort {
    use std::cmp::Ordering;
    use gfx::shade::ShaderParam;
    use super::Object;
    /// Sort by depth, front-to-back. Useful for opaque objects that updates
    /// the depth buffer. The front stuff will occlude more pixels, leaving
    /// less work to be done for the farther objects.
    pub fn front_to_back<S: PartialOrd, K, P: ShaderParam>(
                         a: &Object<S, K, P>, b: &Object<S, K, P>) -> Ordering
    {
        a.cmp_depth(b)
    }
    /// Sort by depth, back-to-front. Useful for transparent objects, since
    /// blending should take into account everything that lies behind.
    pub fn back_to_front<S: PartialOrd, K, P: ShaderParam>(
                         a: &Object<S, K, P>, b: &Object<S, K, P>) -> Ordering
    {
        b.cmp_depth(a)
    }
    /// Sort by shader program. Switching a program is one of the heaviest
    /// state changes, so this variant is useful when the order is not important.
    pub fn program<S, K, P: ShaderParam>(a: &Object<S, K, P>, b: &Object<S, K, P>)
                   -> Ordering
    {
        a.batch.cmp_program(&b.batch)
    }
    /// Sort by mesh. Allows minimizing the vertex format changes.
    pub fn mesh<S, K, P: ShaderParam>(a: &Object<S, K, P>, b: &Object<S, K, P>)
                -> Ordering
    {
        a.batch.cmp_mesh(&b.batch)
    }
    /// Sort by draw state.
    pub fn state<S, K, P: ShaderParam>(a: &Object<S, K, P>, b: &Object<S, K, P>)
                 -> Ordering
    {
        a.batch.cmp_state(&b.batch)
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
    /// Sorting function.
    pub sort: Option<fn(&Object<V::Depth, T::Kernel, T::Params>,
                        &Object<V::Depth, T::Kernel, T::Params>)
                        -> Ordering>,
    /// Phase memory.
    memory: Y,
    /// Sorted draw queue.
    queue: draw_queue::Queue<Object<V::Depth, T::Kernel, T::Params>>,
    /// Batch context.
    context: gfx::batch::Context<R>,
}

/// Memory typedef using a `HashMap`.
pub type CacheMap<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth,
    T: ::Technique<R, M, V>,
> = HashMap<(T::Kernel, gfx::Mesh<R>),
    mem::MemResult<Object<V::Depth, T::Kernel, T::Params>>,
>;

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
> Phase<R, M, V, T, ()> {
    /// Create a new phase from a given technique.
    pub fn new(name: &str, tech: T) -> Phase<R, M, V, T, ()> {
        Phase {
            name: name.to_string(),
            technique: tech,
            sort: None,
            memory: (),
            queue: draw_queue::Queue::new(),
            context: gfx::batch::Context::new(),
        }
    }

    /// Enable caching of created render objects.
    pub fn with_cache(self) -> CachedPhase<R, M, V, T> {
        Phase {
            name: self.name,
            technique: self.technique,
            sort: self.sort,
            memory: HashMap::new(),
            queue: self.queue,
            context: self.context,
        }
    }
}

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth + Copy,
    E: ::Entity<R, M>,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<(T::Kernel, gfx::Mesh<R>),
        Object<V::Depth, T::Kernel, T::Params>,
    >,
>QueuePhase<E, V> for Phase<R, M, V, T, Y> where
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Copy,    
{
    fn test(&self, entity: &E) -> bool {
        self.technique.test(entity.get_mesh().0, entity.get_material())
                      .is_some()
    }

    fn enqueue(&mut self, entity: &E, view_info: V)
               -> Result<(), gfx::batch::Error> {
        let kernel = self.technique.test(
            entity.get_mesh().0, entity.get_material())
            .unwrap(); //TODO?
        let (orig_mesh, slice) = entity.get_mesh();
        let depth = view_info.to_depth();
        let key = (kernel, orig_mesh.clone()); //TODO: avoid clone() here
        // Try recalling from memory
        match self.memory.lookup(&key) {
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
        let object = self.context.make_core(program, mesh, state)
            .map(|b| Object {
                batch: b,
                params: params,
                slice: slice.clone(),
                depth: depth,
                kernel: kernel,
            });
        // Remember and return
        self.memory.store(key, object.clone());
        match object {
            Ok(o) => Ok(self.queue.objects.push(o)),
            Err(e) => {
                warn!("Phase {}: batch creation failed: {:?}", self.name, e);
                Err(e)
            },
        }
    }
}

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth + Copy,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<(T::Kernel, gfx::Mesh<R>),
        Object<V::Depth, T::Kernel, T::Params>,
    >,
>FlushPhase<R> for Phase<R, M, V, T, Y> {
    fn flush<
        O: gfx::Output<R>,
        C: gfx::CommandBuffer<R>,
    >(
        &mut self, output: &O, renderer: &mut gfx::Renderer<R, C>)
            -> Result<(), FlushError> {
        if let Some(fun) = self.sort {
            self.queue.sort(fun);
        }
        // accumulate the draws into the renderer
        for o in self.queue.iter() {
            match renderer.draw(&self.context.bind(&o.batch, &o.slice, &o.params), output) {
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
    M: ::Material,
    V: ::ToDepth + Copy,
    E: ::Entity<R, M>,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<(T::Kernel, gfx::Mesh<R>),
        Object<V::Depth, T::Kernel, T::Params>
    >,
>AbstractPhase<R, E, V> for Phase<R, M, V, T, Y> where
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Copy,
{}
