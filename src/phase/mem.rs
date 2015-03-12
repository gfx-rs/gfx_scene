use std::collections::HashMap;
use std::hash::Hash;
use gfx;

pub type MemResult<T> = Result<T, gfx::batch::Error>; 

pub trait Memory<T, S> {
    fn lookup(&self, T) -> Option<MemResult<S>>;
    fn store(&mut self, T, MemResult<S>);
}

impl<T, S> Memory<T, S> for () {
	  fn lookup(&self, _: T) -> Option<MemResult<S>> { None }
    fn store(&mut self, _: T, _: MemResult<S>) {}
}

impl<T: Hash + Eq, S: Clone> Memory<T, S> for HashMap<T, MemResult<S>> {
    fn lookup(&self, input: T) -> Option<MemResult<S>> {
        self.get(&input).map(|r| r.clone())
    }
    fn store(&mut self, input: T, out: MemResult<S>) {
        self.insert(input, out);
    }
}
