use std::collections::VecDeque;

pub struct Queue<T> {
    limit: usize,
    queue: VecDeque<T>,
}

#[allow(dead_code)]
impl<T> Queue<T> {
    pub fn with_capacity(limit: usize) -> Self {
        Self {
            limit,
            queue: VecDeque::with_capacity(limit),
        }
    }

    pub fn push(&mut self, val: T) {
        if self.queue.len() == self.limit {
            self.queue.pop_front();
        }
        self.queue.push_back(val);
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn has(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.last() == Some(item)
    }

    pub fn last(&self) -> Option<&T> {
        self.queue.back()
    }

    pub fn last_nth(&self, nth: usize) -> Option<&T> {
        self.queue.iter().rev().nth(nth)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.queue.iter().rev()
    }
}
