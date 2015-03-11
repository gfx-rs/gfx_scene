extern crate gfx;

mod phase;

use std::marker::PhantomFn;
pub use self::phase::{FlushError, AbstractPhase, Sort, ToDepth, Phase};

/// Abstract material
pub trait Material: PhantomFn<Self> {}

pub type TechResult<'a, R, P> = (
    &'a gfx::ProgramHandle<R>,  // program
    P,                          // parameters
    Option<&'a gfx::Mesh<R>>,   // insancing
    &'a gfx::DrawState,         // state
);

/// Technique is basically a `Fn(Entity) -> Option<TechResult>`
/// It processes a material, checks for the compatibility, adds a mesh
/// to produce a shader program with associated data (state, parameters).
pub trait Technique<R: gfx::Resources, M, Z> {
    type Params: gfx::shade::ShaderParam<Resources = R>;
    fn does_apply(&self, &gfx::Mesh<R>, &M) -> bool;
    fn compile<'a>(&'a self, &gfx::Mesh<R>, &M, Z)
                   -> TechResult<'a, R, Self::Params>;
    fn fix_params(&self, &M, &Z, &mut Self::Params);
}

/// Abstract Entity
pub trait Entity<R, M> {
    fn get_material(&self) -> &M;
    fn get_mesh(&self) -> (&gfx::Mesh<R>, &gfx::Slice<R>);
}
