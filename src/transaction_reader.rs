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
use itemizer::Itemizer;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

pub struct TransactionReader<'a> {
    reader: BufReader<File>,
    itemizer: &'a mut Itemizer,
}

impl<'a> TransactionReader<'a> {
    pub fn new(path: &str, itemizer: &'a mut Itemizer) -> TransactionReader<'a> {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        TransactionReader {
            reader: reader,
            itemizer,
        }
    }
}

impl<'a> Iterator for TransactionReader<'a> {
    type Item = Vec<Item>;
    fn next(&mut self) -> Option<Vec<Item>> {
        let mut line = String::new();
        loop {
            let len = self.reader.read_line(&mut line).unwrap();
            if len == 0 {
                return None;
            }
            let mut splits = line
                .split(",")
                .map(|s| self.itemizer.id_of(s.trim()))
                .collect::<Vec<Item>>();

            // Some input files have transactions with duplicates items.
            // Remove any duplicates here.
            splits.sort();
            dedupe_sorted(&mut splits);

            if splits.len() > 0 {
                return Some(splits);
            }
        }
    }
}

fn dedupe_sorted(v: &mut Vec<Item>) {
    let mut i = 0;
    let mut k = 0;
    while i < v.len() {
        v[k] = v[i];
        while i < v.len() && v[k] == v[i] {
            i += 1;
        }
        k += 1;
    }
    assert!(k <= v.len());
    v.resize(k, Item::null());
}

#[cfg(test)]
mod tests {

    use item::Item;

    fn to_item_vec(nums: &[u32]) -> Vec<Item> {
        nums.iter().map(|&i| Item::with_id(i)).collect()
    }
    #[test]
    fn test_dedupe_sorted() {
        let cases = [
            (vec![], vec![]),
            (vec![1], vec![1]),
            (vec![1, 2], vec![1, 2]),
            (vec![1, 1], vec![1]),
            (vec![1, 1, 1], vec![1]),
            (vec![1, 1, 2, 2], vec![1, 2]),
            (vec![1, 2, 3], vec![1, 2, 3]),
            (vec![1, 2, 2, 3], vec![1, 2, 3]),
        ];
        for (mut v, e) in cases.iter().map(|(a, b)| (to_item_vec(a), to_item_vec(b))) {
            super::dedupe_sorted(&mut v);
            assert!(v == e);
        }
    }
}
