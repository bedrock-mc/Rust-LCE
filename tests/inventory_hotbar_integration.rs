use lce_rust::world::{
    HOTBAR_SLOTS, INVENTORY_SLOTS, InventoryError, ItemStack, OfflineGameSession,
    OfflineWorldBootstrap, PlayerInventory,
};

#[test]
fn add_item_stacks_then_spills_to_new_slots() {
    let mut inventory = PlayerInventory::new();

    assert_eq!(inventory.add_item(5, 32), 0);
    assert_eq!(inventory.add_item(5, 40), 0);

    let first_slot = inventory
        .get(0)
        .expect("slot read should succeed")
        .expect("slot 0 should contain items");
    let second_slot = inventory
        .get(1)
        .expect("slot read should succeed")
        .expect("slot 1 should contain items");

    assert_eq!(first_slot.item_id, 5);
    assert_eq!(first_slot.count, 64);
    assert_eq!(second_slot.item_id, 5);
    assert_eq!(second_slot.count, 8);
}

#[test]
fn hotbar_selection_and_consumption_updates_selected_stack() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            3,
            Some(ItemStack::new(17, 4).expect("stack should be valid")),
        )
        .expect("slot set should succeed");
    inventory
        .select_hotbar_slot(3)
        .expect("hotbar selection should succeed");

    assert!(inventory.consume_selected(1));
    assert_eq!(inventory.selected_hotbar_slot(), 3);
    assert_eq!(
        inventory
            .selected_stack()
            .expect("selected stack should exist"),
        ItemStack::new(17, 3).expect("stack should be valid")
    );

    assert!(inventory.consume_selected(3));
    assert!(inventory.selected_stack().is_none());
}

#[test]
fn invalid_slots_and_stack_sizes_return_errors() {
    let mut inventory = PlayerInventory::new();

    assert_eq!(
        inventory.select_hotbar_slot(HOTBAR_SLOTS),
        Err(InventoryError::InvalidHotbarSlot(HOTBAR_SLOTS))
    );
    assert_eq!(
        inventory.set(INVENTORY_SLOTS, None),
        Err(InventoryError::InvalidSlot(INVENTORY_SLOTS))
    );
    assert_eq!(
        ItemStack::new(1, 0),
        Err(InventoryError::InvalidStackCount(0))
    );
}

#[test]
fn spawned_player_has_usable_inventory() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("InventoryWorld", 77)
        .expect("world should be created")
        .clone();
    let mut game = OfflineGameSession::new(world);

    let overflow = game.player_mut().inventory.add_item(2, 70);
    assert_eq!(overflow, 0);
    assert_eq!(
        game.player()
            .inventory
            .get(0)
            .expect("slot read should succeed")
            .expect("slot 0 should contain items")
            .count,
        64
    );
}
