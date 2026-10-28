//! Continued critic fitting uses the same residual as the qualified repair.
//! Features are captured production GPU activations, including the actual next
//! decision across sleep gaps; no replay reset state substitutes for a bootstrap.
use alife_runtime::GpuAuthoritativeSession;
use alife_training::{
    CriticResidualRow, FrozenCriticResidualExperiment, PpoBoundary, PpoConfig, PpoTransition,
    PpoValueHeadCheckpoint,
};

pub(super) const OBJECTIVE_VERSION: u16 = 1;
const BATCH_ROWS: usize = 8 * 256;

fn residual_rows(
    transitions: &[PpoTransition],
    feature_states: usize,
    half_life: f32,
) -> super::Result<Vec<CriticResidualRow>> {
    if transitions.is_empty() || !half_life.is_finite() || half_life <= 0.0 {
        return Err("invalid residual transitions or discount half-life".into());
    }
    transitions
        .iter()
        .enumerate()
        .map(|(index, t)| {
            let terminal = t.boundary == PpoBoundary::Terminated;
            let next = if terminal { index } else { index + 1 };
            if index >= feature_states
                || next >= feature_states
                || !t.reward.is_finite()
                || !t.elapsed_seconds.is_finite()
                || t.elapsed_seconds <= 0.0
            {
                return Err(
                    "residual update requires a finite real current/next feature pair".into(),
                );
            }
            Ok(CriticResidualRow {
                current: index as u32,
                next: next as u32,
                reward: t.reward,
                discount: if terminal {
                    0.0
                } else {
                    (-std::f64::consts::LN_2 * f64::from(t.elapsed_seconds) / f64::from(half_life))
                        .exp() as f32
                },
            })
        })
        .collect()
}

fn metrics(values: &[[f32; 4]]) -> serde_json::Value {
    let slice = |rows: &[[f32; 4]]| {
        serde_json::json!({"rows":rows.len(),
        "residual_mse":rows.iter().map(|r|f64::from(r[2]).powi(2)).sum::<f64>()/rows.len() as f64})
    };
    let n = values.len();
    let k = n.min(16);
    serde_json::json!({"initial16":slice(&values[..k]),"whole":slice(values),"last16":slice(&values[n-k..])})
}

fn residual_batch(
    rows: &[CriticResidualRow],
    update: u32,
) -> (std::ops::Range<usize>, usize, Vec<CriticResidualRow>) {
    let batches = rows.len().div_ceil(BATCH_ROWS);
    let start = update as usize % batches * BATCH_ROWS;
    let end = (start + BATCH_ROWS).min(rows.len());
    let feature_end = rows[start..end]
        .iter()
        .map(|r| r.next as usize + 1)
        .max()
        .unwrap()
        .max(end);
    let local = rows[start..end]
        .iter()
        .map(|r| CriticResidualRow {
            current: r.current - start as u32,
            next: r.next - start as u32,
            reward: r.reward,
            discount: r.discount,
        })
        .collect();
    (start..end, feature_end, local)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn train(
    session: &GpuAuthoritativeSession,
    mut checkpoint: PpoValueHeadCheckpoint,
    features: &[Vec<f32>],
    transitions: &[PpoTransition],
    config: PpoConfig,
    updates: u32,
    policy: u64,
) -> super::Result<(PpoValueHeadCheckpoint, serde_json::Value)> {
    config.validate()?;
    if updates == 0 || config.value_coefficient <= 0.0 || features.len() > 36_001 {
        return Err(
            "continued residual critic needs accepted actor updates and bounded real features"
                .into(),
        );
    }
    let rows = residual_rows(
        transitions,
        features.len(),
        config.discount_half_life_seconds,
    )?;
    let initial_age = checkpoint.optimizer_step;
    let feature_sha256 = {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        for row in features {
            #[cfg(target_endian = "little")]
            hash.update(bytemuck::cast_slice(row));
            #[cfg(not(target_endian = "little"))]
            for value in row {
                hash.update(value.to_le_bytes());
            }
        }
        format!("{:x}", hash.finalize())
    };
    // One private GPU head owns the entire fit. A failed update/evaluation drops
    // it without publishing a partially fitted checkpoint to the PPO caller.
    let mut residual: Option<FrozenCriticResidualExperiment> = None;
    let mut last_batch_range: Option<(usize, usize, usize)> = None;
    let mut last_after_evaluation: Option<Vec<[f32; 4]>> = None;
    let mut records = Vec::new();
    for update in 0..updates {
        let (range, feature_end, local) = residual_batch(&rows, update);
        let (start, end) = (range.start, range.end);
        let batch_range = (start, end, feature_end);
        let same_batch = last_batch_range == Some(batch_range);
        let residual = match &mut residual {
            Some(residual) => {
                if !same_batch {
                    residual.replace_batch(
                        session,
                        &features[start..feature_end],
                        &local,
                        local.len(),
                    )?;
                }
                residual
            }
            slot @ None => slot.insert(FrozenCriticResidualExperiment::new(
                session,
                &checkpoint,
                &features[start..feature_end],
                &local,
                local.len(),
            )?),
        };
        let before = match (same_batch, last_after_evaluation.take()) {
            (true, Some(previous_after)) => previous_after,
            _ => residual.evaluate(session)?,
        };
        residual.step_with_coefficient(
            session,
            config.value_optimizer,
            config.value_coefficient,
        )?;
        let after = residual.evaluate(session)?;
        let before_metrics = metrics(&before);
        let after_metrics = metrics(&after);
        last_batch_range = Some(batch_range);
        last_after_evaluation = Some(after);
        records.push(
            serde_json::json!({"update":update+1,"loss_row_start":start,"loss_row_end":end,
            "before":before_metrics,"after":after_metrics,"finite_preflight_passed":true}),
        );
    }
    // Retain the logical PPO-window checkpoint boundary and existing restart
    // codec. Only per-residual-update head transfers/reconstruction are removed.
    checkpoint = residual
        .ok_or("missing residual fit")?
        .checkpoint(session)?;
    checkpoint.last_updated_policy_version = Some(policy);
    let receipt = serde_json::json!({"objective_version":OBJECTIVE_VERSION,
        "objective":"full_sampled_one_step_bellman_residual","feature_authority":"captured_production_GPU_after_inference",
        "features_frozen_before_actor_updates":true,"feature_states":features.len(),"loss_rows":rows.len(),
        "captured_GPU_feature_sha256":feature_sha256,
        "initial_critic_age":initial_age,"final_critic_age":checkpoint.optimizer_step,"optimizer_updates":updates,
        "value_coefficient":config.value_coefficient,"optimizer":config.value_optimizer,
        "actor_GAE_unchanged":true,"bootstrap_is_loss_row":false,"Adam_reset":false,"updates":records});
    Ok((checkpoint, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn transition(boundary: PpoBoundary, elapsed_seconds: f32) -> PpoTransition {
        PpoTransition {
            policy_version: 7,
            trajectory_id: 1,
            step: 0,
            action: alife_training::PpoJointAction {
                candidate_count: 1,
                representative_mask: 1,
                motor_masks: [0; 6],
                representative: 0,
                forced_slots: vec![None],
                motor_candidates: [None; 6],
                old_joint_log_probability: 0.0,
                temperature: 1.0,
            },
            reward: 0.25,
            old_value: 2.0,
            next_value: if boundary == PpoBoundary::Terminated {
                0.0
            } else {
                3.0
            },
            elapsed_seconds,
            boundary,
        }
    }
    #[test]
    fn residual_boundaries_keep_real_next_and_elapsed_discount() {
        for boundary in [PpoBoundary::Continuing, PpoBoundary::Truncated] {
            let t = transition(boundary, 60.0);
            let rows = residual_rows(std::slice::from_ref(&t), 2, 60.0).unwrap();
            assert_eq!(
                (rows[0].current, rows[0].next, rows[0].discount),
                (0, 1, 0.5)
            );
            assert!(residual_rows(&[t], 1, 60.0).is_err());
        }
        let rows = residual_rows(&[transition(PpoBoundary::Terminated, 60.0)], 1, 60.0).unwrap();
        assert_eq!((rows[0].next, rows[0].discount), (0, 0.0));
    }
    #[test]
    fn residual_rotation_keeps_partial_batch_and_local_real_successor() {
        let mut transitions = vec![transition(PpoBoundary::Continuing, 60.0); BATCH_ROWS + 5];
        transitions.last_mut().unwrap().boundary = PpoBoundary::Truncated;
        let rows = residual_rows(&transitions, transitions.len() + 1, 60.0).unwrap();
        for update in 0..6 {
            let (range, feature_end, local) = residual_batch(&rows, update);
            let partial = update % 2 == 1;
            assert_eq!(range.start, if partial { BATCH_ROWS } else { 0 });
            assert_eq!(local.len(), if partial { 5 } else { BATCH_ROWS });
            assert_eq!(feature_end, range.end + 1);
            for (i, row) in local.iter().enumerate() {
                assert_eq!(
                    (row.current, row.next, row.discount),
                    (i as u32, i as u32 + 1, 0.5)
                );
            }
        }
        transitions.last_mut().unwrap().boundary = PpoBoundary::Terminated;
        let rows = residual_rows(&transitions, transitions.len(), 60.0).unwrap();
        let (range, feature_end, local) = residual_batch(&rows, 1);
        assert_eq!(feature_end, range.end);
        let last = local.last().unwrap();
        assert_eq!((last.current, last.next, last.discount), (4, 4, 0.0));
        // A fresh PPO window starts its own rotation, regardless of prior dose.
        assert_eq!(residual_batch(&rows[..3], 0).0, 0..3);
    }

    #[test]
    fn residual_single_batch_keeps_the_same_rows_and_real_successor() {
        for count in [1, 7, BATCH_ROWS] {
            let mut transitions = vec![transition(PpoBoundary::Continuing, 0.05); count];
            transitions.last_mut().unwrap().boundary = PpoBoundary::Truncated;
            let rows = residual_rows(&transitions, count + 1, 60.0).unwrap();
            let (_, _, first) = residual_batch(&rows, 0);
            for update in 1..5 {
                let (range, feature_end, local) = residual_batch(&rows, update);
                assert_eq!(range, 0..count);
                assert_eq!(feature_end, count + 1);
                assert_eq!(local.len(), first.len());
                for (actual, expected) in local.iter().zip(&first) {
                    assert_eq!(
                        (actual.current, actual.next),
                        (expected.current, expected.next)
                    );
                    assert_eq!(actual.reward.to_bits(), expected.reward.to_bits());
                    assert_eq!(actual.discount.to_bits(), expected.discount.to_bits());
                }
            }
        }
    }
    #[test]
    fn residual_rejects_nonfinite_time_reward_and_missing_current() {
        for time in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(residual_rows(&[transition(PpoBoundary::Truncated, time)], 2, 60.0).is_err());
        }
        let mut t = transition(PpoBoundary::Terminated, 0.05);
        t.reward = f32::NAN;
        assert!(residual_rows(&[t], 1, 60.0).is_err());
        assert!(residual_rows(&[transition(PpoBoundary::Terminated, 0.05)], 0, 60.0).is_err());
    }
    #[test]
    fn residual_scalar_gradient_includes_next_and_bias() {
        for gamma in [0.99942255_f64, 0.5, 0.0] {
            let loss = |w: f64, b: f64| 0.5 * (w + b - 0.25 - gamma * (4.0 * w + b)).powi(2);
            let e = 2.0 + 3.0 - 0.25 - gamma * (8.0 + 3.0);
            let eps = 1e-5;
            let dw = (loss(2.0 + eps, 3.0) - loss(2.0 - eps, 3.0)) / (2.0 * eps);
            let db = (loss(2.0, 3.0 + eps) - loss(2.0, 3.0 - eps)) / (2.0 * eps);
            assert!((dw - e * (1.0 - 4.0 * gamma)).abs() < 1e-6);
            assert!((db - e * (1.0 - gamma)).abs() < 1e-6);
        }
    }
    #[test]
    fn residual_slices_expose_initial_transient_instead_of_averaging_it_away() {
        let mut values = vec![[0.0; 4]; 128];
        for v in &mut values[..16] {
            v[2] = 10.0;
        }
        let m = metrics(&values);
        assert_eq!(m["initial16"]["residual_mse"], 100.0);
        assert_eq!(m["whole"]["residual_mse"], 12.5);
        assert_eq!(m["last16"]["residual_mse"], 0.0);
    }

    #[cfg(feature = "gpu-tests")]
    #[test]
    #[ignore = "requires an admitted hardware GPU; synthetic residual windows only"]
    fn persistent_fit_matches_per_update_restore_across_changed_ppo_windows() {
        use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
        use alife_runtime::GpuSessionConsumerKind;
        let session = GpuAuthoritativeSession::new(
            GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap(),
            GpuSessionConsumerKind::Training,
        );
        let mut checkpoint = PpoValueHeadCheckpoint {
            feature_count: 2,
            optimizer_step: 17,
            last_updated_policy_version: Some(9),
            parameters: vec![0.2, -0.3, 0.4, 0.01, -0.02, 0.03, 0.04, 0.05, 0.06],
        };
        let config = PpoConfig {
            value_coefficient: 0.37,
            value_optimizer: alife_training::AdamWConfig {
                learning_rate: 0.007,
                gradient_clip: 0.04,
                weight_decay: 0.02,
                ..Default::default()
            },
            ..Default::default()
        };
        let bits = |v: &[f32]| v.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        // Consecutive small/exact-size batches use the data/evaluation reuse
        // path. Larger windows rotate, and a later small policy must reupload.
        for (policy, count) in [(10, 7), (11, BATCH_ROWS), (12, BATCH_ROWS + 5), (13, 11)] {
            let features = (0..=count)
                .map(|i| vec![i as f32 * 0.001 + policy as f32, (i % 7) as f32 * 0.2])
                .collect::<Vec<_>>();
            let mut transitions = vec![transition(PpoBoundary::Continuing, 0.05); count];
            transitions.last_mut().unwrap().boundary = PpoBoundary::Truncated;
            let (actual, receipt) = train(
                &session,
                checkpoint.clone(),
                &features,
                &transitions,
                config,
                5,
                policy,
            )
            .unwrap();
            // Reference is the former checkpoint/reconstruct-per-update path.
            let rows = residual_rows(
                &transitions,
                features.len(),
                config.discount_half_life_seconds,
            )
            .unwrap();
            let initial_age = checkpoint.optimizer_step;
            let mut expected_records = Vec::new();
            for update in 0..5 {
                let (range, feature_end, local) = residual_batch(&rows, update);
                let mut residual = FrozenCriticResidualExperiment::new(
                    &session,
                    &checkpoint,
                    &features[range.start..feature_end],
                    &local,
                    local.len(),
                )
                .unwrap();
                let before = residual.evaluate(&session).unwrap();
                residual
                    .step_with_coefficient(
                        &session,
                        config.value_optimizer,
                        config.value_coefficient,
                    )
                    .unwrap();
                let after = residual.evaluate(&session).unwrap();
                checkpoint = residual.checkpoint(&session).unwrap();
                expected_records.push(serde_json::json!({"update":update+1,"loss_row_start":range.start,"loss_row_end":range.end,
                    "before":metrics(&before),"after":metrics(&after),"finite_preflight_passed":true}));
            }
            checkpoint.last_updated_policy_version = Some(policy);
            assert_eq!(actual, checkpoint);
            assert_eq!(bits(&actual.parameters), bits(&checkpoint.parameters));
            assert_eq!(receipt["updates"], serde_json::json!(expected_records));
            if count <= BATCH_ROWS {
                assert_eq!(
                    receipt["updates"][1]["before"],
                    receipt["updates"][0]["after"]
                );
            }
            assert_eq!(actual.optimizer_step, initial_age + 5);
            assert_eq!(receipt["initial_critic_age"], initial_age);
            assert_eq!(receipt["final_critic_age"], actual.optimizer_step);
            // Exercise the existing exact PPO boundary/restart representation
            // with nonzero first/second moments and a nonzero integer age.
            let wire = serde_json::to_vec(&actual).unwrap();
            let restored = serde_json::from_slice(&wire).unwrap();
            checkpoint = alife_training::PpoTrainingState::from_checkpoint(restored)
                .unwrap()
                .checkpoint(&session)
                .unwrap();
            assert_eq!(checkpoint, actual);
            assert!(checkpoint.parameters[3..6].iter().any(|v| *v != 0.0));
            assert!(checkpoint.parameters[6..].iter().any(|v| *v > 0.0));
        }
        // Fail after the first accepted residual step: no partial fit is
        // published, and the caller retains its exact pre-window checkpoint.
        let before = checkpoint.clone();
        let count = BATCH_ROWS + 1;
        let features = vec![vec![10.0; 2]; count + 1];
        let mut transitions = vec![transition(PpoBoundary::Continuing, 60.0); count];
        transitions.last_mut().unwrap().reward = f32::MAX;
        assert!(train(
            &session,
            checkpoint.clone(),
            &features,
            &transitions,
            config,
            2,
            14
        )
        .is_err());
        assert_eq!(checkpoint, before);

        // An accepted finite weight update can still overflow the subsequent
        // predictions. The after-evaluation error must also prevent publication.
        let explosive = PpoValueHeadCheckpoint {
            parameters: vec![0.0; 9],
            ..checkpoint
        };
        let features = vec![vec![1000.0; 2]; 2];
        let transitions = vec![transition(PpoBoundary::Truncated, 60.0)];
        let config = PpoConfig {
            value_optimizer: alife_training::AdamWConfig {
                learning_rate: f32::MAX / 10.0,
                ..config.value_optimizer
            },
            ..config
        };
        let rows = residual_rows(&transitions, 2, 60.0).unwrap();
        let mut residual =
            FrozenCriticResidualExperiment::new(&session, &explosive, &features, &rows, 1).unwrap();
        assert!(residual.evaluate(&session).is_ok());
        assert!(residual
            .step_with_coefficient(&session, config.value_optimizer, config.value_coefficient)
            .is_ok());
        assert!(residual.evaluate(&session).is_err());
        assert!(train(&session, explosive, &features, &transitions, config, 1, 14).is_err());
    }
}
