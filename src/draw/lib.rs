extern crate gfx;

mod phase;

pub use self::phase::{FlushError, AbstractPhase, Sort, ToDepth, Phase};

//TODO: generalize
pub type Program = gfx::ProgramHandle<gfx::GlResources>;

/// Abstract material
pub trait Material {
    type Params: gfx::shade::ShaderParam;
    fn get_params(&self) -> Self::Params;
}

pub type TechResult<'a, P> = (&'a Program, &'a gfx::DrawState, P);

pub trait Technique<Z, M> {
    type Params: gfx::shade::ShaderParam;
    fn does_apply(&self, &M, &gfx::Mesh) -> bool;
    fn compile<'a>(&'a self, &M, &gfx::Mesh, Z) -> TechResult<'a, Self::Params>;
    fn fix_params(&self, &Z, &mut Self::Params);
}

pub trait Entity<M> {
    fn get_material(&self) -> &M;
    fn get_mesh(&self) -> (&gfx::Mesh, gfx::Slice);
}
