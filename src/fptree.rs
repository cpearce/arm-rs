// Copyright 2018 Chris Pearce
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use item::Item;
use item_counter::ItemCounter;
use itemizer::Itemizer;
use itertools::Itertools;
use rayon::prelude::*;
use std::cmp;
use std::hash::{Hash, Hasher};

#[derive(Eq, Debug)]
struct FPNode {
    id: usize,
    item: Item,
    count: u32,
    children: Vec<usize>,
    parent: usize,
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
    nodes: Vec<Vec<FPNode>>,
    item_count: ItemCounter,
    next_node_id: usize,
    item_lists: Vec<Vec<usize>>,
}

impl FPNode {
    fn new(id: usize, item: Item, parent: usize) -> FPNode {
        FPNode {
            id,
            item,
            count: 0,
            children: Vec::with_capacity(1),
            parent,
        }
    }

    fn is_root(&self) -> bool {
        self.item.is_null()
    }
}

static FPTREE_SPLAY: usize = 32;

impl FPTree {
    pub fn new() -> FPTree {
        let mut tree = FPTree {
            nodes: vec![],
            item_count: ItemCounter::new(),
            next_node_id: 0,
            item_lists: Vec::new(),
        };
        // Add root.
        tree.add_node(0, Item::null());
        return tree;
    }

    pub fn add_node(&mut self, parent: usize, item: Item) -> usize {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let (cohort, element) = self.sub_indicies_of(id);
        // Should only be at most 1 element too small.
        assert!(cohort <= self.nodes.len());
        if self.nodes.len() <= cohort {
            self.nodes.push(Vec::with_capacity(FPTREE_SPLAY));
        }
        assert!(element == self.nodes[cohort].len());
        self.nodes[cohort].push(FPNode::new(id, item, parent));
        assert!(self.get_node(id).item == item);
        self.get_node_mut(parent).children.push(id);
        self.add_to_item_list(item, id);
        id
    }

    fn add_to_item_list(&mut self, item: Item, id: usize) {
        if item.is_null() {
            return;
        }
        let index = item.as_index();
        if index >= self.item_lists.len() {
            self.item_lists.resize(index + 1, vec![]);
        }
        self.item_lists[index].push(id);
    }

    fn sub_indicies_of(&self, id: usize) -> (usize, usize) {
        (id / FPTREE_SPLAY, id % FPTREE_SPLAY)
    }

    fn get_node_mut(&mut self, id: usize) -> &mut FPNode {
        let (cohort, index) = self.sub_indicies_of(id);
        if cohort >= self.nodes.len() || index >= self.nodes[cohort].len() {
            panic!("Invalid node id")
        }
        &mut self.nodes[cohort][index]
    }

    fn get_node(&self, id: usize) -> &FPNode {
        let (cohort, index) = self.sub_indicies_of(id);
        if cohort >= self.nodes.len() || index >= self.nodes[cohort].len() {
            panic!("Invalid node id")
        }
        &self.nodes[cohort][index]
    }

    pub fn child_of(&self, id: usize, item: Item) -> Option<usize> {
        for &node_id in &self.get_node(id).children {
            if self.get_node(node_id).item == item {
                return Some(node_id);
            }
        }
        None
    }

    fn insert_child(&mut self, id: usize, item: Item, count: u32) -> usize {
        let child_id = match self.child_of(id, item) {
            Some(child_id) => child_id,
            None => self.add_node(id, item),
        };
        self.get_node_mut(child_id).count += count;
        child_id
    }

    pub fn insert(&mut self, transaction: &[Item], count: u32) {
        // Start iterating at the root node.
        let mut id = 0;
        for &item in transaction {
            // Keep a count of item frequencies of what's in the
            // tree to make sorting later easier.
            self.item_count.add(&item, count);
            // Add the item to the tree as a child of the previous node.
            id = self.insert_child(id, item, count);
        }
    }

    fn item_count(&self) -> &ItemCounter {
        &self.item_count
    }

    pub fn construct_conditional_tree(&self, item: Item) -> FPTree {
        let item_list = &self.item_lists[item.as_index()];
        let mut conditional_tree = FPTree::new();
        for &node_id in item_list {
            conditional_tree.insert(
                &self.path_from_root_to_excluding(node_id),
                self.get_node(node_id).count,
            );
        }
        conditional_tree
    }

    fn path_from_root_to_excluding(&self, node_id: usize) -> Vec<Item> {
        let mut path = vec![];
        let mut id = self.get_node(node_id).parent;
        loop {
            let node = self.get_node(id);
            if node.is_root() {
                break;
            }
            path.push(node.item);
            id = node.parent;
        }
        path.reverse();
        path
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, Ord)]
pub struct ItemSet {
    pub items: Vec<Item>,
    pub count: u32,
}

impl PartialOrd for ItemSet {
    fn partial_cmp(&self, other: &ItemSet) -> Option<cmp::Ordering> {
        if other.len() != self.len() {
            return Some(self.len().cmp(&other.len()));
        }
        Some(self.items.cmp(&other.items))
    }
}

impl ItemSet {
    pub fn new(items: Vec<Item>, count: u32) -> ItemSet {
        ItemSet {
            items: items.iter().map(|&x| x).sorted(),
            count: count,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub fn fp_growth(
    fptree: &FPTree,
    min_count: u32,
    path: &[Item],
    path_count: u32,
    itemizer: &Itemizer,
) -> Vec<ItemSet> {
    let mut itemsets: Vec<ItemSet> = vec![];

    // Get list of items in the tree which are above the minimum support
    // threshold.
    let items: Vec<Item> = fptree.item_count().items_with_count_at_least(min_count);

    let x: Vec<ItemSet> = items
        .par_iter()
        .flat_map(|item| -> Vec<ItemSet> {
            // The path to here plus this item must be above the minimum
            // support threshold.
            let mut itemset: Vec<Item> = Vec::from(path);
            let new_path_count = cmp::min(path_count, fptree.item_count().get(&item));
            itemset.push(*item);

            let conditional_tree = fptree.construct_conditional_tree(*item);
            let mut result = fp_growth(
                &conditional_tree,
                min_count,
                &itemset,
                new_path_count,
                itemizer,
            );

            result.push(ItemSet::new(itemset, new_path_count));
            result
        })
        .collect::<Vec<ItemSet>>();

    itemsets.extend(x);
    itemsets
}
