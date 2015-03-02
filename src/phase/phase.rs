extern crate draw_queue;

use std::cmp::Ordering;
use gfx;

pub type FlushError = gfx::DrawError<gfx::batch::OutOfBounds>;

pub trait AbstractPhase<D: gfx::Device, E, Z> {
    /// Check if it makes sense to draw this entity
    fn does_apply(&self, &E) -> bool;
    /// Add an entity to the queue
    fn enqueue(&mut self, &E, Z, &mut gfx::batch::Context<D::Resources>)
               -> Result<(), gfx::batch::BatchError>;
    /// Flush the queue into a given renderer
    fn flush(&mut self, &gfx::Frame<D::Resources>,
             &gfx::batch::Context<D::Resources>,
             &mut gfx::Renderer<D::CommandBuffer>) -> Result<(), FlushError>;
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

pub trait ToDepth {
    type Depth: PartialOrd;
    fn to_depth(&self) -> Self::Depth;
}

/// Phase is doing draw call accumulating and sorting,
/// based a given technique.
pub struct Phase<
    R: gfx::Resources,
    M: ::Material,
    Z: ToDepth,
    T: ::Technique<R, M, Z>,
>{
    pub name: String,
    technique: T,
    sort: Vec<Sort>,
    queue: draw_queue::Queue<Object<Z::Depth, T::Params>>,
}

impl<
    R: gfx::Resources,
    M: ::Material,
    Z: ToDepth,
    T: ::Technique<R, M, Z>,
> Phase<R, M, Z, T> {
    pub fn new(name: &str, tech: T, sort: Sort) -> Phase<R, M, Z, T> {
        Phase {
            name: name.to_string(),
            technique: tech,
            sort: vec![sort],
            queue: draw_queue::Queue::new(),
        }
    }
}

impl<
    D: gfx::Device,
    M: ::Material,
    Z: ToDepth + Copy,
    E: ::Entity<D::Resources, M>,
    T: ::Technique<D::Resources, M, Z>,
>AbstractPhase<D, E, Z> for Phase<D::Resources, M, Z, T> {
    fn does_apply(&self, entity: &E) -> bool {
        self.technique.does_apply(entity.get_mesh().0, entity.get_material())
    }

    fn enqueue(&mut self, entity: &E, data: Z,
               context: &mut gfx::batch::Context<D::Resources>)
               -> Result<(), gfx::batch::BatchError> {
        // unable to use `self.does_apply` here
        debug_assert!(self.technique.does_apply(
            entity.get_mesh().0, entity.get_material()
        ));
        let depth = data.to_depth();
        // TODO: batch cache
        let (mesh, slice) = entity.get_mesh();
        let (program, state, mut param) = self.technique.compile(
            mesh, entity.get_material(), data);
        match context.make_batch(program, mesh, slice, state) {
            Ok(b) => {
                //TODO: only if cached
                self.technique.fix_params(entity.get_material(), &data, &mut param);
                let object = Object {
                    batch: b,
                    parameters: param,
                    depth: depth,
                };
                self.queue.objects.push(object);
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self, frame: &gfx::Frame<D::Resources>,
             context: &gfx::batch::Context<D::Resources>,
             renderer: &mut gfx::Renderer<D::CommandBuffer>)
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
