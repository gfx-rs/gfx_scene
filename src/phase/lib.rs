#![deny(missing_docs)]

//! High-level rendering concepts for Gfx. Allow user code to work with
//! materials, entities, and techniques, instead of batches.

#[macro_use]
extern crate log;
extern crate gfx;
extern crate draw_queue;

mod mem;
mod phase;

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomFn;

pub use self::phase::{Sort, Object, FlushError, QueuePhase, FlushPhase,
                      Ordered, AbstractPhase, Phase, CachedPhase};

/// Abstract material.
pub trait Material: PhantomFn<Self> {}

/// View information that can be transformed into depth.
pub trait ToDepth {
    /// The type of the depth to convert to.
    type Depth: Copy + Debug + PartialOrd;
    /// Convert to depth.
    fn to_depth(&self) -> Self::Depth;
}

/// Resulting type of the technique compilation.
pub type TechResult<'a, R, P> = (
    &'a gfx::ProgramHandle<R>,  // program
    P,                          // parameters
    Option<&'a gfx::Mesh<R>>,   // instancing
    &'a gfx::DrawState,         // state
);

/// Technique is basically a `Fn(Entity) -> Option<TechResult>`.
/// It processes a material, checks for the compatibility, adds a mesh
/// to produce a shader program with associated data (state, parameters).
pub trait Technique<R: gfx::Resources, M: Material, V: ToDepth> {
    /// The most important part of the entity, which is enough to decide
    /// which program or state to use on it.
    type Kernel: Copy + Debug + Eq + Hash;
    /// Associated shader parameters.
    type Params: gfx::shade::ShaderParam<Resources = R>;
    /// Test if this mesh/material can be drawn using the technique.
    fn test(&self, &gfx::Mesh<R>, &M) -> Option<Self::Kernel>;
    /// Compile a given kernel by producing a program, parameter block,
    /// a draw state, and an optional instancing data.
    fn compile<'a>(&'a self, Self::Kernel, V)
                   -> TechResult<'a, R, Self::Params>;
    /// Fix the shader parameters, using an updated material and view info.
    fn fix_params(&self, &M, &V, &mut Self::Params);
}

/// Abstract entity.
pub trait Entity<R: gfx::Resources, M: Material> {
    /// Obtain an associated material.
    fn get_material(&self) -> &M;
    /// Obtain an associated mesh.
    fn get_mesh(&self) -> (&gfx::Mesh<R>, &gfx::Slice<R>);
}
