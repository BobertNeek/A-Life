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
        payload_len: 0,
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
