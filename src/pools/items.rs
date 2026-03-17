use slotmap::{SlotMap, new_key_type};
use crate::check::ty::Item;

new_key_type! { pub struct ItemId; }

#[derive(Debug)]
pub struct Items {
    map: SlotMap<ItemId, Item>,
}

impl Items {
    pub fn new() -> Self {
        Self { map: SlotMap::with_key() }
    }
    pub fn add(&mut self, item: Item) -> ItemId {
        self.map.insert(item)
    }
    pub fn get(&self, item: ItemId) -> &Item {
        self.map.get(item).expect("Items has apparently handed out an invalid ItemId")
    }
    pub fn get_mut(&mut self, item: ItemId) -> &mut Item {
        self.map.get_mut(item).expect("Items has apparently handed out an invalid ItemId")
    }
}
