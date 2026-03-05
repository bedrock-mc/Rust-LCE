use std::array;

use crate::world::{HOTBAR_SLOTS, INVENTORY_SLOTS, PlayerInventory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventorySlotState {
    pub slot: usize,
    pub selected_hotbar_slot: bool,
    pub item_id: Option<u16>,
    pub aux: Option<u16>,
    pub count: Option<u8>,
}

pub fn collect_inventory_state(
    inventory: &PlayerInventory,
) -> [InventorySlotState; INVENTORY_SLOTS] {
    let selected_hotbar_slot = inventory.selected_hotbar_slot();

    array::from_fn(|slot| {
        let stack = inventory.get(slot).ok().flatten();

        InventorySlotState {
            slot,
            selected_hotbar_slot: slot < HOTBAR_SLOTS && slot == selected_hotbar_slot,
            item_id: stack.map(|stack| stack.item_id),
            aux: stack.map(|stack| stack.aux),
            count: stack.map(|stack| stack.count),
        }
    })
}
