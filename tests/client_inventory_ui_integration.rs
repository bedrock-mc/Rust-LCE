use lce_rust::client::inventory_ui::collect_inventory_state;
use lce_rust::world::{HOTBAR_SLOTS, INVENTORY_SLOTS, ItemStack, PlayerInventory};

#[test]
fn inventory_state_reflects_hotbar_and_backpack_slots() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            0,
            Some(ItemStack::new(1, 64).expect("stack should be valid")),
        )
        .expect("slot 0 should be writable");
    inventory
        .set(
            12,
            Some(ItemStack::new(58, 1).expect("stack should be valid")),
        )
        .expect("slot 12 should be writable");
    inventory
        .set(
            35,
            Some(ItemStack::new(50, 16).expect("stack should be valid")),
        )
        .expect("slot 35 should be writable");
    inventory
        .select_hotbar_slot(0)
        .expect("hotbar selection should succeed");

    let state = collect_inventory_state(&inventory);

    assert_eq!(state.len(), INVENTORY_SLOTS);
    assert_eq!(state[0].item_id, Some(1));
    assert_eq!(state[0].count, Some(64));
    assert!(state[0].selected_hotbar_slot);
    assert_eq!(state[12].item_id, Some(58));
    assert_eq!(state[12].count, Some(1));
    assert!(!state[12].selected_hotbar_slot);
    assert_eq!(state[35].item_id, Some(50));
    assert_eq!(state[35].count, Some(16));
}

#[test]
fn only_hotbar_slots_can_be_selected_for_inventory_ui_highlight() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            7,
            Some(ItemStack::new(5, 5).expect("stack should be valid")),
        )
        .expect("slot 7 should be writable");
    inventory
        .set(
            14,
            Some(ItemStack::new(4, 5).expect("stack should be valid")),
        )
        .expect("slot 14 should be writable");
    inventory
        .select_hotbar_slot(7)
        .expect("hotbar selection should succeed");

    let state = collect_inventory_state(&inventory);
    assert!(state[7].selected_hotbar_slot);
    assert!(!state[14].selected_hotbar_slot);
    assert!(
        state[..HOTBAR_SLOTS]
            .iter()
            .enumerate()
            .all(|(slot, state)| state.selected_hotbar_slot == (slot == 7))
    );
}

#[test]
fn inventory_state_tracks_stack_consumption_without_losing_selection() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            4,
            Some(ItemStack::new(17, 3).expect("stack should be valid")),
        )
        .expect("slot 4 should be writable");
    inventory
        .select_hotbar_slot(4)
        .expect("hotbar selection should succeed");

    assert!(inventory.consume_selected(2));
    let state = collect_inventory_state(&inventory);
    assert_eq!(state[4].count, Some(1));
    assert!(state[4].selected_hotbar_slot);
}
