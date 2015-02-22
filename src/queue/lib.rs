#![feature(core)]

struct Id<T>(u32, std::marker::PhantomData<T>);

pub struct QueueIter<'a, T: 'a> {
    objects: &'a [T],
    id_iter: std::slice::Iter<'a, Id<T>>,
}

impl<'a, T> Iterator for QueueIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.id_iter.next().map(|&Id(i, _)|
            &self.objects[i as usize]
        )
    }
}

pub struct Queue<T> {
    pub objects: Vec<T>,
    indices: Vec<Id<T>>,
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        Queue {
            objects: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn is_ready(&self) -> bool {
        self.objects.len() == self.indices.len()
    }

    pub fn update(&mut self) {
        let ni = self.indices.len();
        if self.objects.len() > ni {
            self.indices.extend((ni.. self.objects.len()).map(|i|
                Id(i as u32, std::marker::PhantomData)
            ));
        }else
        if self.objects.len() < ni {
            self.indices.retain(|&Id(i, _)| (i as usize) < ni);
        }
        debug_assert!(self.is_ready());
    }

    pub fn sort<F: Sized + Fn(&T, &T) -> std::cmp::Ordering>(&mut self, fun: F) {
        self.update();
        let objects = self.objects.as_slice();
        self.indices.sort_by(|&Id(a, _), &Id(b, _)|
            fun(&objects[a as usize], &objects[b as usize])
        );
    }

    pub fn iter<'a>(&'a self) -> QueueIter<'a, T> {
        assert!(self.is_ready());
        QueueIter {
            objects: self.objects.as_slice(),
            id_iter: self.indices.iter(),
        }
    }
}
