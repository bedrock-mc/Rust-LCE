use lce_rust::client::hotbar_ui::{collect_hotbar_state, hotbar_count_label, hotbar_item_label};
use lce_rust::world::{HOTBAR_SLOTS, ItemStack, PlayerInventory};

#[test]
fn hotbar_state_reflects_selection_and_stack_updates() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            0,
            Some(ItemStack::new(1, 64).expect("stack should be valid")),
        )
        .expect("slot 0 should be writable");
    inventory
        .set(
            2,
            Some(ItemStack::new(5, 3).expect("stack should be valid")),
        )
        .expect("slot 2 should be writable");
    inventory
        .select_hotbar_slot(2)
        .expect("hotbar selection should succeed");

    let state = collect_hotbar_state(&inventory);
    assert_eq!(state.len(), HOTBAR_SLOTS);
    assert_eq!(state[0].item_id, Some(1));
    assert_eq!(state[0].count, Some(64));
    assert!(!state[0].selected);
    assert_eq!(state[2].item_id, Some(5));
    assert_eq!(state[2].count, Some(3));
    assert!(state[2].selected);

    assert!(inventory.consume_selected(2));
    let updated_state = collect_hotbar_state(&inventory);
    assert_eq!(updated_state[2].count, Some(1));
    assert!(updated_state[2].selected);
}

#[test]
fn item_label_uses_short_codes_with_numeric_fallback() {
    assert_eq!(hotbar_item_label(1), "STN");
    assert_eq!(hotbar_item_label(5), "PLK");
    assert_eq!(hotbar_item_label(280), "280");
}

#[test]
fn count_label_hides_empty_and_single_stacks() {
    assert_eq!(hotbar_count_label(None), "");
    assert_eq!(hotbar_count_label(Some(1)), "");
    assert_eq!(hotbar_count_label(Some(32)), "32");
}
