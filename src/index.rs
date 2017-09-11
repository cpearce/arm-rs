use std::collections::HashSet;
use std::collections::HashMap;

pub struct Index {
    index: HashMap<u32, HashSet<usize>>,
    transaction_count: usize,
}

impl Index {
    pub fn new() -> Index {
        Index{ index: HashMap::new(), transaction_count: 0 }
    }
    pub fn insert(&mut self, transaction: &Vec<u32>) {
        let tid = self.transaction_count;
        self.transaction_count += 1;
        for &item_id in transaction {
            self.index.entry(item_id).or_insert(HashSet::new()).insert(tid);
        }
    }
    pub fn support(&self, transaction: &Vec<u32>) -> f64 {
        if transaction.is_empty() {
            return 0.0;
        }

        // Get the set of transaction id's containing the first item
        // in the transaction...
        let mut transaction_ids: HashSet<usize> = match self.index.get(&transaction[0]) {
            Some(transaction_ids) => transaction_ids.clone(),
            None => return 0.0,
        };

        // ... and intersect them with the all the sets of transaction ids
        // for all the other items...
        for tid in &transaction[1..] {
            transaction_ids = match self.index.get(&tid) {
                Some(other) => transaction_ids.intersection(&other).cloned().collect(),
                None => return 0.0, // None of this item; so support is 0!
            };
        }
        // ... and the support is the size of the set of transaction ids
        // that contain all items.
        (transaction_ids.len() as f64) / (self.transaction_count as f64)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index() {
        use super::Index;
        use super::Itemizer;

        let mut index = Index::new();
        let transactions = vec![
            vec!["a","b","c","d","e","f"],
            vec!["g","h","i","j","k","l"],
            vec!["z","x"],
            vec!["z","x"],
            vec!["z","x","y"],
            vec!["z","x","y","i"]
        ];
        let mut itemizer: Itemizer = Itemizer::new();
        for line in &transactions {
            let transaction = line.iter().map(|s| itemizer.id_of(s.trim()))
                                         .collect::<Vec<u32>>();
            index.insert(&transaction);
        }

        assert!(index.support(&vec![itemizer.id_of("a")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("b")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("c")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("d")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("e")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("f")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("h")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("i")]) == 2.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("j")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("k")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("l")]) == 1.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("z")]) == 4.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("x")]) == 4.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("y")]) == 2.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("x"), itemizer.id_of("z")]) == 4.0 / 6.0);
        assert!(index.support(&vec![itemizer.id_of("x"), itemizer.id_of("y"), itemizer.id_of("z")]) == 2.0 / 6.0);
    }
}
