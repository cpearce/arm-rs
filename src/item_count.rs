use item::Item;
use std::collections::HashMap;

pub struct ItemCount {
    counter: HashMap<Item, u32>,
}

impl ItemCount {
    pub fn new() -> ItemCount {
        ItemCount{counter: HashMap::new()}
    }
    pub fn add(&mut self, item: &Item, count: u32) {
        *self.counter.entry(*item).or_insert(0) += count;
    }
    pub fn get(&self, item: &Item) -> u32 {
        match self.counter.get(&item) {
            Some(count) => *count,
            None => 0,
        }
    }
}