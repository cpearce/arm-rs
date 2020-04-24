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

// Assumes both vectors are sorted.
pub fn union<T>(a: &[T], b: &[T]) -> Vec<T>
where
    T: PartialOrd + Copy,
{
    // Count the length required in the union, to avoid
    // paying for reallocations while pushing onto the end.
    let mut count = 0;
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            count += 1;
            ap += 1;
        } else if b[bp] < a[ap] {
            count += 1;
            bp += 1;
        } else {
            count += 1;
            ap += 1;
            bp += 1;
        }
    }
    count += a.len() - ap;
    count += b.len() - bp;

    let mut c: Vec<T> = Vec::with_capacity(count);
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            c.push(a[ap]);
            ap += 1;
        } else if b[bp] < a[ap] {
            c.push(b[bp]);
            bp += 1;
        } else {
            c.push(a[ap]);
            ap += 1;
            bp += 1;
        }
    }
    while ap < a.len() {
        c.push(a[ap]);
        ap += 1;
    }
    while bp < b.len() {
        c.push(b[bp]);
        bp += 1;
    }
    c
}

pub fn split_out_item<T>(items: &[T], item: T) -> (Vec<T>, Vec<T>)
where
    T: PartialEq + Clone,
{
    let antecedent: Vec<T> = items.iter().filter(|x| **x != item).cloned().collect();
    let consequent: Vec<T> = vec![item];
    (antecedent, consequent)
}

// Removes items in a that aren't in b.
pub fn split_out<T>(a: &[T], b: &[T]) -> Vec<T>
where
    T: PartialOrd + Clone + Copy,
{
    let mut c: Vec<T> = Vec::with_capacity(a.len());
    let mut ap = 0;
    let mut bp = 0;
    while ap < a.len() && bp < b.len() {
        if a[ap] < b[bp] {
            c.push(a[ap]);
            ap += 1;
        } else if b[bp] < a[ap] {
            panic!("Tried to remove item that's not in set!");
        } else {
            ap += 1;
            bp += 1;
        }
    }
    while ap < a.len() {
        c.push(a[ap]);
        ap += 1;
    }
    c
}

#[cfg(test)]
mod tests {
    use item::Item;
    fn to_item_vec(nums: &[u32]) -> Vec<Item> {
        nums.iter().map(|i| Item::with_id(*i)).collect()
    }

    #[test]
    fn test_union() {
        use super::union;

        let test_cases: Vec<(Vec<Item>, Vec<Item>, Vec<Item>)> = [
            (vec![1, 2, 3], vec![4, 5, 6], vec![1, 2, 3, 4, 5, 6]),
            (vec![1, 2, 3], vec![3, 4, 5, 6], vec![1, 2, 3, 4, 5, 6]),
            (vec![], vec![1], vec![1]),
            (vec![1], vec![], vec![1]),
        ]
        .iter()
        .map(|&(ref a, ref b, ref u)| (to_item_vec(a), to_item_vec(b), to_item_vec(u)))
        .collect();

        for &(ref a, ref b, ref c) in &test_cases {
            assert_eq!(&union(&a, &b), c);
        }
    }

    #[test]
    fn test_split_out_item() {
        use super::split_out_item;
        let cases: Vec<(Vec<Item>, Item, (Vec<Item>, Vec<Item>))> = [
            (vec![1], 1, (vec![], vec![1])),
            (vec![1, 2, 3], 1, (vec![2, 3], vec![1])),
            (vec![1, 2, 3], 2, (vec![1, 3], vec![2])),
            (vec![1, 2, 3], 3, (vec![1, 2], vec![3])),
        ]
        .iter()
        .map(|&(ref a, v, (ref b, ref c))| {
            (
                to_item_vec(a),
                Item::with_id(v),
                (to_item_vec(b), to_item_vec(c)),
            )
        })
        .collect();

        for (a, v, (b, c)) in cases.into_iter() {
            let split = split_out_item(&a, v);
            assert!(split == (b, c));
        }
    }
}
