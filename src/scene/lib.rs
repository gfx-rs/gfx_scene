#![deny(missing_docs)]

//! Scene infrastructure to be used with Gfx phases.

extern crate gfx_phase;
extern crate gfx;
extern crate cgmath;

use std::fmt::Debug;
use std::marker::PhantomData;

mod cull;

pub use self::cull::{Culler, CullPhase, Frustum};


/// Scene drawing error.
#[derive(Debug)]
pub enum Error {
    /// Error in creating a batch.
    Batch(gfx::batch::Error),
    /// Error in sending a batch for drawing.
    Flush(gfx_phase::FlushError),
}

/// Number of enitites that failed to enqueue.
pub type FailCount = usize;

/// Abstract scene that can be drawn into something.
pub trait AbstractScene<R: gfx::Resources> {
    /// A type of the view information.
    type ViewInfo;
    /// A type of the entity.
    type Entity;
    /// A type of the camera.
    type Camera;

    /// Draw the contents of the scene with a specific phase into a stream.
    fn draw<H, S>(&self, &mut H, &Self::Camera, &mut S)
            -> Result<FailCount, Error> where
        H: gfx_phase::AbstractPhase<R, Self::Entity, Self::ViewInfo>,
        S: gfx::Stream<R>;
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

impl<
    S: cgmath::BaseFloat + 'static,
    T: cgmath::ToMatrix4<S> + cgmath::Transform3<S> + Clone,
    W: World<Scalar = S, Transform = T>,
    P: cgmath::ToMatrix4<S>
> Camera<P, W::NodePtr> {
    /// Get the view-projection matrix, given the `World`.
    pub fn get_view_projection(&self, world: &W) -> cgmath::Matrix4<S> {
        use cgmath::{Matrix, Transform};
        let node_inverse = world.get_transform(&self.node).invert().unwrap();
        self.projection.to_matrix4().mul_m(&node_inverse.to_matrix4())
    }
}

/// Abstract information about the view. Supposed to containt at least
/// Model-View-Projection transform for the shader.
pub trait ViewInfo<S, T: cgmath::Transform3<S>>: gfx_phase::ToDepth<Depth = S> {
    /// Construct a new information block.
    fn new(mvp: cgmath::Matrix4<S>, view: T, model: T) -> Self;
}

/// An example scene type.
pub struct Scene<R: gfx::Resources, M, W: World, B, P, V> {
    /// A list of entities in the scene.
    pub entities: Vec<Entity<R, M, W, B>>,
    /// A list of cameras. It's not really useful, but `P` needs to be
    /// constrained in order to be able to implement `AbstractScene`.
    pub cameras: Vec<Camera<P, W::NodePtr>>,
    /// Spatial world.
    pub world: W,
    _view_dummy: PhantomData<V>,
}

impl<R: gfx::Resources, M, W: World, B, P, V> Scene<R, M, W, B, P, V> {
    /// Create a new empty scene.
    pub fn new(world: W) -> Scene<R, M, W, B, P, V> {
        Scene {
            entities: Vec::new(),
            cameras: Vec::new(),
            world: world,
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

    fn draw<H, S>(&self, phase: &mut H, camera: &Camera<P, W::NodePtr>,
            stream: &mut S) -> Result<FailCount, Error> where
        H: gfx_phase::AbstractPhase<R, Entity<R, M, W, B>, V>,
        S: gfx::Stream<R>,
    {
        // enqueue entities
        let num_fail = match phase.enqueue_all(self.entities.iter(), &self.world, camera) {
            Ok(num) => num,
            Err(e) => return Err(Error::Batch(e)),
        };
        // flush into the renderer
        match phase.flush(stream) {
            Ok(()) => Ok(num_fail),
            Err(e) => Err(Error::Flush(e)),
        }
    }
}

/// A simple perspective camera based on the `World` trait.
pub type PerspectiveCam<W: World> = Camera<
    cgmath::PerspectiveFov<W::Scalar, cgmath::Rad<W::Scalar>>,
    W::NodePtr
>;
