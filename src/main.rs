use std::time::Duration;

use lce_rust::runtime::{BootSequence, FixedStepLoop};
use lce_rust::world::OfflineWorldBootstrap;

fn main() {
    let mut boot_sequence = BootSequence::new();
    boot_sequence
        .register("world_static_ctors", || Ok(()))
        .expect("failed to register world boot step");
    boot_sequence
        .register("client_static_ctors", || Ok(()))
        .expect("failed to register client boot step");
    let boot_report = boot_sequence.run().expect("boot sequence failed");

    let mut runtime = FixedStepLoop::default();
    let dispatch = runtime.update(Duration::from_millis(16));
    let mut world_bootstrap = OfflineWorldBootstrap::new();
    let world = world_bootstrap
        .create_world("World1", 12345)
        .expect("failed to create world");

    println!(
        "LCE-Rust bootstrap ready (boot steps: {}, world: {}, ticks: {}, alpha: {:.3}, dropped: {})",
        boot_report.completed_steps.len(),
        world.name,
        dispatch.ticks_to_run,
        dispatch.alpha,
        dispatch.dropped_ticks
    );
}
