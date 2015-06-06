//! Phase infrastructure for Gfx.

use std::cmp::Ordering;
use std::collections::HashMap;
use draw_queue;
use gfx;
use mem;

/// Potential error occuring during rendering.
pub type FlushError = gfx::DrawError<gfx::batch::Error>;

/// An abstract rendering phase.
pub trait AbstractPhase<R: gfx::Resources, M, V: ::ToDepth> {
    /// Add an entity to the queue.
    fn enqueue(&mut self, &gfx::Mesh<R>, &gfx::Slice<R>, &M, &V)
               -> Result<bool, gfx::batch::Error>;
    /// Flush the queue into a given stream.
    fn flush<S: gfx::Stream<R>>(&mut self, stream: &mut S)
             -> Result<(), FlushError>;
}

/// A rendering object, encapsulating the batch and additional info
/// needed for sorting. It is only exposed for this matter and
/// accessed by immutable references by the user.
#[allow(missing_docs)]
pub struct Object<S, K, P: gfx::shade::ShaderParam> {
    pub batch: gfx::batch::Core<P>,
    pub params: P,
    pub slice: gfx::Slice<P::Resources>,
    pub instances: Option<gfx::InstanceCount>,
    pub depth: S,
    pub kernel: K,
    pub state: gfx::DrawState,
}

impl<S: Copy, K: Copy, P: gfx::shade::ShaderParam + Clone> Clone
for Object<S, K, P> where
    P::Link: Clone,
{
    fn clone(&self) -> Object<S, K, P> {
        Object {
            batch: self.batch.clone(),
            params: self.params.clone(),
            slice: self.slice.clone(),
            instances: self.instances.clone(),
            depth: self.depth,
            kernel: self.kernel,
            state: self.state
        }
    }
}

impl<'a, S, K, P: gfx::shade::ShaderParam> Object<S, K, P> {
    fn draw<X>(&self, stream: &mut X)
            -> Result<(), gfx::DrawError<gfx::batch::Error>> where
            X: gfx::Stream<P::Resources>,
    {
        let batch = self.batch.with(&self.slice, &self.params, &self.state);
        match self.instances {
            Some(num) => stream.draw_instanced(&batch, num, 0),
            None => stream.draw(&batch),
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
        a.batch.program().cmp_ref(&b.batch.program())
    }
    /// Sort by mesh. Allows minimizing the vertex format changes.
    pub fn mesh<S, K, P: ShaderParam>(a: &Object<S, K, P>, b: &Object<S, K, P>)
                -> Ordering
    {
        for (a, b) in a.batch.mesh().attributes.iter().zip(b.batch.mesh().attributes.iter()) {
            match a.buffer.cmp_ref(&b.buffer) {
                Ordering::Equal => continue,
                x => return x
            }
        }
        Ordering::Equal
    }

    /* TODO
    /// Sort by draw state.
    pub fn state<S, K, P: ShaderParam>(a: &Object<S, K, P>, b: &Object<S, K, P>)
                 -> Ordering
    {
        a.batch.state.partial_cmp(&b.batch.state)
    }*/
}

/// Ordering function.
pub type OrderFun<S, K, P> = fn(&Object<S, K, P>, &Object<S, K, P>) -> Ordering;

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
    pub sort: Option<OrderFun<V::Depth, T::Kernel, T::Params>>,
    /// Phase memory.
    memory: Y,
    /// Sorted draw queue.
    queue: draw_queue::Queue<Object<V::Depth, T::Kernel, T::Params>>,
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
            queue: draw_queue::Queue::new()
        }
    }

    /// Enable sorting of rendered objects.
    pub fn with_sort(self, fun: OrderFun<V::Depth, T::Kernel, T::Params>)
                     -> Phase<R, M, V, T, ()> {
        Phase {
            sort: Some(fun),
            .. self
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
        }
    }
}

impl<
    R: gfx::Resources,
    M: ::Material,
    V: ::ToDepth + Copy,
    T: ::Technique<R, M, V>,
    Y: mem::Memory<(T::Kernel, gfx::Mesh<R>),
        Object<V::Depth, T::Kernel, T::Params>
    >,
>AbstractPhase<R, M, V> for Phase<R, M, V, T, Y> where
    T::Params: Clone,
    <T::Params as gfx::shade::ShaderParam>::Link: Clone,
{
    fn enqueue(&mut self, orig_mesh: &gfx::Mesh<R>, slice: &gfx::Slice<R>,
               material: &M, view_info: &V)
               -> Result<bool, gfx::batch::Error> {
        let kernel = match self.technique.test(orig_mesh, material) {
            Some(k) => k,
            None => return Ok(false),
        };
        let depth = view_info.to_depth();
        let key = (kernel, orig_mesh.clone()); //TODO: avoid clone() here
        // Try recalling from memory
        match self.memory.lookup(&key) {
            Some(Ok(mut o)) => {
                o.slice = slice.clone();
                o.depth = depth;
                assert_eq!(o.kernel, kernel);
                self.technique.fix_params(material, view_info, &mut o.params);
                self.queue.objects.push(o);
                return Ok(true)
            },
            Some(Err(e)) => return Err(e),
            None => ()
        }
        // Compile with the technique
        let (program, mut params, state, instancing) =
            self.technique.compile(kernel, view_info);
        self.technique.fix_params(material, view_info, &mut params);
        let mut temp_mesh = gfx::Mesh::new(orig_mesh.num_vertices);
        let (instances, mesh) = match instancing {
            Some((num, extra_attributes)) => {
                temp_mesh.attributes.extend(orig_mesh.attributes.iter()
                    .chain(extra_attributes.iter()).map(|a| a.clone()));
                (Some(num), &temp_mesh)
            },
            None => (None, orig_mesh),
        };
        // Create queue object
        let object = gfx::batch::Core::new(mesh.clone(), program.clone())
            .map(|b| Object {
                batch: b,
                params: params,
                slice: slice.clone(),
                instances: instances,
                depth: depth,
                kernel: kernel,
                state: *state
            });
        // Remember and return
        self.memory.store(key, object.clone());
        match object {
            Ok(o) => {
                self.queue.objects.push(o);
                Ok(true)
            },
            Err(e) => {
                warn!("Phase {}: batch creation failed: {:?}", self.name, e);
                Err(e)
            },
        }
    }

    fn flush<S: gfx::Stream<R>>(&mut self, stream: &mut S)
             -> Result<(), FlushError> {
        match self.sort {
            Some(fun) => {
                // sort the queue
                self.queue.sort(fun);
                // accumulate the sorted draws into the renderer
                for o in self.queue.iter() {
                    try!(o.draw(stream));
                }
            },
            None => {
                // accumulate the raw draws into the renderer
                for o in self.queue.objects.iter() {
                    try!(o.draw(stream));
                }
            }
        }
        // done
        self.queue.objects.clear();
        Ok(())
    }
}
