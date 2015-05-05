use std::marker::PhantomData;
use cgmath;
use gfx;
use gfx_phase;


/// Generic bound culler.
pub trait Culler<S, B: cgmath::Bound<S>> {
    /// Start a new culling session.
    fn init(&mut self);
    /// Cull a bound with a given transformation matrix.
    fn cull(&mut self, &B, &cgmath::Matrix4<S>) -> cgmath::Relation;
}

impl<S, B: cgmath::Bound<S>> Culler<S, B> for () {
    fn init(&mut self) {}
    fn cull(&mut self, _: &B, _: &cgmath::Matrix4<S>) -> cgmath::Relation {
        cgmath::Relation::Cross
    }
}

/// Frustum culler.
pub struct Frustum<S, B>(PhantomData<(S, B)>);

impl<S, B> Frustum<S, B> {
    /// Create a new frustum culler.
    pub fn new() -> Frustum<S, B> {
        Frustum(PhantomData)
    }
}

impl<S: cgmath::BaseFloat, B: cgmath::Bound<S>> Culler<S, B> for Frustum<S, B> {
    fn init(&mut self) {}
    fn cull(&mut self, bound: &B, mvp: &cgmath::Matrix4<S>) -> cgmath::Relation {
        bound.relate_clip_space(mvp)
    }
}


/// Culler context.
pub struct Context<'a, 'c, W, B, C> where
    W: ::World + 'a,
    B: cgmath::Bound<W::Scalar>,
    C: Culler<W::Scalar, B> + 'c,
{
    world: &'a W,
    culler: &'c mut C,
    cam_inverse: W::Transform,
    projection: cgmath::Matrix4<W::Scalar>,
    dummy: PhantomData<B>,
}

impl<'a, 'c,
    W: ::World,
    B: cgmath::Bound<W::Scalar>,
    C: Culler<W::Scalar, B>,
> Context<'a, 'c, W, B, C> {
    /// Create a new context.
    pub fn new<P>(world: &'a W, culler: &'c mut C, camera: &::Camera<P, W::NodePtr>)
               -> Context<'a, 'c, W, B, C> where
        P: cgmath::Projection<W::Scalar>,
    {
        use cgmath::{Matrix, ToMatrix4, Transform};
        let cam_inverse = world.get_transform(&camera.node)
                               .invert().unwrap();
        let projection = camera.projection.to_matrix4()
                               .mul_m(&cam_inverse.to_matrix4());
        culler.init();
        Context {
            world: world,
            culler: culler,
            cam_inverse: cam_inverse,
            projection: projection,
            dummy: PhantomData,
        }
    }

    /// Check entity visibility.
    pub fn is_visible<V>(&mut self, node: &W::NodePtr, bound: &B)
                      -> Option<V> where
        V: ::ViewInfo<W::Scalar, W::Transform>,
    {
        use cgmath::{Matrix, ToMatrix4, Transform};
        let model = self.world.get_transform(node);
        let view = self.cam_inverse.concat(&model);
        let mvp = self.projection.mul_m(&model.to_matrix4());
        if self.culler.cull(bound, &mvp) != cgmath::Relation::Out {
            Some(::ViewInfo::new(mvp, view, model))
        }else {
            None
        }
    }

    /// Cull and draw the entities into a stream.
    pub fn draw<'b, R, M, V, I, H, S>(&mut self, entities: I, phase: &mut H, stream: &mut S)
                -> Result<::Report, ::Error> where
        W: 'b,
        W::Transform: 'b,
        W::NodePtr: 'b,
        W::SkeletonPtr: 'b,
        B: 'b,
        R: gfx::Resources + 'b,
        R::Buffer: 'b,
        R::ArrayBuffer: 'b,
        R::Shader: 'b,
        R::Program: 'b,
        R::FrameBuffer: 'b,
        R::Surface: 'b,
        R::Texture: 'b,
        R::Sampler: 'b,
        M: 'b,
        V: ::ViewInfo<W::Scalar, W::Transform>,
        I: Iterator<Item = &'b ::Entity<R, M, W, B>>,
        H: gfx_phase::AbstractPhase<R, M, V>,
        S: gfx::Stream<R>,
    {
        let mut report = ::Report::new();
        // enqueue entities fragments
        for ent in entities {
            if !ent.visible {
                report.calls_invisible += ent.fragments.len() as u32;
                continue
            }
            if let Some(view_info) = self.is_visible(&ent.node, &ent.bound) {
                for frag in ent.fragments.iter() {
                    match phase.enqueue(&ent.mesh, &frag.slice, &frag.material, &view_info) {
                        Ok(true)  => report.calls_passed += 1,
                        Ok(false) => report.calls_rejected += 1,
                        Err(e)    => return Err(::Error::Batch(e)),
                    }
                }
            }else {
                report.calls_culled += ent.fragments.len() as u32;
            }
        }
        // flush into the renderer
        match phase.flush(stream) {
            Ok(()) => Ok(report),
            Err(e) => Err(::Error::Flush(e)),
        }
    }
}
