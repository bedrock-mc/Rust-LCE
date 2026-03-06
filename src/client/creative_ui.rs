use std::collections::HashMap;
use std::sync::OnceLock;

use crate::world::inventory::MAX_STACK_SIZE;
use crate::world::{HOTBAR_SLOTS, ItemStack, PlayerInventory};

pub const CREATIVE_SELECTOR_ROWS: usize = 5;
pub const CREATIVE_SELECTOR_COLUMNS: usize = 10;
pub const CREATIVE_SELECTOR_SLOTS: usize = CREATIVE_SELECTOR_ROWS * CREATIVE_SELECTOR_COLUMNS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreativeItemEntry {
    pub item_id: u16,
    pub aux: u16,
}

const fn creative_entry(item_id: u16, aux: u16) -> CreativeItemEntry {
    CreativeItemEntry { item_id, aux }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreativeInventoryTab {
    BuildingBlocks,
    Decorations,
    RedstoneAndTransport,
    Materials,
    Food,
    ToolsWeaponsArmor,
    Brewing,
    Misc,
}

pub const CREATIVE_TABS: [CreativeInventoryTab; 8] = [
    CreativeInventoryTab::BuildingBlocks,
    CreativeInventoryTab::Decorations,
    CreativeInventoryTab::RedstoneAndTransport,
    CreativeInventoryTab::Materials,
    CreativeInventoryTab::Food,
    CreativeInventoryTab::ToolsWeaponsArmor,
    CreativeInventoryTab::Brewing,
    CreativeInventoryTab::Misc,
];

type CreativeEntryCache = HashMap<(CreativeInventoryTab, usize), Vec<CreativeItemEntry>>;

static CREATIVE_ENTRY_CACHE: OnceLock<CreativeEntryCache> = OnceLock::new();

impl CreativeInventoryTab {
    pub fn index(self) -> usize {
        match self {
            Self::BuildingBlocks => 0,
            Self::Decorations => 1,
            Self::RedstoneAndTransport => 2,
            Self::Materials => 3,
            Self::Food => 4,
            Self::ToolsWeaponsArmor => 5,
            Self::Brewing => 6,
            Self::Misc => 7,
        }
    }

    pub fn next(self) -> Self {
        let next = (self.index() + 1) % CREATIVE_TABS.len();
        CREATIVE_TABS[next]
    }

    pub fn previous(self) -> Self {
        let previous = (self.index() + CREATIVE_TABS.len() - 1) % CREATIVE_TABS.len();
        CREATIVE_TABS[previous]
    }
}

pub fn creative_tab_title(tab: CreativeInventoryTab) -> &'static str {
    match tab {
        CreativeInventoryTab::BuildingBlocks => "Structures",
        CreativeInventoryTab::Decorations => "Decoration",
        CreativeInventoryTab::RedstoneAndTransport => "Redstone + Transport",
        CreativeInventoryTab::Materials => "Materials",
        CreativeInventoryTab::Food => "Food",
        CreativeInventoryTab::ToolsWeaponsArmor => "Tools",
        CreativeInventoryTab::Brewing => "Brewing",
        CreativeInventoryTab::Misc => "Misc",
    }
}

pub fn creative_tab_icon_item_id(tab: CreativeInventoryTab) -> u16 {
    match tab {
        CreativeInventoryTab::BuildingBlocks => 1,
        CreativeInventoryTab::Decorations => 397,
        CreativeInventoryTab::RedstoneAndTransport => 66,
        CreativeInventoryTab::Materials => 263,
        CreativeInventoryTab::Food => 260,
        CreativeInventoryTab::ToolsWeaponsArmor => 345,
        CreativeInventoryTab::Brewing => 384,
        CreativeInventoryTab::Misc => 54,
    }
}

const POTION_MASK_REGENERATION: u16 = 0x2001;
const POTION_MASK_SPEED: u16 = 0x2002;
const POTION_MASK_FIRE_RESISTANCE: u16 = 0x2003;
const POTION_MASK_POISON: u16 = 0x2004;
const POTION_MASK_INSTANT_HEALTH: u16 = 0x2005;
const POTION_MASK_NIGHT_VISION: u16 = 0x2006;
const POTION_MASK_WEAKNESS: u16 = 0x2008;
const POTION_MASK_STRENGTH: u16 = 0x2009;
const POTION_MASK_SLOWNESS: u16 = 0x200A;
const POTION_MASK_INSTANT_DAMAGE: u16 = 0x200C;
const POTION_MASK_INVISIBILITY: u16 = 0x200E;

const POTION_MASK_SPLASH: u16 = 0x4000;
const POTION_MASK_LEVEL2: u16 = 0x0020;
const POTION_MASK_EXTENDED: u16 = 0x0040;
const POTION_MASK_LEVEL2_EXTENDED: u16 = 0x0060;

const fn potion_aux(type_mask: u16, strength_mask: u16, effect_mask: u16) -> u16 {
    type_mask | strength_mask | effect_mask
}

const BREWING_GROUP_ITEMS: &[CreativeItemEntry] = &[
    creative_entry(384, 0),
    creative_entry(370, 0),
    creative_entry(376, 0),
    creative_entry(377, 0),
    creative_entry(378, 0),
    creative_entry(382, 0),
    creative_entry(374, 0),
    creative_entry(373, 0),
];

const BREWING_GROUP_POTIONS_BASIC: &[CreativeItemEntry] = &[
    creative_entry(373, potion_aux(0, 0, POTION_MASK_REGENERATION)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_SPEED)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_POISON)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_INSTANT_HEALTH)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_STRENGTH)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_INSTANT_DAMAGE)),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_REGENERATION),
    ),
    creative_entry(373, potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_SPEED)),
    creative_entry(373, potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_POISON)),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_INSTANT_HEALTH),
    ),
    creative_entry(373, potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_STRENGTH)),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_INSTANT_DAMAGE),
    ),
];

const BREWING_GROUP_POTIONS_LEVEL2: &[CreativeItemEntry] = &[
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2, POTION_MASK_REGENERATION),
    ),
    creative_entry(373, potion_aux(0, POTION_MASK_LEVEL2, POTION_MASK_SPEED)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_FIRE_RESISTANCE)),
    creative_entry(373, potion_aux(0, POTION_MASK_LEVEL2, POTION_MASK_POISON)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_WEAKNESS)),
    creative_entry(373, potion_aux(0, POTION_MASK_LEVEL2, POTION_MASK_STRENGTH)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_SLOWNESS)),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2,
            POTION_MASK_REGENERATION,
        ),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, POTION_MASK_LEVEL2, POTION_MASK_SPEED),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_FIRE_RESISTANCE),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, POTION_MASK_LEVEL2, POTION_MASK_POISON),
    ),
    creative_entry(373, potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_WEAKNESS)),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, POTION_MASK_LEVEL2, POTION_MASK_STRENGTH),
    ),
    creative_entry(373, potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_SLOWNESS)),
];

const BREWING_GROUP_POTIONS_EXTENDED: &[CreativeItemEntry] = &[
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_REGENERATION),
    ),
    creative_entry(373, potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_SPEED)),
    creative_entry(373, potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_POISON)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_NIGHT_VISION)),
    creative_entry(373, potion_aux(0, 0, POTION_MASK_INVISIBILITY)),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_STRENGTH),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_REGENERATION,
        ),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, POTION_MASK_EXTENDED, POTION_MASK_SPEED),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, POTION_MASK_EXTENDED, POTION_MASK_POISON),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_NIGHT_VISION),
    ),
    creative_entry(
        373,
        potion_aux(POTION_MASK_SPLASH, 0, POTION_MASK_INVISIBILITY),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_STRENGTH,
        ),
    ),
];

const BREWING_GROUP_POTIONS_LEVEL2_EXTENDED: &[CreativeItemEntry] = &[
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2_EXTENDED, POTION_MASK_REGENERATION),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2_EXTENDED, POTION_MASK_SPEED),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_FIRE_RESISTANCE),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2_EXTENDED, POTION_MASK_POISON),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2, POTION_MASK_INSTANT_HEALTH),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_NIGHT_VISION),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_INVISIBILITY),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_WEAKNESS),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2_EXTENDED, POTION_MASK_STRENGTH),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_EXTENDED, POTION_MASK_SLOWNESS),
    ),
    creative_entry(
        373,
        potion_aux(0, POTION_MASK_LEVEL2, POTION_MASK_INSTANT_DAMAGE),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2_EXTENDED,
            POTION_MASK_REGENERATION,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2_EXTENDED,
            POTION_MASK_SPEED,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_FIRE_RESISTANCE,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2_EXTENDED,
            POTION_MASK_POISON,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2,
            POTION_MASK_INSTANT_HEALTH,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_NIGHT_VISION,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_INVISIBILITY,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_WEAKNESS,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2_EXTENDED,
            POTION_MASK_STRENGTH,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_EXTENDED,
            POTION_MASK_SLOWNESS,
        ),
    ),
    creative_entry(
        373,
        potion_aux(
            POTION_MASK_SPLASH,
            POTION_MASK_LEVEL2,
            POTION_MASK_INSTANT_DAMAGE,
        ),
    ),
];

const DYE_AUX_ORDER: &[u16] = &[1, 14, 11, 10, 12, 6, 4, 5, 13, 9, 15, 7, 8, 0, 2, 3];
const SKULL_AUX_ORDER: &[u16] = &[0, 1, 2, 3, 4];
const WOOL_AUX_ORDER: &[u16] = &[14, 1, 4, 5, 3, 9, 11, 10, 2, 6, 0, 8, 7, 15, 13, 12];
const WOOD_VARIANT_AUX_ORDER: &[u16] = &[0, 1, 2, 3];
const STONE_BRICK_AUX_ORDER: &[u16] = &[0, 1, 2, 3];
const MONSTER_EGG_AUX_ORDER: &[u16] = &[0, 1, 2];
const STONE_SLAB_AUX_ORDER: &[u16] = &[0, 1, 3, 4, 5, 6, 7];
const COBBLE_WALL_AUX_ORDER: &[u16] = &[0, 1];
const SPAWN_EGG_AUX_ORDER: &[u16] = &[
    50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 65, 66, 90, 91, 92, 93, 94, 95, 96, 98, 100,
    8292, 12388, 120,
];

const fn enchanted_book_aux(enchantment_id: u16, max_level: u16) -> u16 {
    (enchantment_id << 8) | (max_level & 0xFF)
}

const ENCHANTED_BOOK_AUX_ORDER: &[u16] = &[
    enchanted_book_aux(0, 4),
    enchanted_book_aux(1, 4),
    enchanted_book_aux(2, 4),
    enchanted_book_aux(3, 4),
    enchanted_book_aux(4, 4),
    enchanted_book_aux(5, 3),
    enchanted_book_aux(6, 1),
    enchanted_book_aux(7, 3),
    enchanted_book_aux(16, 5),
    enchanted_book_aux(17, 5),
    enchanted_book_aux(18, 5),
    enchanted_book_aux(19, 2),
    enchanted_book_aux(20, 2),
    enchanted_book_aux(21, 3),
    enchanted_book_aux(32, 5),
    enchanted_book_aux(33, 1),
    enchanted_book_aux(34, 3),
    enchanted_book_aux(35, 3),
    enchanted_book_aux(48, 5),
    enchanted_book_aux(49, 2),
    enchanted_book_aux(50, 1),
    enchanted_book_aux(51, 1),
];

const FIREWORK_PRESET_AUX_ORDER: &[u16] = &[1, 2, 3, 4, 5];

pub fn creative_tab_items(tab: CreativeInventoryTab) -> &'static [u16] {
    match tab {
        CreativeInventoryTab::BuildingBlocks => &[
            1, 2, 3, 4, 12, 24, 173, 41, 42, 22, 57, 133, 155, 16, 21, 56, 73, 15, 14, 129, 153, 7,
            5, 17, 13, 45, 48, 49, 82, 79, 80, 87, 88, 89, 85, 113, 101, 139, 98, 97, 110, 112,
            121, 155, 96, 107, 324, 330, 44, 126, 53, 135, 134, 136, 67, 108, 109, 114, 128, 156,
            172, 159,
        ],
        CreativeInventoryTab::Decorations => &[
            397, 19, 103, 86, 91, 6, 18, 106, 111, 50, 31, 32, 37, 38, 39, 40, 81, 78, 30, 102, 20,
            321, 389, 323, 47, 390, 170, 35, 171,
        ],
        CreativeInventoryTab::RedstoneAndTransport => &[
            66, 27, 28, 157, 65, 328, 342, 343, 408, 407, 329, 333, 23, 25, 33, 29, 46, 69, 77,
            143, 70, 72, 331, 152, 76, 356, 123, 131, 151, 158, 154, 404, 146, 148, 147,
        ],
        CreativeInventoryTab::Materials => &[
            263, 264, 388, 265, 266, 406, 336, 405, 280, 281, 352, 287, 288, 318, 334, 289, 337,
            348, 295, 362, 361, 296, 338, 344, 353, 341, 369, 371, 372, 351,
        ],
        CreativeInventoryTab::Food => &[
            260, 322, 360, 282, 297, 354, 357, 350, 349, 320, 319, 364, 363, 365, 366, 367, 375,
            392, 393, 394, 391, 396, 400,
        ],
        CreativeInventoryTab::ToolsWeaponsArmor => &[
            345, 298, 299, 300, 301, 268, 269, 270, 271, 290, 395, 302, 303, 304, 305, 272, 273,
            274, 275, 291, 261, 306, 307, 308, 309, 267, 256, 257, 258, 292, 262, 314, 315, 316,
            317, 283, 284, 285, 286, 294, 259, 310, 311, 312, 313, 276, 277, 278, 279, 293, 385,
            347, 359, 346, 398, 420, 419, 418, 417, 403,
        ],
        CreativeInventoryTab::Brewing => &[384, 370, 376, 377, 378, 382, 374, 373],
        CreativeInventoryTab::Misc => &[
            54, 130, 58, 61, 379, 116, 138, 120, 84, 145, 355, 325, 327, 326, 335, 380, 332, 339,
            340, 368, 381, 421, 399, 383, 2256, 2257, 2258, 2259, 2260, 2261, 2262, 2267, 2263,
            2264, 2265, 2266, 401,
        ],
    }
}

pub fn creative_tab_dynamic_group_count(tab: CreativeInventoryTab) -> usize {
    match tab {
        CreativeInventoryTab::Brewing => 0,
        _ => 0,
    }
}

pub fn creative_next_dynamic_group(tab: CreativeInventoryTab, dynamic_group: usize) -> usize {
    let group_count = creative_tab_dynamic_group_count(tab);
    if group_count == 0 {
        0
    } else {
        (dynamic_group + 1) % group_count
    }
}

pub fn creative_tab_items_for_dynamic_group(
    tab: CreativeInventoryTab,
    _dynamic_group: usize,
) -> &'static [u16] {
    creative_tab_items(tab)
}

fn expand_entries_with_aux(tab: CreativeInventoryTab, item_ids: &[u16]) -> Vec<CreativeItemEntry> {
    let mut entries = Vec::with_capacity(item_ids.len() + 32);
    let mut building_quartz_occurrence = 0_u8;

    for item_id in item_ids {
        match (tab, *item_id) {
            (CreativeInventoryTab::BuildingBlocks, 24) => {
                entries.push(creative_entry(24, 0));
                entries.push(creative_entry(24, 1));
                entries.push(creative_entry(24, 2));
            }
            (CreativeInventoryTab::BuildingBlocks, 5)
            | (CreativeInventoryTab::BuildingBlocks, 17)
            | (CreativeInventoryTab::BuildingBlocks, 126)
            | (CreativeInventoryTab::Decorations, 6)
            | (CreativeInventoryTab::Decorations, 18) => {
                for aux in WOOD_VARIANT_AUX_ORDER {
                    entries.push(creative_entry(*item_id, *aux));
                }
            }
            (CreativeInventoryTab::BuildingBlocks, 98) => {
                for aux in STONE_BRICK_AUX_ORDER {
                    entries.push(creative_entry(98, *aux));
                }
            }
            (CreativeInventoryTab::BuildingBlocks, 97) => {
                for aux in MONSTER_EGG_AUX_ORDER {
                    entries.push(creative_entry(97, *aux));
                }
            }
            (CreativeInventoryTab::BuildingBlocks, 155) => {
                if building_quartz_occurrence == 0 {
                    entries.push(creative_entry(155, 0));
                } else {
                    entries.push(creative_entry(155, 1));
                    entries.push(creative_entry(155, 2));
                }
                building_quartz_occurrence = building_quartz_occurrence.saturating_add(1);
            }
            (CreativeInventoryTab::BuildingBlocks, 44) => {
                for aux in STONE_SLAB_AUX_ORDER {
                    entries.push(creative_entry(44, *aux));
                }
            }
            (CreativeInventoryTab::BuildingBlocks, 159) => {
                for aux in WOOL_AUX_ORDER {
                    entries.push(creative_entry(159, *aux));
                }
            }
            (CreativeInventoryTab::Decorations, 397) => {
                for aux in SKULL_AUX_ORDER {
                    entries.push(creative_entry(397, *aux));
                }
            }
            (CreativeInventoryTab::Decorations, 31) => {
                entries.push(creative_entry(31, 0));
                entries.push(creative_entry(31, 1));
                entries.push(creative_entry(31, 2));
            }
            (CreativeInventoryTab::Decorations, 35) | (CreativeInventoryTab::Decorations, 171) => {
                for aux in WOOL_AUX_ORDER {
                    entries.push(creative_entry(*item_id, *aux));
                }
            }
            (CreativeInventoryTab::Materials, 263) => {
                entries.push(creative_entry(263, 0));
                entries.push(creative_entry(263, 1));
            }
            (CreativeInventoryTab::Materials, 351) => {
                for aux in DYE_AUX_ORDER {
                    entries.push(creative_entry(351, *aux));
                }
            }
            (CreativeInventoryTab::ToolsWeaponsArmor, 403) => {
                for aux in ENCHANTED_BOOK_AUX_ORDER {
                    entries.push(creative_entry(403, *aux));
                }
            }
            (CreativeInventoryTab::Food, 322) => {
                entries.push(creative_entry(322, 0));
                entries.push(creative_entry(322, 1));
            }
            (CreativeInventoryTab::Misc, 383) => {
                for aux in SPAWN_EGG_AUX_ORDER {
                    entries.push(creative_entry(383, *aux));
                }
            }
            (CreativeInventoryTab::Misc, 401) => {
                for aux in FIREWORK_PRESET_AUX_ORDER {
                    entries.push(creative_entry(401, *aux));
                }
            }
            (CreativeInventoryTab::BuildingBlocks, 139) => {
                for aux in COBBLE_WALL_AUX_ORDER {
                    entries.push(creative_entry(139, *aux));
                }
            }
            _ => entries.push(creative_entry(*item_id, 0)),
        }
    }

    entries
}

pub fn creative_tab_entries_for_dynamic_group(
    tab: CreativeInventoryTab,
    dynamic_group: usize,
) -> &'static [CreativeItemEntry] {
    let normalized_dynamic_group = normalize_dynamic_group(tab, dynamic_group);
    creative_entry_cache()
        .get(&(tab, normalized_dynamic_group))
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub fn creative_tab_entry_page_count_for_dynamic_group(
    tab: CreativeInventoryTab,
    dynamic_group: usize,
) -> usize {
    let item_count = creative_tab_entries_for_dynamic_group(tab, dynamic_group).len();
    creative_page_count_for_items(item_count)
}

pub fn creative_selector_entries_page_for_dynamic_group(
    tab: CreativeInventoryTab,
    dynamic_group: usize,
    page: usize,
) -> [Option<CreativeItemEntry>; CREATIVE_SELECTOR_SLOTS] {
    let mut selector = [None; CREATIVE_SELECTOR_SLOTS];
    let entries = creative_tab_entries_for_dynamic_group(tab, dynamic_group);
    let start = creative_page_start_index(page);

    for (slot, entry) in selector.iter_mut().zip(entries.iter().skip(start)) {
        *slot = Some(*entry);
    }

    selector
}

pub fn creative_tab_page_count(tab: CreativeInventoryTab) -> usize {
    creative_tab_page_count_for_dynamic_group(tab, 0)
}

pub fn creative_tab_page_count_for_dynamic_group(
    tab: CreativeInventoryTab,
    dynamic_group: usize,
) -> usize {
    let item_count = creative_tab_entries_for_dynamic_group(tab, dynamic_group).len();
    creative_page_count_for_items(item_count)
}

pub fn creative_selector_items_page(
    tab: CreativeInventoryTab,
    page: usize,
) -> [Option<u16>; CREATIVE_SELECTOR_SLOTS] {
    creative_selector_items_page_for_dynamic_group(tab, 0, page)
}

pub fn creative_selector_items_page_for_dynamic_group(
    tab: CreativeInventoryTab,
    dynamic_group: usize,
    page: usize,
) -> [Option<u16>; CREATIVE_SELECTOR_SLOTS] {
    let mut selector = [None; CREATIVE_SELECTOR_SLOTS];
    let entries = creative_tab_entries_for_dynamic_group(tab, dynamic_group);
    let start = creative_page_start_index(page);

    for (slot, entry) in selector.iter_mut().zip(entries.iter().skip(start)) {
        *slot = Some(entry.item_id);
    }

    selector
}

fn creative_entry_cache() -> &'static CreativeEntryCache {
    CREATIVE_ENTRY_CACHE.get_or_init(build_creative_entry_cache)
}

fn creative_page_count_for_items(item_count: usize) -> usize {
    if item_count == 0 {
        return 1;
    }

    let total_rows = item_count.div_ceil(CREATIVE_SELECTOR_COLUMNS);
    total_rows
        .saturating_sub(CREATIVE_SELECTOR_ROWS)
        .saturating_add(1)
}

fn creative_page_start_index(page: usize) -> usize {
    page.saturating_mul(CREATIVE_SELECTOR_COLUMNS)
}

fn build_creative_entry_cache() -> CreativeEntryCache {
    let mut cache = HashMap::new();

    for tab in CREATIVE_TABS {
        let dynamic_group_count = creative_tab_dynamic_group_count(tab).max(1);
        for dynamic_group in 0..dynamic_group_count {
            cache.insert(
                (tab, dynamic_group),
                build_entries_for_dynamic_group(tab, dynamic_group),
            );
        }
    }

    cache
}

fn normalize_dynamic_group(tab: CreativeInventoryTab, dynamic_group: usize) -> usize {
    let dynamic_group_count = creative_tab_dynamic_group_count(tab);
    if dynamic_group_count == 0 {
        0
    } else {
        dynamic_group % dynamic_group_count
    }
}

fn build_entries_for_dynamic_group(
    tab: CreativeInventoryTab,
    _dynamic_group: usize,
) -> Vec<CreativeItemEntry> {
    if tab == CreativeInventoryTab::Brewing {
        let mut entries = Vec::with_capacity(
            BREWING_GROUP_ITEMS.len()
                + BREWING_GROUP_POTIONS_LEVEL2_EXTENDED.len()
                + BREWING_GROUP_POTIONS_EXTENDED.len()
                + BREWING_GROUP_POTIONS_LEVEL2.len()
                + BREWING_GROUP_POTIONS_BASIC.len(),
        );
        entries.extend_from_slice(BREWING_GROUP_ITEMS);
        entries.extend_from_slice(BREWING_GROUP_POTIONS_LEVEL2_EXTENDED);
        entries.extend_from_slice(BREWING_GROUP_POTIONS_EXTENDED);
        entries.extend_from_slice(BREWING_GROUP_POTIONS_LEVEL2);
        entries.extend_from_slice(BREWING_GROUP_POTIONS_BASIC);
        return entries;
    }

    expand_entries_with_aux(tab, creative_tab_items(tab))
}

pub fn target_hotbar_slot_for_creative_item(
    inventory: &PlayerInventory,
    item_id: u16,
    count: u8,
) -> Option<usize> {
    target_hotbar_slot_for_creative_entry(inventory, creative_entry(item_id, 0), count)
}

pub fn target_hotbar_slot_for_creative_entry(
    inventory: &PlayerInventory,
    entry: CreativeItemEntry,
    count: u8,
) -> Option<usize> {
    if count == 0 || count > MAX_STACK_SIZE {
        return None;
    }

    for slot in 0..HOTBAR_SLOTS {
        let existing = inventory
            .get(slot)
            .expect("hotbar slot index should be valid");

        let Some(existing) = existing else {
            continue;
        };

        if existing.item_id != entry.item_id || existing.aux != entry.aux {
            continue;
        }

        if u16::from(existing.count) + u16::from(count) <= u16::from(MAX_STACK_SIZE) {
            return Some(slot);
        }
    }

    for slot in 0..HOTBAR_SLOTS {
        let existing = inventory
            .get(slot)
            .expect("hotbar slot index should be valid");

        if existing.is_none() {
            return Some(slot);
        }
    }

    None
}

pub fn place_creative_item_in_hotbar(
    inventory: &mut PlayerInventory,
    item_id: u16,
    count: u8,
) -> Option<usize> {
    place_creative_entry_in_hotbar(inventory, creative_entry(item_id, 0), count)
}

pub fn place_creative_entry_in_hotbar(
    inventory: &mut PlayerInventory,
    entry: CreativeItemEntry,
    count: u8,
) -> Option<usize> {
    let target_slot = target_hotbar_slot_for_creative_entry(inventory, entry, count)?;
    let existing = inventory
        .get(target_slot)
        .expect("target hotbar slot should be valid");

    let next_stack = if let Some(existing) = existing {
        ItemStack::new_with_aux(
            entry.item_id,
            entry.aux,
            existing.count.saturating_add(count),
        )
        .ok()?
    } else {
        ItemStack::new_with_aux(entry.item_id, entry.aux, count).ok()?
    };

    inventory
        .set(target_slot, Some(next_stack))
        .expect("target hotbar slot should be writable");
    Some(target_slot)
}
