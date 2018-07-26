// Copyright 2018 Chris Pearce
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use item::Item;

pub struct ItemCounter {
    counter: Vec<u32>,
}

impl ItemCounter {
    pub fn new() -> ItemCounter {
        ItemCounter { counter: vec![] }
    }
    pub fn add(&mut self, item: &Item, count: u32) {
        let index = item.as_index();
        if self.counter.len() <= index {
            self.counter.resize(index + 1, 0);
        }
        self.counter[index] += count;
    }
    pub fn get(&self, item: &Item) -> u32 {
        let index = item.as_index();
        if index >= self.counter.len() {
            0
        } else {
            self.counter[index]
        }
    }
    pub fn set(&mut self, item: &Item, count: u32) {
        let index = item.as_index();
        if self.counter.len() <= index {
            self.counter.resize(index + 1, 0);
        }
        self.counter[index] = count;
    }
    pub fn items_with_count_at_least(&self, min_count: u32) -> Vec<Item> {
        let mut v: Vec<Item> = vec![];
        for i in 1..self.counter.len() {
            if self.counter[i] >= min_count {
                v.push(Item::with_id(i as u32));
            }
        }
        v
    }
    pub fn take(&mut self, other: ItemCounter) {
        self.counter = other.counter;
    }
    pub fn sort_descending(&self, v: &mut Vec<Item>) {
        v.sort_by(|a, b| {
            let count_a = self.get(a);
            let count_b = self.get(b);
            if count_a == count_b {
                return b.cmp(a);
            }
            count_b.cmp(&count_a)
        });
    }
}
