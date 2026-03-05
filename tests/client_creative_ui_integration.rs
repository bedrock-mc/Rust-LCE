use lce_rust::client::creative_ui::{
    CREATIVE_SELECTOR_SLOTS, CREATIVE_TABS, CreativeInventoryTab, creative_next_dynamic_group,
    creative_selector_entries_page_for_dynamic_group, creative_selector_items_page,
    creative_selector_items_page_for_dynamic_group, creative_tab_dynamic_group_count,
    creative_tab_entries_for_dynamic_group, creative_tab_icon_item_id, creative_tab_items,
    creative_tab_items_for_dynamic_group, creative_tab_page_count,
    creative_tab_page_count_for_dynamic_group, creative_tab_title, place_creative_entry_in_hotbar,
    place_creative_item_in_hotbar, target_hotbar_slot_for_creative_entry,
    target_hotbar_slot_for_creative_item,
};
use lce_rust::world::{HOTBAR_SLOTS, ItemStack, PlayerInventory};

#[test]
fn creative_tabs_follow_legacy_order_and_titles() {
    assert_eq!(CREATIVE_TABS.len(), 8);
    assert_eq!(CREATIVE_TABS[0], CreativeInventoryTab::BuildingBlocks);
    assert_eq!(CREATIVE_TABS[7], CreativeInventoryTab::Misc);
    assert_eq!(
        creative_tab_title(CreativeInventoryTab::BuildingBlocks),
        "Structures"
    );
    assert_eq!(
        creative_tab_title(CreativeInventoryTab::RedstoneAndTransport),
        "Redstone + Transport"
    );
    assert_eq!(
        creative_tab_title(CreativeInventoryTab::ToolsWeaponsArmor),
        "Tools"
    );
    assert_eq!(
        creative_tab_icon_item_id(CreativeInventoryTab::BuildingBlocks),
        1
    );
    assert_eq!(
        creative_tab_icon_item_id(CreativeInventoryTab::Decorations),
        397
    );
    assert_eq!(
        creative_tab_icon_item_id(CreativeInventoryTab::RedstoneAndTransport),
        66
    );
}

#[test]
fn redstone_transport_tab_matches_legacy_group_order() {
    let redstone_transport = creative_tab_items(CreativeInventoryTab::RedstoneAndTransport);
    assert!(redstone_transport.len() > 9);
    assert_eq!(redstone_transport[0], 66);
    assert_eq!(redstone_transport[1], 27);
    assert_eq!(redstone_transport[8], 333);
    assert_eq!(redstone_transport[9], 23);
}

#[test]
fn brewing_dynamic_group_scaffold_matches_legacy_group_count_and_order() {
    assert_eq!(
        creative_tab_dynamic_group_count(CreativeInventoryTab::Brewing),
        5
    );
    assert_eq!(
        creative_tab_dynamic_group_count(CreativeInventoryTab::Food),
        0
    );

    let brewing_base = creative_tab_items_for_dynamic_group(CreativeInventoryTab::Brewing, 0);
    assert_eq!(brewing_base[0], 384);
    assert!(brewing_base.iter().any(|item_id| *item_id == 370));
    assert!(brewing_base.iter().any(|item_id| *item_id == 373));

    let level2_extended = creative_tab_items_for_dynamic_group(CreativeInventoryTab::Brewing, 1);
    assert_eq!(level2_extended, &[373]);
}

#[test]
fn brewing_dynamic_group_wraps_and_drives_selector_page() {
    assert_eq!(
        creative_next_dynamic_group(CreativeInventoryTab::Brewing, 0),
        1
    );
    assert_eq!(
        creative_next_dynamic_group(CreativeInventoryTab::Brewing, 4),
        0
    );
    assert_eq!(
        creative_next_dynamic_group(CreativeInventoryTab::Food, 2),
        0
    );

    assert_eq!(
        creative_tab_page_count_for_dynamic_group(CreativeInventoryTab::Brewing, 0),
        1
    );
    assert_eq!(
        creative_tab_page_count_for_dynamic_group(CreativeInventoryTab::Brewing, 3),
        1
    );

    let potion_selector =
        creative_selector_items_page_for_dynamic_group(CreativeInventoryTab::Brewing, 3, 0);
    assert_eq!(potion_selector[0], Some(373));
    assert!(
        potion_selector
            .iter()
            .skip(1)
            .any(|entry| *entry == Some(373))
    );
}

#[test]
fn creative_selector_page_surfaces_known_placeable_entries() {
    let structures = creative_selector_items_page(CreativeInventoryTab::BuildingBlocks, 0);

    assert_eq!(structures.len(), CREATIVE_SELECTOR_SLOTS);
    assert_eq!(structures[0], Some(1));
    assert!(structures.iter().any(|entry| *entry == Some(24)));
    assert!(structures.iter().any(|entry| *entry == Some(48)));

    let misc = creative_selector_items_page(CreativeInventoryTab::Misc, 0);
    assert!(misc.iter().any(|entry| *entry == Some(58)));
    assert!(misc.iter().any(|entry| *entry == Some(61)));

    let food = creative_selector_items_page(CreativeInventoryTab::Food, 0);
    assert!(food.iter().any(|entry| *entry == Some(260)));
    assert!(food.iter().any(|entry| *entry == Some(400)));
}

#[test]
fn creative_page_count_defaults_to_one_for_empty_tabs() {
    assert!(creative_tab_items(CreativeInventoryTab::BuildingBlocks).len() > 50);
    assert_eq!(
        creative_tab_page_count(CreativeInventoryTab::BuildingBlocks),
        2
    );
    assert_eq!(creative_tab_page_count(CreativeInventoryTab::Brewing), 1);
}

#[test]
fn creative_hotbar_slot_target_prefers_stack_then_empty() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            2,
            Some(ItemStack::new(1, 10).expect("stack should be valid")),
        )
        .expect("slot should be writable");
    inventory
        .set(
            4,
            Some(ItemStack::new(2, 64).expect("stack should be valid")),
        )
        .expect("slot should be writable");

    let stack_slot = target_hotbar_slot_for_creative_item(&inventory, 1, 1);
    assert_eq!(stack_slot, Some(2));

    let empty_slot = target_hotbar_slot_for_creative_item(&inventory, 3, 1);
    assert_eq!(empty_slot, Some(0));
}

#[test]
fn creative_hotbar_placement_adds_or_fills_slots() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            1,
            Some(ItemStack::new(4, 2).expect("stack should be valid")),
        )
        .expect("slot should be writable");

    let updated_slot = place_creative_item_in_hotbar(&mut inventory, 4, 1);
    assert_eq!(updated_slot, Some(1));
    assert_eq!(
        inventory
            .get(1)
            .expect("slot should be readable")
            .map(|stack| stack.count),
        Some(3)
    );

    for slot in 0..HOTBAR_SLOTS {
        if slot == 1 {
            continue;
        }

        inventory
            .set(
                slot,
                Some(ItemStack::new((slot as u16) + 10, 64).expect("stack should be valid")),
            )
            .expect("slot should be writable");
    }

    let full_hotbar_result = place_creative_item_in_hotbar(&mut inventory, 99, 1);
    assert_eq!(full_hotbar_result, None);
}

#[test]
fn creative_hotbar_pickup_can_create_full_stack() {
    let mut inventory = PlayerInventory::new();

    let target = place_creative_item_in_hotbar(&mut inventory, 45, 64);
    assert_eq!(target, Some(0));
    assert_eq!(
        inventory
            .get(0)
            .expect("slot should be readable")
            .map(|stack| stack.count),
        Some(64)
    );
}

#[test]
fn building_blocks_aux_variants_follow_legacy_scaffold_order() {
    let entries = creative_tab_entries_for_dynamic_group(CreativeInventoryTab::BuildingBlocks, 0);

    let sandstone_index = entries
        .iter()
        .position(|entry| entry.item_id == 24)
        .expect("sandstone should be present");
    assert_eq!(entries[sandstone_index].aux, 0);
    assert_eq!(entries[sandstone_index + 1].aux, 1);
    assert_eq!(entries[sandstone_index + 2].aux, 2);

    let planks_index = entries
        .iter()
        .position(|entry| entry.item_id == 5)
        .expect("planks should be present");
    assert_eq!(entries[planks_index].aux, 0);
    assert_eq!(entries[planks_index + 1].aux, 1);
    assert_eq!(entries[planks_index + 2].aux, 2);
    assert_eq!(entries[planks_index + 3].aux, 3);

    let quartz_aux: Vec<u16> = entries
        .iter()
        .filter(|entry| entry.item_id == 155)
        .map(|entry| entry.aux)
        .collect();
    assert_eq!(quartz_aux, vec![0, 1, 2]);
}

#[test]
fn decoration_and_misc_aux_variants_follow_legacy_scaffold_order() {
    let decoration_entries =
        creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Decorations, 0);

    let wool_index = decoration_entries
        .iter()
        .position(|entry| entry.item_id == 35)
        .expect("wool should be present");
    assert_eq!(decoration_entries[wool_index].aux, 14);
    assert_eq!(decoration_entries[wool_index + 1].aux, 1);
    assert_eq!(decoration_entries[wool_index + 2].aux, 4);
    assert_eq!(decoration_entries[wool_index + 3].aux, 5);

    let misc_entries = creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Misc, 0);
    let wall_index = misc_entries
        .iter()
        .position(|entry| entry.item_id == 139)
        .expect("cobble wall should be present");
    assert_eq!(misc_entries[wall_index].aux, 0);
    assert_eq!(misc_entries[wall_index + 1].aux, 1);
}

#[test]
fn creative_hotbar_target_and_placement_consider_aux_values() {
    let mut inventory = PlayerInventory::new();
    inventory
        .set(
            2,
            Some(ItemStack::new_with_aux(351, 1, 10).expect("stack should be valid")),
        )
        .expect("slot should be writable");

    let same_aux_slot = target_hotbar_slot_for_creative_entry(
        &inventory,
        lce_rust::client::creative_ui::CreativeItemEntry {
            item_id: 351,
            aux: 1,
        },
        1,
    );
    assert_eq!(same_aux_slot, Some(2));

    let different_aux_slot = target_hotbar_slot_for_creative_entry(
        &inventory,
        lce_rust::client::creative_ui::CreativeItemEntry {
            item_id: 351,
            aux: 14,
        },
        1,
    );
    assert_eq!(different_aux_slot, Some(0));

    let updated_slot = place_creative_entry_in_hotbar(
        &mut inventory,
        lce_rust::client::creative_ui::CreativeItemEntry {
            item_id: 351,
            aux: 1,
        },
        5,
    );
    assert_eq!(updated_slot, Some(2));
    assert_eq!(
        inventory
            .get(2)
            .expect("slot should be readable")
            .expect("slot should contain stack")
            .count,
        15
    );
}

#[test]
fn selector_entry_page_surfaces_aux_backed_entries() {
    let entries_page =
        creative_selector_entries_page_for_dynamic_group(CreativeInventoryTab::Decorations, 0, 0);
    assert_eq!(
        entries_page[0].map(|entry| (entry.item_id, entry.aux)),
        Some((397, 0))
    );
    assert_eq!(
        entries_page[1].map(|entry| (entry.item_id, entry.aux)),
        Some((397, 1))
    );
}
