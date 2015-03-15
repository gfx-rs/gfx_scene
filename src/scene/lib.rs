#![deny(missing_docs)]

//! Scene infrastructure to be used with Gfx phases.

extern crate "gfx_phase" as phase;
extern crate gfx;
extern crate cgmath;

use std::marker::PhantomData;

/// Scene drawing error.
#[derive(Debug)]
pub enum Error {
    /// Error in creating a batch.
    Batch(gfx::batch::Error),
    /// Error in sending a batch for drawing.
    Flush(phase::FlushError),
}

/// Abstract information about the view. Supposed to containt at least
/// Model-View-Projection transform for the shader.
pub trait ViewInfo<S, T: cgmath::Transform3<S>>: phase::ToDepth<Depth = S> {
    /// Construct a new information block.
    fn new(mvp: cgmath::Matrix4<S>, view: T, model: T) -> Self;
}

/// Abstract scene that can be drawn into something
pub trait AbstractScene<D: gfx::Device> {
    /// A type of the view information.
    type ViewInfo;
    /// A type of the entity.
    type Entity;
    /// A type of the camera.
    type Camera;

    /// Draw the contents of the scene with a specific phase into a renderer,
    /// using a given camera and a frame.
    fn draw<H: phase::AbstractPhase<D, Self::Entity, Self::ViewInfo> + ?Sized>(
            &mut self, &mut H, &Self::Camera, &gfx::Frame<D::Resources>,
            &mut gfx::Renderer<D::Resources, D::CommandBuffer>) -> Result<(), Error>;
}

/// A class that manages spatial relations between objects
pub trait World {
    /// Type of the scalar used in all associated mathematical constructs.
    type Scalar: cgmath::BaseFloat + 'static;
    /// Type of the rotation that can be decomposed from the transform.
    type Rotation: cgmath::Rotation3<Self::Scalar>;
    /// Type of the transform that every node performs relative to the parent.
    type Transform: cgmath::CompositeTransform3<Self::Scalar, Self::Rotation> + Clone;
    /// Pointer to a node, associated with an entity, camera, or something else.
    type NodePtr;
    /// Pointer to a skeleton, associated with an enttity.
    type SkeletonPtr;
    /// Iterator over transformations, used for walking the bones.
    type Iter: Iterator<Item = Self::Transform>;
    /// Get the transfrormation of a specific node pointer.
    fn get_transform(&self, &Self::NodePtr) -> &Self::Transform;
    /// Iterate over the bones of a specific skeleton.
    fn iter_bones(&self, &Self::SkeletonPtr) -> Self::Iter;
}

/// A simple struct representing an object with a given material, mesh, bound,
/// and spatial relation to other stuff in the world.
pub struct Entity<R: gfx::Resources, M, W: World, B> {
    /// Name of the entity.
    pub name: String,
    /// Assotiated material of the entity.
    pub material: M,
    mesh: gfx::Mesh<R>,
    slice: gfx::Slice<R>,
    node: W::NodePtr,
    skeleton: Option<W::SkeletonPtr>,
    /// Associated spatial bound of the entity.
    pub bound: B,
}

impl<R: gfx::Resources, M: phase::Material, W: World, B> phase::Entity<R, M> for Entity<R, M, W, B> {
    fn get_material(&self) -> &M {
        &self.material
    }
    fn get_mesh(&self) -> (&gfx::Mesh<R>, &gfx::Slice<R>) {
        (&self.mesh, &self.slice)
    }
}

/// A simple camera with generic projection and spatial relation.
pub struct Camera<P, N> {
    /// Name of the camera.
    pub name: String,
    /// Generic projection.
    pub projection: P,
    /// Generic spatial node.
    pub node: N,
}

/// A generic `draw()` routine that takes a phase and some entities, and draws them
/// into a given frame. It does frustum culling and `ViewInfo` construction.
/// It can be used as a helper for user-side scenes.
pub fn draw_entities<'a,
    D: gfx::Device,
    M: phase::Material + 'a,
    W: World + 'a,
    B: cgmath::Bound<W::Scalar> + 'a,
    H: phase::AbstractPhase<D, Entity<D::Resources, M, W, B>, V> + ?Sized,
    P: cgmath::Projection<W::Scalar>,
    V: ViewInfo<W::Scalar, W::Transform>,
    I: Iterator<Item = &'a mut Entity<D::Resources, M, W, B>>,
>
(   entities: I, phase: &mut H, world: &W, camera: &Camera<P, W::NodePtr>,
    frame: &gfx::Frame<D::Resources>, context: &mut gfx::batch::Context<D::Resources>,
    renderer: &mut gfx::Renderer<D::Resources, D::CommandBuffer>)
    -> Result<(), Error>
where
    D::Resources: 'a,
    <D::Resources as gfx::Resources>::Buffer: 'a,
    <D::Resources as gfx::Resources>::ArrayBuffer: 'a,
    <D::Resources as gfx::Resources>::Shader: 'a,
    <D::Resources as gfx::Resources>::Program: 'a,
    <D::Resources as gfx::Resources>::FrameBuffer: 'a,
    <D::Resources as gfx::Resources>::Surface: 'a,
    <D::Resources as gfx::Resources>::Texture: 'a,
    <D::Resources as gfx::Resources>::Sampler: 'a,
    W::Rotation: 'a,
    W::Transform: 'a,
    W::NodePtr: 'a,
    W::SkeletonPtr: 'a,
    W::Iter: 'a,
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
        if entity.bound.relate_clip_space(&mvp) == cgmath::Relation::Out {
            continue
        }
        let view_info = ViewInfo::new(mvp, view, model.clone());
        match phase.enqueue(entity, view_info, context) {
            Ok(()) => (),
            Err(e) => return Err(Error::Batch(e)),
        }
    }
    phase.flush(frame, context, renderer)
         .map_err(|e| Error::Flush(e))
}

/// An example scene type.
pub struct Scene<R: gfx::Resources, M, W: World, B, P, V> {
    /// A list of entities in the scene.
    pub entities: Vec<Entity<R, M, W, B>>,
    /// A list of cameras.
    pub cameras: Vec<Camera<P, W::NodePtr>>,
    /// Spatial world.
    pub world: W,
    context: gfx::batch::Context<R>,
    _view_dummy: PhantomData<V>,
}

impl<
    D: gfx::Device,
    M: phase::Material,
    W: World,
    B: cgmath::Bound<W::Scalar>,
    P: cgmath::Projection<W::Scalar>,
    V: ViewInfo<W::Scalar, W::Transform>,
> AbstractScene<D> for Scene<D::Resources, M, W, B, P, V> {
    type ViewInfo = V;
    type Entity = Entity<D::Resources, M, W, B>;
    type Camera = Camera<P, W::NodePtr>;

    fn draw<H: phase::AbstractPhase<D, Entity<D::Resources, M, W, B>, V> + ?Sized>(
            &mut self, phase: &mut H, camera: &Camera<P, W::NodePtr>,
            frame: &gfx::Frame<D::Resources>,
            renderer: &mut gfx::Renderer<D::Resources, D::CommandBuffer>)
            -> Result<(), Error> {
        draw_entities(self.entities.iter_mut(), phase, &self.world, camera,
                     frame, &mut self.context, renderer)
    }
}

/// Wrapper around a scene that carries a list of phases as well as the
/// `Renderer`, allowing to isolate a command buffer completely.
pub struct PhaseHarness<D: gfx::Device, C: AbstractScene<D>> {
    /// Wrapped scene.
    pub scene: C,
    /// List of phases as trait objects.
    pub phases: Vec<Box<phase::AbstractPhase<D, C::Entity, C::ViewInfo>>>,
    /// Gfx renderer to draw into.
    pub renderer: gfx::Renderer<D::Resources, D::CommandBuffer>,
}

impl<
    D: gfx::Device,
    C: AbstractScene<D>,
> PhaseHarness<D, C> {
    /// Draw the scene into a given frame, using all the phases. 
    pub fn draw(&mut self, camera: &C::Camera, frame: &gfx::Frame<D::Resources>)
                -> Result<(), Error> {
        use std::ops::DerefMut;
        self.renderer.reset();
        for phase in self.phases.iter_mut() {
            match self.scene.draw(phase.deref_mut(), camera, frame, &mut self.renderer) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(()) //TODO: return a command buffer?
    }
}

/// A simple perspective camera based on the `World` trait.
pub type PerspectiveCam<W: World> = Camera<
    cgmath::PerspectiveFov<W::Scalar, cgmath::Rad<W::Scalar>>,
    W::NodePtr
>;
