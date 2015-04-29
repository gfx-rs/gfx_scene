use std::marker::PhantomData;
use cgmath;
use gfx;
use gfx_phase;


/// Culled result on an entity.
#[derive(Clone)]
pub struct CullEntity<'a, R: 'a + gfx::Resources, M: 'a, V> where
    R::Buffer: 'a,
    R::ArrayBuffer: 'a,
    R::Shader: 'a,
    R::Program: 'a,
    R::FrameBuffer: 'a,
    R::Surface: 'a,
    R::Texture: 'a,
    R::Sampler: 'a,
{
    mesh: &'a gfx::Mesh<R>,
    slice: &'a gfx::Slice<R>,
    material: &'a M,
    view_info: V,
}

/// Culling iterator.
pub struct CullIterator<'a, 'b, R, M, W, B, V, I, C> where
    R: gfx::Resources,
    W: ::World + 'a,
    B: cgmath::Bound<W::Scalar>,
    I: Iterator<Item = &'a ::Entity<R, M, W, B>>,
    C: Culler<W::Scalar, B> + 'b,
{
    entities: I,
    world: &'a W,
    culler: &'b mut C,
    cam_inverse: W::Transform,
    projection: cgmath::Matrix4<W::Scalar>,
    dummy: PhantomData<V>,
}

impl<'a, 'c, R, M, W, B, V, I, C> Iterator for CullIterator<'a, 'c, R, M, W, B, V, I, C> where
    R: gfx::Resources,
    R::Buffer: 'a,
    R::ArrayBuffer: 'a,
    R::Shader: 'a,
    R::Program: 'a,
    R::FrameBuffer: 'a,
    R::Surface: 'a,
    R::Texture: 'a,
    R::Sampler: 'a,
    W: ::World + 'a,
    W::Transform: 'a,
    W::NodePtr: 'a,
    W::SkeletonPtr: 'a,
    B: cgmath::Bound<W::Scalar>,
    V: ::ViewInfo<W::Scalar, W::Transform>,
    I: Iterator<Item = &'a ::Entity<R, M, W, B>>,
    C: Culler<W::Scalar, B> + 'c,
{
    type Item = CullEntity<'a, R, M, V>;

    fn next(&mut self) -> Option<CullEntity<'a, R, M, V>> {
        use cgmath::{Matrix, ToMatrix4, Transform};
        while let Some(ent) = self.entities.next() {
            let model = self.world.get_transform(&ent.node);
            let view = self.cam_inverse.concat(&model);
            let mvp = self.projection.mul_m(&model.to_matrix4());
            if self.culler.cull(&ent.bound, &mvp) != cgmath::Relation::Out {
                return Some(CullEntity {
                    mesh: &ent.mesh,
                    slice: &ent.slice,
                    material: &ent.material,
                    view_info: ::ViewInfo::new(mvp, view, model),
                })
            }
        }
        None
    }
}

/// Generic bound culler.
pub trait Culler<S, B: cgmath::Bound<S>> {
    /// Start a new culling session.
    fn init(&mut self);
    /// Cull a bound with a given transformation matrix.
    fn cull(&mut self, &B, &cgmath::Matrix4<S>) -> cgmath::Relation;
    /// Process the whole scene, calling a function for every success.
    fn process<'a, 'c, R, M, T, W, P, V, I>(&'c mut self, entities: I, world: &'a W,
               camera: &::Camera<P, W::NodePtr>)
               -> CullIterator<'a, 'c, R, M, W, B, V, I, Self> where
        R: gfx::Resources,
        S: cgmath::BaseFloat,
        T: cgmath::Transform3<S> + Clone,   // shouldn't be needed
        W: ::World<Scalar = S, Transform = T> + 'a,
        P: cgmath::Projection<S>,
        I: Iterator<Item = &'a ::Entity<R, M, W, B>>,
        Self: Sized,    // magic!
    {
        use cgmath::{Matrix, ToMatrix4, Transform};
        let cam_inverse = world.get_transform(&camera.node)
                               .invert().unwrap();
        let projection = camera.projection.to_matrix4()
                               .mul_m(&cam_inverse.to_matrix4());
        self.init();
        CullIterator {
            entities: entities,
            world: world,
            culler: self,
            cam_inverse: cam_inverse,
            projection: projection,
            dummy: PhantomData,
        }
    }
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
/*
impl<'a, R, M, V, I> ::AbstractScene<R> for I where
    R: gfx::Resources,
    R::Buffer: 'a,
    R::ArrayBuffer: 'a,
    R::Shader: 'a,
    R::Program: 'a,
    R::FrameBuffer: 'a,
    R::Surface: 'a,
    R::Texture: 'a,
    R::Sampler: 'a,
    V, gfx_phase::ToDepth,
    I: Iterator<Item = CullEntity<'a, V, R, M>>,
{
    type ViewInfo = V;
    type Material = M;
    type Camera = ();

    fn draw<H, S>(&self, phase: &mut H, _: &(), stream: &mut S)
            -> Result<::FailCount, ::Error> where
        H: gfx_phase::AbstractPhase<R, Self::Material, Self::ViewInfo>,
        S: gfx::Stream<R>
    {
        let mut fail = 0;
        // enqueue entities
        for e in self {
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
}*/

/// Draw culled entities, specified by an iterator.
pub fn draw<'a, R, M, V, I, H, S>(entities: I, phase: &mut H, stream: &mut S)
            -> Result<::FailCount, ::Error> where
    R: gfx::Resources + 'a,
    R::Buffer: 'a,
    R::ArrayBuffer: 'a,
    R::Shader: 'a,
    R::Program: 'a,
    R::FrameBuffer: 'a,
    R::Surface: 'a,
    R::Texture: 'a,
    R::Sampler: 'a,
    M: 'a,
    V: gfx_phase::ToDepth,
    I: Iterator<Item = CullEntity<'a, R, M, V>>,
    H: gfx_phase::AbstractPhase<R, M, V>,
    S: gfx::Stream<R>
{
    let mut fail = 0;
    // enqueue entities
    for e in entities {
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
