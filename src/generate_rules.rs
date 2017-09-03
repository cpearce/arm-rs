use index::Index;
use itemizer::Itemizer;
use std::collections::HashSet;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};


#[derive(Eq, Clone, Debug)]
pub struct Rule {
    antecedent: HashSet<u32>,
    consequent: HashSet<u32>,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Rule) -> bool {
        self.antecedent == other.antecedent &&
        self.consequent == other.consequent
    }
}

impl Hash for Rule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let antecedent_mask : u64 = 1 << 63;
        let consequent_mask : u64 = 1 << 62;
        for i in &self.antecedent {
            (*i as u64 | antecedent_mask).hash(state);
        }
        for i in &self.consequent {
            (*i as u64 | consequent_mask).hash(state);
        }
    }
}

impl Rule {
    pub fn new(antecedent: HashSet<u32>, consequent: HashSet<u32>) -> Rule {
        Rule{antecedent: antecedent, consequent: consequent}
    }
    fn is_valid(&self) -> bool {
        !self.antecedent.is_empty() && !self.consequent.is_empty()
    }
    fn to_string(&self, itemizer: &Itemizer) -> String {
        let a: Vec<String> = self.antecedent.iter().map(|&id| itemizer.str_of(id)).collect();
        let b: Vec<String> = self.consequent.iter().map(|&id| itemizer.str_of(id)).collect();
        [a.join(","), " -> ".to_owned(), b.join(",")].join("")
    }
}

fn confidence(rule: &Rule, index: &Index) -> f64 {
    // Confidence of rule A->C defined as:
    // P(AC) / P(C)
    let ac: Vec<u32> = (&rule.antecedent | &rule.consequent).into_iter().collect();
    let a: Vec<u32> = rule.antecedent.iter().cloned().collect();
    index.support(&ac) / index.support(&a)
}

fn lift(rule: &Rule, index: &Index) -> f64 {
    // Lift of rule A->C defined as:
    // P(AC) / (P(A) * P(B))
    let a: Vec<u32> = rule.antecedent.iter().cloned().collect();
    let c: Vec<u32> = rule.consequent.iter().cloned().collect();
    let ac: Vec<u32> = (&rule.antecedent | &rule.consequent).into_iter().collect();
    index.support(&ac) / (index.support(&a) * index.support(&c))
}

pub fn generate_rules(itemsets: &Vec<Vec<u32>>,
                      min_confidence: f64,
                      min_lift: f64,
                      index: &Index) -> Vec<Rule> {
    let mut rules: Vec<Rule> = vec![];
    for itemset in itemsets.iter().filter(|i| i.len() > 1) {
        let mut candidates: Vec<Rule> = vec![];
        // First level candidates are all the rules with consequents of size 1.
        let items: HashSet<u32> = itemset.iter().cloned().collect();
        for &item in itemset {
            let antecedent: HashSet<u32> = [item].iter().cloned().collect();
            let consequent: HashSet<u32> = items.difference(&antecedent).cloned().collect();
            let rule = Rule::new(antecedent.clone(), consequent.clone());
            println!("Item={} items={:?} antecedent={:?} consequent={:?} conf={}",
                item, items, antecedent, consequent, confidence(&rule, &index));
            assert!(rule.is_valid());
            if confidence(&rule, &index) >= min_confidence {
                candidates.push(rule);
            }
        }

        // Subsequent generations are created by combining each rule with
        // every other, such that:
        //  - the antecedent is the intersection of both rules' antecedents, and
        //  - the consequent is the unions of both rules' consequents.
        let mut next_candidates: HashSet<Rule> = HashSet::new();
        while !candidates.is_empty() {
            for candidate_index in 0..candidates.len() {
                let candidate = &candidates[candidate_index];
                for other_index in (candidate_index + 1)..candidates.len() {
                    let other = &candidates[other_index];
                    let rule = Rule::new(&candidate.antecedent | &other.antecedent,
                                         &candidate.consequent & &other.consequent);
                    if rule.is_valid() && confidence(&rule, &index) >= min_confidence {
                        println!("Adding rule {:?} to next_candidates", rule);
                        next_candidates.insert(rule);                   
                    }
                }
            }
            // Move the previous generation into the output set.
            for r in candidates.into_iter().filter(|r| lift(&r, &index) >= min_lift) {
                rules.push(r);
            }
            // rules.append(&mut candidates);
            // assert!(candidates.is_empty()); 
            // Copy the current generation into the candidates list, so that we
            // use it to calculate the next generation. Note we filter by minimum
            // lift threshold here too.
            candidates = next_candidates.iter()
                                        .filter(|r| lift(&r, &index) >= min_lift)
                                        .cloned().collect();
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
            vec!["a","b","c"],
            vec!["d","b","c"],
            vec!["a","b","e"],
            vec!["f","g","c"],
            vec!["d","g","e"],
            vec!["f","b","c"],
            vec!["f","b","c"],
            vec!["a","b","e"],
            vec!["a","b","c"],
            vec!["a","b","e"],
            vec!["a","b","e"],
        ];
        let mut itemizer: Itemizer = Itemizer::new();
        for line in &transactions {
            let transaction = line.iter().map(|s| itemizer.id_of(s))
                                         .collect::<Vec<u32>>();
            index.insert(&transaction);
        }

        // Frequent itemsets generated by HARM with 
        //  -m fptree -minconf 0.05 -minlift 1 -minsup 0.05
        // (itemset, support)
        let itemsets = [
            vec!["a"],
            vec!["a","b"],
            vec!["b"],
            vec!["c"],
            vec!["b","c"],
            vec!["a","c"],
            vec!["a","b","c"],
            vec!["d"],
            vec!["b","d"],
            vec!["c","d"],
            vec!["b","c","d"],
            vec!["d","e"],
            vec!["e"],
            vec!["b","e"],
            vec!["a","e"],
            vec!["a","b","e"],
            vec!["f"],
            vec!["c","f"],
            vec!["b","f"],
            vec!["b","c","f"],
            vec!["g"],
            vec!["c","g"],
            vec!["d","g"],
            vec!["d","e","g"],
            vec!["e","g"],
            vec!["f","g"],
            vec!["c","f","g"],
        ].iter().map(|s| itemizer.to_id_vec(s))
                .collect::<Vec<Vec<u32>>>();

        let rules = super::generate_rules(&itemsets, 0.05, 1.0, &index);
        println!("Rules generated: {:?}", &rules);
        println!("Generated {} rules", rules.len());
        
        for rule in rules {
            println!("{} confidence={} lift={}",
                rule.to_string(&itemizer),
                super::confidence(&rule, &index),
                super::lift(&rule, &index));
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
