extern crate queue;

use std::cmp::Ordering;
use gfx;

pub type FlushError = gfx::DrawError<gfx::batch::OutOfBounds>;

pub trait AbstractPhase<D: gfx::Device, Z, E> {
    /// Check if it makes sense to draw this entity
    fn does_apply(&self, &E) -> bool;
    /// Add an entity to the queue
    fn enqueue(&mut self, &E, Z, &mut gfx::batch::Context)
               -> Result<(), gfx::batch::BatchError>;
    /// Flush the queue into a given renderer
    fn flush(&mut self, &gfx::Frame<D::Resources>, &gfx::batch::Context,
             &mut gfx::Renderer<D>) -> Result<(), FlushError>;
}

struct Object<S, P: gfx::shade::ShaderParam> {
    batch: gfx::batch::RefBatch<P>,
    parameters: P,
    depth: S,
}

impl<S: PartialOrd, P: gfx::shade::ShaderParam> Object<S, P> {
    fn cmp_depth(&self, other: &Object<S, P>) -> Ordering {
        self.depth.partial_cmp(&other.depth)
            .unwrap_or(Ordering::Equal)
    }
}

pub enum Sort {
    FrontToBack,
    BackToFront,
    Program,
    Mesh,
    DrawState,
}

pub trait ToDepth<S> {
    fn to_depth(&self) -> S;
}

pub struct Phase<S, Z, M: ::Material, T: ::Technique<Z, M>> {
    pub name: String,
    technique: T,
    sort: Vec<Sort>,
    //TODO: queue::Queue<Object<S, (M::Params, T::Params)>>,
    queue: queue::Queue<Object<S, T::Params>>,
}

impl<
    S: PartialOrd,
    Z: ToDepth<S>,
    M: ::Material,
    E: ::Entity<M>,
    T: ::Technique<Z, M>
>AbstractPhase<gfx::GlDevice, Z, E> for Phase<S, Z, M, T> {
    fn does_apply(&self, entity: &E) -> bool {
        self.technique.does_apply(entity.get_material(), entity.get_mesh().0)
    }

    fn enqueue(&mut self, entity: &E, data: Z, context: &mut gfx::batch::Context)
               -> Result<(), gfx::batch::BatchError> {
        debug_assert!(self.does_apply(entity));
        let depth = data.to_depth();
        // TODO: batch cache
        let (mesh, slice) = entity.get_mesh();
        let (program, state, tparam) = self.technique.compile(
            entity.get_material(), mesh, data);
        match context.make_batch(program, mesh, slice, state) {
            Ok(b) => {
                let _mparam = entity.get_material().get_params();
                //TODO: only if cached
                //self.technique.fix_params(&data, &mut tparam);
                let object = Object {
                    batch: b,
                    parameters: tparam, //TODO: (mparam, tparam)
                    depth: depth,
                };
                self.queue.objects.push(object);
                Ok(())
            },
            Err(e) => Err(e),
        }
    }


    fn flush(&mut self, frame: &gfx::Frame<gfx::GlResources>,
             context: &gfx::batch::Context,
             renderer: &mut gfx::Renderer<gfx::GlDevice>)
             -> Result<(), FlushError> {
        // sort the queue
        match self.sort.first() {
            Some(&Sort::FrontToBack) =>
                self.queue.sort(|a, b| a.cmp_depth(&b)),
            Some(&Sort::BackToFront) =>
                self.queue.sort(|a, b| b.cmp_depth(&a)),
            Some(&Sort::Program) =>
                self.queue.sort(|a, b| a.batch.cmp_program(&b.batch)),
            Some(&Sort::Mesh) =>
                self.queue.sort(|a, b| a.batch.cmp_mesh(&b.batch)),
            Some(&Sort::DrawState) =>
                self.queue.sort(|a, b| a.batch.cmp_state(&b.batch)),
            None => (),
        }
        // call the draws
        for o in self.queue.iter() {
            match renderer.draw(&(&o.batch, &o.parameters, context), frame) {
                Ok(_) => (),
                e => return e,
            }
        }
        // done
        self.queue.objects.clear();
        Ok(())
    }
}
