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
use rayon::prelude::*;
use rule::Rule;
use vec_sets::{split_out, split_out_item, union};

pub type ItemsetSupport = FnvHashMap<Vec<Item>, f64>;

fn create_support_lookup(itemsets: &Vec<ItemSet>, dataset_size: u32) -> ItemsetSupport {
    itemsets
        .iter()
        .map(|itemset| {
            (
                itemset.items.clone(),
                itemset.count as f64 / dataset_size as f64,
            )
        })
        .collect()
}

fn stats(
    support: f64,
    antecedent: &[Item],
    consequent: &[Item],
    itemset_support: &ItemsetSupport,
) -> (f64, f64) {
    let a_sup = itemset_support[antecedent];
    let confidence = support / a_sup;
    let c_sup = itemset_support[consequent];
    let lift = support / (a_sup * c_sup);
    (confidence, lift)
}

// Returns the number of items that match in a and b, starting from offset 0.
fn prefx_match_len(a: &[Item], b: &[Item]) -> usize {
    if a.len() != b.len() {
        panic!("prefx_match_len called on pair with different length");
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            return i;
        }
    }
    a.len()
}

fn generate_rules_for_itemset(
    itemset: &[Item],
    support: f64,
    itemset_support: &ItemsetSupport,
    min_confidence: f64,
    min_lift: f64,
) -> Vec<Rule> {
    // Generate rules via appgenrules algorithm. Combine consequents until
    // all combinations have been tested.
    let mut output = vec![];
    // First level consequent candidates are all single items in the itemset.
    let mut candidates: Vec<Vec<Item>> = vec![];
    for item in itemset.iter() {
        let (antecedent, consequent) = split_out_item(itemset, *item);
        let (confidence, lift) = stats(support, &antecedent, &consequent, &itemset_support);
        if confidence < min_confidence {
            continue;
        }
        if lift >= min_lift {
            output.push(Rule {
                antecedent,
                consequent: consequent.clone(),
                confidence,
                lift,
                support,
            });
        }
        candidates.push(consequent)
    }

    // Create subsequent generations by merging consequents which have size-1 items
    // in common in the consequent.

    let k = itemset.len();
    while !candidates.is_empty() && candidates[0].len() + 1 < k {
        // Note: candidates must be sorted here.
        let mut next_gen = vec![];
        let m = candidates[0].len(); // size of consequent.
        for i1 in 0..candidates.len() {
            for i2 in i1 + 1..candidates.len() {
                let c1 = &candidates[i1];
                let c2 = &candidates[i2];
                if prefx_match_len(c1, c2) != m - 1 {
                    // Consequents in the candidates list are sorted, and the
                    // candidates list itself is sorted. So we can stop
                    // testing combinations once our iteration reaches another
                    // candidate that no longer shares an m-1 prefix. Stopping
                    // the iteration here is a significant optimization. This
                    // ensures that we don't generate or test duplicate
                    // rules.
                    break;
                }
                let consequent = union(c1, c2);
                let antecedent = split_out(&itemset, &consequent);
                let (confidence, lift) = stats(support, &antecedent, &consequent, &itemset_support);
                if confidence < min_confidence {
                    continue;
                }
                if lift >= min_lift {
                    output.push(Rule {
                        antecedent,
                        consequent: consequent.clone(),
                        confidence,
                        lift,
                        support,
                    });
                }
                next_gen.push(consequent)
            }
        }
        candidates = next_gen;
        candidates.sort();
    }

    output
}

pub fn generate_rules(
    itemsets: &Vec<ItemSet>,
    dataset_size: u32,
    min_confidence: f64,
    min_lift: Option<f64>,
) -> Vec<Vec<Rule>> {
    // Create a lookup of itemset to support, so we can quickly determine
    // an itemset's support during rule generation.
    let itemset_support = create_support_lookup(itemsets, dataset_size);

    let min_lift = min_lift.unwrap_or(0.0);

    itemsets
        .par_iter()
        .filter(|&i| i.items.len() > 1)
        .map(|ref i| -> Vec<Rule> {
            let support = i.count as f64 / dataset_size as f64;
            generate_rules_for_itemset(
                &i.items,
                support,
                &itemset_support,
                min_confidence,
                min_lift,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {

    use super::create_support_lookup;
    use super::stats;
    use super::ItemsetSupport;
    use fnv::FnvHashSet;
    use fptree::ItemSet;
    use item::Item;
    use rule::Rule;
    use std::collections::HashMap;
    use vec_sets::union;

    type RuleSet = FnvHashSet<Rule>;

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
            let both = union(antecedent, consequent);
            let support = itemset_support[&both];
            let (confidence, lift) = stats(support, &antecedent, &consequent, &itemset_support);
            let min_lift = min_lift.unwrap_or(0.0);
            if confidence >= min_confidence && lift >= min_lift {
                rules.insert(Rule {
                    antecedent: antecedent.to_vec(),
                    consequent: consequent.to_vec(),
                    confidence,
                    lift,
                    support,
                });
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
        })
        .collect();

        let generated_rules = super::generate_rules(&kosarak, 990002, 0.05, Some(1.5));
        let num_rules: usize = generated_rules.iter().map(|ref x| x.len()).sum();
        assert_eq!(num_rules, expected_rules.len());

        let naive_rules = naive_generate_rules(&kosarak, 990002, 0.05, Some(1.5));
        assert_eq!(naive_rules.len(), num_rules);

        for rule in &naive_rules {
            let k = (rule.antecedent.clone(), rule.consequent.clone());
            assert_eq!(expected_rules.contains_key(&k), true);
            let (confidence, lift, support) = expected_rules[&k];
            assert!(fuzzy_float_eq(rule.confidence, confidence));
            assert!(fuzzy_float_eq(rule.lift, lift));
            assert!(fuzzy_float_eq(rule.support, support));
        }

        for chunk in &generated_rules {
            for rule in chunk {
                let k = (rule.antecedent.clone(), rule.consequent.clone());
                assert_eq!(naive_rules.contains(rule), true);
                let (confidence, lift, support) = expected_rules[&k];
                assert!(fuzzy_float_eq(rule.confidence, confidence));
                assert!(fuzzy_float_eq(rule.lift, lift));
                assert!(fuzzy_float_eq(rule.support, support));
            }
        }
    }
}
