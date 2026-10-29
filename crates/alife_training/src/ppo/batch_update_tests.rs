//! CPU orchestration evidence only; these fixtures do not execute GPU arithmetic.
use std::cell::RefCell;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Failure {
    Backup,
    Prepare,
    ActorBeforeSubmit,
    ActorAfterSubmit,
    Restore,
}

struct Host {
    operations: RefCell<Vec<&'static str>>,
    // Two coordinates in each of the five value-head planes, with literal bits.
    value: RefCell<[u32; 10]>,
    actor: [u32; 6],
    actor_age: u32,
    ready: bool,
    failure: Option<Failure>,
}

impl Host {
    fn new(failure: Option<Failure>) -> Self {
        Self {
            operations: RefCell::new(Vec::new()),
            value: RefCell::new([
                1.0_f32.to_bits(),
                (-0.0_f32).to_bits(),
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
            ]),
            actor: [10, 11, 12, 13, 14, 15],
            actor_age: 17,
            ready: true,
            failure,
        }
    }

    fn apply(&mut self, enabled: bool) -> Result<u32, TrainingError> {
        apply_ppo_batch_update(
            self,
            enabled,
            |host| {
                host.operations.borrow_mut().push("backup");
                if host.failure == Some(Failure::Backup) {
                    return Err(TrainingError::GpuSubmission);
                }
                Ok(*host.value.borrow())
            },
            |host, enabled| {
                host.operations.borrow_mut().push("prepare");
                if host.failure == Some(Failure::Prepare) {
                    return Err(invalid());
                }
                let dispatch = value_update_dispatch_enabled(host.ready, enabled)?;
                if dispatch {
                    host.operations.borrow_mut().push("value_dispatch");
                }
                // A dispatched zero-coefficient GPU kernel still returns before
                // writing. Model that separately from whether it was dispatched.
                Ok(dispatch && enabled)
            },
            |host, dispatch| {
                host.operations.borrow_mut().push("actor");
                if host.failure == Some(Failure::ActorBeforeSubmit) {
                    return Err(invalid());
                }
                if dispatch {
                    for word in &mut host.value.borrow_mut()[..6] {
                        *word ^= 1;
                    }
                }
                for word in &mut host.actor {
                    *word ^= 1;
                }
                if matches!(
                    host.failure,
                    Some(Failure::ActorAfterSubmit | Failure::Restore)
                ) {
                    return Err(TrainingError::MalformedReadback);
                }
                host.actor_age += 1;
                Ok(host.actor_age)
            },
            |host, backup| {
                host.operations.borrow_mut().push("restore");
                if host.failure == Some(Failure::Restore) {
                    return Err(TrainingError::GpuSubmission);
                }
                *host.value.borrow_mut() = backup;
                Ok(())
            },
        )
    }
}

#[test]
fn zero_value_update_skips_backup_and_dispatch_but_keeps_actor_work() {
    for coefficient in [0.0_f32, -0.0_f32] {
        let mut host = Host::new(None);
        let value = *host.value.borrow();
        let mut positive = Host::new(None);
        assert_eq!(positive.apply(true).unwrap(), 18);
        assert_eq!(host.apply(coefficient > 0.0).unwrap(), 18);
        assert_eq!(*host.operations.borrow(), ["prepare", "actor"]);
        assert_eq!(
            *host.value.borrow(),
            value,
            "all five value planes remain exact"
        );
        assert_eq!(host.actor, positive.actor);
        assert_eq!(host.actor_age, positive.actor_age);
    }
}

#[test]
fn positive_value_update_retains_backup_apply_and_actor_failure_restore_order() {
    let mut host = Host::new(None);
    let value = *host.value.borrow();
    assert_eq!(host.apply(true).unwrap(), 18);
    assert_eq!(
        *host.operations.borrow(),
        ["backup", "prepare", "value_dispatch", "actor"]
    );
    assert_ne!(host.value.borrow()[..6], value[..6]);
    assert_eq!(host.value.borrow()[6..], value[6..]);
    for failure in [Failure::ActorBeforeSubmit, Failure::ActorAfterSubmit] {
        let mut host = Host::new(Some(failure));
        let value = *host.value.borrow();
        let actor = host.actor;
        assert!(host.apply(true).is_err());
        assert_eq!(
            *host.operations.borrow(),
            ["backup", "prepare", "value_dispatch", "actor", "restore"]
        );
        assert_eq!(*host.value.borrow(), value);
        assert_eq!(host.actor_age, 17);
        assert_eq!(
            host.actor == actor,
            failure == Failure::ActorBeforeSubmit,
            "the helper restores value only; it does not add actor rollback"
        );
    }
}

#[test]
fn zero_value_actor_failures_propagate_without_value_cleanup() {
    for failure in [
        Failure::ActorBeforeSubmit,
        Failure::ActorAfterSubmit,
        Failure::Restore,
    ] {
        let mut host = Host::new(Some(failure));
        let value = *host.value.borrow();
        let result = host.apply(false);
        if failure == Failure::ActorBeforeSubmit {
            assert!(matches!(result, Err(TrainingError::Contract(_))));
        } else {
            assert!(matches!(result, Err(TrainingError::MalformedReadback)));
        }
        assert_eq!(*host.operations.borrow(), ["prepare", "actor"]);
        assert_eq!(*host.value.borrow(), value);
        assert_eq!(host.actor_age, 17);
    }
}

#[test]
fn backup_prepare_and_restore_failures_keep_existing_error_precedence() {
    for (failure, operations) in [
        (Failure::Backup, vec!["backup"]),
        (Failure::Prepare, vec!["backup", "prepare"]),
        (
            Failure::Restore,
            vec!["backup", "prepare", "value_dispatch", "actor", "restore"],
        ),
    ] {
        let mut host = Host::new(Some(failure));
        let result = host.apply(true);
        if failure == Failure::Prepare {
            assert!(matches!(result, Err(TrainingError::Contract(_))));
        } else {
            assert!(matches!(result, Err(TrainingError::GpuSubmission)));
        }
        assert_eq!(*host.operations.borrow(), operations);
        assert_eq!(host.actor_age, 17);
    }
}

#[test]
fn disabled_value_update_still_requires_a_ready_objective() {
    for enabled in [false, true] {
        assert!(value_update_dispatch_enabled(false, enabled).is_err());
        assert_eq!(
            value_update_dispatch_enabled(true, enabled).unwrap(),
            enabled
        );
        let mut host = Host::new(None);
        host.ready = false;
        assert!(matches!(
            host.apply(enabled),
            Err(TrainingError::Contract(_))
        ));
        assert!(!host.operations.borrow().contains(&"actor"));
        assert!(!host.operations.borrow().contains(&"restore"));
    }
}

#[test]
fn coefficient_boundaries_validate_before_disabling_value_work() {
    for coefficient in [
        0.0_f32,
        -0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        0.5,
        f32::MAX,
    ] {
        PpoConfig {
            value_coefficient: coefficient,
            ..Default::default()
        }
        .validate()
        .unwrap();
        assert_eq!(coefficient > 0.0, coefficient != 0.0);
    }
    for coefficient in [
        -f32::from_bits(1),
        -0.5,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NAN,
    ] {
        assert!(PpoConfig {
            value_coefficient: coefficient,
            ..Default::default()
        }
        .validate()
        .is_err());
    }
    for coefficient in [0.0_f32, -0.0] {
        let mut config = PpoConfig {
            value_coefficient: coefficient,
            ..Default::default()
        };
        config.value_optimizer.learning_rate = f32::NAN;
        assert!(
            config.validate().is_err(),
            "disabled value work cannot bypass optimizer validation"
        );
    }
}
