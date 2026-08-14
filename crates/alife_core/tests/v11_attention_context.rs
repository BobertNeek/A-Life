use alife_core::{
    select_focal_targets, AttentionSelectionPolicy, CognitiveContextFrame, Confidence,
    ExperienceSequenceId, HysteresisState, NormalizedScalar, OrganismId, PeripheralSummary,
    SalienceComponents, StableFocusIdentity, Tick, TrackedObjectId, Validate,
};

fn summary(id: u64, salience: SalienceComponents) -> PeripheralSummary {
    PeripheralSummary {
        identity: StableFocusIdentity::TrackedObject(TrackedObjectId(id)),
        salience,
        confidence: Confidence::new(1.0).unwrap(),
    }
}

fn context_for(attention: alife_core::AttentionFrame) -> CognitiveContextFrame {
    let mut context = CognitiveContextFrame::empty(
        OrganismId(1),
        ExperienceSequenceId(1),
        Tick::new(7),
    )
    .unwrap();
    context.attention = attention.clone();
    context.peripheral.summaries = attention.peripheral_summaries.clone();
    context.focal.identities = attention.focal_targets.clone();
    context.focal.salience = attention.salience_components.clone();
    context.focal.hysteresis = attention.hysteresis;
    context.budget.peripheral_capacity = attention.budget_receipt.peripheral_capacity;
    context.budget.focal_capacity = attention.budget_receipt.focal_capacity;
    context.budget.work_used = attention.budget_receipt.work_units;
    context.budget.work_limit = attention.budget_receipt.work_units;
    context.validate_contract().unwrap();
    context
}

#[test]
fn memory_salience_changes_focal_identity_and_predecision_context() {
    let policy = AttentionSelectionPolicy {
        focal_capacity: 1,
        requested_focal_count: 1,
        ..AttentionSelectionPolicy::default()
    };
    let base = SalienceComponents {
        peripheral_intensity: NormalizedScalar::new(0.2).unwrap(),
        ..SalienceComponents::default()
    };
    let mut summaries = vec![summary(1, base), summary(2, base)];
    let first = select_focal_targets(
        OrganismId(1),
        ExperienceSequenceId(1),
        Tick::new(7),
        &summaries,
        HysteresisState::default(),
        policy,
    )
    .unwrap();
    assert_eq!(
        first.focal_targets,
        vec![StableFocusIdentity::TrackedObject(TrackedObjectId(1))]
    );

    summaries[1].salience.memory_expectancy = NormalizedScalar::new(1.0).unwrap();
    let changed = select_focal_targets(
        OrganismId(1),
        ExperienceSequenceId(1),
        Tick::new(7),
        &summaries,
        HysteresisState::default(),
        policy,
    )
    .unwrap();
    assert_eq!(
        changed.focal_targets,
        vec![StableFocusIdentity::TrackedObject(TrackedObjectId(2))]
    );
    assert_ne!(
        context_for(first).canonical_digest().unwrap(),
        context_for(changed).canonical_digest().unwrap()
    );
}
