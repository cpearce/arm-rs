use std::error::Error;
use std::io::prelude::*;
use std::process;
use std::fs::File;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::cmp::Ordering;

fn read_records(itemizer: &mut Itemizer, path: &str) -> Vec<Vec<u32>> {
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let mut records = vec![];
    for line in contents.lines() {
        let mut record: Vec<u32> = vec![];
        for s in line.split(",") {
            let k = s.trim();
            record.push(itemizer.id_of(k));
        }
        if record.len() > 0 {
            records.push(record);
        }
    }
    records
}

fn count_item_frequencies(transactions: &Vec<Vec<u32>>) -> Result<HashMap<u32, u32>, Box<Error>> {
    let mut item_count: HashMap<u32, u32> = HashMap::new();
    for transaction in transactions {
        for item in transaction {
            let counter = item_count.entry(*item).or_insert(0);
            *counter += 1;
        }
    }
    Ok(item_count)
}

#[derive(Eq, PartialEq, Debug)]
struct FPNode {
    item: u32,
    count: u32,
    end_count: u32,
    children: HashMap<u32, FPNode>,
}

impl Hash for FPNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item.hash(state);
    }
}

// impl Eq for FPNode {
//     fn eq(&self, other: &FPNode) -> bool {
//         self.item == other.item;
//     }
// }

struct FPTree {
    root: FPNode,
    num_transactions: u32,
    item_count: HashMap<u32, u32>,
}

impl FPNode {
    fn new(item: u32) -> FPNode {
        FPNode {
            item: item,
            count: 0,
            end_count: 0,
            children: HashMap::new()
        }
    }

    fn insert(&mut self, transaction: &[u32], count: u32) {
        if transaction.len() == 0 {
            return;
        }
        // if transaction.len()
        let item = transaction[0];
        let mut child = self.children.entry(item).or_insert(FPNode::new(item));
        child.count += 1;
        if transaction.len() == 1 {
            child.end_count += count;
        } else {
            child.insert(&transaction[1 ..], count);
        }
    }

    fn is_root(&self) -> bool {
        self.item == 0
    }

    fn print(&self, itemizer: &Itemizer, item_count: &HashMap<u32, u32>, level: u32) {

        let mut items: Vec<u32> = self.children.keys().cloned().collect();
        sort_transaction(&mut items, item_count);

        for _ in 0 .. level {
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

    fn new() -> FPTree {
        let root_node = FPNode::new(0);
        return FPTree {
            root: root_node,
            num_transactions: 0,
            item_count: HashMap::new()
        };
    }

    fn insert(&mut self, transaction: &[u32], count: u32) {
        // Keep a count of item frequencies of what's in the
        // tree to make sorting later easier.
        for item in transaction {
            *self.item_count.entry(*item).or_insert(0) += count;
        }
        self.root.insert(&transaction, count);       
        self.num_transactions += 1; 
    }

    fn num_transactions(&self) -> u32 {
        self.num_transactions
    }

    fn root(&self) -> &FPNode {
        &self.root
    }

    fn item_count(&self) -> &HashMap<u32, u32> {
        &self.item_count
    }

    fn print(&self, itemizer: &Itemizer) {
        let mut items: Vec<u32> = self.root.children.keys().cloned().collect();
        sort_transaction(&mut items, &self.item_count);
        for item in items {
            if let Some(node) = self.root.children.get(&item) {
                node.print(itemizer, &self.item_count, 1);
            }
        }
    }
}


struct Itemizer {
    next_item_id: u32,
    item_str_to_id: HashMap<String, u32>,
    item_id_to_str: HashMap<u32,String>,
}

impl Itemizer {
    fn new() -> Itemizer {
        Itemizer {
            next_item_id: 1,
            item_str_to_id: HashMap::new(),
            item_id_to_str: HashMap::new(),
        }
    }
    fn id_of(&mut self, item: &str) -> u32 {
        if let Some(id) = self.item_str_to_id.get(item) {
            return *id;
        }
        let id = self.next_item_id;
        self.next_item_id += 1;
        self.item_str_to_id.insert(String::from(item), id);
        self.item_id_to_str.insert(id, String::from(item));
        return id;
    }
    fn str_of(&self, id: u32) -> String {
        match self.item_id_to_str.get(&id) {
            Some(s) => s.clone(),
            _ => String::from("Unknown"),
        }
    }
}

fn get_item_count(item: u32, item_count: &HashMap<u32, u32>) -> u32 {
    match item_count.get(&item) {
        Some(count) => *count,
        None => 0
    }
}

fn sort_transaction(transaction: &mut [u32],
                    item_count: &HashMap<u32, u32>) {
    transaction.sort_by(|a,b| {
        if a == b {
            return Ordering::Equal;
        }
        let a_count = get_item_count(*a, item_count);
        let b_count = get_item_count(*b, item_count);
        if b_count < a_count {
            return Ordering::Less;
        }
        Ordering::Greater
    });
    for i in 1..transaction.len() {
        assert!(get_item_count(transaction[i - 1], &item_count) >= get_item_count(transaction[i], &item_count));
    }
}


fn add_parents_to_table<'a>(node: &'a FPNode, table: &mut HashMap<&'a FPNode, &'a FPNode>) {
    for child in node.children.values() {
        table.insert(child, node);
        add_parents_to_table(child, table)
    }
}

fn make_parent_table<'a>(fptree: &'a FPTree) -> HashMap<&'a FPNode, &'a FPNode> {
    let mut table = HashMap::new();
    add_parents_to_table(fptree.root(), &mut table);
    table
}

fn add_nodes_to_index<'a>(node: &'a FPNode,
                          index: &mut HashMap<u32, Vec<&'a FPNode>>) {
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

fn path_from_root_to<'a>(node: &'a FPNode,
                    parent_table: &HashMap<&'a FPNode, &'a FPNode>)
                    -> Vec<u32> {
    let mut path = vec![];
    let mut n = node;
    loop {
        match parent_table.get(n) {
            Some(parent) if !parent.is_root() => {
                path.push(parent.item);
                n = parent;
                continue;
            },
            _ => { break; }
        }
    }
    path.reverse();
    path
}

fn construct_conditional_tree<'a>(parent_table: &HashMap<&'a FPNode, &'a FPNode>,
                                  item_list: &Vec<&'a FPNode>) -> FPTree {
    let mut conditional_tree = FPTree::new();

    for node in item_list {
        let path = path_from_root_to(node, parent_table);
        conditional_tree.insert(&path, node.count);
    }
    // let paths = item_index.g
    conditional_tree
}

fn fp_growth(fptree: &FPTree, min_count: u32, path: &[u32]) -> Vec<Vec<u32>> {
    // println!("fpgrowth path={:?}", path);
    let mut itemsets = vec![];

    // Maps a node to its parent.
    let parent_table = make_parent_table(&fptree);

    // Maps item id to vec of &FPNode's for those items.
    let item_index = make_item_index(&fptree);

    // Get list of items in the tree which are above the minimum support
    // threshold. Sort the list in increasing order of frequency.
    let mut items: Vec<u32> =
        item_index.keys()
                  .map(|x| *x)
                  .filter(|x|
                      get_item_count(*x, fptree.item_count()) > min_count)
                  .collect();
    sort_transaction(&mut items, fptree.item_count());
    items.reverse();

    // println!("fp_growth items {:?}", items);

    for item in items {
        // The path to here plus this item must be above the minimum
        // support threshold.
        let mut itemset: Vec<u32> = Vec::from(path);
        itemset.push(item);

        if let Some(item_list) = item_index.get(&item) {
            let conditional_tree =
                construct_conditional_tree(&parent_table,
                                           item_list);
            itemsets.extend(fp_growth(&conditional_tree, min_count, &itemset));
        };

        itemsets.push(itemset);

    }

    itemsets
}

fn run(path: &str) -> Result<(), Box<Error>> {
    println!("Entering run()");

    // Make one pass of the dataset to calculate the item frequencies
    // for the initial tree.
    let mut itemizer: Itemizer = Itemizer::new();
    let transactions = read_records(&mut itemizer, path);
    let item_count = count_item_frequencies(&transactions).unwrap();

    println!("Counted item frequencies.");

    // Load the initial tree.
    let mut fptree = FPTree::new();
    // For each record in file...
    for mut transaction in transactions {
        // Convert the transaction from a vector of strings to vector of
        // item ids.
        sort_transaction(&mut transaction, &item_count);
        fptree.insert(&transaction, 1);
    }

    println!("Loaded tree");
    fptree.print(&itemizer);

    println!("Starting mining");

    // let min_support = 0.2;
    let min_count = 2;//2 / fptree.num_transactions();

    let patterns = fp_growth(&fptree, min_count, &vec![]);


    println!("patterns: ({}) min_count={}", patterns.len(), min_count);
    for itemset in patterns {
        let mut pretty = vec![];
        for x in itemset {
            pretty.push(itemizer.str_of(x));
        }
        println!("{:?}", pretty);
    }

    Ok(())
}

fn main() {
    if let Err(err) = run("c:\\Users\\chris\\src\\rust\\arm\\datasets\\test.csv") {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
