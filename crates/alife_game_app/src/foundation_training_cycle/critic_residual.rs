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
            for value in row {
                hash.update(value.to_le_bytes());
            }
        }
        format!("{:x}", hash.finalize())
    };
    let batches = rows.len().div_ceil(BATCH_ROWS);
    let mut records = Vec::new();
    for update in 0..updates {
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
            .collect::<Vec<_>>();
        let mut residual = FrozenCriticResidualExperiment::new(
            session,
            &checkpoint,
            &features[start..feature_end],
            &local,
            local.len(),
        )?;
        let before = residual.evaluate(session)?;
        residual.step_with_coefficient(
            session,
            config.value_optimizer,
            config.value_coefficient,
        )?;
        let after = residual.evaluate(session)?;
        checkpoint = residual.checkpoint(session)?;
        records.push(
            serde_json::json!({"update":update+1,"loss_row_start":start,"loss_row_end":end,
            "before":metrics(&before),"after":metrics(&after),"finite_preflight_passed":true}),
        );
    }
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
}
