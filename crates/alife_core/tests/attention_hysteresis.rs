use alife_core::{
    select_focal_targets, AttentionFrame, AttentionSelectionPolicy, Confidence,
    ExperienceSequenceId, HysteresisState, NormalizedScalar, OrganismId, PeripheralSummary,
    SalienceComponents, StableFocusIdentity, Tick, TrackedObjectId, Validate,
};

fn identity(raw: u64) -> StableFocusIdentity {
    StableFocusIdentity::TrackedObject(TrackedObjectId(raw))
}

fn summary(raw: u64, drive: f32) -> PeripheralSummary {
    PeripheralSummary {
        identity: identity(raw),
        salience: SalienceComponents {
            drive: NormalizedScalar::new(drive).unwrap(),
            ..SalienceComponents::default()
        },
        confidence: Confidence::new(1.0).unwrap(),
    }
}

fn select(
    summaries: &[PeripheralSummary],
    previous: HysteresisState,
    requested: u8,
) -> AttentionFrame {
    select_focal_targets(
        OrganismId(7),
        ExperienceSequenceId(1),
        Tick::new(10),
        summaries,
        previous,
        AttentionSelectionPolicy {
            focal_capacity: requested,
            requested_focal_count: requested,
            ..AttentionSelectionPolicy::default()
        },
    )
    .unwrap()
}

fn incumbent(raw: u64) -> HysteresisState {
    HysteresisState {
        previous_identity: Some(identity(raw)),
        retained_ticks: 4,
        ..HysteresisState::default()
    }
}

#[test]
fn one_focal_slot_retains_incumbent_against_small_salience_changes() {
    let first = select(&[summary(1, 0.5), summary(2, 0.6)], incumbent(1), 1);
    assert_eq!(first.focal_targets, vec![identity(1)]);
    assert_eq!(first.hysteresis.previous_identity, Some(identity(1)));
    assert_eq!(first.hysteresis.retained_ticks, 5);

    let second = select(&[summary(2, 0.7), summary(1, 0.5)], first.hysteresis, 1);
    assert_eq!(second.focal_targets, vec![identity(1)]);
    assert_eq!(second.hysteresis.retained_ticks, 6);
    second.validate_contract().unwrap();
}

#[test]
fn decisive_challenger_replaces_incumbent_without_waiting() {
    let attention = select(&[summary(1, 0.5), summary(2, 0.9)], incumbent(1), 1);
    assert_eq!(attention.focal_targets, vec![identity(2)]);
    assert_eq!(attention.hysteresis.retained_ticks, 1);
    attention.validate_contract().unwrap();
}

#[test]
fn switch_cost_boundary_retains_but_next_float_above_switches() {
    // Only drive contributes: 0.20 * 0.25 is exactly the default 0.05 cost.
    let boundary = select(&[summary(1, 0.0), summary(2, 0.25)], incumbent(1), 1);
    assert_eq!(boundary.focal_targets, vec![identity(1)]);
    let above = f32::from_bits(0.25_f32.to_bits() + 1);
    let switched = select(
        &[summary(1, 0.0), summary(2, above)],
        boundary.hysteresis,
        1,
    );
    assert_eq!(switched.focal_targets, vec![identity(2)]);
    assert_eq!(switched.hysteresis.retained_ticks, 1);
}

#[test]
fn absent_incumbent_is_never_reintroduced() {
    // A stale identity cannot restore a target absent from current summaries.
    // World action legality remains separate, including for visible dead bodies.
    let attention = select(&[summary(2, 0.1)], incumbent(1), 1);
    assert_eq!(attention.focal_targets, vec![identity(2)]);
    assert_eq!(attention.hysteresis.retained_ticks, 1);

    let empty = select(&[], incumbent(1), 1);
    assert!(empty.focal_targets.is_empty());
    assert_eq!(empty.hysteresis.previous_identity, None);
    assert_eq!(empty.hysteresis.retained_ticks, 0);
}

#[test]
fn multiple_focal_slots_keep_incumbent_and_strongest_remaining_targets() {
    let summaries = [summary(3, 0.55), summary(2, 0.6), summary(1, 0.5)];
    let attention = select(&summaries, incumbent(1), 2);
    assert_eq!(attention.focal_targets, vec![identity(1), identity(2)]);
    assert_eq!(attention.salience_components.len(), 2);
    assert_eq!(attention.budget_receipt.work_units, 15);
    attention.validate_contract().unwrap();
}

#[test]
fn zero_focal_slots_do_not_retain_an_incumbent() {
    let attention = select(&[summary(1, 0.5), summary(2, 0.6)], incumbent(1), 0);
    assert!(attention.focal_targets.is_empty());
    assert_eq!(attention.hysteresis.previous_identity, None);
    assert_eq!(attention.hysteresis.retained_ticks, 0);
    attention.validate_contract().unwrap();
}

#[test]
fn no_incumbent_preserves_raw_salience_order_and_canonical_ties() {
    let attention = select(
        &[summary(3, 0.6), summary(2, 0.6), summary(1, 0.5)],
        HysteresisState::default(),
        2,
    );
    assert_eq!(attention.focal_targets, vec![identity(2), identity(3)]);
    attention.validate_contract().unwrap();
}

#[test]
fn nonadjacent_duplicate_focus_identities_are_rejected() {
    let mut attention = select(
        &[summary(1, 0.9), summary(2, 0.8), summary(3, 0.7)],
        HysteresisState::default(),
        3,
    );
    attention.focal_targets[2] = attention.focal_targets[0];
    assert!(attention.validate_contract().is_err());
}
