use item::Item;
use itemizer::Itemizer;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::collections::HashMap;

#[derive(Clone, Hash, Eq, Debug)]
pub struct Rule {
    antecedent: Vec<Item>,
    consequent: Vec<Item>,
    confidence: OrderedFloat<f64>,
    lift: OrderedFloat<f64>,
    support: OrderedFloat<f64>,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Rule) -> bool {
        self.antecedent == other.antecedent && self.consequent == other.consequent
    }
}

// Assumes both itemsets are sorted.
fn union(a: &Vec<Item>, b: &Vec<Item>) -> Vec<Item> {
    let mut c: Vec<Item> = Vec::new();
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            c.push(a[ap]);
            ap += 1;
        } else if b[bp] < a[ap] {
            c.push(b[bp]);
            bp += 1;
        } else {
            c.push(a[ap]);
            ap += 1;
            bp += 1;
        }
    }
    while ap < a.len() {
        c.push(a[ap]);
        ap += 1;
    }
    while bp < b.len() {
        c.push(b[bp]);
        bp += 1;
    }
    c
}

// Assumes both itemsets are sorted.
fn intersection(a: &Vec<Item>, b: &Vec<Item>) -> Vec<Item> {
    let mut c: Vec<Item> = Vec::new();
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            ap += 1;
        } else if b[bp] < a[ap] {
            bp += 1;
        } else {
            // a[ap] == b[bp]
            c.push(a[ap]);
            ap += 1;
            bp += 1;
        }
    }
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
    pub fn make(
        antecedent: Vec<Item>,
        consequent: Vec<Item>,
        itemset_support: &HashMap<Vec<Item>, f64>,
        min_confidence: f64,
        min_lift: f64,
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
        if lift < min_lift {
            return None;
        }

        // Note: We sort the antecedent and consequent so that equality
        // tests are consistent.
        Some(Rule {
            antecedent: antecedent.iter().cloned().sorted(),
            consequent: consequent.iter().cloned().sorted(),
            confidence: OrderedFloat::from(confidence),
            lift: OrderedFloat::from(lift),
            support: OrderedFloat::from(ac_sup),
        })
    }

    // Creates a new Rule with:
    //  - the antecedent is the union of both rules' antecedents, and
    //  - the consequent is the intersection of both rules' consequents,
    // provided the new rule would be would be above the min_confidence threshold.
    pub fn merge(
        a: &Rule,
        b: &Rule,
        itemset_support: &HashMap<Vec<Item>, f64>,
        min_confidence: f64,
        min_lift: f64,
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

    pub fn union_size(&self) -> usize {
        self.antecedent.len() + self.consequent.len()
    }
}
