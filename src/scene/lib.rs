extern crate draw;
extern crate gfx;
extern crate cgmath;

pub mod space;

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
    type WorldInfo;

    fn draw<P: draw::AbstractPhase<D, Self::WorldInfo, Self::Entity> + ?Sized>(
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

pub struct Scene<R: gfx::Resources, S, T, M> {
    pub entities: Vec<Entity<R, M>>,
    pub world: space::World<S, T>,
    context: gfx::batch::Context<R>,
}

pub struct WorldInfo<S> {
    depth: S,
    _vertex_mx: Matrix4<S>,
    _normal_mx: Matrix3<S>,
}

impl<S: Copy + PartialOrd> draw::ToDepth for WorldInfo<S> {
    type Depth = S;
    fn to_depth(&self) -> S {
        self.depth
    }
}

impl<
    D: gfx::Device,
    S: BaseFloat,
    T: Transform3<S>,
    M: draw::Material
> AbstractScene<D> for Scene<D::Resources, S, T, M> {
    type Scalar = S;
    type Entity = Entity<D::Resources, M>;
    type WorldInfo = WorldInfo<S>;

    fn draw<P: draw::AbstractPhase<D, WorldInfo<S>, Entity<D::Resources, M>> + ?Sized>(
            &mut self, phase: &mut P, _camera: &Camera<S>,
            frame: &gfx::Frame<D::Resources>,
            renderer: &mut gfx::Renderer<D::CommandBuffer>)
            -> Result<(), DrawError> {
        for entity in self.entities.iter_mut() {
            if !phase.does_apply(entity) {
                 continue
            }
            //TODO: cull `ent.bounds` here
            //TODO: compute depth here
            let data = WorldInfo {
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

pub struct PhaseHarness<D: gfx::Device, C, P> {
    pub scene: C,
    pub phases: Vec<P>,
    pub renderer: gfx::Renderer<D::CommandBuffer>,
}

impl<
    D: gfx::Device,
    C: AbstractScene<D>,
    P: draw::AbstractPhase<D, C::WorldInfo, C::Entity>
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

/*//TODO: rust bug
pub type StandardScene<D: gfx::Device, S, T, M: draw::Material<D::Resources>> =
PhaseHarness<
    D, Scene<D::Resources, S, T, M>,
    Box<draw::AbstractPhase<D, Load<S>, Entity<D::Resources, M>>>
>;*/
