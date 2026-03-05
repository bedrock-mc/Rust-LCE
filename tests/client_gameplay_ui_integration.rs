use lce_rust::client::gameplay_ui::{
    allow_cursor_capture, allow_first_person_view, hide_gameplay_overlay, show_death_screen,
    show_pause_menu,
};

#[test]
fn gameplay_overlay_hides_for_inventory_pause_or_death() {
    assert!(!hide_gameplay_overlay(false, false, false));
    assert!(hide_gameplay_overlay(true, false, false));
    assert!(hide_gameplay_overlay(false, true, false));
    assert!(hide_gameplay_overlay(false, false, true));
}

#[test]
fn cursor_capture_is_only_allowed_in_active_gameplay() {
    assert!(allow_cursor_capture(false, false, false));
    assert!(!allow_cursor_capture(true, false, false));
    assert!(!allow_cursor_capture(false, true, false));
    assert!(!allow_cursor_capture(false, false, true));
}

#[test]
fn pause_and_death_screen_visibility_follows_expected_states() {
    assert!(show_pause_menu(true, false));
    assert!(!show_pause_menu(true, true));
    assert!(!show_pause_menu(false, false));

    assert!(show_death_screen(true));
    assert!(!show_death_screen(false));
}

#[test]
fn first_person_overlay_requires_captured_cursor_and_no_modal_ui() {
    assert!(allow_first_person_view(true, false, false, false));
    assert!(!allow_first_person_view(false, false, false, false));
    assert!(!allow_first_person_view(true, true, false, false));
    assert!(!allow_first_person_view(true, false, true, false));
    assert!(!allow_first_person_view(true, false, false, true));
}
