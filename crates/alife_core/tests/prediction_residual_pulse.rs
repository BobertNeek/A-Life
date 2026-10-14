use alife_core::*;

fn fixture() -> (CreaturePhenotype, BiochemistryState) {
    let phenotype = CreatureGenome::early_mammal_founder(
        0xA0A_2001,
        FoundationGeneticIdentity::new(20, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let maturation = u64::from(phenotype.development.maturation_duration_ticks);
    let tick = Tick(maturation.div_ceil(120) * 120);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    (phenotype, state)
}

fn pulse(state: &BiochemistryState) -> BiochemistryState {
    state
        .with_prediction_residual_pulse(ExperienceSequenceId::new(7).unwrap(), 11, 0.8, 1.0)
        .unwrap()
}

#[test]
fn prediction_pulse_changes_next_chemistry_once_and_survives_both_save_boundaries() {
    let (phenotype, state) = fixture();
    let queued = pulse(&state);
    assert_eq!(queued.graph_state(), state.graph_state());
    assert_eq!(queued.homeostasis, state.homeostasis);
    let queued: BiochemistryState =
        serde_json::from_str(&serde_json::to_string(&queued).unwrap()).unwrap();
    let next = Tick(state.tick.raw() + 1);
    let emission = NeuralEmissionFrame::new(
        state.tick,
        11,
        vec![NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.8, 1.0).unwrap()],
    )
    .unwrap();
    let expected = state
        .advance_with_neural_emission(
            next,
            next,
            BodyEventDelta::zero(),
            Some(&emission),
            &phenotype,
        )
        .unwrap();
    let actual = queued
        .advance(next, BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(actual, expected);
    let neutral = state
        .advance(next, BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert!(actual.homeostasis.hormones.adrenaline > neutral.homeostasis.hormones.adrenaline);
    assert!(
        actual.homeostasis.hormones.learning_modulator
            > neutral.homeostasis.hormones.learning_modulator
    );
    assert!(serde_json::to_value(actual)
        .unwrap()
        .get("pending_prediction_residual")
        .is_none());
    let restored: BiochemistryState =
        serde_json::from_str(&serde_json::to_string(&actual).unwrap()).unwrap();
    let later = Tick(next.raw() + 1);
    assert_eq!(
        restored
            .advance(later, BodyEventDelta::zero(), &phenotype)
            .unwrap(),
        expected
            .advance(later, BodyEventDelta::zero(), &phenotype)
            .unwrap()
    );
}

#[test]
fn prediction_pulse_preserves_other_neural_sources_and_cannot_launder_stale_frames() {
    let (phenotype, state) = fixture();
    let queued = pulse(&state);
    let next = Tick(state.tick.raw() + 1);
    let arousal = NeuralEmission::new(NeuralEmissionClass::RegionalArousal, 0.2, 0.9).unwrap();
    let existing = NeuralEmissionFrame::new(state.tick, 19, vec![arousal]).unwrap();
    let combined = NeuralEmissionFrame::new(
        state.tick,
        19,
        vec![
            arousal,
            NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.8, 1.0).unwrap(),
        ],
    )
    .unwrap();
    let actual = queued
        .advance_with_neural_emission(
            next,
            next,
            BodyEventDelta::zero(),
            Some(&existing),
            &phenotype,
        )
        .unwrap();
    assert_eq!(
        actual,
        state
            .advance_with_neural_emission(
                next,
                next,
                BodyEventDelta::zero(),
                Some(&combined),
                &phenotype
            )
            .unwrap()
    );
    let stale = NeuralEmissionFrame::new(Tick(state.tick.raw() - 1), 19, vec![arousal]).unwrap();
    assert!(queued
        .advance_with_neural_emission(next, next, BodyEventDelta::zero(), Some(&stale), &phenotype)
        .is_err());
    let mut invalid_schema = existing.clone();
    invalid_schema.schema_version += 1;
    assert!(queued
        .advance_with_neural_emission(
            next,
            next,
            BodyEventDelta::zero(),
            Some(&invalid_schema),
            &phenotype
        )
        .is_err());
    assert_eq!(
        queued
            .advance_with_neural_emission(
                next,
                next,
                BodyEventDelta::zero(),
                Some(&existing),
                &phenotype
            )
            .unwrap(),
        actual
    );
}

#[test]
fn catch_up_update_consumes_one_pulse_without_repeating_it_for_elapsed_ticks() {
    let (phenotype, state) = fixture();
    let next = Tick(state.tick.raw() + 13);
    let emission = NeuralEmissionFrame::new(
        state.tick,
        11,
        vec![NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.8, 1.0).unwrap()],
    )
    .unwrap();
    let expected = state
        .advance_with_neural_emission(
            next,
            next,
            BodyEventDelta::zero(),
            Some(&emission),
            &phenotype,
        )
        .unwrap();
    let actual = pulse(&state)
        .advance(next, BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(actual, expected);
    let later = Tick(next.raw() + 1);
    assert_eq!(
        actual
            .advance(later, BodyEventDelta::zero(), &phenotype)
            .unwrap(),
        expected
            .advance(later, BodyEventDelta::zero(), &phenotype)
            .unwrap()
    );
}

#[test]
fn legacy_state_encoding_stays_identical_without_a_pending_pulse() {
    let (_, state) = fixture();
    let legacy_bytes = serde_json::to_vec(&state).unwrap();
    assert!(serde_json::to_value(state)
        .unwrap()
        .get("pending_prediction_residual")
        .is_none());
    let legacy: BiochemistryState = serde_json::from_slice(&legacy_bytes).unwrap();
    assert_eq!(serde_json::to_vec(&legacy).unwrap(), legacy_bytes);
    assert_eq!(legacy, state);
    let mut explicit_empty = serde_json::to_value(state).unwrap();
    explicit_empty["pending_prediction_residual"] = serde_json::Value::Null;
    let explicit_empty: BiochemistryState = serde_json::from_value(explicit_empty).unwrap();
    assert_eq!(serde_json::to_vec(&explicit_empty).unwrap(), legacy_bytes);
}

#[test]
fn prediction_pulse_rejects_overwrite_and_invalid_saved_provenance() {
    let (_, state) = fixture();
    let queued = pulse(&state);
    assert!(queued
        .with_prediction_residual_pulse(ExperienceSequenceId::new(8).unwrap(), 12, 0.9, 1.0)
        .is_err());
    assert!(state
        .with_prediction_residual_pulse(ExperienceSequenceId::new(7).unwrap(), 0, 0.8, 1.0)
        .is_err());
    for (field, value) in [
        ("schema_version", serde_json::json!(2)),
        ("source_tick", serde_json::json!(state.tick.raw() + 1)),
        ("source_sequence_id", serde_json::json!(0)),
        ("activity", serde_json::json!(1.1)),
    ] {
        let mut wire = serde_json::to_value(queued).unwrap();
        wire["pending_prediction_residual"][field] = value;
        let invalid: BiochemistryState = serde_json::from_value(wire).unwrap();
        assert!(invalid.validate_contract().is_err(), "accepted {field}");
    }
}
