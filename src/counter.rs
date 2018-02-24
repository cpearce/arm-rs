use std::collections::HashMap;
use std::cmp;
use std::hash::Hash;

pub struct Counter<T> {
    counter: HashMap<T, u32>,
}

impl<T> Counter<T>
where
    T: cmp::Eq,
    T: Hash,
    T: Copy,
{
    pub fn new() -> Counter<T> {
        Counter {
            counter: HashMap::new(),
        }
    }
    pub fn add(&mut self, item: &T, count: u32) {
        *self.counter.entry(*item).or_insert(0) += count;
    }
    pub fn get(&self, item: &T) -> u32 {
        match self.counter.get(&item) {
            Some(count) => *count,
            None => 0,
        }
    }

    pub fn items_with_count_at_least(&self, min_count: u32) -> Vec<T> {
        self.counter
            .keys()
            .cloned()
            .filter(|item| self.get(item) > min_count)
            .collect()
    }
}
