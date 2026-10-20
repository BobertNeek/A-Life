//! AOA-OBS-004: bounded, opt-in copies of already prepared offline PPO targets.
use std::path::Path;

use alife_training::{PpoBatch, PpoBoundary, PpoConfig};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::FoundationReplayRecordRef;

pub(super) const MAX_ROWS: usize = 64;

pub(super) fn write_if_enabled(
    enabled: bool,
    output: &Path,
    batch: &PpoBatch,
    config: PpoConfig,
    references: &[FoundationReplayRecordRef],
    context: impl FnOnce() -> Value,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if !enabled {
        return Ok(None);
    }
    if references.len() < batch.len() {
        return Err("PPO diagnostic replay rows are incomplete".into());
    }
    let rows = batch
        .transitions()
        .iter()
        .enumerate()
        .take(MAX_ROWS)
        .map(|(row, transition)| {
            let elapsed = f64::from(transition.elapsed_seconds);
            let mut value = json!({
                "row": row,
                "trajectory_id": transition.trajectory_id,
                "step": transition.step,
                "decision_tick": references[row].tick,
                "reward": transition.reward,
                "old_value": transition.old_value,
                "next_value": transition.next_value,
                "elapsed_seconds": transition.elapsed_seconds,
                "gamma": (-std::f64::consts::LN_2 * elapsed
                    / f64::from(config.discount_half_life_seconds)).exp(),
                "lambda": (-std::f64::consts::LN_2 * elapsed
                    / f64::from(config.gae_half_life_seconds)).exp(),
                "boundary": match transition.boundary {
                    PpoBoundary::Continuing => "continuing",
                    PpoBoundary::Terminated => "terminated",
                    PpoBoundary::Truncated => "truncated",
                },
                "advantage": batch.advantages()[row],
                "return": batch.returns()[row],
            });
            if batch.actor_advantage_centering_mean().is_some() {
                value["actor_advantage"] = json!(batch.actor_advantage(row));
            }
            value
        })
        .collect::<Vec<_>>();
    let mut diagnostic = context();
    if !diagnostic.is_object() {
        return Err("PPO diagnostic context must be an object".into());
    }
    diagnostic["schema_version"] = json!(1);
    diagnostic["phase"] = json!("prepared_before_optimizer_update");
    diagnostic["training_commit_requires_cycle_receipt"] = json!(true);
    diagnostic["policy_version"] = json!(batch.policy_version());
    diagnostic["sampling_temperature"] = json!(config.temperature);
    diagnostic["discount_half_life_seconds"] = json!(config.discount_half_life_seconds);
    diagnostic["gae_half_life_seconds"] = json!(config.gae_half_life_seconds);
    diagnostic["total_rows"] = json!(batch.len());
    diagnostic["emitted_rows"] = json!(rows.len());
    diagnostic["rows_truncated"] = json!(rows.len() != batch.len());
    diagnostic["rows"] = json!(rows);
    if let Some(mean) = batch.actor_advantage_centering_mean() {
        diagnostic["actor_advantage_centering_mean"] = json!(mean);
    }
    // One bounded write per rollout, after target preparation and before updates.
    // A prepared sidecar alone is never proof of committed optimizer updates.
    let bytes = serde_json::to_vec_pretty(&diagnostic)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    std::fs::write(output.join("ppo-rollout-diagnostics.json"), bytes)?;
    Ok(Some(digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_training::{PpoJointAction, PpoTransition};
    use std::{fs, path::PathBuf, time::SystemTime};

    struct Output(PathBuf);
    impl Output {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("alife-ppo-diag-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn read(&self) -> Value {
            serde_json::from_slice(&fs::read(self.0.join("ppo-rollout-diagnostics.json")).unwrap())
                .unwrap()
        }
    }
    impl Drop for Output {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0.join("ppo-rollout-diagnostics.json"));
            let _ = fs::remove_dir(&self.0);
        }
    }

    fn action() -> PpoJointAction {
        PpoJointAction {
            candidate_count: 1,
            representative_mask: 1,
            motor_masks: [0; 6],
            representative: 0,
            forced_slots: vec![Some(4)],
            motor_candidates: [None, None, None, None, Some(0), None],
            old_joint_log_probability: 0.0,
            temperature: 1.0,
        }
    }

    fn fixture(count: usize) -> (PpoBatch, Vec<FoundationReplayRecordRef>) {
        let transitions = (0..count)
            .map(|row| PpoTransition {
                policy_version: 7,
                trajectory_id: 17,
                step: row as u64,
                action: action(),
                reward: 0.25,
                old_value: 0.0,
                next_value: 0.0,
                elapsed_seconds: 0.05,
                boundary: if row + 1 == count {
                    PpoBoundary::Truncated
                } else {
                    PpoBoundary::Continuing
                },
            })
            .collect();
        let references = (0..count)
            .map(|row| FoundationReplayRecordRef {
                cohort: None,
                record_index: row as u64,
                segment: 0,
                tick: row as u64,
                encoded_bytes: 0,
                digest: [0; 32],
            })
            .collect();
        (
            PpoBatch::from_rollout(7, transitions, PpoConfig::default()).unwrap(),
            references,
        )
    }

    #[test]
    fn actor_centering_is_reported_separately_without_rewriting_raw_diagnostics() {
        let raw_output = Output::new();
        let centered_output = Output::new();
        let (raw, references) = fixture(4);
        let mut centered = raw.clone();
        let mean = centered.center_actor_advantages().unwrap();
        write_if_enabled(
            true,
            &raw_output.0,
            &raw,
            PpoConfig::default(),
            &references,
            || json!({"purpose":"actor_credit_comparison"}),
        )
        .unwrap();
        write_if_enabled(
            true,
            &centered_output.0,
            &centered,
            PpoConfig::default(),
            &references,
            || json!({"purpose":"actor_credit_comparison"}),
        )
        .unwrap();
        let raw_json = raw_output.read();
        let mut centered_json = centered_output.read();
        assert!(raw_json.get("actor_advantage_centering_mean").is_none());
        let recorded_mean = centered_json["actor_advantage_centering_mean"]
            .as_f64()
            .unwrap();
        assert!((recorded_mean - mean).abs() <= f64::EPSILON * mean.abs().max(1.0));
        centered_json
            .as_object_mut()
            .unwrap()
            .remove("actor_advantage_centering_mean");
        for (row, value) in centered_json["rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            // JSON parsing can differ in the last f64 digit. The actor credit
            // contract is the exact f32 word uploaded to WGSL.
            assert_eq!(
                (value["actor_advantage"].as_f64().unwrap() as f32).to_bits(),
                centered.actor_advantage(row).unwrap().to_bits()
            );
            value.as_object_mut().unwrap().remove("actor_advantage");
        }
        assert_eq!(centered_json, raw_json);
    }

    #[test]
    fn disabled_diagnostic_has_no_io_or_context_evaluation() {
        let (batch, _) = fixture(1);
        let output = Output::new();
        let digest = write_if_enabled(false, &output.0, &batch, PpoConfig::default(), &[], || {
            panic!("disabled context evaluated")
        })
        .unwrap();
        assert_eq!(digest, None);
        assert_eq!(fs::read_dir(&output.0).unwrap().count(), 0);
    }

    #[test]
    fn serialized_targets_are_exact_and_do_not_mutate_the_prepared_batch() {
        let (mut batch, references) = fixture(3);
        let mut transitions = batch.transitions().to_vec();
        transitions[0].reward = 1.0;
        transitions[0].old_value = 2.0;
        transitions[0].next_value = 4.0;
        transitions[1].reward = 3.0;
        transitions[1].old_value = 4.0;
        transitions[1].next_value = 8.0;
        transitions[1].boundary = PpoBoundary::Truncated;
        transitions[2].trajectory_id = 18;
        transitions[2].step = 0;
        transitions[2].reward = 5.0;
        transitions[2].old_value = 6.0;
        transitions[2].next_value = 0.0;
        transitions[2].boundary = PpoBoundary::Terminated;
        for transition in &mut transitions {
            transition.elapsed_seconds = 60.0;
        }
        batch = PpoBatch::from_rollout(7, transitions, PpoConfig::default()).unwrap();
        let before = batch.clone();
        let output = Output::new();
        let digest = write_if_enabled(
            true,
            &output.0,
            &batch,
            PpoConfig::default(),
            &references,
            || json!({"seed":17}),
        )
        .unwrap();
        let saved_bytes = fs::read(output.0.join("ppo-rollout-diagnostics.json")).unwrap();
        assert_eq!(digest, Some(format!("{:x}", Sha256::digest(&saved_bytes))));
        let diagnostic = output.read();
        assert_eq!(batch, before);
        assert_eq!(diagnostic["phase"], "prepared_before_optimizer_update");
        assert_eq!(diagnostic["training_commit_requires_cycle_receipt"], true);
        assert_eq!(diagnostic["rows_truncated"], false);
        for (row, transition) in batch.transitions().iter().enumerate() {
            let saved = &diagnostic["rows"][row];
            for (key, expected) in [
                ("reward", transition.reward),
                ("old_value", transition.old_value),
                ("next_value", transition.next_value),
                ("advantage", batch.advantages()[row]),
                ("return", batch.returns()[row]),
            ] {
                assert_eq!(
                    (saved[key].as_f64().unwrap() as f32).to_bits(),
                    expected.to_bits()
                );
            }
            assert!((saved["gamma"].as_f64().unwrap() - 0.5).abs() < 1e-15);
            assert!((saved["lambda"].as_f64().unwrap() - 1.0 / 64.0).abs() < 1e-15);
        }
        assert_eq!(diagnostic["rows"][1]["boundary"], "truncated");
        assert_eq!(diagnostic["rows"][2]["boundary"], "terminated");
        assert!(diagnostic["rows"][2]["reward"].as_f64().unwrap() > 0.0);
        assert!(diagnostic["rows"][2]["advantage"].as_f64().unwrap() < 0.0);
    }

    #[test]
    fn bounded_output_keeps_full_rollout_advantages_and_marks_omitted_rows() {
        let (batch, references) = fixture(MAX_ROWS + 1);
        let before = batch.clone();
        let output = Output::new();
        let digest = write_if_enabled(
            true,
            &output.0,
            &batch,
            PpoConfig::default(),
            &references,
            || json!({}),
        )
        .unwrap();
        assert!(digest.is_some());
        let diagnostic = output.read();
        assert_eq!(batch, before);
        assert_eq!(diagnostic["total_rows"], MAX_ROWS + 1);
        assert_eq!(diagnostic["emitted_rows"], MAX_ROWS);
        assert_eq!(diagnostic["rows_truncated"], true);
        let last = &diagnostic["rows"][MAX_ROWS - 1];
        assert_eq!(last["boundary"], "continuing");
        assert_eq!(
            last["advantage"].as_f64().unwrap() as f32,
            batch.advantages()[MAX_ROWS - 1]
        );
    }
}
