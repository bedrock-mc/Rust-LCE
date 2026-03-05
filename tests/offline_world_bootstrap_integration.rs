use lce_rust::world::{OfflineWorldBootstrap, WorldBootstrapError};

#[test]
fn create_tick_save_and_reload_world_round_trip() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    bootstrap
        .create_world("Tutorial", 9001)
        .expect("world should be created");
    bootstrap
        .tick_active_world(42)
        .expect("world ticks should be applied");

    let snapshot = bootstrap
        .save_active_world()
        .expect("snapshot should be saved");

    let mut reload = OfflineWorldBootstrap::new();
    let world = reload.load_world(snapshot);

    assert_eq!(world.name, "Tutorial");
    assert_eq!(world.seed, 9001);
    assert_eq!(world.tick_count, 42);
}

#[test]
fn save_without_active_world_returns_error() {
    let bootstrap = OfflineWorldBootstrap::new();

    let result = bootstrap.save_active_world();

    assert_eq!(result, Err(WorldBootstrapError::MissingActiveWorld));
}

#[test]
fn rejects_invalid_world_names() {
    let mut bootstrap = OfflineWorldBootstrap::new();

    let result = bootstrap.create_world("   ", 17);

    assert_eq!(result, Err(WorldBootstrapError::InvalidWorldName));
}
