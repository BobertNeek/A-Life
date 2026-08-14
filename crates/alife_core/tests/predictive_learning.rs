use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::{ActionId, ExperienceSequenceId, OrganismId, PredictionTargetReceipt, Tick};

fn transition(action: u32, sequence: u64, successor_features: Vec<f32>) -> PredictionTargetReceipt {
    PredictionTargetReceipt::for_successor(
        OrganismId(7),
        ExperienceSequenceId(sequence),
        ActionId(action),
        Tick::new(sequence),
        [17, 29, 41, 53],
        1,
        successor_features,
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
        .predict(
            move_left.source_digest,
            move_left.decision,
            move_left.successor_features.len(),
        )
        .unwrap();
    let right_prediction = predictor
        .predict(
            move_right.source_digest,
            move_right.decision,
            move_right.successor_features.len(),
        )
        .unwrap();
    let constant_loss = (0.4_f32 * 0.4 + 0.4 * 0.4) / 2.0;

    assert!(
        mean_squared_error(
            &left_prediction.predicted_successor,
            &move_left.successor_features
        ) < constant_loss * 0.25
    );
    assert!(
        mean_squared_error(
            &right_prediction.predicted_successor,
            &move_right.successor_features
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
