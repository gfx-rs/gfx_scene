#![deny(missing_docs)]

//! Scene infrastructure to be used with Gfx phases.

extern crate gfx_phase;
extern crate gfx;
extern crate cgmath;

use std::fmt::Debug;
use std::marker::PhantomData;

/// Scene drawing error.
#[derive(Debug)]
pub enum Error {
    /// Error in creating a batch.
    Batch(gfx::batch::Error),
    /// Error in sending a batch for drawing.
    Flush(gfx_phase::FlushError),
}

/// Abstract information about the view. Supposed to containt at least
/// Model-View-Projection transform for the shader.
pub trait ViewInfo<S, T: cgmath::Transform3<S>>: gfx_phase::ToDepth<Depth = S> {
    /// Construct a new information block.
    fn new(mvp: cgmath::Matrix4<S>, view: T, model: T) -> Self;
}

/// Abstract scene that can be drawn into something.
pub trait AbstractScene<R: gfx::Resources> {
    /// A type of the view information.
    type ViewInfo;
    /// A type of the entity.
    type Entity;
    /// A type of the camera.
    type Camera;

    /// Draw the contents of the scene with a specific phase into a renderer,
    /// using a given camera and a frame.
    fn draw<C: gfx::CommandBuffer<R>, H: gfx_phase::AbstractPhase<R, C, Self::Entity, Self::ViewInfo> + ?Sized>(
            &mut self, &mut H, &Self::Camera, &gfx::Frame<R>, &mut gfx::Renderer<R, C>) -> Result<(), Error>;
}

/// A class that manages spatial relations between objects.
pub trait World {
    /// Type of the scalar used in all associated mathematical constructs.
    type Scalar: cgmath::BaseFloat + 'static;
    /// Type of the transform that every node performs relative to the parent.
    type Transform: cgmath::Transform3<Self::Scalar> + Clone;
    /// Pointer to a node, associated with an entity, camera, or something else.
    type NodePtr;
    /// Pointer to a skeleton, associated with an enttity.
    type SkeletonPtr;
    /// Get the transformation of a specific node pointer.
    fn get_transform(&self, &Self::NodePtr) -> Self::Transform;
}

/// A simple struct representing an object with a given material, mesh, bound,
/// and spatial relation to other stuff in the world.
pub struct Entity<R: gfx::Resources, M, W: World, B> {
    /// Name of the entity.
    pub name: String,
    /// Assotiated material of the entity.
    pub material: M,
    /// Mesh.
    pub mesh: gfx::Mesh<R>,
    /// Mesh slice.
    pub slice: gfx::Slice<R>,
    /// Node pointer into the world.
    pub node: W::NodePtr,
    /// Skeleton pointer.
    pub skeleton: Option<W::SkeletonPtr>,
    /// Associated spatial bound of the entity.
    pub bound: B,
}

impl<R: gfx::Resources, M: gfx_phase::Material, W: World, B> gfx_phase::Entity<R, M> for Entity<R, M, W, B> {
    fn get_material(&self) -> &M {
        &self.material
    }
    fn get_mesh(&self) -> (&gfx::Mesh<R>, &gfx::Slice<R>) {
        (&self.mesh, &self.slice)
    }
}

/// A simple camera with generic projection and spatial relation.
#[derive(Clone, Debug)]
pub struct Camera<P, N> {
    /// Name of the camera.
    pub name: String,
    /// Generic projection.
    pub projection: P,
    /// Generic spatial node.
    pub node: N,
}

/// A helper routine to enqueue an iterator of entities into a phase.
/// It does frustum culling and `ViewInfo` construction.
/// It can be used as a helper for user-side scenes.
pub fn enqueue<'a,
    R: gfx::Resources,
    M: gfx_phase::Material + 'a,
    W: World + 'a,
    B: cgmath::Bound<W::Scalar> + Debug + 'a,
    H: gfx_phase::QueuePhase<R, Entity<R, M, W, B>, V> + ?Sized,
    P: cgmath::Projection<W::Scalar>,
    V: ViewInfo<W::Scalar, W::Transform>,
    I: Iterator<Item = &'a mut Entity<R, M, W, B>>,
>
(   entities: I, phase: &mut H, world: &W, camera: &Camera<P, W::NodePtr>,
    cull_frustum: bool, context: &mut gfx::batch::Context<R>)
    -> Result<(), gfx::batch::Error>
where
    R: 'a,
    R::Buffer: 'a,
    R::ArrayBuffer: 'a,
    R::Shader: 'a,
    R::Program: 'a,
    R::FrameBuffer: 'a,
    R::Surface: 'a,
    R::Texture: 'a,
    R::Sampler: 'a,
    W::Transform: 'a,
    W::NodePtr: 'a,
    W::SkeletonPtr: 'a,
{
    use cgmath::{Matrix, ToMatrix4, Transform};
    let cam_inverse = world.get_transform(&camera.node)
                           .invert().unwrap();
    let projection = camera.projection.to_matrix4()
                           .mul_m(&cam_inverse.to_matrix4());
    for entity in entities {
        if !phase.test(entity) {
            continue
        }
        let model = world.get_transform(&entity.node);
        let view = cam_inverse.concat(&model);
        let mvp = projection.mul_m(&model.to_matrix4());
        if cull_frustum && entity.bound.relate_clip_space(&mvp) == cgmath::Relation::Out {
            continue
        }
        let view_info = ViewInfo::new(mvp, view, model.clone());
        match phase.enqueue(entity, view_info, context) {
            Ok(()) => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// An example scene type.
pub struct Scene<R: gfx::Resources, M, W: World, B, P, V> {
    /// A list of entities in the scene.
    pub entities: Vec<Entity<R, M, W, B>>,
    /// A list of cameras. It's not really useful, but `P` needs to be
    /// constrained in order to be able to implement `AbstractScene`.
    pub cameras: Vec<Camera<P, W::NodePtr>>,
    /// A flag controlling the frustum culling.
    pub cull_frustum: bool,
    /// Spatial world.
    pub world: W,
    /// Sorting criteria vector. The first element has the highest priority. 
    pub sort: Vec<gfx_phase::Sort>,
    context: gfx::batch::Context<R>,
    _view_dummy: PhantomData<V>,
}

impl<R: gfx::Resources, M, W: World, B, P, V> Scene<R, M, W, B, P, V> {
    /// Create a new empty scene.
    pub fn new(world: W) -> Scene<R, M, W, B, P, V> {
        Scene {
            entities: Vec::new(),
            cameras: Vec::new(),
            cull_frustum: true,
            world: world,
            sort: Vec::new(),
            context: gfx::batch::Context::new(),
            _view_dummy: PhantomData,
        }
    }
}

impl<
    R: gfx::Resources,
    M: gfx_phase::Material,
    W: World,
    B: cgmath::Bound<W::Scalar> + Debug,
    P: cgmath::Projection<W::Scalar>,
    V: ViewInfo<W::Scalar, W::Transform>,
> AbstractScene<R> for Scene<R, M, W, B, P, V> {
    type ViewInfo = V;
    type Entity = Entity<R, M, W, B>;
    type Camera = Camera<P, W::NodePtr>;

    fn draw<C: gfx::CommandBuffer<R>, H: gfx_phase::AbstractPhase<R, C, Entity<R, M, W, B>, V> + ?Sized>(
            &mut self, phase: &mut H, camera: &Camera<P, W::NodePtr>,
            frame: &gfx::Frame<R>, renderer: &mut gfx::Renderer<R, C>)
            -> Result<(), Error> {
        // enqueue entities
        match enqueue(self.entities.iter_mut(), phase, &self.world, camera,
                      self.cull_frustum, &mut self.context) {
            Ok(()) => (),
            Err(e) => return Err(Error::Batch(e)),
        };
        // sort the scene
        phase.sort(&self.sort);
        // flush into the renderer
        phase.flush(frame, &mut self.context, renderer)
             .map_err(|e| Error::Flush(e))
    }
}

/// Wrapper around a scene that carries a list of phases as well as the
/// `Renderer`, allowing to isolate a command buffer completely.
pub struct PhaseHarness<D: gfx::Device, C: AbstractScene<D::Resources>> {
    /// Wrapped scene.
    pub scene: C,
    /// Optional clear data.
    pub clear: Option<gfx::ClearData>,
    /// List of phases as trait objects.
    pub phases: Vec<Box<gfx_phase::AbstractPhase<D::Resources, D::CommandBuffer, C::Entity, C::ViewInfo>>>,
    /// Gfx renderer to draw into.
    pub renderer: gfx::Renderer<D::Resources, D::CommandBuffer>,
}

impl<D: gfx::Device, C: AbstractScene<D::Resources>> PhaseHarness<D, C> {
    /// Create a new empty phase harness.
    pub fn new(scene: C, renderer: gfx::Renderer<D::Resources, D::CommandBuffer>)
               -> PhaseHarness<D, C> {
        PhaseHarness {
            scene: scene,
            clear: None,
            phases: Vec::new(),
            renderer: renderer,
        }
    }

    /// Draw the scene into a given frame, using all the phases. 
    pub fn draw(&mut self, camera: &C::Camera, frame: &gfx::Frame<D::Resources>)
                -> Result<gfx::SubmitInfo<D>, Error> {
        use std::ops::DerefMut;
        self.renderer.reset();
        match self.clear {
            Some(data) => self.renderer.clear(data, gfx::COLOR | gfx::DEPTH | gfx::STENCIL, frame),
            None => (),
        }
        for phase in self.phases.iter_mut() {
            match self.scene.draw(phase.deref_mut(), camera, frame, &mut self.renderer) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(self.renderer.as_buffer())
    }
}

/// A simple perspective camera based on the `World` trait.
pub type PerspectiveCam<W: World> = Camera<
    cgmath::PerspectiveFov<W::Scalar, cgmath::Rad<W::Scalar>>,
    W::NodePtr
>;
