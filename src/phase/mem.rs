use gfx;

pub type MemResult<T> = Result<T, gfx::batch::Error>; 

pub trait Memory<R: gfx::Resources, T> {
    fn recall(&self, &gfx::Mesh<R>, &::Material)
              -> Option<MemResult<T>>;
    fn store(&mut self, &gfx::Mesh<R>, &::Material,
             MemResult<T>);
}

impl<R: gfx::Resources, T> Memory<R, T> for () {
	fn recall(&self, _: &gfx::Mesh<R>, _: &::Material)
              -> Option<MemResult<T>> { None }
    fn store(&mut self, _: &gfx::Mesh<R>, _: &::Material,
             _: MemResult<T>) {}
}
