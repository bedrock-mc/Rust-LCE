use lce_rust::world::{
    DEFAULT_GROUND_Y, MovementInput, OfflineGameSession, OfflineWorldBootstrap,
    SPRINT_SPEED_MULTIPLIER, WALK_SPEED_BLOCKS_PER_SECOND,
};

const EPSILON: f32 = 0.0001;

#[test]
fn player_spawns_at_ground_origin() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("SpawnTest", 123)
        .expect("world should be created")
        .clone();

    let game = OfflineGameSession::new(world);
    let player = game.player();

    assert_approx(player.position.x, 0.0);
    assert_approx(player.position.y, DEFAULT_GROUND_Y);
    assert_approx(player.position.z, 0.0);
    assert!(player.on_ground);
    assert!(!player.is_flying);
}

#[test]
fn forward_input_moves_player_one_second_of_ticks() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("MoveTest", 321)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    for _ in 0..20 {
        game.tick(MovementInput {
            strafe: 0.0,
            forward: 1.0,
            jump: false,
        });
    }

    let player = game.player();
    assert_approx(player.position.x, 0.0);
    assert_approx(player.position.z, WALK_SPEED_BLOCKS_PER_SECOND);
    assert_approx(player.position.y, DEFAULT_GROUND_Y);
    assert_eq!(game.world().tick_count, 20);
}

#[test]
fn jump_input_lifts_then_returns_player_to_ground() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("JumpTest", 999)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    let after_jump_y = game.player().position.y;
    assert!(after_jump_y > DEFAULT_GROUND_Y);
    assert!(!game.player().on_ground);

    for _ in 0..60 {
        game.tick(MovementInput::default());
    }

    let landed = game.player();
    assert!(landed.on_ground);
    assert_approx(landed.position.y, DEFAULT_GROUND_Y);
}

#[test]
fn jump_reaches_one_block_clearance_height() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("JumpHeight", 1_122)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });

    let mut peak = game.player().position.y;
    for _ in 0..20 {
        game.tick(MovementInput::default());
        peak = peak.max(game.player().position.y);
    }

    assert!(
        peak >= DEFAULT_GROUND_Y + 1.0,
        "peak jump height was {peak}"
    );
}

#[test]
fn double_tap_jump_enables_flight_and_stops_gravity() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("FlightEnableTest", 2_024)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());

    assert!(game.player().is_flying);
    let altitude = game.player().position.y;

    for _ in 0..20 {
        game.tick(MovementInput::default());
    }

    assert!(game.player().is_flying);
    assert_approx(game.player().position.y, altitude);
}

#[test]
fn double_tap_jump_while_flying_disables_flight_and_lands() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("FlightDisableTest", 3_031)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    assert!(game.player().is_flying);

    game.tick(MovementInput::default());
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());

    assert!(!game.player().is_flying);

    for _ in 0..80 {
        game.tick(MovementInput::default());
    }

    assert!(game.player().on_ground);
    assert_approx(game.player().position.y, DEFAULT_GROUND_Y);
}

#[test]
fn flight_toggle_is_blocked_when_flight_is_disabled() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("FlightBlockedTest", 4_117)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);
    game.set_player_allow_flight(false);

    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());

    assert!(!game.player().allow_flight);
    assert!(!game.player().is_flying);
}

#[test]
fn sprinting_scales_ground_speed_by_thirty_percent() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("SprintGroundSpeed", 5_002)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    for _ in 0..20 {
        game.set_player_sprinting(true);
        game.tick(MovementInput {
            strafe: 0.0,
            forward: 1.0,
            jump: false,
        });
    }

    let expected = WALK_SPEED_BLOCKS_PER_SECOND * SPRINT_SPEED_MULTIPLIER;
    assert_approx(game.player().position.z, expected);
}

#[test]
fn sprinting_while_flying_scales_vertical_ascent_speed() {
    let mut bootstrap = OfflineWorldBootstrap::new();
    let world = bootstrap
        .create_world("SprintFlightSpeed", 5_003)
        .expect("world should be created")
        .clone();

    let mut game = OfflineGameSession::new(world);

    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    game.tick(MovementInput::default());
    assert!(game.player().is_flying);

    let baseline_start = game.player().position.y;
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    let baseline_delta = game.player().position.y - baseline_start;

    let sprint_start = game.player().position.y;
    game.set_player_sprinting(true);
    game.tick(MovementInput {
        strafe: 0.0,
        forward: 0.0,
        jump: true,
    });
    let sprint_delta = game.player().position.y - sprint_start;

    assert!(sprint_delta > baseline_delta);
    assert_approx(sprint_delta, baseline_delta * SPRINT_SPEED_MULTIPLIER);
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}"
    );
}
