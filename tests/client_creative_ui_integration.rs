use lce_rust::client::creative_ui::{
    CREATIVE_SELECTOR_COLUMNS, CREATIVE_SELECTOR_ROWS, CREATIVE_SELECTOR_SLOTS, CREATIVE_TABS,
    CreativeInventoryTab, creative_next_dynamic_group,
    creative_selector_entries_page_for_dynamic_group, creative_selector_items_page,
    creative_tab_dynamic_group_count, creative_tab_entries_for_dynamic_group,
    creative_tab_icon_item_id, creative_tab_items, creative_tab_items_for_dynamic_group,
    creative_tab_page_count, creative_tab_page_count_for_dynamic_group, creative_tab_title,
    place_creative_entry_in_hotbar, place_creative_item_in_hotbar,
    target_hotbar_slot_for_creative_entry, target_hotbar_slot_for_creative_item,
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
        creative_tab_title(CreativeInventoryTab::Decorations),
        "Decoration"
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
    assert!(redstone_transport.len() > 12);
    assert_eq!(redstone_transport[0], 66);
    assert_eq!(redstone_transport[1], 27);
    assert_eq!(redstone_transport[8], 408);
    assert_eq!(redstone_transport[11], 333);
    assert_eq!(redstone_transport[12], 23);
    assert!(redstone_transport.contains(&404));
}

#[test]
fn brewing_tab_uses_legacy_static_group_paging_model() {
    assert_eq!(
        creative_tab_dynamic_group_count(CreativeInventoryTab::Brewing),
        0
    );
    assert_eq!(
        creative_tab_dynamic_group_count(CreativeInventoryTab::Food),
        0
    );

    let brewing_base = creative_tab_items_for_dynamic_group(CreativeInventoryTab::Brewing, 4);
    assert_eq!(
        &brewing_base[..8],
        &[384, 370, 376, 377, 378, 382, 374, 373]
    );

    let brewing_entries = creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Brewing, 0);
    assert!(brewing_entries.len() > CREATIVE_SELECTOR_SLOTS);
    assert_eq!(
        brewing_entries[0].item_id, 384,
        "xp bottle should be first brewing entry"
    );
    assert_eq!(
        brewing_entries[7].aux, 0,
        "water bottle should end brewing base group"
    );
    assert_eq!(
        brewing_entries[8].aux, 0x2061,
        "next entry should begin level2+extended potion group"
    );
}

#[test]
fn brewing_tab_row_scrolls_without_dynamic_group_cycle() {
    assert_eq!(
        creative_next_dynamic_group(CreativeInventoryTab::Brewing, 0),
        0
    );
    assert_eq!(
        creative_next_dynamic_group(CreativeInventoryTab::Brewing, 4),
        0
    );
    assert_eq!(
        creative_next_dynamic_group(CreativeInventoryTab::Food, 2),
        0
    );

    let brewing_entries = creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Brewing, 0);
    let expected_brewing_pages = brewing_entries
        .len()
        .div_ceil(CREATIVE_SELECTOR_COLUMNS)
        .saturating_sub(CREATIVE_SELECTOR_ROWS)
        .saturating_add(1);

    assert_eq!(
        creative_tab_page_count_for_dynamic_group(CreativeInventoryTab::Brewing, 0),
        expected_brewing_pages
    );

    let page_0 =
        creative_selector_entries_page_for_dynamic_group(CreativeInventoryTab::Brewing, 0, 0);
    let page_1 =
        creative_selector_entries_page_for_dynamic_group(CreativeInventoryTab::Brewing, 0, 1);

    assert_eq!(page_0[0].map(|entry| entry.item_id), Some(384));
    assert_eq!(page_0[CREATIVE_SELECTOR_COLUMNS], page_1[0]);
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
    let structure_entries =
        creative_tab_entries_for_dynamic_group(CreativeInventoryTab::BuildingBlocks, 0).len();
    let expected_structure_pages = structure_entries
        .div_ceil(CREATIVE_SELECTOR_COLUMNS)
        .saturating_sub(CREATIVE_SELECTOR_ROWS)
        .saturating_add(1);

    assert_eq!(
        creative_tab_page_count(CreativeInventoryTab::BuildingBlocks),
        expected_structure_pages
    );
    assert!(creative_tab_page_count(CreativeInventoryTab::BuildingBlocks) > 1);

    let brewing_entries =
        creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Brewing, 0).len();
    let expected_brewing_pages = brewing_entries
        .div_ceil(CREATIVE_SELECTOR_COLUMNS)
        .saturating_sub(CREATIVE_SELECTOR_ROWS)
        .saturating_add(1);
    assert_eq!(
        creative_tab_page_count(CreativeInventoryTab::Brewing),
        expected_brewing_pages
    );
}

#[test]
fn creative_selector_scrolls_by_one_row_per_page() {
    let page_0 = creative_selector_entries_page_for_dynamic_group(
        CreativeInventoryTab::BuildingBlocks,
        0,
        0,
    );
    let page_1 = creative_selector_entries_page_for_dynamic_group(
        CreativeInventoryTab::BuildingBlocks,
        0,
        1,
    );

    assert_eq!(page_0[CREATIVE_SELECTOR_COLUMNS], page_1[0]);

    let overlap_slots = CREATIVE_SELECTOR_SLOTS - CREATIVE_SELECTOR_COLUMNS;
    assert_eq!(
        page_0[CREATIVE_SELECTOR_SLOTS - 1],
        page_1[overlap_slots - 1]
    );
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

    let coal_block_index = entries
        .iter()
        .position(|entry| entry.item_id == 173 && entry.aux == 0)
        .expect("coal block should be present");
    let gold_block_index = entries
        .iter()
        .position(|entry| entry.item_id == 41 && entry.aux == 0)
        .expect("gold block should be present");
    assert!(coal_block_index < gold_block_index);

    let fence_index = entries
        .iter()
        .position(|entry| entry.item_id == 85 && entry.aux == 0)
        .expect("oak fence should be present");
    assert_eq!(entries[fence_index + 1].item_id, 113);
    assert_eq!(entries[fence_index + 2].item_id, 101);
    assert_eq!(entries[fence_index + 3].item_id, 139);

    let hardened_clay_index = entries
        .iter()
        .position(|entry| entry.item_id == 172)
        .expect("hardened clay should be present");
    assert_eq!(entries[hardened_clay_index].aux, 0);

    let stained_hardened_clay_aux: Vec<u16> = entries
        .iter()
        .filter(|entry| entry.item_id == 159)
        .map(|entry| entry.aux)
        .collect();
    assert_eq!(
        stained_hardened_clay_aux,
        vec![14, 1, 4, 5, 3, 9, 11, 10, 2, 6, 0, 8, 7, 15, 13, 12]
    );
}

#[test]
fn decoration_and_misc_aux_variants_follow_legacy_scaffold_order() {
    let decoration_entries =
        creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Decorations, 0);

    let wool_index = decoration_entries
        .iter()
        .position(|entry| entry.item_id == 35)
        .expect("wool should be present");
    let hay_block_index = decoration_entries
        .iter()
        .position(|entry| entry.item_id == 170)
        .expect("hay block should be present");
    assert!(hay_block_index < wool_index);
    assert_eq!(decoration_entries[wool_index].aux, 14);
    assert_eq!(decoration_entries[wool_index + 1].aux, 1);
    assert_eq!(decoration_entries[wool_index + 2].aux, 4);
    assert_eq!(decoration_entries[wool_index + 3].aux, 5);

    let building_entries =
        creative_tab_entries_for_dynamic_group(CreativeInventoryTab::BuildingBlocks, 0);
    let wall_index = building_entries
        .iter()
        .position(|entry| entry.item_id == 139)
        .expect("cobble wall should be present");
    assert_eq!(building_entries[wall_index].aux, 0);
    assert_eq!(building_entries[wall_index + 1].aux, 1);

    let misc_entries = creative_tab_entries_for_dynamic_group(CreativeInventoryTab::Misc, 0);

    let beacon_index = misc_entries
        .iter()
        .position(|entry| entry.item_id == 138)
        .expect("beacon should be present");
    let end_portal_frame_index = misc_entries
        .iter()
        .position(|entry| entry.item_id == 120)
        .expect("end portal frame should be present");
    assert!(beacon_index < end_portal_frame_index);

    let spawn_egg_aux: Vec<u16> = misc_entries
        .iter()
        .filter(|entry| entry.item_id == 383)
        .map(|entry| entry.aux)
        .collect();
    assert!(spawn_egg_aux.contains(&65));
    assert!(spawn_egg_aux.contains(&66));
    assert!(spawn_egg_aux.contains(&100));
    assert!(spawn_egg_aux.contains(&8292));
    assert!(spawn_egg_aux.contains(&12388));

    let firework_aux: Vec<u16> = misc_entries
        .iter()
        .filter(|entry| entry.item_id == 401)
        .map(|entry| entry.aux)
        .collect();
    assert_eq!(firework_aux, vec![1, 2, 3, 4, 5]);
}

#[test]
fn tools_tab_includes_legacy_horse_and_book_entries() {
    let tool_items = creative_tab_items(CreativeInventoryTab::ToolsWeaponsArmor);
    assert!(tool_items.contains(&395));
    assert!(tool_items.contains(&420));
    assert!(tool_items.contains(&419));
    assert!(tool_items.contains(&418));
    assert!(tool_items.contains(&417));

    let tool_entries =
        creative_tab_entries_for_dynamic_group(CreativeInventoryTab::ToolsWeaponsArmor, 0);
    let enchanted_books = tool_entries
        .iter()
        .filter(|entry| entry.item_id == 403)
        .count();
    assert_eq!(enchanted_books, 22);
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
