use std::collections::HashMap;

pub struct Itemizer {
    next_item_id: u32,
    item_str_to_id: HashMap<String, u32>,
    item_id_to_str: HashMap<u32, String>,
}

impl Itemizer {
    pub fn new() -> Itemizer {
        Itemizer {
            next_item_id: 1,
            item_str_to_id: HashMap::new(),
            item_id_to_str: HashMap::new(),
        }
    }
    pub fn id_of(&mut self, item: &str) -> u32 {
        if let Some(id) = self.item_str_to_id.get(item) {
            return *id;
        }
        let id = self.next_item_id;
        self.next_item_id += 1;
        self.item_str_to_id.insert(String::from(item), id);
        self.item_id_to_str.insert(id, String::from(item));
        return id;
    }
    pub fn str_of(&self, id: u32) -> String {
        match self.item_id_to_str.get(&id) {
            Some(s) => s.clone(),
            _ => String::from("Unknown"),
        }
    }
    #[cfg(test)]
    pub fn to_id_vec(&mut self, vec_of_str: &[&str]) -> Vec<u32> {
        vec_of_str.iter().map(|s| self.id_of(s)).collect()
    }
}
