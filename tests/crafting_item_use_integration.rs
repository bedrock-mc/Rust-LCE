use lce_rust::world::{
    INVENTORY_SLOTS, ItemStack, PlayerInventory, block_id_for_item, can_craft_recipe, craft_recipe,
    recipe_by_id, use_selected_item_for_placement,
};

#[test]
fn crafts_planks_from_logs_and_consumes_ingredients() {
    let mut inventory = PlayerInventory::new();
    inventory.add_item(17, 3);

    let outcome = craft_recipe(&mut inventory, "planks_from_log", 2);

    assert_eq!(outcome.crafted_times, 2);
    assert_eq!(outcome.produced_count, 8);
    assert_eq!(inventory.total_count(17), 1);
    assert_eq!(inventory.total_count(5), 8);
}

#[test]
fn does_not_craft_when_ingredients_are_missing() {
    let mut inventory = PlayerInventory::new();
    inventory.add_item(17, 1);

    let outcome = craft_recipe(&mut inventory, "crafting_table", 1);

    assert_eq!(outcome.crafted_times, 0);
    assert_eq!(outcome.produced_count, 0);
    assert_eq!(inventory.total_count(58), 0);
}

#[test]
fn uses_selected_placeable_item_and_consumes_one() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            0,
            Some(ItemStack::new(4, 2).expect("stack should be valid")),
        )
        .expect("slot should be writable");
    inventory
        .select_hotbar_slot(0)
        .expect("slot selection should succeed");

    let block_id = use_selected_item_for_placement(&mut inventory);

    assert_eq!(block_id, Some(4));
    assert_eq!(inventory.total_count(4), 1);
}

#[test]
fn does_not_use_non_placeable_selected_item() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            0,
            Some(ItemStack::new(280, 4).expect("stack should be valid")),
        )
        .expect("slot should be writable");
    inventory
        .select_hotbar_slot(0)
        .expect("slot selection should succeed");

    let block_id = use_selected_item_for_placement(&mut inventory);

    assert_eq!(block_id, None);
    assert_eq!(inventory.total_count(280), 4);
}

#[test]
fn recipe_lookup_exposes_known_recipes() {
    let recipe = recipe_by_id("furnace").expect("recipe should exist");

    assert_eq!(recipe.output_item_id, 61);
    assert_eq!(recipe.output_count, 1);
}

#[test]
fn block_backed_items_map_to_expected_placeable_tile_ids() {
    assert_eq!(block_id_for_item(356), Some(93));
    assert_eq!(block_id_for_item(379), Some(117));
    assert_eq!(block_id_for_item(380), Some(118));
    assert_eq!(block_id_for_item(390), Some(140));
}

#[test]
fn can_craft_allows_output_when_ingredients_free_space() {
    let mut inventory = PlayerInventory::new();
    for slot in 0..INVENTORY_SLOTS {
        inventory
            .set(
                slot,
                Some(ItemStack::new(1, 64).expect("stack should be valid")),
            )
            .expect("slot should be writable");
    }

    inventory
        .set(
            0,
            Some(ItemStack::new(17, 1).expect("stack should be valid")),
        )
        .expect("slot should be writable");

    assert!(can_craft_recipe(&inventory, "planks_from_log", 1));
}

#[test]
fn can_craft_rejects_when_output_still_overflows_after_consumption() {
    let mut inventory = PlayerInventory::new();
    for slot in 0..INVENTORY_SLOTS {
        inventory
            .set(
                slot,
                Some(ItemStack::new(5, 64).expect("stack should be valid")),
            )
            .expect("slot should be writable");
    }

    inventory
        .set(
            0,
            Some(ItemStack::new(17, 17).expect("stack should be valid")),
        )
        .expect("slot should be writable");

    assert!(!can_craft_recipe(&inventory, "planks_from_log", 17));
}
