extern crate draw;
extern crate gfx;
extern crate cgmath;

use cgmath::{BaseFloat, Zero, Matrix3, Matrix4};

//TODO
pub struct Camera<S>(S);

//TODO
pub struct World<S>(S);

#[derive(Debug)]
pub enum DrawError {
    Batch(gfx::batch::BatchError),
    Flush(draw::FlushError),
}

pub trait AbstractScene<D: gfx::Device> {
    type Scalar;
    type Entity;
    type Load;

    fn draw<P: draw::AbstractPhase<D, Self::Load, Self::Entity> + ?Sized>(
            &mut self, &mut P, &Camera<Self::Scalar>, &gfx::Frame<D::Resources>,
            &mut gfx::Renderer<D>) -> Result<(), DrawError>;
}

pub struct Entity<M> {
    material: M,
    mesh: gfx::Mesh,
    slice: gfx::Slice,
}

impl<M> draw::Entity<M> for Entity<M> {
    fn get_material(&self) -> &M {
        &self.material
    }
    fn get_mesh(&self) -> (&gfx::Mesh, gfx::Slice) {
        (&self.mesh, self.slice)
    }
}

pub struct Scene<S, M> {
    pub entities: Vec<Entity<M>>,
    pub world: World<S>,
    context: gfx::batch::Context,
}

pub struct Load<S> {
    depth: S,
    _vertex_mx: Matrix4<S>,
    _normal_mx: Matrix3<S>,
}

impl<S: Copy> draw::ToDepth<S> for Load<S> {
    fn to_depth(&self) -> S {
        self.depth
    }
}

impl<S: BaseFloat, M: draw::Material>
AbstractScene<gfx::GlDevice> for Scene<S, M> {
    type Scalar = S;
    type Entity = Entity<M>;
    type Load = Load<S>;

    fn draw<P: draw::AbstractPhase<gfx::GlDevice, Load<S>, Entity<M>> + ?Sized>(
            &mut self, phase: &mut P, _camera: &Camera<S>,
            frame: &gfx::Frame<gfx::GlResources>,
            renderer: &mut gfx:: Renderer<gfx::GlDevice>)
            -> Result<(), DrawError> {
        for entity in self.entities.iter_mut() {
            if !phase.does_apply(entity) {
                 continue
            }
            //TODO: cull `ent.bounds` here
            //TODO: compute depth here
            let data = Load {
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
    pub renderer: gfx::Renderer<D>,
}

impl<
    D: gfx::Device,
    C: AbstractScene<D>,
    P: draw::AbstractPhase<D, C::Load, C::Entity>
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

pub type StandardScene<D, S, M> = PhaseHarness<
    D, Scene<S, M>,
    Box<draw::AbstractPhase<D, Load<S>, Entity<M>>>
>;
