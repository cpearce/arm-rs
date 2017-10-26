#[derive(Copy, Clone, Hash, PartialOrd, PartialEq, Eq, Ord, Debug)]
pub struct Item {
    id: u32,
}

impl Item {
    pub fn null() -> Item {
        Item{id: 0}
    }
    pub fn with_id(id: u32) -> Item {
        Item{id: id}
    }
    pub fn as_index(&self) -> usize {
        self.id as usize
    }
    pub fn is_null(&self) -> bool {
        self.id == 0
    }
}
