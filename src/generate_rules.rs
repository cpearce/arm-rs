use index::Index;
use itemizer::Itemizer;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};


#[derive(Clone, Debug)]
pub struct Rule {
    antecedent: HashSet<u32>,
    consequent: HashSet<u32>,
    confidence: f64,
    lift: f64,
    support: f64,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Rule) -> bool {
        self.antecedent == other.antecedent && self.consequent == other.consequent
    }
}

// Can't derive Eq as f64 doesn't satisfy Eq.
impl Eq for Rule {}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let antecedent_mask: u64 = 1 << 63;
        let consequent_mask: u64 = 1 << 62;
        for i in &self.antecedent {
            (*i as u64 | antecedent_mask).hash(state);
        }
        for i in &self.consequent {
            (*i as u64 | consequent_mask).hash(state);
        }
    }
}

impl Rule {
    pub fn to_string(&self, itemizer: &Itemizer) -> String {
        let mut a: Vec<String> = self.antecedent
            .iter()
            .map(|&id| itemizer.str_of(id))
            .collect();
        a.sort();
        let mut b: Vec<String> = self.consequent
            .iter()
            .map(|&id| itemizer .str_of(id))
            .collect();
        b.sort();
        [a.join(" "), " => ".to_owned(), b.join(" ")].join("")
    }

    // Creates a new Rule from (antecedent,consequent) if the rule
    // would be above the min_confidence threshold.
    fn make(
        antecedent: HashSet<u32>,
        consequent: HashSet<u32>,
        index: &Index,
        min_confidence: f64,
    ) -> Option<Rule> {
        if antecedent.is_empty() || consequent.is_empty() {
            return None;
        }

        // TODO: I can just pass the HashSet to index::support().
        let ac_vec: Vec<u32> = (&antecedent | &consequent).into_iter().collect();
        let a_vec: Vec<u32> = antecedent.iter().cloned().collect();
        let ac_sup = index.support(&ac_vec);
        let a_sup = index.support(&a_vec);
        let confidence = ac_sup / a_sup;
        if confidence < min_confidence {
            return None;
        }

        let c_vec = consequent.iter().cloned().collect();
        let c_sup = index.support(&c_vec);
        let lift = ac_sup / (a_sup * c_sup);

        Some(Rule {
            antecedent: antecedent,
            consequent: consequent,
            confidence: confidence,
            lift: lift,
            support: ac_sup,
        })
    }

    // Creates a new Rule with:
    //  - the antecedent is the intersection of both rules' antecedents, and
    //  - the consequent is the union of both rules' consequents,
    // provided the new rule would be would be above the min_confidence threshold.
    fn merge(a: &Rule, b: &Rule, index: &Index, min_confidence: f64) -> Option<Rule> {
        let antecedent = &a.antecedent & &b.antecedent;
        let consequent = &a.consequent | &b.consequent;
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
    let mut rules: Vec<Rule> = vec![];
    for itemset in itemsets.iter().filter(|i| i.len() > 1) {
        let mut candidates: Vec<Rule> = vec![];
        // First level candidates are all the rules with consequents of size 1.
        let items: HashSet<u32> = itemset.iter().cloned().collect();
        for &item in itemset {
            let antecedent: HashSet<u32> = [item].iter().cloned().collect();
            let consequent: HashSet<u32> = items.difference(&antecedent).cloned().collect();
            if let Some(rule) = Rule::make(antecedent, consequent, &index, min_confidence) {
                candidates.push(rule);
            }
        }

        // Subsequent generations are created by merging with each other rule.
        let mut next_candidates: HashSet<Rule> = HashSet::new();
        while !candidates.is_empty() {
            for candidate_index in 0..candidates.len() {
                for other_index in (candidate_index + 1)..candidates.len() {
                    if let Some(rule) = Rule::merge(
                        &candidates[candidate_index],
                        &candidates[other_index],
                        &index,
                        min_confidence,
                    ) {
                        next_candidates.insert(rule);
                    }
                }
            }
            // Move the previous generation into the output set.
            for r in candidates.into_iter().filter(|r| r.lift >= min_lift) {
                rules.push(r);
            }
            // rules.append(&mut candidates);
            // assert!(candidates.is_empty());
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
            vec!["a"],
            vec!["a", "b"],
            vec!["b"],
            vec!["c"],
            vec!["b", "c"],
            vec!["a", "c"],
            vec!["a", "b", "c"],
            vec!["d"],
            vec!["b", "d"],
            vec!["c", "d"],
            vec!["b", "c", "d"],
            vec!["d", "e"],
            vec!["e"],
            vec!["b", "e"],
            vec!["a", "e"],
            vec!["a", "b", "e"],
            vec!["f"],
            vec!["c", "f"],
            vec!["b", "f"],
            vec!["b", "c", "f"],
            vec!["g"],
            vec!["c", "g"],
            vec!["d", "g"],
            vec!["d", "e", "g"],
            vec!["e", "g"],
            vec!["f", "g"],
            vec!["c", "f", "g"],
        ].iter()
            .map(|s| itemizer.to_id_vec(s))
            .collect::<Vec<Vec<u32>>>();

        let rules = super::generate_rules(&itemsets, 0.05, 1.0, &index);
        println!("Rules generated: {:?}", &rules);
        println!("Generated {} rules", rules.len());

        for rule in rules {
            println!(
                "{} confidence={} lift={}",
                rule.to_string(&itemizer),
                rule.confidence(),
                rule.lift()
            );
        }

        /*
a, -> e, conf=0.6666666666666667 lift=1.4666666666666668 support=0.36363636363636365
a, -> b, conf=1.0 lift=1.222222222222222 support=0.5454545454545454
a,b, -> e, conf=0.6666666666666667 lift=1.4666666666666668 support=0.36363636363636365
a,c, -> b, conf=1.0 lift=1.222222222222222 support=0.18181818181818182
a,e, -> b, conf=1.0 lift=1.222222222222222 support=0.36363636363636365
b, -> c,d, conf=0.1111111111111111 lift=1.222222222222222 support=0.09090909090909091
b, -> a,e, conf=0.4444444444444444 lift=1.222222222222222 support=0.36363636363636365
b, -> a, conf=0.6666666666666666 lift=1.2222222222222223 support=0.5454545454545454
b, -> c, conf=0.5555555555555555 lift=1.0185185185185184 support=0.45454545454545453
b, -> a,c, conf=0.2222222222222222 lift=1.222222222222222 support=0.18181818181818182
b,c, -> d, conf=0.2 lift=1.1 support=0.09090909090909091
b,c, -> f, conf=0.4 lift=1.4666666666666668 support=0.18181818181818182
b,d, -> c, conf=1.0 lift=1.8333333333333335 support=0.09090909090909091
b,e, -> a, conf=1.0 lift=1.8333333333333335 support=0.36363636363636365
b,f, -> c, conf=1.0 lift=1.8333333333333335 support=0.18181818181818182
c, -> b,d, conf=0.16666666666666669 lift=1.8333333333333335 support=0.09090909090909091
c, -> f,g, conf=0.16666666666666669 lift=1.8333333333333335 support=0.09090909090909091
c, -> f, conf=0.5 lift=1.8333333333333335 support=0.2727272727272727
c, -> b,f, conf=0.33333333333333337 lift=1.8333333333333335 support=0.18181818181818182
c, -> b, conf=0.8333333333333334 lift=1.0185185185185186 support=0.45454545454545453
c,d, -> b, conf=1.0 lift=1.222222222222222 support=0.09090909090909091
c,f, -> g, conf=0.33333333333333337 lift=1.8333333333333335 support=0.09090909090909091
c,g, -> f, conf=1.0 lift=3.666666666666667 support=0.09090909090909091
d, -> b,c, conf=0.5 lift=1.1 support=0.09090909090909091
d, -> e, conf=0.5 lift=1.1 support=0.09090909090909091
d, -> g, conf=0.5 lift=2.75 support=0.09090909090909091
d, -> e,g, conf=0.5 lift=5.5 support=0.09090909090909091
d,e, -> g, conf=1.0 lift=5.5 support=0.09090909090909091
d,g, -> e, conf=1.0 lift=2.2 support=0.09090909090909091
e, -> d, conf=0.2 lift=1.1 support=0.09090909090909091
e, -> g, conf=0.2 lift=1.1 support=0.09090909090909091
e, -> d,g, conf=0.2 lift=2.2 support=0.09090909090909091
e, -> a,b, conf=0.8 lift=1.4666666666666668 support=0.36363636363636365
e, -> a, conf=0.8 lift=1.4666666666666668 support=0.36363636363636365
e,g, -> d, conf=1.0 lift=5.5 support=0.09090909090909091
f, -> g, conf=0.33333333333333337 lift=1.8333333333333335 support=0.09090909090909091
f, -> c,g, conf=0.33333333333333337 lift=3.666666666666667 support=0.09090909090909091
f, -> c, conf=1.0 lift=1.8333333333333335 support=0.2727272727272727
f, -> b,c, conf=0.6666666666666667 lift=1.4666666666666668 support=0.18181818181818182
f,g, -> c, conf=1.0 lift=1.8333333333333335 support=0.09090909090909091
g, -> e, conf=0.5 lift=1.1 support=0.09090909090909091
g, -> d, conf=0.5 lift=2.75 support=0.09090909090909091
g, -> d,e, conf=0.5 lift=5.5 support=0.09090909090909091
g, -> f, conf=0.5 lift=1.8333333333333335 support=0.09090909090909091
g, -> c,f, conf=0.5 lift=1.8333333333333335 support=0.09090909090909091
        */
    }
}
