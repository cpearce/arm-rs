use item::Item;
use rayon::prelude::*;
use itertools::Itertools;
use std::collections::HashSet;
use std::collections::HashMap;
use fptree::ItemSet;
use rule::Rule;
use rule::{difference, intersection, union};

pub fn split_out_item(items: &Vec<Item>, item: Item) -> (Vec<Item>, Vec<Item>) {
    let antecedent: Vec<Item> = items.iter().filter(|&&x| x != item).cloned().collect();
    let consequent: Vec<Item> = vec![item];
    (antecedent, consequent)
}

struct ConsequentTree {
    children: Vec<ConsequentTree>,
    item: Item,
}

impl ConsequentTree {
    pub fn new(item: Item) -> ConsequentTree {
        ConsequentTree {
            children: vec![],
            item: item,
        }
    }
    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
    pub fn insert(&mut self, consequent: &[Item]) {
        if consequent.is_empty() {
            return;
        }
        let item = consequent[0];
        for child in self.children.iter_mut() {
            if child.item == item {
                child.insert(&consequent[1..]);
                return;
            }
        }
        let mut child = ConsequentTree::new(item);
        child.insert(&consequent[1..]);
        self.children.push(child);
    }
    pub fn generate_candidate_rules(
        &self,
        items: &[Item],
        min_confidence: f64,
        min_lift: f64,
        itemset_support: &HashMap<Vec<Item>, f64>,
    ) -> HashSet<Rule> {
        let mut rules = HashSet::new();
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
        itemset_support: &HashMap<Vec<Item>, f64>,
        rules: &mut HashSet<Rule>,
        path: &mut Vec<Item>,
    ) {
        if self.is_leaf() {
            return;
        }

        let leaf_children: Vec<&ConsequentTree> = self.children
            .iter()
            .filter(|&child| child.is_leaf())
            .collect();
        for (child1, child2) in leaf_children.iter().tuple_combinations() {
            let mut consequent = path.clone();
            if child1.item < child2.item {
                consequent.push(child1.item);
                consequent.push(child2.item);
            } else {
                consequent.push(child2.item);
                consequent.push(child1.item);
            }
            let antecedent = difference(items, &consequent);
            if let Some(rule) = Rule::make(
                antecedent,
                consequent,
                itemset_support,
                min_confidence,
                min_lift,
            ) {
                // Passes confidence and lift threshold, keep rule.
                assert!(!rules.contains(&rule));
                rules.insert(rule);
            }
        }
    }
}

pub fn generate_itemset_rules(
    itemset: &ItemSet,
    rules: &HashSet<Rule>,
    min_confidence: f64,
    min_lift: f64,
    itemset_support: &HashMap<Vec<Item>, f64>,
) -> HashSet<Rule> {
    let mut rule_tree = ConsequentTree::new(Item::null());
    // let tree_depth = itemset.items.len() - 1;
    for rule in rules.iter() {
        rule_tree.insert(&rule.consequent);
    }
    rule_tree.generate_candidate_rules(&itemset.items, min_confidence, min_lift, itemset_support)
}

pub fn generate_rules(
    itemsets: &Vec<ItemSet>,
    dataset_size: u32,
    min_confidence: f64,
    min_lift: f64,
) -> HashSet<Rule> {
    // Create a lookup of itemset to support, so we can quickly determine
    // an itemset's support during rule generation.
    let mut itemset_support: HashMap<Vec<Item>, f64> = HashMap::with_capacity(itemsets.len());
    for ref i in itemsets.iter() {
        itemset_support.insert(i.items.clone(), i.count as f64 / dataset_size as f64);
    }

    let rv: Vec<HashSet<Rule>> = itemsets
        .par_iter()
        // .iter()
        .filter(|i| i.items.len() > 1)
        .map(|ref itemset| {
            let mut rules: HashSet<Rule> = HashSet::new();
            // let mut candidates: Vec<Rule> = Vec::new();
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
                    assert!(!rules.contains(&rule));
                    rules.insert(rule);
                }
            }
            let mut candidates = rules.clone();

            while !candidates.is_empty() {
                let next_gen = generate_itemset_rules(
                    itemset, &rules, min_confidence, min_lift, &itemset_support);
                for rule in next_gen.iter() {
                    rules.insert(rule.clone());
                }
                candidates = next_gen;
            }
            rules
        })
        .collect();

    let mut rules: HashSet<Rule> = HashSet::new();
    for set in rv.into_iter() {
        for rule in set {
            rules.insert(rule);
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index() {
        use index::Index;
        use super::ItemSet;
        use item::Item;
        use itemizer::Itemizer;
        use std::collections::HashMap;

        // HARM's census2.csv test dataset.

        // Load entire dataset into index.
        let mut index = Index::new();
        let transactions = vec![
            vec!["a", "b", "c"],
            vec!["d", "b", "c"],
            vec!["a", "b", "e"],
            vec!["f", "g", "c"],
            vec!["d", "g", "e"],
            vec!["f", "b", "c"],
            vec!["f", "b", "c"],
            vec!["a", "b", "e"],
            vec!["a", "b", "c"],
            vec!["a", "b", "e"],
            vec!["a", "b", "e"],
        ];
        let mut itemizer: Itemizer = Itemizer::new();
        for line in &transactions {
            let transaction = line.iter()
                .map(|s| itemizer.id_of(s))
                .collect::<Vec<Item>>();
            index.insert(&transaction);
        }

        let itemsets = [
            vec!["b", "e"],
            vec!["a", "e"],
            vec!["a", "b", "e"],
            vec!["f"],
            vec!["c", "f"],
            vec!["b", "f"],
            vec!["b", "c", "f"],
            vec!["g"],
            vec!["a"],
            vec!["a", "b"],
            vec!["b"],
            vec!["c"],
            vec!["b", "c"],
            vec!["c", "g"],
            vec!["d", "g"],
            vec!["d", "e", "g"],
            vec!["e", "g"],
            vec!["f", "g"],
            vec!["c", "f", "g"],
            vec!["a", "c"],
            vec!["a", "b", "c"],
            vec!["d"],
            vec!["b", "d"],
            vec!["c", "d"],
            vec!["b", "c", "d"],
            vec!["d", "e"],
            vec!["e"],
        ].iter()
            .map(|s| itemizer.to_id_vec(s))
            .map(|i| {
                ItemSet::new(
                    i.clone(),
                    (index.support(&i) * transactions.len() as f64) as u32,
                )
            })
            .collect::<Vec<ItemSet>>();

        let rules = super::generate_rules(&itemsets, transactions.len() as u32, 0.05, 1.0);

        let mut expected_rules: HashMap<&str, u32> = [
            ("a ==> b", 0),
            ("a ==> b e", 0),
            ("a ==> e", 0),
            ("a b ==> e", 0),
            ("a c ==> b", 0),
            ("a e ==> b", 0),
            ("b ==> a", 0),
            ("b ==> a e", 0),
            ("b ==> c", 0),
            ("b ==> c d", 0),
            ("b c ==> d", 0),
            ("b c ==> f", 0),
            ("b d ==> c", 0),
            ("b e ==> a", 0),
            ("b f ==> c", 0),
            ("c ==> b", 0),
            ("c ==> b d", 0),
            ("c ==> f", 0),
            ("c ==> f g", 0),
            ("c d ==> b", 0),
            ("c f ==> g", 0),
            ("c g ==> f", 0),
            ("d ==> b c", 0),
            ("d ==> e", 0),
            ("d ==> e g", 0),
            ("d ==> g", 0),
            ("d e ==> g", 0),
            ("d g ==> e", 0),
            ("e ==> a", 0),
            ("e ==> a b", 0),
            ("e ==> d", 0),
            ("e ==> d g", 0),
            ("e ==> g", 0),
            ("e g ==> d", 0),
            ("f ==> c", 0),
            ("f ==> c g", 0),
            ("f ==> g", 0),
            ("f g ==> c", 0),
            ("g ==> c f", 0),
            ("g ==> d", 0),
            ("g ==> d e", 0),
            ("g ==> e", 0),
            ("g ==> f", 0),
        ].iter()
            .cloned()
            .collect();

        assert_eq!(rules.len(), expected_rules.len());

        for rule_str in rules.iter().map(|r| r.to_string(&itemizer)) {
            assert_eq!(expected_rules.contains_key::<str>(&rule_str), true);
            if let Some(count) = expected_rules.get_mut::<str>(&rule_str) {
                *count += 1;
            }
        }

        for count in expected_rules.values() {
            assert_eq!(*count, 1);
        }
    }
}
