pub fn hide_gameplay_overlay(inventory_open: bool, pause_open: bool, is_dead: bool) -> bool {
    inventory_open || pause_open || is_dead
}

pub fn allow_cursor_capture(inventory_open: bool, pause_open: bool, is_dead: bool) -> bool {
    !inventory_open && !pause_open && !is_dead
}

pub fn show_pause_menu(pause_open: bool, is_dead: bool) -> bool {
    pause_open && !is_dead
}

pub fn show_death_screen(is_dead: bool) -> bool {
    is_dead
}

pub fn allow_first_person_view(
    cursor_captured: bool,
    inventory_open: bool,
    pause_open: bool,
    is_dead: bool,
) -> bool {
    cursor_captured && !inventory_open && !pause_open && !is_dead
}

pub fn allow_first_person_item_view(
    cursor_captured: bool,
    inventory_open: bool,
    pause_open: bool,
    is_dead: bool,
) -> bool {
    !inventory_open && !is_dead && (cursor_captured || pause_open)
}
