extern crate draw;
extern crate gfx;
extern crate cgmath;

use cgmath::{BaseFloat, Zero, Matrix3, Matrix4, Transform3};

//TODO
pub struct Camera<S>(S);

#[derive(Debug)]
pub enum DrawError {
    Batch(gfx::batch::BatchError),
    Flush(draw::FlushError),
}

pub trait AbstractScene<D: gfx::Device> {
    type Scalar;
    type Entity;
    type SpaceData;

    fn draw<P: draw::AbstractPhase<D, Self::SpaceData, Self::Entity> + ?Sized>(
            &mut self, &mut P, &Camera<Self::Scalar>, &gfx::Frame<D::Resources>,
            &mut gfx::Renderer<D::CommandBuffer>) -> Result<(), DrawError>;
}

pub struct Entity<R: gfx::Resources, M> {
    material: M,
    mesh: gfx::Mesh<R>,
    slice: gfx::Slice<R>,
}

impl<R: gfx::Resources, M> draw::Entity<R, M> for Entity<R, M> {
    fn get_material(&self) -> &M {
        &self.material
    }
    fn get_mesh(&self) -> (&gfx::Mesh<R>, gfx::Slice<R>) {
        (&self.mesh, self.slice)
    }
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

pub struct Scene<R: gfx::Resources, M, W> {
    pub entities: Vec<Entity<R, M>>,
    pub world: W,
    context: gfx::batch::Context<R>,
}

pub struct SpaceData<S> {
    depth: S,
    _vertex_mx: Matrix4<S>,
    _normal_mx: Matrix3<S>,
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
> AbstractScene<D> for Scene<D::Resources, M, W> {
    type Scalar = W::Scalar;
    type Entity = Entity<D::Resources, M>;
    type SpaceData = SpaceData<W::Scalar>;

    fn draw<P: draw::AbstractPhase<D, SpaceData<W::Scalar>, Entity<D::Resources, M>> + ?Sized>(
            &mut self, phase: &mut P, _camera: &Camera<W::Scalar>,
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
                _vertex_mx: Matrix4::identity(),
                _normal_mx: Matrix3::identity(),
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
    P: draw::AbstractPhase<D, C::SpaceData, C::Entity>
> PhaseHarness<D, C, P> {
    pub fn draw(&mut self, camera: &Camera<C::Scalar>,
                frame: &gfx::Frame<D::Resources>) -> Result<(), DrawError> {
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

/// A typical scene to be used in demoes. Has a heterogeneous vector of phases.
pub type StandardScene<
    D: gfx::Device,
    M: draw::Material,
    W: World,
> = PhaseHarness<
    D, Scene<D::Resources, M, W>,
    Box<draw::AbstractPhase<D, SpaceData<W::Scalar>, Entity<D::Resources, M>>>
>;
