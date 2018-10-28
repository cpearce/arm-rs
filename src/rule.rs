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
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub struct Rule {
    pub antecedent: Vec<Item>,
    pub consequent: Vec<Item>,
    pub confidence: f64,
    pub lift: f64,
    pub support: f64,
}

// Custom hash that excludes floating point values which aren't hashable.
impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.antecedent.hash(state);
        self.consequent.hash(state);
    }
}

// Override equality check, as floating point values can't be reliably compared for equality.
impl PartialEq for Rule {
    fn eq(&self, other: &Rule) -> bool {
        self.antecedent == other.antecedent && self.consequent == other.consequent
    }
}

impl Eq for Rule {}
