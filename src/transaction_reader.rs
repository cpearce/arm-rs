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
use std::collections::HashSet;
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
            let splits = line
                .split(",")
                .map(|s| self.itemizer.id_of(s.trim()))
                .collect::<HashSet<Item>>()
                .iter()
                .cloned()
                .collect::<Vec<Item>>();
            if splits.len() > 0 {
                return Some(splits);
            }
        }
    }
}
