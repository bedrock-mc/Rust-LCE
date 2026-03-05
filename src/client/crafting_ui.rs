use crate::world::{PlayerInventory, RECIPES, can_craft_recipe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftingRecipeUiState {
    pub recipe_id: &'static str,
    pub output_item_id: u16,
    pub output_count: u8,
    pub craftable: bool,
}

pub fn collect_crafting_recipe_state(inventory: &PlayerInventory) -> Vec<CraftingRecipeUiState> {
    RECIPES
        .iter()
        .map(|recipe| CraftingRecipeUiState {
            recipe_id: recipe.id,
            output_item_id: recipe.output_item_id,
            output_count: recipe.output_count,
            craftable: can_craft_recipe(inventory, recipe.id, 1),
        })
        .collect()
}

pub fn crafting_recipe_title(recipe_id: &str) -> &'static str {
    match recipe_id {
        "planks_from_log" => "Planks",
        "sticks_from_planks" => "Sticks",
        "crafting_table" => "Crafting Table",
        "furnace" => "Furnace",
        _ => "Recipe",
    }
}

pub fn crafting_recipe_count_label(output_count: u8) -> String {
    if output_count > 1 {
        format!("x{output_count}")
    } else {
        String::new()
    }
}
