#[cfg(test)]
use itemizer::Itemizer;
use item::Item;

pub struct Index {
    index: Vec<Vec<usize>>,
    transaction_count: usize,
}

impl Index {
    pub fn new() -> Index {
        Index {
            index: Vec::new(),
            transaction_count: 0,
        }
    }
    pub fn insert(&mut self, transaction: &[Item]) {
        let tid = self.transaction_count;
        self.transaction_count += 1;
        for &item in transaction {
            while self.index.len() <= item.as_index() {
                self.index.push(vec![]);
            }
            self.index[item.as_index()].push(tid);
        }
    }

    pub fn count(&self, transaction: &[Item]) -> usize {
        if transaction.is_empty() {
            return 0;
        }

        if transaction.len() == 1 {
            let item_index = transaction[0].as_index();
            if item_index >= self.index.len() {
                return 0;
            }
            return self.index[item_index].len();
        }

        let mut tid_lists: Vec<&Vec<usize>> = vec![];
        for &item in transaction.iter() {
            tid_lists.push(&self.index[item.as_index()]);
        }

        let mut p: Vec<usize> = vec![0; tid_lists.len()];

        // For each tid in the transaction's first item's list of tids.
        let mut count = 0;
        for &tid in tid_lists[0].iter() {
            // Check whether all the other tid lists contain that tid.
            let mut tid_in_all_item_tid_lists = true;
            for i in 1..tid_lists.len() {
                while p[i] < tid_lists[i].len() && tid_lists[i][p[i]] < tid {
                    p[i] += 1;
                }
                if p[i] == tid_lists[i].len() || tid_lists[i][p[i]] != tid {
                    // This tidlist doesn't include that tid. So this tid cannot
                    // have all items in it.
                    tid_in_all_item_tid_lists = false;
                    break;
                }
            }
            if tid_in_all_item_tid_lists {
                count += 1
            }
        }

        count
    }

    #[allow(dead_code)]
    pub fn support(&self, transaction: &[Item]) -> f64 {
        let count = self.count(transaction);
        (count as f64) / (self.transaction_count as f64)
    }

    pub fn num_transactions(&self) -> usize {
        self.transaction_count
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index() {
        use super::Index;
        use super::Itemizer;
        use super::Item;

        let mut index = Index::new();
        let transactions = vec![
            vec!["a", "b", "c", "d", "e", "f"],
            vec!["g", "h", "i", "j", "k", "l"],
            vec!["z", "x"],
            vec!["z", "x"],
            vec!["z", "x", "y"],
            vec!["z", "x", "y", "i"],
        ];
        let mut itemizer: Itemizer = Itemizer::new();
        for line in &transactions {
            let transaction = line.iter()
                .map(|s| itemizer.id_of(s.trim()))
                .collect::<Vec<Item>>();
            index.insert(&transaction);
        }

        assert_eq!(index.support(&vec![itemizer.id_of("a")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("b")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("c")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("d")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("e")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("f")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("h")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("i")]), 2.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("j")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("k")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("l")]), 1.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("z")]), 4.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("x")]), 4.0 / 6.0);
        assert_eq!(index.support(&vec![itemizer.id_of("y")]), 2.0 / 6.0);
        assert_eq!(
            index.support(&vec![itemizer.id_of("x"), itemizer.id_of("z")]),
            4.0 / 6.0
        );
        assert!(
            index.support(&vec![
                itemizer.id_of("x"),
                itemizer.id_of("y"),
                itemizer.id_of("z"),
            ]) == 2.0 / 6.0
        );
    }
}
