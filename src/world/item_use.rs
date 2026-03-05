use crate::world::PlayerInventory;

pub fn block_id_for_item(item_id: u16) -> Option<u16> {
    if (1..=255).contains(&item_id) {
        return Some(item_id);
    }

    match item_id {
        323 => Some(63),
        324 => Some(64),
        330 => Some(71),
        355 => Some(26),
        356 => Some(93),
        379 => Some(117),
        380 => Some(118),
        390 => Some(140),
        _ => None,
    }
}

pub fn use_selected_item_for_placement(inventory: &mut PlayerInventory) -> Option<u16> {
    let selected = inventory.selected_stack()?;
    let block_id = block_id_for_item(selected.item_id)?;

    if inventory.consume_selected(1) {
        Some(block_id)
    } else {
        None
    }
}
