use std::collections::HashMap;
use item::Item;

pub struct Itemizer {
    next_item_id: u32,
    item_str_to_id: HashMap<String, Item>,
    item_id_to_str: HashMap<Item, String>,
}

impl Itemizer {
    pub fn new() -> Itemizer {
        Itemizer {
            next_item_id: 1,
            item_str_to_id: HashMap::new(),
            item_id_to_str: HashMap::new(),
        }
    }
    pub fn id_of(&mut self, item: &str) -> Item {
        if let Some(id) = self.item_str_to_id.get(item) {
            return *id;
        }
        let id = self.next_item_id;
        self.next_item_id += 1;
        self.item_str_to_id
            .insert(String::from(item), Item::with_id(id));
        self.item_id_to_str
            .insert(Item::with_id(id), String::from(item));
        Item::with_id(id)
    }
    pub fn str_of(&self, id: Item) -> String {
        match self.item_id_to_str.get(&id) {
            Some(s) => s.clone(),
            _ => String::from("Unknown"),
        }
    }
    #[cfg(test)]
    pub fn to_id_vec(&mut self, vec_of_str: &[&str]) -> Vec<Item> {
        vec_of_str.iter().map(|s| self.id_of(s)).collect()
    }
}
