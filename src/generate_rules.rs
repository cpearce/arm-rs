use item::Item;
use rayon::prelude::*;
use itertools::Itertools;
use fnv::{FnvHashMap, FnvHashSet};
use fptree::ItemSet;
use rule::Rule;

pub fn split_out_item(items: &Vec<Item>, item: Item) -> (Vec<Item>, Vec<Item>) {
    let antecedent: Vec<Item> = items.iter().filter(|&&x| x != item).cloned().collect();
    let consequent: Vec<Item> = vec![item];
    (antecedent, consequent)
}

struct ConsequentTree {
    children: Vec<ConsequentTree>,
    rules: Vec<Rule>,
    item: Item,
}

impl ConsequentTree {
    pub fn new(item: Item) -> ConsequentTree {
        ConsequentTree {
            children: vec![],
            rules: vec![],
            item: item,
        }
    }
    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
    fn insert(&mut self, consequent: &[Item], rule: &Rule) {
        if consequent.is_empty() {
            self.rules.push(rule.clone());
            return;
        }
        let item = consequent[0];
        for child in self.children.iter_mut() {
            if child.item == item {
                child.insert(&consequent[1..], rule);
                return;
            }
        }
        let mut child = ConsequentTree::new(item);
        child.insert(&consequent[1..], rule);
        self.children.push(child);
    }
    pub fn generate_candidate_rules(
        &self,
        items: &[Item],
        min_confidence: f64,
        min_lift: f64,
        itemset_support: &FnvHashMap<Vec<Item>, f64>,
    ) -> FnvHashSet<Rule> {
        let mut rules = FnvHashSet::default();
        let mut path = vec![];
        self.generate_candidate_rules_recursive(
            items,
            min_confidence,
            min_lift,
            itemset_support,
            &mut rules,
            &mut path,
        );
        rules
    }

    pub fn generate_candidate_rules_recursive(
        &self,
        items: &[Item],
        min_confidence: f64,
        min_lift: f64,
        itemset_support: &FnvHashMap<Vec<Item>, f64>,
        rules: &mut FnvHashSet<Rule>,
        path: &mut Vec<Item>,
    ) {
        if self.is_leaf() {
            return;
        }
        // Filter out this node's children which store rules.
        let leaf_children: Vec<&ConsequentTree> = self.children
            .iter()
            .filter(|&child| !child.rules.is_empty())
            .collect();
        // Foreach possible combination of two leaf children.
        for (child1, child2) in leaf_children.iter().tuple_combinations() {
            // Try merging each of those children's leaves.
            for (r1, r2) in child1.rules.iter().cartesian_product(&child2.rules) {
                if let Some(rule) = Rule::merge(&r1, &r2, itemset_support, min_confidence, min_lift)
                {
                    // Passes confidence and lift threshold, keep rule.
                    rules.insert(rule);
                }
            }
        }
        // Push out item into the consequent path.
        if !self.item.is_null() {
            path.push(self.item);
        }
        // Recurse onto each child.
        for ref child in self.children.iter() {
            child.generate_candidate_rules_recursive(
                items,
                min_confidence,
                min_lift,
                itemset_support,
                rules,
                path,
            );
        }
        // Undo the consequent push; backtrack.
        if !self.item.is_null() {
            path.pop();
        }
    }
}

pub fn generate_itemset_rules(
    itemset: &ItemSet,
    rules: &FnvHashSet<Rule>,
    min_confidence: f64,
    min_lift: f64,
    itemset_support: &FnvHashMap<Vec<Item>, f64>,
) -> FnvHashSet<Rule> {
    // Build a trie of all the consequents. The consequents are sorted. So
    // we can use the overlap in the trie branches in order to only attempt
    // to merge rules which have overlapping consequents.
    let mut rule_tree = ConsequentTree::new(Item::null());
    for rule in rules.iter() {
        rule_tree.insert(&rule.consequent, &rule);
    }
    rule_tree.generate_candidate_rules(&itemset.items, min_confidence, min_lift, itemset_support)
}

pub fn generate_rules(
    itemsets: &Vec<ItemSet>,
    dataset_size: u32,
    min_confidence: f64,
    min_lift: f64,
) -> FnvHashSet<Rule> {
    // Create a lookup of itemset to support, so we can quickly determine
    // an itemset's support during rule generation.
    let mut itemset_support: FnvHashMap<Vec<Item>, f64> =
        FnvHashMap::with_capacity_and_hasher(itemsets.len(), Default::default());
    for ref i in itemsets.iter() {
        itemset_support.insert(i.items.clone(), i.count as f64 / dataset_size as f64);
    }

    let rv: Vec<FnvHashSet<Rule>> = itemsets
        .par_iter()
        .filter(|i| i.items.len() > 1)
        .map(|ref itemset| {
            let mut rules: FnvHashSet<Rule> = FnvHashSet::default();
            // First level candidates are all the rules with consequents of size 1.
            for &item in itemset.items.iter() {
                let (antecedent, consequent) = split_out_item(&itemset.items, item);
                if let Some(rule) = Rule::make(
                    antecedent,
                    consequent,
                    &itemset_support,
                    min_confidence,
                    min_lift,
                ) {
                    // Passes confidence and lift threshold, keep rule.
                    rules.insert(rule);
                }
            }
            let mut candidates = rules.clone();
            while !candidates.is_empty() {
                let next_gen = generate_itemset_rules(
                    itemset,
                    &candidates,
                    min_confidence,
                    min_lift,
                    &itemset_support,
                );
                for rule in next_gen.iter() {
                    rules.insert(rule.clone());
                }
                candidates = next_gen;
            }
            rules
        })
        .collect();

    let mut rules: FnvHashSet<Rule> = FnvHashSet::default();
    for set in rv.into_iter() {
        for rule in set {
            rules.insert(rule);
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    use fptree::ItemSet;
    use item::Item;
    use fnv::FnvHashMap;

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
        ].iter()
            .map(|&(ref i, c)| ItemSet::new(to_item_vec(&i), c))
            .collect();

        // (Antecedent, Consequent) -> (Confidence, Lift, Support)
        let expected_rules: FnvHashMap<(Vec<Item>, Vec<Item>), (f64, f64, f64)> = [
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
        ].iter()
            .map(|&((ref a, ref c), (cnf, lft, sup))| {
                ((to_item_vec(a), to_item_vec(c)), (cnf, lft, sup))
            })
            .collect();

        let generated_rules = super::generate_rules(&kosarak, 990002, 0.05, 1.5);
        assert_eq!(generated_rules.len(), expected_rules.len());

        for rule in &generated_rules {
            let k = (rule.antecedent.clone(), rule.consequent.clone());
            assert_eq!(expected_rules.contains_key(&k), true);
            let (confidence, lift, support) = expected_rules[&k];
            assert!(fuzzy_float_eq(rule.confidence(), confidence));
            assert!(fuzzy_float_eq(rule.lift(), lift));
            assert!(fuzzy_float_eq(rule.support(), support));
        }
    }
}
