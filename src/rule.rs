use item::Item;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use fnv::FnvHashMap;

#[derive(Clone, Hash, Eq, Debug)]
pub struct Rule {
    pub antecedent: Vec<Item>,
    pub consequent: Vec<Item>,
    pub confidence: OrderedFloat<f64>,
    pub lift: OrderedFloat<f64>,
    pub support: OrderedFloat<f64>,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Rule) -> bool {
        self.antecedent == other.antecedent && self.consequent == other.consequent
    }
}

// Assumes both itemsets are sorted.
pub fn union(a: &[Item], b: &[Item]) -> Vec<Item> {
    // Count the length required in the union, to avoid
    // paying for reallocations while pushing onto the end.
    let mut count = 0;
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            count += 1;
            ap += 1;
        } else if b[bp] < a[ap] {
            count += 1;
            bp += 1;
        } else {
            count += 1;
            ap += 1;
            bp += 1;
        }
    }
    count += a.len() - ap;
    count += b.len() - bp;

    let mut c: Vec<Item> = Vec::with_capacity(count);
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
pub fn intersection(a: &[Item], b: &[Item]) -> Vec<Item> {
    // Count the length required in the intersection, to avoid
    // paying for reallocations while pushing onto the end.
    let mut count = 0;
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            ap += 1;
        } else if b[bp] < a[ap] {
            bp += 1;
        } else {
            count += 1;
            ap += 1;
            bp += 1;
        }
    }

    let mut c: Vec<Item> = Vec::with_capacity(count);
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            ap += 1;
        } else if b[bp] < a[ap] {
            bp += 1;
        } else {
            c.push(a[ap]);
            ap += 1;
            bp += 1;
        }
    }
    c
}

impl Rule {
    // Creates a new Rule from (antecedent,consequent) if the rule
    // would be above the min_confidence threshold.
    pub fn make(
        antecedent: Vec<Item>,
        consequent: Vec<Item>,
        itemset_support: &FnvHashMap<Vec<Item>, f64>,
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

        let mut antecedent = antecedent;
        antecedent.sort();
        let mut consequent = consequent;
        consequent.sort();

        // Note: We sort the antecedent and consequent so that equality
        // tests are consistent.
        Some(Rule {
            antecedent,
            consequent,
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
        itemset_support: &FnvHashMap<Vec<Item>, f64>,
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
}

#[cfg(test)]
mod tests {
    use super::Item;
    fn to_item_vec(nums: &[u32]) -> Vec<Item> {
        nums.iter().map(|i| Item::with_id(*i)).collect()
    }

    #[test]
    fn test_union() {
        use super::union;

        let test_cases: Vec<(Vec<Item>, Vec<Item>, Vec<Item>)> = [
            (vec![1, 2, 3], vec![4, 5, 6], vec![1, 2, 3, 4, 5, 6]),
            (vec![1, 2, 3], vec![3, 4, 5, 6], vec![1, 2, 3, 4, 5, 6]),
            (vec![], vec![1], vec![1]),
            (vec![1], vec![], vec![1]),
        ].iter()
            .map(|&(ref a, ref b, ref u)| (to_item_vec(a), to_item_vec(b), to_item_vec(u)))
            .collect();

        for &(ref a, ref b, ref c) in &test_cases {
            assert_eq!(&union(&a, &b), c);
        }
    }

    #[test]
    fn test_intersection() {
        use super::intersection;

        let test_cases: Vec<(Vec<Item>, Vec<Item>, Vec<Item>)> = [
            (vec![1], vec![1], vec![1]),
            (vec![1, 2, 3, 4, 5], vec![4, 5, 6], vec![4, 5]),
            (vec![1, 2, 3], vec![3, 4, 5, 6], vec![3]),
            (vec![], vec![1], vec![]),
            (vec![1], vec![], vec![]),
        ].iter()
            .map(|&(ref a, ref b, ref u)| (to_item_vec(a), to_item_vec(b), to_item_vec(u)))
            .collect();

        for &(ref a, ref b, ref c) in &test_cases {
            assert_eq!(&intersection(&a, &b), c);
        }
    }
}
