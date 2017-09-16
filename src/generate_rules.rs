use index::Index;
use itemizer::Itemizer;
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct Rule {
    antecedent: Vec<u32>,
    consequent: Vec<u32>,
    confidence: f64,
    lift: f64,
    support: f64,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Rule) -> bool {
        self.antecedent == other.antecedent && self.consequent == other.consequent
    }
}

fn union(a: &Vec<u32>, b: &Vec<u32>) -> Vec<u32> {
    let mut c: Vec<u32> = Vec::new();
    for &i in a.iter() {
        c.push(i);
    }
    for &i in b.iter() {
        if !c.contains(&i) {
            c.push(i);
        }
    }
    c.sort();
    c
}

fn intersection(a: &Vec<u32>, b: &Vec<u32>) -> Vec<u32> {
    let mut c: Vec<u32> = Vec::new();
    for &i in a.iter() {
        if b.contains(&i) {
            c.push(i);
        }
    }
    c.sort();
    c
}

// If all items in the itemset convert to an integer, order by that integer,
// otherwise order lexicographically.
fn ensure_sorted(a: &mut Vec<String>) {
    let all_items_convert_to_ints = a.iter().all(|ref x| match x.parse::<u32>() {
        Ok(_) => true,
        Err(_) => false,
    });
    if all_items_convert_to_ints {
        a.sort_by(|ref x, ref y| {
            let _x = match x.parse::<u32>() {
                Ok(i) => i,
                Err(_) => 0,
            };
            let _y = match y.parse::<u32>() {
                Ok(i) => i,
                Err(_) => 0,
            };
            _x.cmp(&_y)
        });
    } else {
        a.sort();
    }
}

impl Rule {
    pub fn to_string(&self, itemizer: &Itemizer) -> String {
        let mut a: Vec<String> = self.antecedent
            .iter()
            .map(|&id| itemizer.str_of(id))
            .collect();
        ensure_sorted(&mut a);
        let mut b: Vec<String> = self.consequent
            .iter()
            .map(|&id| itemizer.str_of(id))
            .collect();
        ensure_sorted(&mut b);
        [a.join(" "), " ==> ".to_owned(), b.join(" ")].join("")
    }

    // Creates a new Rule from (antecedent,consequent) if the rule
    // would be above the min_confidence threshold.
    fn make(
        antecedent: Vec<u32>,
        consequent: Vec<u32>,
        index: &Index,
        min_confidence: f64,
    ) -> Option<Rule> {
        if antecedent.is_empty() || consequent.is_empty() {
            return None;
        }

        let ac_vec: Vec<u32> = union(&antecedent, &consequent);
        let ac_sup = index.support(&ac_vec);
        let a_sup = index.support(&antecedent);
        let confidence = ac_sup / a_sup;
        if confidence < min_confidence {
            return None;
        }

        let c_sup = index.support(&consequent);
        let lift = ac_sup / (a_sup * c_sup);

        // Note: We sort the antecedent and consequent so that equality
        // tests are consistent.
        Some(Rule {
            antecedent: antecedent.iter().cloned().sorted(),
            consequent: consequent.iter().cloned().sorted(),
            confidence: confidence,
            lift: lift,
            support: ac_sup,
        })
    }

    // Creates a new Rule with:
    //  - the antecedent is the union of both rules' antecedents, and
    //  - the consequent is the intersection of both rules' consequents,
    // provided the new rule would be would be above the min_confidence threshold.
    fn merge(a: &Rule, b: &Rule, index: &Index, min_confidence: f64) -> Option<Rule> {
        let antecedent = union(&a.antecedent, &b.antecedent);
        let consequent = intersection(&a.consequent, &b.consequent);
        Rule::make(antecedent, consequent, &index, min_confidence)
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn lift(&self) -> f64 {
        self.lift
    }

    pub fn support(&self) -> f64 {
        self.support
    }
}

pub fn generate_rules(
    itemsets: &Vec<Vec<u32>>,
    min_confidence: f64,
    min_lift: f64,
    index: &Index,
) -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    for itemset in itemsets.iter().filter(|i| i.len() > 1) {
        let mut candidates: Vec<Rule> = Vec::new();
        // First level candidates are all the rules with consequents of size 1.
        for &item in itemset {
            let antecedent: Vec<u32> = vec![item];
            let consequent: Vec<u32> = itemset.iter().filter(|&&x| x != item).cloned().collect();
            if let Some(rule) = Rule::make(antecedent, consequent, &index, min_confidence) {
                candidates.push(rule);
            }
        }

        // Subsequent generations are created by merging with each other rule.
        let mut next_candidates: Vec<Rule> = Vec::new();
        while !candidates.is_empty() {
            for (candidate, other) in candidates.iter().tuple_combinations() {
                // Try combining all pairs of the last generation's candidates
                // together. If the new rule is below the minimum confidence
                // threshold, the merge will fail, and we'll not keep the new
                // rule.
                if let Some(rule) = Rule::merge(&candidate, &other, &index, min_confidence) {
                    if !next_candidates.contains(&rule) {
                        next_candidates.push(rule);
                    }
                }
            }

            // Move the previous generation into the output set, provided the lift
            // constraint is satisfied.
            for r in candidates.into_iter().filter(|r| r.lift >= min_lift) {
                if !rules.contains(&r) {
                    rules.push(r);
                }
            }

            // Copy the current generation into the candidates list, so that we
            // use it to calculate the next generation. Note we filter by minimum
            // lift threshold here too.
            candidates = next_candidates
                .iter()
                .filter(|r| r.lift >= min_lift)
                .cloned()
                .collect();

            next_candidates.clear();
        }
    }

    rules
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index() {
        use super::Index;
        use super::Itemizer;
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
            let transaction = line.iter().map(|s| itemizer.id_of(s)).collect::<Vec<u32>>();
            index.insert(&transaction);
        }

        // Frequent itemsets generated by HARM with
        //  -m fptree -minconf 0.05 -minlift 1 -minsup 0.05
        // (itemset, support)
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
            .collect::<Vec<Vec<u32>>>();

        let rules = super::generate_rules(&itemsets, 0.05, 1.0, &index);

        let mut expected_rules: HashMap<&str, u32> = [
            ("a ==> b", 0),
            ("a ==> b e", 0),
            ("a ==> e", 0),
            ("a b ==> e", 0),
            ("a c ==> b", 0),
            ("a e ==> b", 0),
            ("b ==> a", 0),
            ("b ==> a c", 0),
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
            ("c ==> b f", 0),
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
            ("f ==> b c", 0),
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
