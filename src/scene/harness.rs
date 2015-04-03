use gfx;
use gfx_phase;


/// Wrapper around a scene that carries a list of phases as well as the
/// `Renderer`, allowing to isolate a command buffer completely.
pub struct PhaseHarness<D: gfx::Device, C: ::AbstractScene<D::Resources>> {
    /// Wrapped scene.
    pub scene: C,
    /// Optional clear data.
    pub clear: Option<gfx::ClearData>,
    /// List of phases as trait objects.
    pub phases: Vec<Box<gfx_phase::AbstractPhase<D::Resources, D::CommandBuffer, C::Entity, C::ViewInfo>>>,
    /// Gfx renderer to draw into.
    pub renderer: gfx::Renderer<D::Resources, D::CommandBuffer>,
}

impl<D: gfx::Device, C: ::AbstractScene<D::Resources>> PhaseHarness<D, C> {
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
                -> Result<gfx::SubmitInfo<D>, ::Error> {
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