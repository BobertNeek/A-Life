//! Synthetic numerical equivalence only; no world or learning-performance claim.
use super::*;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};

fn checkpoint() -> PpoValueHeadCheckpoint {
    PpoValueHeadCheckpoint {
        feature_count: 2,
        optimizer_step: 17,
        last_updated_policy_version: Some(9),
        parameters: vec![0.2, -0.3, 0.4, 0.01, -0.02, 0.03, 0.04, 0.05, 0.06],
    }
}

fn batch(n: usize, shift: f32) -> (Vec<Vec<f32>>, Vec<CriticResidualRow>) {
    let features = (0..=n)
        .map(|i| vec![shift + i as f32 * 0.01, (i % 5) as f32 * 0.2 - shift])
        .collect();
    let rows = (0..n)
        .map(|i| CriticResidualRow {
            current: i as u32,
            next: i as u32 + 1,
            reward: i as f32 * 0.03 - 0.5,
            discount: if i % 3 == 0 { 0.5 } else { 0.99942255 },
        })
        .collect();
    (features, rows)
}

fn bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn equal_checkpoint(a: &PpoValueHeadCheckpoint, b: &PpoValueHeadCheckpoint) {
    assert_eq!(a.feature_count, b.feature_count);
    assert_eq!(a.optimizer_step, b.optimizer_step);
    assert_eq!(a.last_updated_policy_version, b.last_updated_policy_version);
    assert_eq!(bits(&a.parameters), bits(&b.parameters));
}

#[test]
#[ignore = "requires an admitted hardware GPU; synthetic critic fixture only"]
fn persistent_batches_match_reconstruction_growth_and_exact_restart() {
    let session = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap(),
        GpuSessionConsumerKind::Training,
    );
    let (features, rows) = batch(3, 0.0);
    let mut expected = checkpoint();
    let mut persistent =
        FrozenCriticResidualExperiment::new(&session, &expected, &features, &rows, 3).unwrap();
    let head = persistent.head.clone();
    let pipelines = (
        persistent.predict.clone(),
        persistent.gradient.clone(),
        persistent.update.clone(),
    );
    let config = AdamWConfig {
        learning_rate: 0.007,
        weight_decay: 0.02,
        gradient_clip: 0.04,
        ..AdamWConfig::default()
    };
    // Same-size changed corpus, growth, partial fits, then return to larger data.
    for (index, (n, fit, shift)) in [(3, 3, 0.3), (67, 65, -0.4), (2, 1, 0.9), (67, 67, 0.1)]
        .into_iter()
        .enumerate()
    {
        let (features, rows) = batch(n, shift);
        let old_features = persistent.features.clone();
        let old_rows = persistent.rows.clone();
        let old_output = persistent.output.clone();
        persistent
            .replace_batch(&session, &features, &rows, fit)
            .unwrap();
        assert_eq!(persistent.head, head);
        assert_eq!(
            (
                &persistent.predict,
                &persistent.gradient,
                &persistent.update
            ),
            (&pipelines.0, &pipelines.1, &pipelines.2)
        );
        if index != 1 {
            assert_eq!(persistent.features, old_features);
            assert_eq!(persistent.rows, old_rows);
            assert_eq!(persistent.output, old_output);
        }
        let mut restored =
            FrozenCriticResidualExperiment::new(&session, &expected, &features, &rows, fit)
                .unwrap();
        assert_eq!(
            persistent.evaluate(&session).unwrap(),
            restored.evaluate(&session).unwrap()
        );
        assert_eq!(
            bits(&persistent.gradient_snapshot(&session).unwrap()),
            bits(&restored.gradient_snapshot(&session).unwrap())
        );
        for residual in [&mut persistent, &mut restored] {
            residual
                .step_with_coefficient(&session, config, 0.37)
                .unwrap();
        }
        assert_eq!(
            persistent.evaluate(&session).unwrap(),
            restored.evaluate(&session).unwrap()
        );
        expected = restored.checkpoint(&session).unwrap();
        assert_eq!(expected.optimizer_step, 18 + index as u32);
        equal_checkpoint(&persistent.checkpoint(&session).unwrap(), &expected);
    }
    assert!(expected.parameters[3..6].iter().any(|v| *v != 0.0));
    assert!(expected.parameters[6..].iter().any(|v| *v > 0.0));
    let encoded = serde_json::to_vec(&expected).unwrap();
    let restarted: PpoValueHeadCheckpoint = serde_json::from_slice(&encoded).unwrap();
    equal_checkpoint(&restarted, &expected);
    let (features, rows) = batch(5, -0.8);
    let mut restored =
        FrozenCriticResidualExperiment::new(&session, &restarted, &features, &rows, 5).unwrap();
    persistent
        .replace_batch(&session, &features, &rows, 5)
        .unwrap();
    for residual in [&mut persistent, &mut restored] {
        residual
            .step_with_coefficient(&session, config, 0.37)
            .unwrap();
    }
    equal_checkpoint(
        &persistent.checkpoint(&session).unwrap(),
        &restored.checkpoint(&session).unwrap(),
    );

    let before = persistent.checkpoint(&session).unwrap();
    assert!(persistent
        .replace_batch(&session, &[vec![f32::NAN; 2]], &rows, 5)
        .is_err());
    equal_checkpoint(&persistent.checkpoint(&session).unwrap(), &before);
    // Finite inputs whose prediction/gradient overflows must fail GPU preflight
    // without advancing weights, nonzero moments, or integer optimizer age.
    let mut overflow = rows[..1].to_vec();
    overflow[0].reward = f32::MAX;
    persistent
        .replace_batch(&session, &[vec![10.0; 2], vec![10.0; 2]], &overflow, 1)
        .unwrap();
    assert!(persistent
        .step_with_coefficient(&session, config, 1.0)
        .is_err());
    equal_checkpoint(&persistent.checkpoint(&session).unwrap(), &before);
}
