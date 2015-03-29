use std::fmt::Debug;
use std::marker::PhantomFn;
use cgmath;
use gfx;
use gfx_phase;
use super::{World, Camera, Entity, ViewInfo};

/// An extension trait for a Phase that supports frustum culling.
pub trait CullPhase<
    R: gfx::Resources,
    M: gfx_phase::Material,
    E: gfx_phase::Entity<R, M>,
    W: World,
    V, //ViewInfo, necessary to be constrained
>: PhantomFn<R> + PhantomFn<M> + PhantomFn<V> {
    /// Enqueue a series of entities given by an iterator.
    /// Do frustum culling and `ViewInfo` construction on the fly.
    fn enqueue_all<'a,
        I: Iterator<Item = &'a E>,
        P: cgmath::Projection<W::Scalar>,
    >(  &mut self, entities: I, world: &W, camera: &Camera<P, W::NodePtr>,
        cull_frustum: bool) -> Result<(), gfx::batch::Error>;
}

impl<
    R: gfx::Resources,
    M: gfx_phase::Material,
    W: World,
    B: cgmath::Bound<W::Scalar> + Debug,
    V: ViewInfo<W::Scalar, W::Transform>,
    H: gfx_phase::QueuePhase<Entity<R, M, W, B>, V> + ?Sized,
> CullPhase<R, M, Entity<R, M, W, B>, W, V> for H {
    fn enqueue_all<'a,
        I: Iterator<Item = &'a Entity<R, M, W, B>>,
        P: cgmath::Projection<W::Scalar>,
    >(  &mut self, entities: I, world: &W, camera: &Camera<P, W::NodePtr>,
        cull_frustum: bool) -> Result<(), gfx::batch::Error>
    where
        R: 'a,
        R::Buffer: 'a,
        R::ArrayBuffer: 'a,
        R::Shader: 'a,
        R::Program: 'a,
        R::FrameBuffer: 'a,
        R::Surface: 'a,
        R::Texture: 'a,
        R::Sampler: 'a,
        M: 'a,
        W: 'a,
        W::Transform: 'a,
        W::NodePtr: 'a,
        W::SkeletonPtr: 'a,
        B: 'a,
    {
        use cgmath::{Matrix, ToMatrix4, Transform};
        let cam_inverse = world.get_transform(&camera.node)
                               .invert().unwrap();
        let projection = camera.projection.to_matrix4()
                               .mul_m(&cam_inverse.to_matrix4());
        for entity in entities {
            if !self.test(entity) {
                continue
            }
            let model = world.get_transform(&entity.node);
            let view = cam_inverse.concat(&model);
            let mvp = projection.mul_m(&model.to_matrix4());
            if cull_frustum && entity.bound.relate_clip_space(&mvp) == cgmath::Relation::Out {
                continue
            }
            let view_info = ViewInfo::new(mvp, view, model.clone());
            match self.enqueue(entity, view_info) {
                Ok(()) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
