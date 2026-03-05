use lce_rust::world::{
    DEFAULT_GROUND_Y, MovementInput, OfflineGameSession, OfflineWorldBootstrap, PLAYER_MAX_HEALTH,
};

#[test]
fn damage_can_kill_player_and_freezes_movement_until_respawn() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("CombatWorld", 8080)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);
    let died = game.apply_player_damage(PLAYER_MAX_HEALTH);
    assert!(died);
    assert!(game.player().is_dead);
    assert_eq!(game.player().health, 0);

    let before = game.player().position;
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 1.0,
        jump: true,
        sneak: false,
    });
    assert_eq!(game.player().position, before);
}

#[test]
fn healing_clamps_and_respawn_restores_state() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("RespawnWorld", 99)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);
    game.apply_player_damage(6);
    game.heal_player(3);
    assert_eq!(game.player().health, PLAYER_MAX_HEALTH - 3);

    game.heal_player(20);
    assert_eq!(game.player().health, PLAYER_MAX_HEALTH);

    game.apply_player_damage(PLAYER_MAX_HEALTH);
    assert!(game.player().is_dead);
    game.heal_player(4);
    assert_eq!(game.player().health, 0);

    game.respawn_player();
    assert!(!game.player().is_dead);
    assert_eq!(game.player().health, PLAYER_MAX_HEALTH);
    assert_eq!(game.player().position.y, DEFAULT_GROUND_Y);
}
