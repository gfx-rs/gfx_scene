extern crate "gfx_phase" as phase;
extern crate gfx;
extern crate cgmath;

use std::marker::PhantomData;

#[derive(Debug)]
pub enum Error {
    Batch(gfx::batch::Error),
    Flush(phase::FlushError),
}

pub trait ViewInfo<S, T: cgmath::Transform3<S>>: phase::ToDepth<Depth = S> {
    fn new(mvp: cgmath::Matrix4<S>, view: T, model: T) -> Self;
}

/// Abstract scene that can be drawn into something
pub trait AbstractScene<D: gfx::Device> {
    type ViewInfo;
    type Entity;
    type Camera;

    fn draw<H: phase::AbstractPhase<D, Self::Entity, Self::ViewInfo> + ?Sized>(
            &mut self, &mut H, &Self::Camera, &gfx::Frame<D::Resources>,
            &mut gfx::Renderer<D::Resources, D::CommandBuffer>) -> Result<(), Error>;
}

/// A class that manages spatial relations between objects
pub trait World {
    type Scalar: cgmath::BaseFloat + 'static;
    type Rotation: cgmath::Rotation3<Self::Scalar>;
    type Transform: cgmath::CompositeTransform3<Self::Scalar, Self::Rotation> + Clone;
    type NodePtr;
    type SkeletonPtr;
    type Iter: Iterator<Item = Self::Transform>;

    fn get_transform(&self, &Self::NodePtr) -> &Self::Transform;
    fn iter_bones(&self, &Self::SkeletonPtr) -> Self::Iter;
}

pub struct Entity<R: gfx::Resources, M, W: World, B> {
    pub name: String,
    pub material: M,
    mesh: gfx::Mesh<R>,
    slice: gfx::Slice<R>,
    node: W::NodePtr,
    skeleton: Option<W::SkeletonPtr>,
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

pub struct Camera<P, N> {
    pub name: String,
    pub projection: P,
    pub node: N,
}

pub struct Scene<R: gfx::Resources, M, W: World, B, P, V> {
    pub entities: Vec<Entity<R, M, W, B>>,
    pub cameras: Vec<Camera<P, W::NodePtr>>,
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
        use cgmath::{Matrix, ToMatrix4, Transform};
        let cam_inverse = self.world.get_transform(&camera.node)
                                    .invert().unwrap();
        let projection = camera.projection.to_matrix4()
                               .mul_m(&cam_inverse.to_matrix4());
        for entity in self.entities.iter_mut() {
            if !phase.test(entity) {
                continue
            }
            let model = self.world.get_transform(&entity.node);
            let view = cam_inverse.concat(&model);
            let mvp = projection.mul_m(&model.to_matrix4());
            if entity.bound.relate_clip_space(&mvp) == cgmath::Relation::Out {
                continue
            }
            let view_info = ViewInfo::new(mvp, view, model.clone());
            match phase.enqueue(entity, view_info, &mut self.context) {
                Ok(()) => (),
                Err(e) => return Err(Error::Batch(e)),
            }
        }
        phase.flush(frame, &self.context, renderer)
             .map_err(|e| Error::Flush(e))
    }
}

/// Wrapper around a scene that carries a list of phases as well as the
/// `Renderer`, allowing to isolate a command buffer completely.
pub struct PhaseHarness<D: gfx::Device, C: AbstractScene<D>> {
    pub scene: C,
    pub phases: Vec<Box<phase::AbstractPhase<D, C::Entity, C::ViewInfo>>>,
    pub renderer: gfx::Renderer<D::Resources, D::CommandBuffer>,
}

impl<
    D: gfx::Device,
    C: AbstractScene<D>,
> PhaseHarness<D, C> {
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
        Ok(())
    }
}

pub type PerspectiveCam<W: World> = Camera<
    cgmath::PerspectiveFov<W::Scalar, cgmath::Rad<W::Scalar>>,
    W::NodePtr
>;
