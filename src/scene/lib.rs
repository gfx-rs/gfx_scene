#![deny(missing_docs)]

//! Scene infrastructure to be used with Gfx phases.

extern crate gfx_phase;
extern crate gfx;
extern crate cgmath;

mod cull;

pub use self::cull::{Culler, Frustum, Context};

/// Scene drawing error.
#[derive(Debug)]
pub enum Error {
    /// Error in creating a batch.
    Batch(gfx::batch::Error),
    /// Error in sending a batch for drawing.
    Flush(gfx_phase::FlushError),
}

/// Type of the call counter.
pub type Count = u32;

/// Rendering success report.
#[derive(Clone, Debug)]
pub struct Report {
    /// Number of calls in invisible entities.
    pub calls_invisible: Count,
    /// Number of calls that got culled out.
    pub calls_culled: Count,
    /// Number of calls that the phase doesn't apply to.
    pub calls_rejected: Count,
    /// Number of calls that failed to link batches.
    pub calls_failed: Count,
    /// Number of calls issued to the GPU.
    pub calls_passed: Count,
    /// Number of primitives rendered.
    pub primitives_rendered: Count,
}

impl Report {
    /// Create an empty `Report`.
    pub fn new() -> Report {
        Report {
            calls_rejected: 0,
            calls_failed: 0,
            calls_culled: 0,
            calls_invisible: 0,
            calls_passed: 0,
            primitives_rendered: 0,
        }
    }

    /// Get total number of draw calls.
    pub fn get_calls_total(&self) -> Count {
        self.calls_invisible + self.calls_culled +
        self.calls_rejected  + self.calls_failed +
        self.calls_passed
    }

    /// Get the rendered/submitted calls ratio.
    pub fn get_calls_ratio(&self) -> f32 {
        self.calls_passed as f32 / self.get_calls_total() as f32
    }
}

/// Abstract scene that can be drawn into something.
pub trait AbstractScene<R: gfx::Resources> {
    /// A type of the view information.
    type ViewInfo;
    /// A type of the material.
    type Material;
    /// A type of the camera.
    type Camera;
    /// the status information from the render results
    /// this can be used to communicate meta from the render
    type Status;

    /// Draw the contents of the scene with a specific phase into a stream.
    fn draw<H, S>(&self, &mut H, &Self::Camera, &mut S)
            -> Result<Self::Status, Error> where
        H: gfx_phase::AbstractPhase<R, Self::Material, Self::ViewInfo>,
        S: gfx::Stream<R>;
}

/// A fragment of an entity, contains a single draw call.
#[derive(Clone, Debug)]
pub struct Fragment<R: gfx::Resources, M> {
    /// Fragment material.
    pub material: M,
    /// Mesh slice.
    pub slice: gfx::Slice<R>,
}

impl<R: gfx::Resources, M> Fragment<R, M> {
    /// Create a new fragment.
    pub fn new(mat: M, slice: gfx::Slice<R>) -> Fragment<R, M> {
        Fragment {
            material: mat,
            slice: slice,
        }
    }
}

/// An abstract node in space.
pub trait Node {
    /// Associated abstract transformation (affine matrix, decomposed, etc).
    type Transform;
    /// Get local -> world transform.
    fn get_transform(&self) -> Self::Transform;
}

/// An abstract entity.
pub trait Entity<R: gfx::Resources, M>: Node {
    /// Type of the spatial bound (box, sphere, etc).
    type Bound;
    /// Check if it's visible.
    fn is_visible(&self) -> bool { true }
    /// Get the local bound.
    fn get_bound(&self) -> Self::Bound;
    /// Get the mesh.
    fn get_mesh(&self) -> &gfx::Mesh<R>;
    /// Get the drawable fragments of this entity.
    fn get_fragments(&self) -> &[Fragment<R, M>];
}

/// An abstract camera.
pub trait Camera<S>: Node {
    /// Associated projection type (perspective, ortho, etc)
    type Projection: cgmath::Projection<S>;
    /// Get the projection.
    fn get_projection(&self) -> Self::Projection;
    /// Compute the view-projection matrix.
    fn get_view_projection(&self) -> cgmath::Matrix4<S> where
        S: cgmath::BaseFloat,
        Self::Transform : cgmath::Transform3<S>
    {
        use cgmath::{Matrix, Transform};
        let view = self.get_transform().invert().unwrap();
        self.get_projection().into().mul_m(&view.into())
    }
}

/// Abstract information about the view. Supposed to containt at least
/// Model-View-Projection transform for the shader.
pub trait ViewInfo<S, T>: gfx_phase::ToDepth<Depth = S> {
    /// Construct a new information block.
    fn new(mvp: cgmath::Matrix4<S>, view: T, model: T) -> Self;
}
