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

use fnv::FnvHashMap;
use fptree::ItemSet;
use item::Item;
use itertools::Itertools;
use rayon::prelude::*;
use rule::Rule;
use rule::RuleSet;
use vec_sets::intersection_size;
use vec_sets::split_out_item;

pub type ItemsetSupport = FnvHashMap<Vec<Item>, f64>;

pub fn generate_itemset_rules(
    rules: &RuleSet,
    min_confidence: f64,
    min_lift: Option<f64>,
    itemset_support: &ItemsetSupport,
) -> RuleSet {
    // Try to combine rules with consequents of the same size together
    // which have size-1 items in common.
    rules
        .iter()
        .tuple_combinations()
        .filter(|&(ref rule1, ref rule2)| &rule1.consequent.len() == &rule2.consequent.len())
        .filter(|&(ref rule1, ref rule2)| {
            let overlap = intersection_size(&rule1.consequent, &rule2.consequent);
            overlap == rule2.consequent.len() - 1
        }).filter_map(|(rule1, rule2)| {
            Rule::merge(rule1, rule2, itemset_support, min_confidence, min_lift)
        }).collect()
}

fn create_support_lookup(itemsets: &Vec<ItemSet>, dataset_size: u32) -> ItemsetSupport {
    itemsets
        .iter()
        .map(|itemset| {
            (
                itemset.items.clone(),
                itemset.count as f64 / dataset_size as f64,
            )
        }).collect()
}

pub fn generate_rules(
    itemsets: &Vec<ItemSet>,
    dataset_size: u32,
    min_confidence: f64,
    min_lift: Option<f64>,
) -> RuleSet {
    // Create a lookup of itemset to support, so we can quickly determine
    // an itemset's support during rule generation.
    let itemset_support = create_support_lookup(itemsets, dataset_size);

    itemsets
        .par_iter()
        .map(|ref itemset| &itemset.items)
        .filter(|ref items| items.len() > 1)
        .map(|ref items| -> RuleSet {
            // First generation candidates are all the rules with consequents of size 1.
            items
                .iter()
                .filter_map(|&item| -> Option<Rule> {
                    let (antecedent, consequent) = split_out_item(&items, item);
                    Rule::make(
                        antecedent,
                        consequent,
                        &itemset_support,
                        min_confidence,
                        None,
                    )
                }).collect()
        }).flat_map(|candidates| -> RuleSet {
            // Create subsequent generations by merging pairs of rules of
            // whose consequent is size N and which have N-1 items in common.
            let mut rules: RuleSet = RuleSet::default();
            let mut candidates = candidates;
            while !candidates.is_empty() {
                let next_gen =
                    generate_itemset_rules(&candidates, min_confidence, None, &itemset_support);
                rules.extend(candidates);
                candidates = next_gen;
            }
            rules
        }).filter(|candidate| min_lift.map_or(true, |m| candidate.lift >= m))
        .collect()
}

#[cfg(test)]
mod tests {

    use super::create_support_lookup;
    use super::ItemsetSupport;
    use fptree::ItemSet;
    use item::Item;
    use rule::Rule;
    use rule::RuleSet;
    use std::collections::HashMap;

    fn naive_add_rules_for(
        rules: &mut RuleSet,
        items: &[Item],
        antecedent: &mut Vec<Item>,
        consequent: &mut Vec<Item>,
        itemset_support: &ItemsetSupport,
        min_confidence: f64,
        min_lift: Option<f64>,
    ) {
        if items.is_empty() {
            if antecedent.is_empty() || consequent.is_empty() {
                return;
            }
            if let Some(r) = Rule::make(
                antecedent.to_vec(),
                consequent.to_vec(),
                &itemset_support,
                min_confidence,
                min_lift,
            ) {
                rules.insert(r);
            }
            return;
        }
        let item = items[0];

        antecedent.push(item);
        naive_add_rules_for(
            rules,
            &items[1..],
            antecedent,
            consequent,
            itemset_support,
            min_confidence,
            min_lift,
        );
        antecedent.pop();

        consequent.push(item);
        naive_add_rules_for(
            rules,
            &items[1..],
            antecedent,
            consequent,
            itemset_support,
            min_confidence,
            min_lift,
        );
        consequent.pop();
    }

    // Naive implementation of rule generation which simply tries all
    // combinations of rules. Compare cleverer approach with this to ensure
    // the cleverer approach isn't over-pruning.
    fn naive_generate_rules(
        itemsets: &Vec<ItemSet>,
        dataset_size: u32,
        min_confidence: f64,
        min_lift: Option<f64>,
    ) -> RuleSet {
        // Create a lookup of itemset to support, so we can quickly determine
        // an itemset's support during rule generation.
        let itemset_support = create_support_lookup(itemsets, dataset_size);
        itemsets
            .iter()
            .map(|ref itemset| &itemset.items)
            .filter(|ref items| items.len() > 1)
            .fold(
                RuleSet::default(),
                |mut rules: RuleSet, ref items| -> RuleSet {
                    naive_add_rules_for(
                        &mut rules,
                        &items,
                        &mut vec![],
                        &mut vec![],
                        &itemset_support,
                        min_confidence,
                        min_lift,
                    );
                    rules
                },
            )
    }

    fn to_item_vec(nums: &[u32]) -> Vec<Item> {
        nums.iter().map(|&i| Item::with_id(i)).collect()
    }

    fn fuzzy_float_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.001
    }

    #[test]
    fn test_kosarak() {
        // Kosarak's itemsets with minsup=0.05, minconf=0.05.
        // 990002 transactions.
        let kosarak: Vec<ItemSet> = [
            (vec![1, 6, 11], 86092),
            (vec![1, 11], 91882),
            (vec![1, 3, 6], 57802),
            (vec![1, 3], 84660),
            (vec![1, 6], 132113),
            (vec![1], 197522),
            (vec![55], 65412),
            (vec![4], 78097),
            (vec![6], 601374),
            (vec![3, 6, 11], 143682),
            (vec![3, 11], 161286),
            (vec![6, 11], 324013),
            (vec![11], 364065),
            (vec![6, 148, 218], 56838),
            (vec![6, 11, 148, 218], 49866),
            (vec![11, 148, 218], 50098),
            (vec![148, 218], 58823),
            (vec![6, 11, 148], 55230),
            (vec![11, 148], 55759),
            (vec![6, 148], 64750),
            (vec![148], 69922),
            (vec![6, 11, 218], 60630),
            (vec![11, 218], 61656),
            (vec![6, 218], 77675),
            (vec![218], 88598),
            (vec![6, 7, 11], 55835),
            (vec![7, 11], 57074),
            (vec![6, 7], 73610),
            (vec![7], 86898),
            (vec![3, 6], 265180),
            (vec![3], 450031),
            (vec![6, 27], 59418),
            (vec![27], 72134),
        ]
            .iter()
            .map(|&(ref i, c)| ItemSet::new(to_item_vec(&i), c))
            .collect();

        // (Antecedent, Consequent) -> (Confidence, Lift, Support)
        let expected_rules: HashMap<(Vec<Item>, Vec<Item>), (f64, f64, f64)> = [
            ((vec![6], vec![1, 11]), (0.143, 1.542, 0.0870)),
            ((vec![11], vec![1, 6]), (0.236, 1.772, 0.0870)),
            ((vec![218], vec![148]), (0.664, 9.400, 0.059)),
            ((vec![148, 218], vec![6]), (0.966, 1.591, 0.057)),
            ((vec![1, 6], vec![11]), (0.652, 1.772, 0.087)),
            ((vec![11, 218], vec![6, 148]), (0.809, 12.366, 0.050)),
            ((vec![11], vec![7]), (0.157, 1.786, 0.058)),
            ((vec![11], vec![6, 148, 218]), (0.137, 2.386, 0.050)),
            ((vec![11], vec![148, 218]), (0.138, 2.316, 0.051)),
            ((vec![11, 218], vec![6]), (0.983, 1.619, 0.061)),
            ((vec![7, 11], vec![6]), (0.978, 1.610, 0.056)),
            ((vec![148], vec![11]), (0.797, 2.168, 0.056)),
            ((vec![11], vec![6, 148]), (0.152, 2.319, 0.056)),
            ((vec![218], vec![11]), (0.696, 1.892, 0.062)),
            ((vec![218], vec![11, 148]), (0.565, 10.040, 0.051)),
            ((vec![148], vec![6]), (0.926, 1.524, 0.065)),
            ((vec![6, 11], vec![148]), (0.170, 2.413, 0.056)),
            ((vec![11], vec![6, 7]), (0.153, 2.063, 0.056)),
            ((vec![11, 148], vec![218]), (0.898, 10.040, 0.051)),
            ((vec![148], vec![6, 11, 218]), (0.713, 11.645, 0.050)),
            ((vec![6], vec![11, 148, 218]), (0.083, 1.639, 0.050)),
            ((vec![7], vec![6, 11]), (0.643, 1.963, 0.056)),
            ((vec![6, 11, 148], vec![218]), (0.903, 10.089, 0.050)),
            ((vec![148], vec![6, 218]), (0.813, 10.360, 0.057)),
            ((vec![148], vec![6, 11]), (0.790, 2.413, 0.056)),
            ((vec![6, 148], vec![218]), (0.878, 9.809, 0.057)),
            ((vec![11], vec![148]), (0.153, 2.168, 0.056)),
            ((vec![11, 148], vec![6]), (0.991, 1.631, 0.056)),
            ((vec![6, 148, 218], vec![11]), (0.877, 2.386, 0.050)),
            ((vec![6], vec![148, 218]), (0.095, 1.591, 0.057)),
            ((vec![11], vec![6, 218]), (0.167, 2.123, 0.061)),
            ((vec![218], vec![6, 148]), (0.642, 9.809, 0.057)),
            ((vec![6, 148], vec![11]), (0.853, 2.319, 0.056)),
            ((vec![6, 11], vec![7]), (0.172, 1.963, 0.056)),
            ((vec![218], vec![6, 11, 148]), (0.563, 10.089, 0.050)),
            ((vec![148, 218], vec![11]), (0.852, 2.316, 0.051)),
            ((vec![6, 148], vec![11, 218]), (0.770, 12.366, 0.050)),
            ((vec![148], vec![11, 218]), (0.716, 11.504, 0.051)),
            ((vec![218], vec![6, 11]), (0.684, 2.091, 0.061)),
            ((vec![11, 148, 218], vec![6]), (0.995, 1.639, 0.050)),
            ((vec![11], vec![218]), (0.169, 1.892, 0.062)),
            ((vec![1, 11], vec![6]), (0.937, 1.542, 0.087)),
            ((vec![6, 11], vec![218]), (0.187, 2.091, 0.061)),
            ((vec![6], vec![148]), (0.108, 1.524, 0.065)),
            ((vec![6], vec![11, 148]), (0.092, 1.631, 0.056)),
            ((vec![148, 218], vec![6, 11]), (0.848, 2.590, 0.050)),
            ((vec![6, 218], vec![11]), (0.781, 2.123, 0.061)),
            ((vec![6, 7], vec![11]), (0.759, 2.063, 0.056)),
            ((vec![6], vec![11, 218]), (0.101, 1.619, 0.061)),
            ((vec![11, 218], vec![148]), (0.813, 11.504, 0.051)),
            ((vec![6, 11], vec![148, 218]), (0.154, 2.590, 0.050)),
            ((vec![148], vec![218]), (0.841, 9.400, 0.059)),
            ((vec![7], vec![11]), (0.657, 1.786, 0.058)),
            ((vec![6, 218], vec![11, 148]), (0.642, 11.398, 0.050)),
            ((vec![6, 11, 218], vec![148]), (0.822, 11.645, 0.050)),
            ((vec![6, 218], vec![148]), (0.732, 10.360, 0.057)),
            ((vec![6], vec![7, 11]), (0.093, 1.610, 0.056)),
            ((vec![11, 148], vec![6, 218]), (0.894, 11.398, 0.050)),
        ]
            .iter()
            .map(|&((ref a, ref c), (cnf, lft, sup))| {
                ((to_item_vec(a), to_item_vec(c)), (cnf, lft, sup))
            }).collect();

        let generated_rules = super::generate_rules(&kosarak, 990002, 0.05, Some(1.5));
        assert_eq!(generated_rules.len(), expected_rules.len());

        let naive_rules = naive_generate_rules(&kosarak, 990002, 0.05, Some(1.5));
        assert_eq!(naive_rules.len(), generated_rules.len());

        for rule in &naive_rules {
            let k = (rule.antecedent.clone(), rule.consequent.clone());
            assert_eq!(expected_rules.contains_key(&k), true);
            let (confidence, lift, support) = expected_rules[&k];
            assert!(fuzzy_float_eq(rule.confidence(), confidence));
            assert!(fuzzy_float_eq(rule.lift(), lift));
            assert!(fuzzy_float_eq(rule.support(), support));
        }

        for rule in &generated_rules {
            let k = (rule.antecedent.clone(), rule.consequent.clone());
            assert_eq!(naive_rules.contains(rule), true);
            let (confidence, lift, support) = expected_rules[&k];
            assert!(fuzzy_float_eq(rule.confidence(), confidence));
            assert!(fuzzy_float_eq(rule.lift(), lift));
            assert!(fuzzy_float_eq(rule.support(), support));
        }
    }
}
