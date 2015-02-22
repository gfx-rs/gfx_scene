extern crate draw;
extern crate gfx;
extern crate cgmath;

use cgmath::{BaseFloat, Zero, Matrix3, Matrix4};

//TODO: generalize
pub type Renderer = gfx::Renderer<gfx::GlDevice>;

//TODO
pub struct Camera<S>(S);
impl<S: Copy> Camera<S> {
    fn get_s(&self) -> S {
         self.0
    }
}

//TODO
pub struct World;

#[derive(Debug)]
pub enum DrawError {
    Batch(gfx::batch::BatchError),
    Flush(draw::FlushError),
}

pub trait AbstractScene<S, Z, E> {
    fn draw<P: draw::AbstractPhase<Z, E> + ?Sized>(&mut self, &mut P,
            &Camera<S>, &gfx::Frame, &mut Renderer) -> Result<(), DrawError>;
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

pub struct Scene<M> {
    pub entities: Vec<Entity<M>>,
    pub world: World,
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
AbstractScene<S, Load<S>, Entity<M>> for Scene<M> {
    fn draw<P: draw::AbstractPhase<Load<S>, Entity<M>> + ?Sized>(&mut self,
            phase: &mut P, _camera: &Camera<S>, frame: &gfx::Frame,
            renderer: &mut Renderer) -> Result<(), DrawError> {
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

pub struct PhaseHarness<Z, E, C> {
    pub scene: C,
    pub phases: Vec<Box<draw::AbstractPhase<Z, E>>>,
    renderer: Renderer,
}

//impl<S, Z, E, C: AbstractScene<S, Z, E>> PhaseHarness<Z, E, C> {
impl<S, E, C: AbstractScene<S, Load<S>, E>> PhaseHarness<Load<S>, E, C> {
    pub fn draw(&mut self, camera: &Camera<S>, frame: &gfx::Frame)
                -> Result<(), DrawError> {
        self.renderer.reset();
        for phase in self.phases.iter_mut() {
            match self.scene.draw(&mut **phase, camera, frame, &mut self.renderer) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

pub type StandardScene<S, M> = PhaseHarness<S, M, Scene<M>>;
