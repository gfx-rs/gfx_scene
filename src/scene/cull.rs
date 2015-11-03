use std::marker::PhantomData;
use cgmath;
use collision;
use gfx;
use gfx_phase;
use hprof;


/// Generic bound culler.
pub trait Culler<S, B: collision::Bound<S>> {
    /// Start a new culling session.
    fn init(&mut self);
    /// Cull a bound with a given transformation matrix.
    fn cull(&mut self, &B, &cgmath::Matrix4<S>) -> collision::Relation;
}

impl<S, B: collision::Bound<S>> Culler<S, B> for () {
    fn init(&mut self) {}
    fn cull(&mut self, _: &B, _: &cgmath::Matrix4<S>) -> collision::Relation {
        collision::Relation::Cross
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

impl<S: cgmath::BaseFloat, B: collision::Bound<S>> Culler<S, B> for Frustum<S, B> {
    fn init(&mut self) {}
    fn cull(&mut self, bound: &B, mvp: &cgmath::Matrix4<S>) -> collision::Relation {
        bound.relate_clip_space(mvp)
    }
}


/// Culler context.
pub struct Context<'u, S, B, T, U> where
    B: collision::Bound<S>,
    U: Culler<S, B> + 'u,
{
    culler: &'u mut U,
    cam_inverse: T,
    view_projection: cgmath::Matrix4<S>,
    dummy: PhantomData<B>,
}

impl<'u,
    S: cgmath::BaseFloat,
    B: collision::Bound<S>,
    T: cgmath::Transform3<S> + Clone,
    U: Culler<S, B>,
> Context<'u, S, B, T, U> {
    /// Create a new context.
    pub fn new<C>(culler: &'u mut U, camera: &C) -> Context<'u, S, B, T, U> where
        C: ::Camera<S, Transform = T>,
    {
        use cgmath::{Matrix, Transform};
        let cam_inverse = camera.get_transform().invert().unwrap();
        let mx_proj: cgmath::Matrix4<S> = camera.get_projection().into();
        let mx_view_proj = mx_proj.mul_m(&cam_inverse.clone().into());
        culler.init();
        Context {
            culler: culler,
            cam_inverse: cam_inverse,
            view_projection: mx_view_proj,
            dummy: PhantomData
        }
    }

    /// Check entity visibility.
    pub fn is_visible<N, V>(&mut self, node: &N, bound: &B)
                      -> Option<V> where
        N: ::Node<Transform = T>,
        V: ::ViewInfo<S, T>,
    {
        use cgmath::{Matrix, Transform};
        let model = node.get_transform();
        let view = self.cam_inverse.concat(&model);
        let mvp = self.view_projection.mul_m(&model.clone().into());
        if self.culler.cull(bound, &mvp) != collision::Relation::Out {
            Some(::ViewInfo::new(mvp, view, model))
        }else {
            None
        }
    }

    /// Cull and draw the entities into a stream.
    pub fn draw<'b, R, M, E, I, V, H, X>(&mut self,
                entities: I, phase: &mut H, stream: &mut X)
                -> Result<::Report, ::Error> where
        R: gfx::Resources + 'b,
        M: 'b,
        E: ::Entity<R, M, Bound = B, Transform = T> + 'b,
        I: Iterator<Item = &'b E>,
        V: ::ViewInfo<S, T>,
        H: gfx_phase::AbstractPhase<R, M, V>,
        X: gfx::Stream<R>,
    {
        let mut report = ::Report::new();
        // attach the profiler to the phase


        let g = hprof::enter("enqueue");
        // enqueue entities fragments
        for ent in entities {
            let frag_count = ent.get_fragments().len() as ::Count;
            if !ent.is_visible() {
                report.calls_invisible += frag_count;
                continue
            }
            if let Some(view_info) = self.is_visible(ent, &ent.get_bound()) {
                for frag in ent.get_fragments().iter() {
                    match phase.enqueue(ent.get_mesh(), &frag.slice, &frag.material, &view_info) {
                        Ok(true)  => {
                            report.primitives_rendered += frag.slice.get_prim_count();
                            report.calls_passed += 1;
                        },
                        Ok(false) => report.calls_rejected += 1,
                        Err(e)    => return Err(::Error::Batch(e)),
                    }
                }
            } else {
                report.calls_culled += frag_count;
            }
        }
        drop(g);

        let _g = hprof::enter("flush");
        // flush into the renderer
        match phase.flush(stream) {
            Ok(()) => Ok(report),
            Err(e) => Err(::Error::Flush(e)),
        }
    }
}
