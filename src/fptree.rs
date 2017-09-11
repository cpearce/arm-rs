use itemizer::Itemizer;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Eq, Debug)]
struct FPNode {
    id: u32,
    item: u32,
    count: u32,
    children: HashMap<u32, FPNode>,
}

impl PartialEq for FPNode {
    fn eq(&self, other: &FPNode) -> bool {
        self.id == other.id
    }
}

impl Hash for FPNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct FPTree {
    root: FPNode,
    num_transactions: u32,
    item_count: HashMap<u32, u32>,
    node_count: u32,
}

impl FPNode {
    fn new(id: u32, item: u32) -> FPNode {
        FPNode {
            id: id,
            item: item,
            count: 0,
            children: HashMap::new(),
        }
    }

    fn insert(&mut self, transaction: &[u32], count: u32, next_node_id: u32) -> u32 {
        if transaction.len() == 0 {
            return 0;
        }

        let item = transaction[0];
        let mut new_nodes = 0;
        let mut child = self.children.entry(item).or_insert_with(|| {
            new_nodes += 1;
            FPNode::new(next_node_id, item)
        });

        child.count += count;
        if transaction.len() > 1 {
            new_nodes += child.insert(&transaction[1..], count, next_node_id + new_nodes);
        }
        new_nodes
    }

    fn is_root(&self) -> bool {
        self.item == 0
    }

    fn print(&self, itemizer: &Itemizer, item_count: &HashMap<u32, u32>, level: u32) {
        let mut items: Vec<u32> = self.children.keys().cloned().collect();
        sort_transaction(&mut items, item_count, SortOrder::Decreasing);

        for _ in 0..level {
            print!("  ");
        }
        println!("{}:{}", itemizer.str_of(self.item), self.count);
        for item in items {
            if let Some(node) = self.children.get(&item) {
                node.print(itemizer, item_count, level + 1);
            }
        }
    }
}

impl FPTree {
    pub fn new() -> FPTree {
        let root_node = FPNode::new(0, 0);
        return FPTree {
            root: root_node,
            num_transactions: 0,
            item_count: HashMap::new(),
            node_count: 1,
        };
    }

    pub fn insert(&mut self, transaction: &[u32], count: u32) {
        // Keep a count of item frequencies of what's in the
        // tree to make sorting later easier.
        for item in transaction {
            *self.item_count.entry(*item).or_insert(0) += count;
        }
        self.node_count += self.root.insert(&transaction, count, self.node_count);
        self.num_transactions += count;
    }

    pub fn num_transactions(&self) -> u32 {
        self.num_transactions
    }

    fn root(&self) -> &FPNode {
        &self.root
    }

    fn item_count(&self) -> &HashMap<u32, u32> {
        &self.item_count
    }

    pub fn print(&self, itemizer: &Itemizer) {
        let mut items: Vec<u32> = self.root.children.keys().cloned().collect();
        sort_transaction(&mut items, &self.item_count, SortOrder::Decreasing);
        for item in items {
            if let Some(node) = self.root.children.get(&item) {
                node.print(itemizer, &self.item_count, 1);
            }
        }
    }
}

fn get_item_count(item: u32, item_count: &HashMap<u32, u32>) -> u32 {
    match item_count.get(&item) {
        Some(count) => *count,
        None => 0,
    }
}

pub enum SortOrder {
    Increasing,
    Decreasing,
}

fn item_cmp(a: &u32, b: &u32, item_count: &HashMap<u32, u32>) -> Ordering {
    let a_count = get_item_count(*a, item_count);
    let b_count = get_item_count(*b, item_count);
    if a_count == b_count {
        return a.cmp(b);
    }
    a_count.cmp(&b_count)
}

pub fn sort_transaction(transaction: &mut [u32], item_count: &HashMap<u32, u32>, order: SortOrder) {
    match order {
        SortOrder::Increasing => transaction.sort_by(|a, b| item_cmp(a, b, item_count)),
        SortOrder::Decreasing => transaction.sort_by(|a, b| item_cmp(b, a, item_count)),
    }
}

fn add_parents_to_table<'a>(node: &'a FPNode, table: &mut HashMap<&'a FPNode, &'a FPNode>) {
    for child in node.children.values() {
        assert!(!table.contains_key(child));
        table.insert(child, node);
        add_parents_to_table(child, table)
    }
}

fn make_parent_table<'a>(fptree: &'a FPTree) -> HashMap<&'a FPNode, &'a FPNode> {
    let mut table = HashMap::new();
    add_parents_to_table(fptree.root(), &mut table);
    table
}

fn add_nodes_to_index<'a>(node: &'a FPNode, index: &mut HashMap<u32, Vec<&'a FPNode>>) {
    for child in node.children.values() {
        index.entry(child.item).or_insert(vec![]).push(&child);
        add_nodes_to_index(child, index)
    }
}

fn make_item_index<'a>(fptree: &'a FPTree) -> HashMap<u32, Vec<&'a FPNode>> {
    let mut index = HashMap::new();
    add_nodes_to_index(fptree.root(), &mut index);
    index
}

fn path_from_root_to<'a>(
    node: &'a FPNode,
    parent_table: &HashMap<&'a FPNode, &'a FPNode>,
) -> Vec<u32> {
    let mut path = vec![];
    let mut n = node;
    loop {
        match parent_table.get(n) {
            Some(parent) if !parent.is_root() => {
                path.push(parent.item);
                n = parent;
                continue;
            }
            _ => {
                break;
            }
        }
    }
    path.reverse();
    path
}

fn construct_conditional_tree<'a>(
    parent_table: &HashMap<&'a FPNode, &'a FPNode>,
    item_list: &Vec<&'a FPNode>,
) -> FPTree {
    let mut conditional_tree = FPTree::new();

    for node in item_list {
        let path = path_from_root_to(node, parent_table);
        conditional_tree.insert(&path, node.count);
    }
    conditional_tree
}

pub fn fp_growth(
    fptree: &FPTree,
    min_count: u32,
    path: &[u32],
    itemizer: &Itemizer,
) -> Vec<Vec<u32>> {
    let mut itemsets: Vec<Vec<u32>> = vec![];

    // Maps a node to its parent.
    let parent_table = make_parent_table(&fptree);

    // Maps item id to vec of &FPNode's for those items.
    let item_index = make_item_index(&fptree);

    // Get list of items in the tree which are above the minimum support
    // threshold. Sort the list in increasing order of frequency.
    let mut items: Vec<u32> = item_index
        .keys()
        .map(|x| *x)
        .filter(|x| get_item_count(*x, fptree.item_count()) > min_count)
        .collect();
    sort_transaction(&mut items, fptree.item_count(), SortOrder::Increasing);

    let x: Vec<Vec<u32>> = items
        .par_iter()
        .flat_map(|item| -> Vec<Vec<u32>> {
            // The path to here plus this item must be above the minimum
            // support threshold.
            let mut itemset: Vec<u32> = Vec::from(path);
            itemset.push(*item);

            let mut result: Vec<Vec<u32>> = Vec::new();

            if let Some(item_list) = item_index.get(item) {
                let conditional_tree = construct_conditional_tree(&parent_table, item_list);
                let mut y = fp_growth(&conditional_tree, min_count, &itemset, itemizer);
                result.append(&mut y);
            };
            result.push(itemset);
            result
        })
        .collect::<Vec<Vec<u32>>>();

    itemsets.extend(x);
    itemsets
}
