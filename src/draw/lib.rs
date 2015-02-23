extern crate gfx;

mod phase;

pub use self::phase::{FlushError, AbstractPhase, Sort, ToDepth, Phase};

/// Abstract material
/// The only thing that we need from it is `ShaderParam`.
pub trait Material {
    type Params: gfx::shade::ShaderParam;
    fn get_params(&self) -> Self::Params;
}

pub type TechResult<'a, P> = (
    &'a gfx::ProgramHandle<gfx::GlResources>,
    &'a gfx::DrawState, P
);

/// Technique is basically a `Fn(Entity) -> Option<TechResult>`
/// It processes a material, checks for the compatibility, adds a mesh
/// to produce a shader program with associated data (state, parameters).
pub trait Technique<Z, M> {
    type Params: gfx::shade::ShaderParam;
    fn does_apply(&self, &M, &gfx::Mesh) -> bool;
    fn compile<'a>(&'a self, &M, &gfx::Mesh, Z) -> TechResult<'a, Self::Params>;
    fn fix_params(&self, &Z, &mut Self::Params);
}

/// Abstract Entity
pub trait Entity<M> {
    fn get_material(&self) -> &M;
    fn get_mesh(&self) -> (&gfx::Mesh, gfx::Slice);
}
