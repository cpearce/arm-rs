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
}
