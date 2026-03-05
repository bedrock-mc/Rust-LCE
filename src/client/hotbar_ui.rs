use std::array;

use crate::world::{HOTBAR_SLOTS, PlayerInventory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotbarSlotState {
    pub slot: usize,
    pub selected: bool,
    pub item_id: Option<u16>,
    pub aux: Option<u16>,
    pub count: Option<u8>,
}

pub fn collect_hotbar_state(inventory: &PlayerInventory) -> [HotbarSlotState; HOTBAR_SLOTS] {
    let selected_slot = inventory.selected_hotbar_slot();

    array::from_fn(|slot| {
        let stack = inventory.get(slot).ok().flatten();

        HotbarSlotState {
            slot,
            selected: slot == selected_slot,
            item_id: stack.map(|stack| stack.item_id),
            aux: stack.map(|stack| stack.aux),
            count: stack.map(|stack| stack.count),
        }
    })
}

pub fn hotbar_item_label(item_id: u16) -> String {
    let label = match item_id {
        1 => "STN",
        2 => "GRS",
        3 => "DIR",
        4 => "COB",
        5 => "PLK",
        12 => "SND",
        17 => "LOG",
        20 => "GLS",
        50 => "TOR",
        58 => "TBL",
        _ => return item_id.to_string(),
    };

    label.to_string()
}

pub fn hotbar_count_label(count: Option<u8>) -> String {
    match count {
        Some(count) if count > 1 => count.to_string(),
        _ => String::new(),
    }
}
