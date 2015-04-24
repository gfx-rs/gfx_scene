#![deny(missing_docs)]

//! Generic draw queue that keeps item ordering, supposedly minimizing
//! the sorting time per frame by exploiting temporal coherency.

type IdType = u32;
struct Id<T>(IdType, std::marker::PhantomData<T>);

/// Iterator over queue objects.
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

/// Generic draw queue.
pub struct Queue<T> {
    /// Exposed objects list that can be modified directly with no harm.
    pub objects: Vec<T>,
    indices: Vec<Id<T>>,
}

impl<T> Queue<T> {
    /// Create an empty queue.
    pub fn new() -> Queue<T> {
        Queue {
            objects: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn is_ready(&self) -> bool {
        self.objects.len() == self.indices.len()
    }

    /// Synchronize the indices with objects.
    pub fn update(&mut self) {
        let ni = self.indices.len();
        if self.objects.len() > ni {
            self.indices.extend((ni.. self.objects.len()).map(|i|
                Id(i as IdType, std::marker::PhantomData)
            ));
        }else
        if self.objects.len() < ni {
            let no = self.objects.len();
            self.indices.retain(|&Id(i, _)| (i as usize) < no);
        }
        debug_assert!(self.is_ready());
    }

    /// Sort the draw queue.
    pub fn sort<F: Sized + Fn(&T, &T) -> std::cmp::Ordering>(&mut self, fun: F) {
        self.update();
        let objects = &self.objects;
        self.indices.sort_by(|&Id(a, _), &Id(b, _)|
            fun(&objects[a as usize], &objects[b as usize])
        );
    }

    /// Iterate over sorted objects.
    pub fn iter<'a>(&'a self) -> QueueIter<'a, T> {
        assert!(self.is_ready());
        QueueIter {
            objects: &self.objects,
            id_iter: self.indices.iter(),
        }
    }
}
