extern crate csv;

use std::error::Error;
use std::io;
use std::process;
use std::fs::File;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};  


fn count_item_frequencies(itemizer: &mut Itemizer, path: &str) -> Result<HashMap<u32, u32>, Box<Error>> {
    // Build the CSV reader and iterate over each record.
    let mut item_count: HashMap<u32, u32> = HashMap::new();
    let mut file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here..
        let record = result?;
        for item in record {
            let counter = item_count.entry(itemizer.id_of(&item)).or_insert(0);
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

}


struct Itemizer {
    next_item_id: u32,
    item_str_to_id: HashMap<String, u32>,
}

impl Itemizer {
    fn new() -> Itemizer {
        Itemizer { next_item_id: 1, item_str_to_id: HashMap::new() }
    }
    fn id_of(&mut self, item: &str) -> u32 {
        if let Some(id) = self.item_str_to_id.get(item) {
            return *id;
        }
        let id = self.next_item_id;
        self.next_item_id += 1;
        self.item_str_to_id.insert(String::from(item), id);
        return id;
    }
}

fn get_item_count(item: u32, item_count: &HashMap<u32, u32>) -> u32 {
    match item_count.get(&item) {
        Some(count) => *count,
        None => 0
    }
}

fn partition_transaction(v: &mut [u32],
                        item_count: &HashMap<u32, u32>) -> usize {
  let pivot = v.len() - 1;
  let pivot_count = get_item_count(v[pivot], item_count);

  let mut split = 0; // everything before this is less than pivot.
  for index in 0..v.len() {
    if get_item_count(v[index], item_count) >= pivot_count {
      v.swap(split, index);
      split += 1;
    }
  }
  assert!(split > 0);
  split - 1
}


fn sort_transaction(transaction: &mut [u32],
                    item_count: &HashMap<u32, u32>) {
    if transaction.len() <= 1 {
        return;
    }
    let pivot = partition_transaction(transaction, item_count);
    sort_transaction(&mut transaction[0..pivot], item_count);
    sort_transaction(&mut transaction[pivot+1..], item_count);
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

fn construct_conditional_tree<'a>(fptree: &'a FPTree,
                                  item: u32,
                                  parent_table: &HashMap<&'a FPNode, &'a FPNode>,
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
                  .filter(|x| get_item_count(*x, fptree.item_count()) > min_count)
                  .collect();
    sort_transaction(&mut items, fptree.item_count());
    items.reverse();

    // println!("fp_growth items {:?}", items);

    for item in items {
        // The path to here plus this item must be above the minimum
        // support threshold.
        let mut itemset: Vec<u32> = Vec::from(path);
        itemset.push(item);

        let item_list = if let Some(item_list) = item_index.get(&item) {
            let conditional_tree =
                construct_conditional_tree(&fptree,
                                           item,
                                           &parent_table,
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
    let item_count = count_item_frequencies(&mut itemizer, path).unwrap();

    println!("Counted item frequencies.");

    // Load the initial tree.
    let mut fptree = FPTree::new();
    // For each record in file...
    let mut file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.records() {
        // Convert the transaction from a vector of strings to vector of
        // item ids.
        let record = result?;
        let mut transaction: Vec<u32> = vec![];
        for item in record {
            transaction.push(itemizer.id_of(&item));
        }
        sort_transaction(&mut transaction, &item_count);
        for i in 1..transaction.len() {
            assert!(get_item_count(transaction[i - 1], &item_count) >= get_item_count(transaction[i], &item_count));
        }
        fptree.insert(&transaction, 1);
    }

    println!("Starting mining");

    // let min_support = 0.2;
    let min_count = fptree.num_transactions() / 5;

    let patterns = fp_growth(&fptree, min_count, &vec![]);

    Ok(())
}

fn main() {
    if let Err(err) = run("c:\\Users\\chris\\src\\rust\\arm\\datasets\\test.csv") {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
