use std::cell::RefCell;
use std::rc::Rc;

use lce_rust::runtime::{BootSequence, BootstrapError};

#[test]
fn runs_boot_steps_in_registration_order() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut sequence = BootSequence::new();

    {
        let order = Rc::clone(&order);
        sequence
            .register("world_static_ctors", move || {
                order.borrow_mut().push("world_static_ctors");
                Ok(())
            })
            .expect("failed to register world step");
    }

    {
        let order = Rc::clone(&order);
        sequence
            .register("client_static_ctors", move || {
                order.borrow_mut().push("client_static_ctors");
                Ok(())
            })
            .expect("failed to register client step");
    }

    let report = sequence.run().expect("boot sequence should succeed");

    assert_eq!(
        order.borrow().as_slice(),
        ["world_static_ctors", "client_static_ctors"]
    );
    assert_eq!(
        report.completed_steps,
        vec!["world_static_ctors", "client_static_ctors"]
    );
}

#[test]
fn rejects_duplicate_boot_step_ids() {
    let mut sequence = BootSequence::new();

    sequence
        .register("world_static_ctors", || Ok(()))
        .expect("initial registration should succeed");

    let duplicate = sequence.register("world_static_ctors", || Ok(()));

    assert_eq!(
        duplicate,
        Err(BootstrapError::DuplicateStep("world_static_ctors"))
    );
}

#[test]
fn stops_at_first_failure_and_reports_progress() {
    let mut sequence = BootSequence::new();

    sequence
        .register("world_static_ctors", || Ok(()))
        .expect("world step should register");
    sequence
        .register("item_registry", || Err("item init failed".to_string()))
        .expect("item step should register");
    sequence
        .register("client_boot", || Ok(()))
        .expect("client step should register");

    let result = sequence.run();

    assert_eq!(
        result,
        Err(BootstrapError::StepFailed {
            step: "item_registry",
            reason: "item init failed".to_string(),
            completed_steps: vec!["world_static_ctors"],
        })
    );
}
