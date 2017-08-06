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

// #[derive(Hash, Eq, PartialEq, Debug)]
// Parent node must outlive child nodes.
struct FPNode<'a> {
    parent: Option<&'a FPNode<'a>>,
    item: u32,
    count: u32,
    end_count: u32,
    children: HashMap<u32, FPNode<'a>>,
}

impl<'a> Hash for FPNode<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.item.hash(state);
    }
}

impl <'a> FPNode<'a> {
    fn insert(&mut self, transaction: &[u32]) {

    }
}

struct FPTree<'a> {
    root: FPNode<'a>,
}

impl <'a> FPTree<'a> {

    fn new() -> FPTree<'a> {
        let root_node = FPNode {
            parent: None,
            item: 0,
            count: 0,
            end_count: 0,
            children: HashMap::new()
        };
        return FPTree{ root: root_node };
    }

    fn insert(&mut self, transaction: &[u32]) {
        let mut node = &mut self.root;
        for item in transaction {
            if let Some(child) = node.children.get(item) {
                node = child;
                node.count += 1;
                continue;
            }
            // node.children.insert(item,)
            // let mut child
        }
        
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


fn run(path: &str) -> Result<(), Box<Error>> {
    let mut itemizer: Itemizer = Itemizer::new();
    let item_count = count_item_frequencies(&mut itemizer, path).unwrap();

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
        fptree.insert(&transaction);
    }


    Ok(())
}

fn main() {
    if let Err(err) = run("c:\\Users\\chris\\src\\rust\\arm\\datasets\\UCI-zoo.csv") {
        println!("error running example: {}", err);
        process::exit(1);
    }
}
