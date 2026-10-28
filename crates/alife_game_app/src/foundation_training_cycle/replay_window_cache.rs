//! Policy-local immutable replay snapshots; allocation admission is never cached.
use super::{FoundationReplayBudget, PpoTrainingWindow, ScaffoldContractError};
use alife_training::TrainingError;
use std::mem::size_of;

const MAX_WINDOWS: usize = 8;
const MAX_CHARGED_BYTES: u64 = 512 * 1024 * 1024;

pub(super) struct PpoWindowSnapshotCache {
    windows: Vec<Option<(PpoTrainingWindow, u64)>>,
    charged_bytes: u64,
    byte_limit: u64,
}

impl PpoWindowSnapshotCache {
    pub(super) fn new(window_count: usize) -> Self {
        Self {
            windows: vec![
                None;
                if window_count <= MAX_WINDOWS {
                    window_count
                } else {
                    0
                }
            ],
            charged_bytes: 0,
            byte_limit: MAX_CHARGED_BYTES,
        }
    }

    pub(super) fn get(
        &self,
        index: usize,
        budget: FoundationReplayBudget,
    ) -> Result<Option<PpoTrainingWindow>, TrainingError> {
        self.get_with_admission(index, |charge| {
            budget.check_additional(charge).map_err(|error| {
                eprintln!("PPO replay snapshot clone exceeds measured host allowance: {error}");
                TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
            })
        })
    }

    fn get_with_admission(
        &self,
        index: usize,
        admit: impl FnOnce(u64) -> Result<(), TrainingError>,
    ) -> Result<Option<PpoTrainingWindow>, TrainingError> {
        let Some(Some((window, charge))) = self.windows.get(index) else {
            return Ok(None);
        };
        // A hit skips storage, not the guard on allocating a fresh owned copy.
        admit(*charge)?;
        Ok(Some(window.clone()))
    }

    pub(super) fn retain(
        &mut self,
        index: usize,
        window: &PpoTrainingWindow,
        budget: FoundationReplayBudget,
    ) {
        self.retain_with_admission(index, window, |charge| {
            budget.check_additional(charge).is_ok()
        });
    }

    fn retain_with_admission(
        &mut self,
        index: usize,
        window: &PpoTrainingWindow,
        admit: impl FnOnce(u64) -> bool,
    ) -> bool {
        let Some(slot) = self.windows.get_mut(index) else {
            return false;
        };
        let Some(charge) = window_clone_charge(window) else {
            return false;
        };
        let old_charge = slot.as_ref().map_or(0, |(_, charge)| *charge);
        let Some(next_charge) = self
            .charged_bytes
            .checked_sub(old_charge)
            .and_then(|bytes| bytes.checked_add(charge))
            .filter(|bytes| *bytes <= self.byte_limit)
        else {
            return false;
        };
        // Retention is optional. Do not fail already-admitted uncached work if
        // there is insufficient headroom for this additional owned copy.
        if !admit(charge) {
            return false;
        }
        *slot = Some((window.clone(), charge));
        self.charged_bytes = next_charge;
        true
    }
}

// Clone allocates from owned lengths, not the compressed replay length. Count
// every nested allocation, then double with small padding for allocator overhead.
// This is a conservative retention charge, not an RSS measurement; the measured
// process guard remains required before each clone, including cache hits.
fn window_clone_charge(window: &PpoTrainingWindow) -> Option<u64> {
    fn add<T>(bytes: &mut u64, len: usize) -> Option<()> {
        *bytes = bytes.checked_add((len as u64).checked_mul(size_of::<T>() as u64)?)?;
        Some(())
    }
    let mut bytes = size_of::<PpoTrainingWindow>() as u64;
    let sequence = &window.sequence;
    add::<f32>(&mut bytes, sequence.initial.activations.len())?;
    add::<f32>(&mut bytes, sequence.initial.activity_ema.len())?;
    add::<f32>(&mut bytes, sequence.initial.metabolic_load.len())?;
    let branches = sequence.initial.dendrites.branches();
    add::<alife_core::DendriticBranch>(&mut bytes, branches.len())?;
    for branch in branches {
        add::<alife_core::DendriticInputRef>(&mut bytes, branch.inputs.len())?;
    }
    add::<alife_training::TrainingReplayTick>(&mut bytes, sequence.ticks.len())?;
    for tick in &sequence.ticks {
        add::<f32>(&mut bytes, tick.encoded_inputs.len())?;
        add::<bool>(&mut bytes, tick.enabled_routes.len())?;
        add::<f32>(&mut bytes, tick.effective_weight_offsets.len())?;
        add::<alife_training::TrainingFrozenSynapse>(&mut bytes, tick.structural_synapses.len())?;
        add::<alife_training::TrainingReplayCandidate>(&mut bytes, tick.candidates.len())?;
    }
    add::<alife_training::PpoTransition>(&mut bytes, window.batch.transitions().len())?;
    for transition in window.batch.transitions() {
        add::<Option<u8>>(&mut bytes, transition.action.forced_slots.len())?;
    }
    add::<f32>(&mut bytes, window.batch.advantages().len())?;
    add::<f32>(&mut bytes, window.batch.returns().len())?;
    if let Some((targets, _)) = &window.auxiliary {
        add::<alife_training::ImitationTarget>(&mut bytes, targets.len())?;
    }
    add::<Option<alife_training::ReplaySpeechTarget>>(&mut bytes, window.speech_targets.len())?;
    bytes.checked_mul(2)?.checked_add(4096)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_training::{
        ImitationTarget, PpoBatch, PpoBoundary, PpoConfig, PpoJointAction, PpoTransition,
        TrainingInitialState, TrainingReplayTick, TrainingSequence,
    };

    // Ownership/charge fixture only; no neural or GPU admission is claimed.
    fn window() -> PpoTrainingWindow {
        let transition = PpoTransition {
            policy_version: 7,
            trajectory_id: 1,
            step: 0,
            action: PpoJointAction {
                candidate_count: 1,
                representative_mask: 1,
                motor_masks: [0; 6],
                representative: 0,
                forced_slots: vec![None],
                motor_candidates: [None; 6],
                old_joint_log_probability: -0.0,
                temperature: 1.0,
            },
            reward: -0.0,
            old_value: 0.0,
            next_value: 0.0,
            elapsed_seconds: 0.05,
            boundary: PpoBoundary::Terminated,
        };
        PpoTrainingWindow {
            sequence: TrainingSequence {
                phenotype_hash: alife_core::PhenotypeHash([1; 4]),
                initial: TrainingInitialState {
                    activations: vec![-0.0, f32::from_bits(1)],
                    activity_ema: vec![0.0, 1.0],
                    metabolic_load: vec![0.25, 0.75],
                    dendrites: Default::default(),
                },
                ticks: vec![TrainingReplayTick {
                    encoded_inputs: vec![-0.0, 1.0],
                    projection_gain: 1.0,
                    local_threshold_shift: -0.0,
                    microstep_count: 1,
                    enabled_routes: vec![true],
                    effective_weight_offsets: vec![-0.0, 0.5],
                    structural_synapses: vec![],
                    candidates: vec![],
                }],
                burn_in_ticks: 0,
                memory_candidate_gain: -0.0,
            },
            batch: PpoBatch::from_rollout(7, vec![transition], PpoConfig::default()).unwrap(),
            auxiliary: None,
            speech_targets: vec![None],
        }
    }

    #[test]
    fn ppo_snapshot_cache_admits_before_clone_and_skips_optional_retention() {
        let original = window();
        let charge = window_clone_charge(&original).unwrap();
        let mut cache = PpoWindowSnapshotCache::new(1);
        assert!(!cache.retain_with_admission(0, &original, |bytes| {
            assert_eq!(bytes, charge);
            false
        }));
        assert_eq!(cache.charged_bytes, 0);
        assert!(cache
            .get_with_admission(0, |_| panic!("empty cache"))
            .unwrap()
            .is_none());
        assert!(cache.retain_with_admission(0, &original, |_| true));
        assert!(cache
            .get_with_admission(0, |bytes| {
                assert_eq!(bytes, charge);
                Err(ScaffoldContractError::InvalidDecisionEvidence.into())
            })
            .is_err());
        assert_eq!(cache.charged_bytes, charge);
        let mut copy = cache.get_with_admission(0, |_| Ok(())).unwrap().unwrap();
        let wire = |window: &PpoTrainingWindow| {
            bincode::serde::encode_to_vec(&window.sequence, bincode::config::standard()).unwrap()
        };
        assert_eq!(wire(&copy), wire(&original));
        assert_eq!(copy.batch, original.batch);
        assert_eq!(
            copy.batch.transitions()[0].reward.to_bits(),
            (-0.0_f32).to_bits()
        );
        copy.sequence.initial.activations[0] = 9.0;
        assert_eq!(
            wire(&cache.get_with_admission(0, |_| Ok(())).unwrap().unwrap()),
            wire(&original)
        );
    }

    #[test]
    fn ppo_snapshot_cache_bounds_slots_and_total_clone_charge() {
        let original = window();
        let charge = window_clone_charge(&original).unwrap();
        let mut cache = PpoWindowSnapshotCache::new(MAX_WINDOWS);
        for index in 0..MAX_WINDOWS {
            assert!(cache.retain_with_admission(index, &original, |_| true));
        }
        assert_eq!(cache.charged_bytes, charge * MAX_WINDOWS as u64);
        assert!(!cache.retain_with_admission(MAX_WINDOWS, &original, |_| panic!("outside slots")));
        let mut disabled = PpoWindowSnapshotCache::new(MAX_WINDOWS + 1);
        assert!(!disabled.retain_with_admission(0, &original, |_| panic!("disabled cache")));
        let mut bounded = PpoWindowSnapshotCache::new(2);
        bounded.byte_limit = charge;
        assert!(bounded.retain_with_admission(0, &original, |_| true));
        assert!(
            !bounded.retain_with_admission(1, &original, |_| panic!("byte cap before admission"))
        );
        assert_eq!(bounded.charged_bytes, charge);
        assert!(bounded.retain_with_admission(0, &original, |_| true));
        assert_eq!(
            bounded.charged_bytes, charge,
            "replacement does not double charge"
        );
        assert_eq!(PpoWindowSnapshotCache::new(1).byte_limit, 512 * 1024 * 1024);
    }

    #[test]
    fn ppo_snapshot_charge_covers_nested_owned_lengths() {
        let original = window();
        let base = window_clone_charge(&original).unwrap();
        let mut expanded = original.clone();
        expanded.sequence.initial.activations.push(1.0);
        assert_eq!(
            window_clone_charge(&expanded).unwrap(),
            base + 2 * size_of::<f32>() as u64
        );
        expanded = original.clone();
        expanded.sequence.ticks[0]
            .effective_weight_offsets
            .push(1.0);
        assert!(window_clone_charge(&expanded).unwrap() > base);
        expanded = original.clone();
        expanded.sequence.ticks[0].structural_synapses.push(
            alife_training::TrainingFrozenSynapse {
                source: 0,
                target: 1,
                route: 0,
                cadence: 1,
                effective_weight: -0.0,
            },
        );
        assert!(window_clone_charge(&expanded).unwrap() > base);
        expanded = original.clone();
        expanded
            .sequence
            .ticks
            .push(expanded.sequence.ticks[0].clone());
        assert!(window_clone_charge(&expanded).unwrap() > base);
        expanded = original.clone();
        let mut transitions = expanded.batch.transitions().to_vec();
        transitions[0].action.candidate_count = 2;
        transitions[0].action.forced_slots.push(None);
        expanded.batch = PpoBatch::from_rollout(7, transitions, PpoConfig::default()).unwrap();
        assert_eq!(
            window_clone_charge(&expanded).unwrap(),
            base + 2 * size_of::<Option<u8>>() as u64,
        );
        expanded = original.clone();
        expanded.sequence.initial.dendrites =
            alife_core::DendriticBranchSet::new(vec![alife_core::DendriticBranch::new(
                0,
                -0.0,
                1.0,
                vec![alife_core::DendriticInputRef::new(1, -0.0).unwrap()],
            )
            .unwrap()])
            .unwrap();
        assert!(window_clone_charge(&expanded).unwrap() > base);
        expanded = original.clone();
        expanded.auxiliary = Some((
            vec![ImitationTarget {
                representative: None,
                motors: [None; 6],
            }],
            0.1,
        ));
        assert!(window_clone_charge(&expanded).unwrap() > base);
        expanded = original;
        expanded.speech_targets.push(None);
        assert!(window_clone_charge(&expanded).unwrap() > base);
    }
}
