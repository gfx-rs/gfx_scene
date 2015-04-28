use std::marker::PhantomData;
use cgmath;
use gfx;
use gfx_phase;


struct CullEntity<'a, V, R: 'a + gfx::Resources, M: 'a> where
    R::Buffer: 'a,
    R::ArrayBuffer: 'a,
    R::Shader: 'a,
    R::Program: 'a,
    R::FrameBuffer: 'a,
    R::Surface: 'a,
    R::Texture: 'a,
    R::Sampler: 'a,
{
    view_info: V,
    mesh: &'a gfx::Mesh<R>,
    slice: &'a gfx::Slice<R>,
    material: &'a M,
}

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

impl<S: cgmath::BaseFloat, B: cgmath::Bound<S>> Culler<S, B> for Frustum<S, B> {
    fn init(&mut self) {}
    fn cull(&mut self, bound: &B, mvp: &cgmath::Matrix4<S>) -> cgmath::Relation {
        bound.relate_clip_space(mvp)
    }
}

/// Culled scene - stores the culler as well as the current resulting
/// list of entity components (meshes, materials, etc)
pub struct CullScene<'a, C, V, R: 'a + gfx::Resources, M: 'a> {
    culler: C,
    entities: Vec<CullEntity<'a, V, R, M>>,
}

impl<'a, C, V: gfx_phase::ToDepth, R: gfx::Resources, M> CullScene<'a, C, V, R, M> where
    R::Buffer: 'static,
    R::ArrayBuffer: 'static,
    R::Shader: 'static,
    R::Program: 'static,
    R::FrameBuffer: 'static,
    R::Surface: 'static,
    R::Texture: 'static,
    R::Sampler: 'static,
{
    /// Transform into an empty state, dropping the lifetime.
    pub fn into_reset(mut self) -> CullScene<'static, C, V, R, M> {
        use std::mem::transmute;
        self.entities.clear();
        // technically safe, since the vec is empty
        CullScene {
            culler: self.culler,
            entities: unsafe { transmute(self.entities) },
        }
    }

    /// Draw using an abstract phase.
    pub fn draw_with<H, S>(&self, phase: &mut H, stream: &mut S)
                     -> Result<::FailCount, ::Error> where
        H: gfx_phase::AbstractPhase<R, M, V>,
        S: gfx::Stream<R>,
    {
        let mut fail = 0;
        // enqueue entities
        for e in self.entities.iter() {
            match phase.enqueue(e.mesh, e.slice, e.material, &e.view_info) {
                Ok(true) => (),
                Ok(false) => fail += 1,
                Err(e) => return Err(::Error::Batch(e)),
            }
        }
        // flush into the renderer
        match phase.flush(stream) {
            Ok(()) => Ok(fail),
            Err(e) => Err(::Error::Flush(e)),
        }
    }
}

impl<C, V, R: gfx::Resources, M> CullScene<'static, C, V, R, M> {
    /// Transform into a full state by culling the given entities.
    pub fn into_cull<'a, B, W, I, P>(self, entities: I, world: &W,
                     camera: &::Camera<P, W::NodePtr>)
                     -> CullScene<'a, C, V, R, M> where
        R::Buffer: 'static,
        R::ArrayBuffer: 'static,
        R::Shader: 'static,
        R::Program: 'static,
        R::FrameBuffer: 'static,
        R::Surface: 'static,
        R::Texture: 'static,
        R::Sampler: 'static,
        W: ::World + 'a,
        W::Transform: 'a,
        W::NodePtr: 'a,
        W::SkeletonPtr: 'a,
        V: ::ViewInfo<W::Scalar, W::Transform>,
        B: cgmath::Bound<W::Scalar> + 'a,
        C: Culler<W::Scalar, B>,
        I: Iterator<Item = &'a ::Entity<R, M, W, B>>,
        P: cgmath::Projection<W::Scalar>,
    {
        use cgmath::{Matrix, ToMatrix4, Transform};
        let cam_inverse = world.get_transform(&camera.node)
                               .invert().unwrap();
        let projection = camera.projection.to_matrix4()
                               .mul_m(&cam_inverse.to_matrix4());
        let mut out = CullScene {
            culler: self.culler,
            entities: self.entities,
        };
        debug_assert!(out.entities.is_empty());
        out.culler.init();
        for entity in entities {
            let model = world.get_transform(&entity.node);
            let view = cam_inverse.concat(&model);
            let mvp = projection.mul_m(&model.to_matrix4());
            if out.culler.cull(&entity.bound, &mvp) != cgmath::Relation::Out {
                out.entities.push(CullEntity {
                    view_info: ::ViewInfo::new(mvp, view, model),
                    mesh: &entity.mesh,
                    slice: &entity.slice,
                    material: &entity.material,
                });
            }
        }
        out
    }
}
