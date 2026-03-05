use lce_rust::client::crafting_ui::{
    collect_crafting_recipe_state, crafting_recipe_count_label, crafting_recipe_title,
};
use lce_rust::world::PlayerInventory;

#[test]
fn crafting_recipe_state_marks_recipe_craftable_when_requirements_are_met() {
    let mut inventory = PlayerInventory::new();
    inventory.add_item(17, 1);

    let state = collect_crafting_recipe_state(&inventory);

    let planks = state
        .iter()
        .find(|recipe| recipe.recipe_id == "planks_from_log")
        .expect("planks recipe should exist");
    let sticks = state
        .iter()
        .find(|recipe| recipe.recipe_id == "sticks_from_planks")
        .expect("sticks recipe should exist");

    assert!(planks.craftable);
    assert!(!sticks.craftable);
}

#[test]
fn crafting_recipe_state_reflects_missing_ingredients_and_output_metadata() {
    let mut inventory = PlayerInventory::new();
    inventory.add_item(4, 8);

    let state = collect_crafting_recipe_state(&inventory);

    let furnace = state
        .iter()
        .find(|recipe| recipe.recipe_id == "furnace")
        .expect("furnace recipe should exist");
    let table = state
        .iter()
        .find(|recipe| recipe.recipe_id == "crafting_table")
        .expect("crafting table recipe should exist");

    assert!(furnace.craftable);
    assert_eq!(furnace.output_item_id, 61);
    assert_eq!(furnace.output_count, 1);
    assert!(!table.craftable);
}

#[test]
fn crafting_ui_label_helpers_provide_expected_text() {
    assert_eq!(crafting_recipe_title("planks_from_log"), "Planks");
    assert_eq!(crafting_recipe_title("furnace"), "Furnace");
    assert_eq!(crafting_recipe_title("unknown"), "Recipe");

    assert_eq!(crafting_recipe_count_label(4), "x4");
    assert_eq!(crafting_recipe_count_label(1), "");
}
