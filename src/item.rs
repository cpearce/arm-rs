use itemizer::Itemizer;

#[derive(Copy, Clone, Hash, PartialOrd, PartialEq, Eq, Ord, Debug)]
pub struct Item {
    id: u32,
}

impl Item {
    pub fn null() -> Item {
        Item { id: 0 }
    }
    pub fn with_id(id: u32) -> Item {
        Item { id: id }
    }
    pub fn as_index(&self) -> usize {
        self.id as usize
    }
    pub fn is_null(&self) -> bool {
        self.id == 0
    }
    pub fn item_vec_to_string(items: &[Item], itemizer: &Itemizer) -> String {
        let mut a: Vec<&str> = items.iter().map(|&id| itemizer.str_of(id)).collect();
        ensure_sorted(&mut a);
        a.join(" ")
    }
}

// If all items in the itemset convert to an integer, order by that integer,
// otherwise order lexicographically.
fn ensure_sorted(a: &mut Vec<&str>) {
    let all_items_convert_to_ints = a.iter().all(|ref x| match x.parse::<u32>() {
        Ok(_) => true,
        Err(_) => false,
    });
    if all_items_convert_to_ints {
        a.sort_by(|ref x, ref y| {
            let _x = match x.parse::<u32>() {
                Ok(i) => i,
                Err(_) => 0,
            };
            let _y = match y.parse::<u32>() {
                Ok(i) => i,
                Err(_) => 0,
            };
            _x.cmp(&_y)
        });
    } else {
        a.sort();
    }
}
