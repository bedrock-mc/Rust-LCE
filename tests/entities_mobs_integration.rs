use lce_rust::world::{
    EntityKind, MobKind, MovementInput, OfflineGameSession, OfflineWorldBootstrap, Vec3,
};

#[test]
fn session_bootstrap_includes_default_mob_entity() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("EntitiesBootstrap", 12)
        .expect("world should be created")
        .clone();

    let game = OfflineGameSession::new(world);
    assert_eq!(game.entities().mob_count(), 1);

    let only_mob = game
        .entities()
        .entities()
        .next()
        .expect("default mob should exist");
    assert!(matches!(only_mob.kind, EntityKind::Mob(MobKind::Pig)));
    assert!(!only_mob.is_dead);
}

#[test]
fn mob_simulation_is_seed_stable_across_sessions() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world_a = bootstrap
        .create_world("EntitiesStableA", 777)
        .expect("world should be created")
        .clone();
    let world_b = bootstrap
        .create_world("EntitiesStableB", 777)
        .expect("world should be created")
        .clone();

    let mut a = OfflineGameSession::new(world_a);
    let mut b = OfflineGameSession::new(world_b);

    let custom_a = a.spawn_mob(MobKind::Zombie, Vec3::new(8.0, 64.0, 8.0));
    let custom_b = b.spawn_mob(MobKind::Zombie, Vec3::new(8.0, 64.0, 8.0));

    for _ in 0..40 {
        a.tick(MovementInput::default());
        b.tick(MovementInput::default());
    }

    let a_state = a
        .entities()
        .entity(custom_a)
        .expect("entity should exist in session a");
    let b_state = b
        .entities()
        .entity(custom_b)
        .expect("entity should exist in session b");

    assert_eq!(a_state.position, b_state.position);
    assert_eq!(a_state.velocity, b_state.velocity);
    assert_eq!(a_state.on_ground, b_state.on_ground);
}

#[test]
fn entity_damage_path_updates_health_and_death_state() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("EntityDamage", 404)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);
    let entity_id = game.spawn_mob(MobKind::Zombie, Vec3::new(2.0, 64.0, -2.0));

    let killed = game.apply_entity_damage(entity_id, 4);
    assert!(!killed);
    let state = game
        .entities()
        .entity(entity_id)
        .expect("entity should still exist after partial damage");
    assert_eq!(state.health, 6);
    assert!(!state.is_dead);

    let killed = game.apply_entity_damage(entity_id, 6);
    assert!(killed);

    let state = game
        .entities()
        .entity(entity_id)
        .expect("entity should remain addressable after death");
    assert_eq!(state.health, 0);
    assert!(state.is_dead);
}
