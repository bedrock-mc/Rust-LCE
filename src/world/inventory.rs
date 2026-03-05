use std::fmt;

pub const HOTBAR_SLOTS: usize = 9;
pub const INVENTORY_SLOTS: usize = 36;
pub const MAX_STACK_SIZE: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStack {
    pub item_id: u16,
    pub aux: u16,
    pub count: u8,
}

impl ItemStack {
    pub fn new(item_id: u16, count: u8) -> Result<Self, InventoryError> {
        Self::new_with_aux(item_id, 0, count)
    }

    pub fn new_with_aux(item_id: u16, aux: u16, count: u8) -> Result<Self, InventoryError> {
        if count == 0 {
            return Err(InventoryError::InvalidStackCount(0));
        }

        if count > MAX_STACK_SIZE {
            return Err(InventoryError::InvalidStackCount(count));
        }

        Ok(Self {
            item_id,
            aux,
            count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    InvalidSlot(usize),
    InvalidHotbarSlot(usize),
    InvalidStackCount(u8),
}

impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSlot(slot) => write!(f, "invalid inventory slot: {slot}"),
            Self::InvalidHotbarSlot(slot) => write!(f, "invalid hotbar slot: {slot}"),
            Self::InvalidStackCount(count) => write!(f, "invalid stack count: {count}"),
        }
    }
}

impl std::error::Error for InventoryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInventory {
    slots: [Option<ItemStack>; INVENTORY_SLOTS],
    selected_hotbar_slot: usize,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            slots: [None; INVENTORY_SLOTS],
            selected_hotbar_slot: 0,
        }
    }
}

impl PlayerInventory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected_hotbar_slot(&self) -> usize {
        self.selected_hotbar_slot
    }

    pub fn select_hotbar_slot(&mut self, slot: usize) -> Result<(), InventoryError> {
        if slot >= HOTBAR_SLOTS {
            return Err(InventoryError::InvalidHotbarSlot(slot));
        }

        self.selected_hotbar_slot = slot;
        Ok(())
    }

    pub fn selected_stack(&self) -> Option<ItemStack> {
        self.slots[self.selected_hotbar_slot]
    }

    pub fn get(&self, slot: usize) -> Result<Option<ItemStack>, InventoryError> {
        if slot >= INVENTORY_SLOTS {
            return Err(InventoryError::InvalidSlot(slot));
        }

        Ok(self.slots[slot])
    }

    pub fn set(&mut self, slot: usize, stack: Option<ItemStack>) -> Result<(), InventoryError> {
        if slot >= INVENTORY_SLOTS {
            return Err(InventoryError::InvalidSlot(slot));
        }

        if let Some(stack) = stack
            && (stack.count == 0 || stack.count > MAX_STACK_SIZE)
        {
            return Err(InventoryError::InvalidStackCount(stack.count));
        }

        self.slots[slot] = stack;
        Ok(())
    }

    pub fn add_item(&mut self, item_id: u16, count: u32) -> u32 {
        self.add_item_with_aux(item_id, 0, count)
    }

    pub fn can_add_item(&self, item_id: u16, count: u32) -> bool {
        self.can_add_item_with_aux(item_id, 0, count)
    }

    pub fn can_add_item_with_aux(&self, item_id: u16, aux: u16, mut count: u32) -> bool {
        if count == 0 {
            return true;
        }

        for slot in &self.slots {
            if count == 0 {
                return true;
            }

            if let Some(stack) = slot
                && stack.item_id == item_id
                && stack.aux == aux
                && stack.count < MAX_STACK_SIZE
            {
                let remaining_space = u32::from(MAX_STACK_SIZE - stack.count);
                count = count.saturating_sub(remaining_space.min(count));
            }
        }

        let empty_slots = self.slots.iter().filter(|slot| slot.is_none()).count();
        let free_capacity = u32::try_from(empty_slots)
            .unwrap_or(u32::MAX)
            .saturating_mul(u32::from(MAX_STACK_SIZE));

        count <= free_capacity
    }

    pub fn add_item_with_aux(&mut self, item_id: u16, aux: u16, mut count: u32) -> u32 {
        if count == 0 {
            return 0;
        }

        for slot in &mut self.slots {
            if count == 0 {
                break;
            }

            if let Some(stack) = slot
                && stack.item_id == item_id
                && stack.aux == aux
                && stack.count < MAX_STACK_SIZE
            {
                let remaining_space = u32::from(MAX_STACK_SIZE - stack.count);
                let to_add = remaining_space.min(count);
                stack.count = stack
                    .count
                    .saturating_add(u8::try_from(to_add).expect("to_add should fit u8"));
                count -= to_add;
            }
        }

        for slot in &mut self.slots {
            if count == 0 {
                break;
            }

            if slot.is_none() {
                let to_add = u32::from(MAX_STACK_SIZE).min(count);
                *slot = Some(ItemStack {
                    item_id,
                    aux,
                    count: u8::try_from(to_add).expect("stack count should fit u8"),
                });
                count -= to_add;
            }
        }

        count
    }

    pub fn total_count(&self, item_id: u16) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|stack| stack.item_id == item_id)
            .map(|stack| u32::from(stack.count))
            .sum()
    }

    pub fn remove_item(&mut self, item_id: u16, mut count: u32) -> u32 {
        if count == 0 {
            return 0;
        }

        for slot in &mut self.slots {
            if count == 0 {
                break;
            }

            let Some(stack) = slot else {
                continue;
            };

            if stack.item_id != item_id {
                continue;
            }

            let take = u32::from(stack.count).min(count);
            stack.count = stack
                .count
                .saturating_sub(u8::try_from(take).expect("take should fit u8"));
            count -= take;

            if stack.count == 0 {
                *slot = None;
            }
        }

        count
    }

    pub fn consume_item_exact(&mut self, item_id: u16, count: u32) -> bool {
        if self.total_count(item_id) < count {
            return false;
        }

        let remaining = self.remove_item(item_id, count);
        remaining == 0
    }

    pub fn consume_selected(&mut self, amount: u8) -> bool {
        if amount == 0 {
            return true;
        }

        let slot = &mut self.slots[self.selected_hotbar_slot];
        let Some(stack) = slot else {
            return false;
        };

        if stack.count < amount {
            return false;
        }

        stack.count -= amount;
        if stack.count == 0 {
            *slot = None;
        }

        true
    }
}
