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

use fnv::FnvHashSet;
use generate_rules::ItemsetSupport;
use item::Item;
use std::hash::{Hash, Hasher};
use vec_sets::intersection;
use vec_sets::union;

pub type RuleSet = FnvHashSet<Rule>;

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

impl Rule {
    // Creates a new Rule from (antecedent,consequent) if the rule
    // would be above the min_confidence threshold. Assumes both
    // antecedent and consequent are already sorted.
    pub fn make(
        antecedent: Vec<Item>,
        consequent: Vec<Item>,
        itemset_support: &ItemsetSupport,
        min_confidence: f64,
        min_lift: Option<f64>,
    ) -> Option<Rule> {
        if antecedent.is_empty() || consequent.is_empty() {
            return None;
        }

        let ac_vec: Vec<Item> = union(&antecedent, &consequent);
        let ac_sup = match itemset_support.get(&ac_vec) {
            Some(support) => support.clone(),
            None => return None,
        };

        let a_sup = match itemset_support.get(&antecedent) {
            Some(support) => support.clone(),
            None => return None,
        };

        let confidence = ac_sup / a_sup;
        if confidence < min_confidence {
            return None;
        }
        let c_sup = match itemset_support.get(&consequent) {
            Some(support) => support.clone(),
            None => return None,
        };

        let lift = ac_sup / (a_sup * c_sup);
        if min_lift.is_some() && lift < min_lift.unwrap() {
            return None;
        }

        Some(Rule {
            antecedent,
            consequent,
            confidence,
            lift: lift,
            support: ac_sup,
        })
    }

    // Creates a new Rule with:
    //  - the antecedent is the union of both rules' antecedents, and
    //  - the consequent is the intersection of both rules' consequents,
    // provided the new rule would be would be above the min_confidence threshold.
    pub fn merge(
        a: &Rule,
        b: &Rule,
        itemset_support: &ItemsetSupport,
        min_confidence: f64,
        min_lift: Option<f64>,
    ) -> Option<Rule> {
        let antecedent = intersection(&a.antecedent, &b.antecedent);
        let consequent = union(&a.consequent, &b.consequent);
        Rule::make(
            antecedent,
            consequent,
            itemset_support,
            min_confidence,
            min_lift,
        )
    }

    pub fn confidence(&self) -> f64 {
        self.confidence.into()
    }

    pub fn lift(&self) -> f64 {
        self.lift.into()
    }

    pub fn support(&self) -> f64 {
        self.support.into()
    }
}
