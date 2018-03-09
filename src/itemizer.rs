use std::collections::HashMap;
use item::Item;

pub struct Itemizer {
    next_item_id: u32,
    item_str_to_id: HashMap<String, Item>,
    item_id_to_str: Vec<String>,
}

impl Itemizer {
    pub fn new() -> Itemizer {
        Itemizer {
            next_item_id: 1,
            item_str_to_id: HashMap::new(),
            item_id_to_str: vec![],
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
        self.item_id_to_str.push(String::from(item));
        assert_eq!(self.item_id_to_str.len(), id as usize);
        assert_eq!(self.str_of(Item::with_id(id)), item);
        Item::with_id(id)
    }
    pub fn str_of(&self, id: Item) -> &str {
        &self.item_id_to_str[id.as_index() - 1]
    }
}
