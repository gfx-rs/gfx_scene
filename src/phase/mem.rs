use std::collections::HashMap;
use std::hash::Hash;
use gfx;

pub type MemResult<T> = Result<T, gfx::batch::Error>; 

pub trait Memory<T, K> {
    fn lookup(&self, T) -> Option<MemResult<K>>;
    fn store(&mut self, T, MemResult<K>);
}

impl<T, K> Memory<T, K> for () {
	  fn lookup(&self, _: T) -> Option<MemResult<K>> { None }
    fn store(&mut self, _: T, _: MemResult<K>) {}
}

impl<T: Hash + Eq, K: Clone> Memory<T, K> for HashMap<T, MemResult<K>> {
    fn lookup(&self, input: T) -> Option<MemResult<K>> {
        self.get(&input).map(|r| r.clone())
    }
    fn store(&mut self, input: T, out: MemResult<K>) {
        self.insert(input, out);
    }
}
