use crate::world::PlayerInventory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftRecipe {
    pub id: &'static str,
    pub output_item_id: u16,
    pub output_count: u8,
    pub ingredients: &'static [(u16, u8)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftOutcome {
    pub crafted_times: u32,
    pub produced_count: u32,
}

pub const RECIPES: &[CraftRecipe] = &[
    CraftRecipe {
        id: "planks_from_log",
        output_item_id: 5,
        output_count: 4,
        ingredients: &[(17, 1)],
    },
    CraftRecipe {
        id: "sticks_from_planks",
        output_item_id: 280,
        output_count: 4,
        ingredients: &[(5, 2)],
    },
    CraftRecipe {
        id: "crafting_table",
        output_item_id: 58,
        output_count: 1,
        ingredients: &[(5, 4)],
    },
    CraftRecipe {
        id: "furnace",
        output_item_id: 61,
        output_count: 1,
        ingredients: &[(4, 8)],
    },
];

pub fn recipe_by_id(id: &str) -> Option<&'static CraftRecipe> {
    RECIPES.iter().find(|recipe| recipe.id == id)
}

pub fn craft_recipe(
    inventory: &mut PlayerInventory,
    recipe_id: &str,
    requested_times: u32,
) -> CraftOutcome {
    let Some(recipe) = recipe_by_id(recipe_id) else {
        return CraftOutcome {
            crafted_times: 0,
            produced_count: 0,
        };
    };

    if requested_times == 0 {
        return CraftOutcome {
            crafted_times: 0,
            produced_count: 0,
        };
    }

    let mut crafted_times = 0_u32;
    for _ in 0..requested_times {
        if !recipe
            .ingredients
            .iter()
            .all(|(item_id, count)| inventory.total_count(*item_id) >= u32::from(*count))
        {
            break;
        }

        let mut trial_inventory = inventory.clone();
        let mut valid = true;

        for (item_id, count) in recipe.ingredients {
            if !trial_inventory.consume_item_exact(*item_id, u32::from(*count)) {
                valid = false;
                break;
            }
        }

        if !valid {
            break;
        }

        let overflow =
            trial_inventory.add_item(recipe.output_item_id, u32::from(recipe.output_count));
        if overflow > 0 {
            break;
        }

        *inventory = trial_inventory;
        crafted_times = crafted_times.saturating_add(1);
    }

    CraftOutcome {
        crafted_times,
        produced_count: crafted_times.saturating_mul(u32::from(recipe.output_count)),
    }
}
