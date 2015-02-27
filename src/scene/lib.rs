extern crate draw;
extern crate gfx;
extern crate cgmath;

use cgmath::{BaseFloat, Zero, Matrix3, Matrix4, Transform3};

#[derive(Debug)]
pub enum DrawError {
    Batch(gfx::batch::BatchError),
    Flush(draw::FlushError),
}

pub trait AbstractScene<D: gfx::Device> {
    type SpaceData;
    type Entity;
    type Camera;

    fn draw<P: draw::AbstractPhase<D, Self::SpaceData, Self::Entity> + ?Sized>(
            &mut self, &mut P, &Self::Camera, &gfx::Frame<D::Resources>,
            &mut gfx::Renderer<D::CommandBuffer>) -> Result<(), DrawError>;
}

/// A class that manages spatial relations between objects
pub trait World {
    type Scalar: cgmath::BaseFloat;
    type Transform: cgmath::Transform3<Self::Scalar>;
    type NodePtr;
    type SkeletonPtr;
    type Iter: Iterator<Item = Self::Transform>;

    fn get_transform(&self, Self::NodePtr) -> &Self::Transform;
    fn iter_bones(&self, Self::SkeletonPtr) -> Self::Iter;
}

pub struct Entity<R: gfx::Resources, M, W: World> {
    pub name: String,
    pub material: M,
    mesh: gfx::Mesh<R>,
    slice: gfx::Slice<R>,
    node: W::NodePtr,
    skeleton: Option<W::SkeletonPtr>,
}

impl<R: gfx::Resources, M, W: World> draw::Entity<R, M> for Entity<R, M, W> {
    fn get_material(&self) -> &M {
        &self.material
    }
    fn get_mesh(&self) -> (&gfx::Mesh<R>, gfx::Slice<R>) {
        (&self.mesh, self.slice)
    }
}

pub struct Camera<P, N> {
    pub name: String,
    pub projection: P,
    pub node: N,
}

pub struct Scene<R: gfx::Resources, M, W: World, P> {
    pub entities: Vec<Entity<R, M, W>>,
    pub cameras: Vec<Camera<P, W::NodePtr>>,
    pub world: W,
    context: gfx::batch::Context<R>,
}

pub struct SpaceData<S> {
    depth: S,
    vertex_mx: Matrix4<S>,
    normal_mx: Matrix3<S>,
}

impl<S: Copy + PartialOrd> draw::ToDepth for SpaceData<S> {
    type Depth = S;
    fn to_depth(&self) -> S {
        self.depth
    }
}

impl<
    D: gfx::Device,
    M: draw::Material,
    W: World,
    P: cgmath::Projection<W::Scalar>,
> AbstractScene<D> for Scene<D::Resources, M, W, P> {
    type SpaceData = SpaceData<W::Scalar>;
    type Entity = Entity<D::Resources, M, W>;
    type Camera = Camera<P, W::NodePtr>;

    fn draw<H: draw::AbstractPhase<D, SpaceData<W::Scalar>, Entity<D::Resources, M, W>> + ?Sized>(
            &mut self, phase: &mut H, _camera: &Camera<P, W::NodePtr>,
            frame: &gfx::Frame<D::Resources>,
            renderer: &mut gfx::Renderer<D::CommandBuffer>)
            -> Result<(), DrawError> {
        for entity in self.entities.iter_mut() {
            if !phase.does_apply(entity) {
                 continue
            }
            //TODO: cull `ent.bounds` here
            //TODO: compute depth here
            let data = SpaceData {
                depth: Zero::zero(),
                vertex_mx: Matrix4::identity(),
                normal_mx: Matrix3::identity(),
            };
            match phase.enqueue(entity, data, &mut self.context) {
                Ok(()) => (),
                Err(e) => return Err(DrawError::Batch(e)),
            }
        }
        phase.flush(frame, &self.context, renderer)
             .map_err(|e| DrawError::Flush(e))
    }
}

/// Wrapper around a scene that carries a list of phases as well as the
/// `Renderer`, allowing to isolate a command buffer completely.
pub struct PhaseHarness<D: gfx::Device, C, P> {
    pub scene: C,
    pub phases: Vec<P>,
    pub renderer: gfx::Renderer<D::CommandBuffer>,
}

impl<
    D: gfx::Device,
    C: AbstractScene<D>,
    H: draw::AbstractPhase<D, C::SpaceData, C::Entity>
> PhaseHarness<D, C, H> {
    pub fn draw(&mut self, camera: &C::Camera, frame: &gfx::Frame<D::Resources>)
                -> Result<(), DrawError> {
        self.renderer.reset();
        for phase in self.phases.iter_mut() {
            match self.scene.draw(phase, camera, frame, &mut self.renderer) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

pub type PerspectiveCam<W: World> = Camera<
    cgmath::PerspectiveFov<W::Scalar, cgmath::Rad<W::Scalar>>,
    W::NodePtr
>;

/// A typical scene to be used in demoes. Has a heterogeneous vector of phases
/// and a perspective fov-based camera.
pub type StandardScene<
    D: gfx::Device,
    M: draw::Material,
    W: World,
    P: cgmath::Projection<W::Scalar>,
> = PhaseHarness<D,
    Scene<D::Resources, M, W, P>,
    Box<draw::AbstractPhase<D, SpaceData<W::Scalar>, Entity<D::Resources, M, W>>>
>;
