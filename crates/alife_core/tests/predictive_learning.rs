use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::{
    ActionId, ExperienceSequenceId, JointMotorCondition, MotorChannel, MotorChannelFactor,
    OrganismId, PredictionTargetReceipt, SemanticStateVector, Tick, Vec3f,
};

fn motor_condition(action: u32) -> JointMotorCondition {
    JointMotorCondition::new(vec![MotorChannelFactor {
        channel: MotorChannel::Locomotion,
        primitive: ActionId(action),
        intensity: 0.8,
        duration_ticks: 2,
        direction: Vec3f::new(1.0, 0.0, 0.0),
        stand_off_distance: 0.0,
        confidence: 0.9,
        target: None,
        payload: Vec::new(),
        coordination_group: 0,
    }])
    .unwrap()
}

fn transition(action: u32, sequence: u64, target_values: Vec<f32>) -> PredictionTargetReceipt {
    PredictionTargetReceipt::for_successor(
        OrganismId(7),
        ExperienceSequenceId(sequence),
        ActionId(action),
        Tick::new(sequence),
        [17, 29, 41, 53],
        SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
        motor_condition(action),
        SemanticStateVector::new(target_values).unwrap(),
    )
    .unwrap()
}

fn mean_squared_error(prediction: &[f32], target: &[f32]) -> f32 {
    prediction
        .iter()
        .zip(target)
        .map(|(prediction, target)| {
            let error = prediction - target;
            error * error
        })
        .sum::<f32>()
        / prediction.len() as f32
}

fn r06_command(channel: MotorChannel, entity: u64, payload: u32) -> alife_core::ChannelCommand {
    alife_core::ChannelCommand {
        channel,
        primitive: ActionId(7),
        target: Some(alife_core::ActionTarget::new(
            Some(alife_core::WorldEntityId(entity)),
            None,
        )),
        direction: Vec3f::ZERO,
        intensity: alife_core::Intensity::new(0.8).unwrap(),
        duration_ticks: alife_core::DurationTicks::new(2),
        stand_off_distance: 0.0,
        payload: alife_core::BoundedMotorPayload::new(vec![payload]).unwrap(),
        confidence: alife_core::Confidence::new(0.9).unwrap(),
        coordination_group: 0,
    }
}

fn r06_condition(commands: &[alife_core::ChannelCommand]) -> JointMotorCondition {
    JointMotorCondition::new(
        commands
            .iter()
            .map(|command| MotorChannelFactor::from_command(command).unwrap())
            .collect(),
    )
    .unwrap()
}

fn r06_transition(condition: JointMotorCondition, target: Vec<f32>) -> PredictionTargetReceipt {
    PredictionTargetReceipt::for_successor(
        OrganismId(7),
        ExperienceSequenceId(1),
        ActionId(7),
        Tick::new(1),
        [17, 29, 41, 53],
        SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
        condition,
        SemanticStateVector::new(target).unwrap(),
    )
    .unwrap()
}

fn r06_assert_identity_change(changed: alife_core::ChannelCommand) {
    let original = r06_command(MotorChannel::Vocal, 100, 7);
    assert_ne!(
        r06_condition(&[original.clone()])
            .canonical_digest()
            .unwrap(),
        r06_condition(&[changed]).canonical_digest().unwrap()
    );
}

#[test]
fn r06_target_entity_binds_condition_identity() {
    r06_assert_identity_change(r06_command(MotorChannel::Vocal, 200, 7));
}

#[test]
fn r06_equal_length_payload_content_binds_condition_identity() {
    r06_assert_identity_change(r06_command(MotorChannel::Vocal, 100, 8));
}

fn r06_assert_distinct_learning(changed: alife_core::ChannelCommand) {
    let left = r06_transition(
        r06_condition(&[r06_command(MotorChannel::Vocal, 100, 7)]),
        vec![0.1, 0.9],
    );
    let right = r06_transition(r06_condition(&[changed]), vec![0.9, 0.1]);
    let mut predictor = GroundedSuccessorPredictor::default();
    for _ in 0..96 {
        predictor.observe(&left).unwrap();
        predictor.observe(&right).unwrap();
    }
    let left_prediction = predictor
        .predict(left.source_state(), left.motor_condition())
        .unwrap();
    let right_prediction = predictor
        .predict(right.source_state(), right.motor_condition())
        .unwrap();
    assert!(
        right_prediction.predicted_successor[0] - left_prediction.predicted_successor[0] > 0.4,
        "distinct exact identities must support distinct learned expectations"
    );
}

#[test]
fn r06_target_entities_learn_distinct_successors() {
    r06_assert_distinct_learning(r06_command(MotorChannel::Vocal, 200, 7));
}

#[test]
fn r06_equal_length_payloads_learn_distinct_successors() {
    r06_assert_distinct_learning(r06_command(MotorChannel::Vocal, 100, 8));
}

fn r06_assert_stable_channel(existing: MotorChannel, extra: MotorChannel) {
    let known = r06_command(existing, 100, 7);
    let target = r06_transition(r06_condition(&[known.clone()]), vec![0.9, 0.1]);
    let mut predictor = GroundedSuccessorPredictor::default();
    for _ in 0..32 {
        predictor.observe(&target).unwrap();
    }
    let before = predictor
        .predict(target.source_state(), target.motor_condition())
        .unwrap();
    let mut extra_command = r06_command(extra, 200, 8);
    extra_command.primitive = ActionId(9);
    extra_command.intensity = alife_core::Intensity::new(0.1).unwrap();
    let with_extra = r06_condition(&[known, extra_command]);
    let after = predictor
        .predict(target.source_state(), &with_extra)
        .unwrap();
    for (left, right) in before
        .predicted_successor
        .iter()
        .zip(&after.predicted_successor)
    {
        assert!(
            (left - right).abs() < 1.0e-6,
            "unlearned channel must not readdress learned weights"
        );
    }
}

#[test]
fn r06_unlearned_locomotion_does_not_move_vocal_coefficients() {
    r06_assert_stable_channel(MotorChannel::Vocal, MotorChannel::Locomotion);
}

#[test]
fn r06_unlearned_species_channel_does_not_move_another_species_coefficients() {
    r06_assert_stable_channel(
        MotorChannel::SpeciesSpecific(255),
        MotorChannel::SpeciesSpecific(0),
    );
}

fn r06_assert_relabeling_geometry(primitive: bool) {
    let mut predictions = Vec::new();
    for labels in [[7, 8], [u32::MAX, 1]] {
        let mut commands = [
            r06_command(MotorChannel::Vocal, 100, 7),
            r06_command(MotorChannel::Vocal, 100, 7),
        ];
        for (command, label) in commands.iter_mut().zip(labels) {
            if primitive {
                command.primitive = ActionId(label);
            } else {
                command.payload.values[0] = label;
            }
        }
        let left = r06_transition(r06_condition(&commands[..1]), vec![0.1, 0.9]);
        let right = r06_transition(r06_condition(&commands[1..]), vec![0.9, 0.1]);
        let mut predictor = GroundedSuccessorPredictor::default();
        for _ in 0..96 {
            predictor.observe(&left).unwrap();
            predictor.observe(&right).unwrap();
        }
        predictions.push([
            predictor
                .predict(left.source_state(), left.motor_condition())
                .unwrap()
                .predicted_successor,
            predictor
                .predict(right.source_state(), right.motor_condition())
                .unwrap()
                .predicted_successor,
        ]);
    }
    for (left, right) in predictions[0]
        .iter()
        .flatten()
        .zip(predictions[1].iter().flatten())
    {
        assert!((left - right).abs() < 1.0e-6);
    }
}

#[test]
fn r06_categorical_relabeling_does_not_change_learning_geometry() {
    r06_assert_relabeling_geometry(false);
}

#[test]
fn r06_primitive_relabeling_does_not_change_learning_geometry() {
    r06_assert_relabeling_geometry(true);
}

#[test]
fn r06_channel_decoding_is_bounded_before_condition_validation() {
    let condition = r06_condition(&[r06_command(MotorChannel::Vocal, 100, 7)]);
    let mut encoded = serde_json::to_value(condition).unwrap();
    encoded["channels"] = serde_json::json!(vec![encoded["channels"][0].clone(); 7]);
    let error = serde_json::from_value::<JointMotorCondition>(encoded).unwrap_err();
    assert!(error.to_string().contains("entry limit"));
}

#[test]
fn r06_coordination_group_255_is_an_exact_supported_category() {
    let mut command = r06_command(MotorChannel::Vocal, 100, 7);
    command.coordination_group = 255;
    r06_assert_identity_change(command.clone());
    r06_assert_distinct_learning(command.clone());
    let target = r06_transition(r06_condition(&[command]), vec![0.1, 0.9]);
    let mut predictor = GroundedSuccessorPredictor::default();
    predictor.observe(&target).unwrap();
    let encoded = serde_json::to_value(&predictor).unwrap();
    assert!(encoded["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |row| row["key"]["Category"]["category"]["CoordinationGroup"] == serde_json::json!(255)
        ));
    assert_eq!(
        serde_json::from_value::<GroundedSuccessorPredictor>(encoded).unwrap(),
        predictor
    );
}

#[test]
fn r06_full_category_storage_preserves_keys_and_reports_unmodelled_inputs() {
    let known = r06_transition(
        r06_condition(&[r06_command(MotorChannel::Vocal, 100, 7)]),
        vec![0.1, 0.9],
    );
    let mut predictor = GroundedSuccessorPredictor::default();
    predictor.observe(&known).unwrap();
    let mut encoded = serde_json::to_value(&predictor).unwrap();
    let remaining = alife_core::MAX_PREDICTOR_CATEGORIES - predictor.stored_category_count();
    // A valid, full saved coefficient table. These unused exact identities carry
    // zero coefficients; the test concerns bounded admission, not learned skill.
    let rows = encoded["rows"].as_array_mut().unwrap();
    for primitive in 1..=remaining {
        rows.push(serde_json::json!({
            "key": {"Category": {"channel": 511, "category": {"Primitive": primitive}}},
            "weights": [0.0, 0.0]
        }));
    }
    predictor = serde_json::from_value(encoded).unwrap();
    assert_eq!(
        predictor.stored_category_count(),
        alife_core::MAX_PREDICTOR_CATEGORIES
    );
    let novel = r06_transition(
        r06_condition(&[
            r06_command(MotorChannel::Vocal, 200, 7),
            r06_command(MotorChannel::Locomotion, 300, 8),
        ]),
        vec![0.9, 0.1],
    );
    let before_forecast = predictor
        .predict(novel.source_state(), novel.motor_condition())
        .unwrap();
    assert!(before_forecast.category_coverage.unmodelled_categories > 0);
    let before = serde_json::to_value(&predictor).unwrap();
    predictor.observe(&novel).unwrap();
    let after = serde_json::to_value(&predictor).unwrap();
    let keys = |value: &serde_json::Value| {
        value["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["key"].get("Category").is_some())
            .map(|row| row["key"].clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(&before),
        keys(&after),
        "no eviction, remapping or hidden admission"
    );
    assert_ne!(
        before["rows"][0]["weights"], after["rows"][0]["weights"],
        "known coefficients still learn"
    );
    assert!(
        after["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["key"]["Continuous"]["channel"] == serde_json::json!(0)),
        "new continuous channels remain learnable"
    );
    let forecast = predictor
        .predict(novel.source_state(), novel.motor_condition())
        .unwrap();
    assert_eq!(
        forecast.category_coverage.unmodelled_categories,
        before_forecast.category_coverage.unmodelled_categories
    );
    assert_eq!(forecast.category_coverage.stored_categorical_keys, 4096);
    let roundtrip: GroundedSuccessorPredictor = serde_json::from_value(after).unwrap();
    assert_eq!(roundtrip, predictor);
    assert_eq!(
        roundtrip
            .predict(novel.source_state(), novel.motor_condition())
            .unwrap(),
        forecast
    );

    let frozen = predictor.clone();
    let mut invalid = novel;
    invalid.target_state.values[0] = f32::NAN;
    assert!(predictor.observe(&invalid).is_err());
    assert_eq!(
        predictor, frozen,
        "failed update is atomic even at capacity"
    );
}

#[test]
fn r06_predictor_decoder_accepts_only_proven_empty_legacy_state() {
    let empty = serde_json::json!({"semantic_state_abi":0,"semantic_state_count":0,
        "motor_condition_abi":0,"input_feature_count":0,"learning_rate":0.25,"weights":[],"last_update":null});
    let migrated: GroundedSuccessorPredictor = serde_json::from_value(empty.clone()).unwrap();
    assert_eq!(migrated, GroundedSuccessorPredictor::default());
    for (field, value) in [
        ("weights", serde_json::json!([0.125])),
        ("semantic_state_count", serde_json::json!(2)),
        ("motor_condition_abi", serde_json::json!(1)),
        ("input_feature_count", serde_json::json!(285)),
    ] {
        let mut legacy = empty.clone();
        legacy[field] = value;
        assert!(serde_json::from_value::<GroundedSuccessorPredictor>(legacy).is_err());
    }
    let mut missing = empty;
    missing.as_object_mut().unwrap().remove("last_update");
    assert!(serde_json::from_value::<GroundedSuccessorPredictor>(missing).is_err());
}

#[test]
fn r06_predictor_decoder_rejects_invalid_sparse_rows() {
    let target = r06_transition(
        r06_condition(&[r06_command(MotorChannel::Vocal, 100, 7)]),
        vec![0.1, 0.9],
    );
    let mut predictor = GroundedSuccessorPredictor::default();
    predictor.observe(&target).unwrap();
    let original = serde_json::to_value(&predictor).unwrap();
    let mut duplicate = original.clone();
    let row = duplicate["rows"][0].clone();
    duplicate["rows"].as_array_mut().unwrap().push(row.clone());
    let mut oversized_weights = original.clone();
    oversized_weights["rows"][0]["weights"] = serde_json::json!(vec![0.0; 33]);
    let mut oversized_rows = original.clone();
    oversized_rows["rows"] = serde_json::json!(vec![row; 10_000]);
    let mut bad_channel = original.clone();
    let index = bad_channel["rows"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["key"].get("Continuous").is_some())
        .unwrap();
    bad_channel["rows"][index]["key"]["Continuous"]["channel"] = serde_json::json!(65535);
    let mut bad_component = original.clone();
    bad_component["rows"][index]["key"]["Continuous"]["component"] =
        serde_json::json!("Unrecognized");
    let mut bad_weight = original.clone();
    bad_weight["rows"][0]["weights"][0] = serde_json::Value::Null;
    let mut empty_rows = original.clone();
    empty_rows["rows"] = serde_json::json!([]);
    let mut no_bias = original.clone();
    no_bias["rows"].as_array_mut().unwrap().remove(0);
    let mut no_semantic = original.clone();
    no_semantic["rows"].as_array_mut().unwrap().remove(1);
    let mut no_update = original.clone();
    no_update["last_update"] = serde_json::Value::Null;
    let mut bad_prediction = original.clone();
    bad_prediction["last_update"]["prediction"]["predicted_successor"][0] = serde_json::json!(2.0);
    let mut bad_error = original.clone();
    bad_error["last_update"]["error"][0] = serde_json::json!(-2.0);
    let mut bad_mean = original.clone();
    bad_mean["last_update"]["mean_squared_error"] = serde_json::json!(2.0);
    for invalid in [
        duplicate,
        oversized_weights,
        oversized_rows,
        bad_channel,
        bad_component,
        bad_weight,
        empty_rows,
        no_bias,
        no_semantic,
        no_update,
        bad_prediction,
        bad_error,
        bad_mean,
    ] {
        assert!(serde_json::from_value::<GroundedSuccessorPredictor>(invalid).is_err());
    }
    assert_eq!(
        serde_json::from_value::<GroundedSuccessorPredictor>(original).unwrap(),
        predictor
    );
}

#[test]
fn grounded_action_conditioned_prediction_rejects_constant_successor_collapse() {
    let move_left = transition(101, 1, vec![0.1, 0.9]);
    let move_right = transition(202, 2, vec![0.9, 0.1]);
    let mut predictor = GroundedSuccessorPredictor::default();

    for _ in 0..12 {
        predictor.observe(&move_left).unwrap();
        predictor.observe(&move_right).unwrap();
    }

    let left_prediction = predictor
        .predict(move_left.source_state(), move_left.motor_condition())
        .unwrap();
    let right_prediction = predictor
        .predict(move_right.source_state(), move_right.motor_condition())
        .unwrap();
    let constant_loss = (0.4_f32 * 0.4 + 0.4 * 0.4) / 2.0;
    eprintln!(
        "twelve-exposure predictions: left={:?}, right={:?}, mse_left={}, mse_right={}",
        left_prediction.predicted_successor,
        right_prediction.predicted_successor,
        mean_squared_error(
            &left_prediction.predicted_successor,
            &move_left.target_state().values
        ),
        mean_squared_error(
            &right_prediction.predicted_successor,
            &move_right.target_state().values
        ),
    );

    assert!(
        mean_squared_error(
            &left_prediction.predicted_successor,
            &move_left.target_state().values
        ) < constant_loss * 0.25
    );
    assert!(
        mean_squared_error(
            &right_prediction.predicted_successor,
            &move_right.target_state().values
        ) < constant_loss * 0.25
    );
    assert!(left_prediction.predicted_successor[0] < 0.3);
    assert!(right_prediction.predicted_successor[0] > 0.7);
    assert!(
        (left_prediction.predicted_successor[0] - right_prediction.predicted_successor[0]).abs()
            > 0.4
    );

    let update = predictor.observe(&move_left).unwrap();
    assert_eq!(update.target_digest, move_left.target_digest());
    assert!(update.error.iter().any(|error| *error != 0.0));
    assert_eq!(predictor.last_update(), Some(&update));
}

#[test]
fn legacy_digest_predictor_state_is_rejected_instead_of_reset() {
    let legacy = serde_json::json!({
        "successor_feature_abi": 1,
        "successor_feature_count": 2,
        "learning_rate": 0.25,
        "action_heads": [],
        "last_update": null,
    });
    assert!(serde_json::from_value::<GroundedSuccessorPredictor>(legacy).is_err());
}
