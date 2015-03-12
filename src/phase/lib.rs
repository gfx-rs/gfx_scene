extern crate gfx;

mod mem;
mod phase;

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomFn;

pub use self::phase::{FlushError, AbstractPhase, Sort, Phase};

/// Abstract material
pub trait Material: PhantomFn<Self> {}

/// View information that can be transformed into depth
pub trait ToDepth {
    type Depth: PartialOrd;
    fn to_depth(&self) -> Self::Depth;
}

pub type TechResult<'a, R, P> = (
    &'a gfx::ProgramHandle<R>,  // program
    P,                          // parameters
    Option<&'a gfx::Mesh<R>>,   // insancing
    &'a gfx::DrawState,         // state
);

/// Technique is basically a `Fn(Entity) -> Option<TechResult>`
/// It processes a material, checks for the compatibility, adds a mesh
/// to produce a shader program with associated data (state, parameters).
pub trait Technique<R: gfx::Resources, M: Material, V: ToDepth> {
    type Essense: Copy + Debug + Eq + Hash;
    type Params: gfx::shade::ShaderParam<Resources = R>;
    fn test(&self, &gfx::Mesh<R>, &M) -> Option<Self::Essense>;
    fn compile<'a>(&'a self, Self::Essense, V)
                   -> TechResult<'a, R, Self::Params>;
    fn fix_params(&self, &M, &V, &mut Self::Params);
}

/// Abstract Entity
pub trait Entity<R: gfx::Resources, M: Material> {
    fn get_material(&self) -> &M;
    fn get_mesh(&self) -> (&gfx::Mesh<R>, &gfx::Slice<R>);
}
