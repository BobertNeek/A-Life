//! Policy-local immutable replay snapshots; allocation admission is never cached.
use super::{FoundationReplayBudget, PpoTrainingWindow, ScaffoldContractError};
use alife_training::TrainingError;
use std::{borrow::Borrow, mem::size_of, sync::Arc};

const MAX_WINDOWS: usize = 8;
const MAX_CHARGED_BYTES: u64 = 512 * 1024 * 1024;

// A miss keeps the already-admitted owned loader result. Only hits share the
// cache's compact, immutable copy; callers may explicitly copy-on-write it.
// Boxing Owned would add an unguarded allocation on disabled/refused retention.
#[allow(clippy::large_enum_variant)]
pub(super) enum PpoWindowHandle {
    Owned(PpoTrainingWindow),
    Shared(Arc<PpoTrainingWindow>),
}
impl Borrow<PpoTrainingWindow> for PpoWindowHandle {
    fn borrow(&self) -> &PpoTrainingWindow {
        match self {
            Self::Owned(window) => window,
            Self::Shared(window) => window,
        }
    }
}

pub(super) struct PpoWindowSnapshotCache {
    windows: Vec<Option<(Arc<PpoTrainingWindow>, u64)>>,
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
    ) -> Result<Option<PpoWindowHandle>, TrainingError> {
        self.get_with_admission(index, |charge| {
            budget.check_additional(charge).map_err(|error| {
                eprintln!("PPO replay snapshot exceeds measured host allowance: {error}");
                TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
            })
        })
    }

    fn get_with_admission(
        &self,
        index: usize,
        admit: impl FnOnce(u64) -> Result<(), TrainingError>,
    ) -> Result<Option<PpoWindowHandle>, TrainingError> {
        let Some(Some((window, charge))) = self.windows.get(index) else {
            return Ok(None);
        };
        // Keep the existing full reservation on every hit. Replay packing still
        // allocates fresh buffers later; sharing is not a new admission shortcut.
        admit(*charge)?;
        Ok(Some(PpoWindowHandle::Shared(Arc::clone(window))))
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
        self.retain_with_freezer(index, window, admit, |window| Arc::new(window.clone()))
    }

    fn retain_with_freezer(
        &mut self,
        index: usize,
        window: &PpoTrainingWindow,
        admit: impl FnOnce(u64) -> bool,
        freeze: impl FnOnce(&PpoTrainingWindow) -> Arc<PpoTrainingWindow>,
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
        // Keep the original compact deep copy: charging lengths is valid for
        // that clone, not necessarily for the loader's spare capacities. The
        // unchanged doubled charge + padding also covers the Arc control block.
        *slot = Some((freeze(window), charge));
        self.charged_bytes = next_charge;
        true
    }
}

// The initial compact clone allocates from owned lengths, not compressed bytes. Count
// every nested allocation, then double with small padding for allocator overhead.
// This is a conservative retention charge, not an RSS measurement; the measured
// process guard remains required on each access, including shared cache hits.
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
        let wire = |handle: &PpoWindowHandle| {
            let window: &PpoTrainingWindow = handle.borrow();
            bincode::serde::encode_to_vec(&window.sequence, bincode::config::standard()).unwrap()
        };
        assert_eq!(wire(&copy), wire(&PpoWindowHandle::Owned(original.clone())));
        assert_eq!(
            Borrow::<PpoTrainingWindow>::borrow(&copy).batch,
            original.batch
        );
        assert_eq!(
            Borrow::<PpoTrainingWindow>::borrow(&copy)
                .batch
                .transitions()[0]
                .reward
                .to_bits(),
            (-0.0_f32).to_bits()
        );
        let PpoWindowHandle::Shared(ref mut shared) = copy else {
            panic!("hit must share")
        };
        Arc::make_mut(shared).sequence.initial.activations[0] = 9.0;
        assert_eq!(
            wire(&cache.get_with_admission(0, |_| Ok(())).unwrap().unwrap()),
            wire(&PpoWindowHandle::Owned(original))
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
    fn shared(handle: PpoWindowHandle) -> Arc<PpoTrainingWindow> {
        match handle {
            PpoWindowHandle::Shared(value) => value,
            PpoWindowHandle::Owned(_) => panic!("expected shared hit"),
        }
    }
    fn rich_window() -> PpoTrainingWindow {
        let mut value = window();
        value.sequence.initial.dendrites =
            alife_core::DendriticBranchSet::new(vec![alife_core::DendriticBranch::new(
                0,
                -0.0,
                1.0,
                vec![alife_core::DendriticInputRef::new(1, f32::from_bits(1)).unwrap()],
            )
            .unwrap()])
            .unwrap();
        value.sequence.ticks[0]
            .structural_synapses
            .push(alife_training::TrainingFrozenSynapse {
                source: 0,
                target: 1,
                route: 0,
                cadence: 1,
                effective_weight: -0.0,
            });
        value.sequence.ticks[0]
            .candidates
            .push(alife_training::TrainingReplayCandidate {
                family: alife_core::CandidateActionFamily::Inspect,
                decoder_inputs: [f32::from_bits(1); 54],
                innate_bias: -0.0,
            });
        value.auxiliary = Some((
            vec![ImitationTarget {
                representative: Some(0),
                motors: [None; 6],
            }],
            -0.0,
        ));
        value.speech_targets[0] = Some(alife_training::ReplaySpeechTarget {
            token: Some(1),
            continuation: [0; 5],
            act: alife_core::SpeechActKind::Declare,
            weight: f32::from_bits(1),
        });
        value
    }
    fn sequence_and_speech_bytes(value: &PpoTrainingWindow) -> Vec<u8> {
        bincode::serde::encode_to_vec(
            (&value.sequence, &value.speech_targets),
            bincode::config::standard(),
        )
        .unwrap()
    }
    #[test]
    fn ppo_snapshot_cache_hits_share_exact_compact_payload_and_cow_is_isolated() {
        let original = rich_window();
        let mut cache = PpoWindowSnapshotCache::new(1);
        assert!(cache.retain_with_admission(0, &original, |_| true));
        let first = shared(cache.get_with_admission(0, |_| Ok(())).unwrap().unwrap());
        let mut second = shared(cache.get_with_admission(0, |_| Ok(())).unwrap().unwrap());
        assert!(Arc::ptr_eq(&first, &second));
        assert_ne!(
            first.sequence.initial.activations.as_ptr(),
            original.sequence.initial.activations.as_ptr(),
            "first retain must compact-copy, never alias the loader"
        );
        assert_eq!(
            sequence_and_speech_bytes(&first),
            sequence_and_speech_bytes(&original)
        );
        assert_eq!(first.batch, original.batch);
        assert_eq!(
            first.batch.transitions()[0]
                .action
                .old_joint_log_probability
                .to_bits(),
            (-0.0_f32).to_bits()
        );
        assert_eq!(
            first.auxiliary.as_ref().unwrap().1.to_bits(),
            original.auxiliary.as_ref().unwrap().1.to_bits()
        );
        assert_eq!(
            first.auxiliary.as_ref().unwrap().0,
            original.auxiliary.as_ref().unwrap().0
        );
        Arc::make_mut(&mut second).sequence.ticks[0].effective_weight_offsets[0] = 9.0;
        assert!(!Arc::ptr_eq(&first, &second));
        let unchanged = shared(cache.get_with_admission(0, |_| Ok(())).unwrap().unwrap());
        assert!(Arc::ptr_eq(&first, &unchanged));
        assert_eq!(
            sequence_and_speech_bytes(&unchanged),
            sequence_and_speech_bytes(&original)
        );
        let owned = PpoWindowHandle::Owned(original);
        let borrowed: &PpoTrainingWindow = owned.borrow();
        assert_eq!(borrowed.batch.policy_version(), 7);
    }
    #[test]
    fn ppo_snapshot_cache_compacts_spare_capacity_before_length_charging() {
        let mut original = rich_window();
        original.sequence.initial.activations.reserve(1000);
        original.sequence.ticks.reserve(1000);
        original.sequence.ticks[0].encoded_inputs.reserve(1000);
        original.sequence.ticks[0].enabled_routes.reserve(1000);
        original.sequence.ticks[0]
            .effective_weight_offsets
            .reserve(1000);
        original.sequence.ticks[0].structural_synapses.reserve(1000);
        original.sequence.ticks[0].candidates.reserve(1000);
        original.auxiliary.as_mut().unwrap().0.reserve(1000);
        original.speech_targets.reserve(1000);
        let charge = window_clone_charge(&original).unwrap();
        let mut cache = PpoWindowSnapshotCache::new(1);
        assert!(cache.retain_with_admission(0, &original, |n| {
            assert_eq!(n, charge);
            true
        }));
        let retained = shared(
            cache
                .get_with_admission(0, |n| {
                    assert_eq!(n, charge);
                    Ok(())
                })
                .unwrap()
                .unwrap(),
        );
        macro_rules! compact {
            ($v:expr) => {
                assert_eq!($v.len(), $v.capacity())
            };
        }
        compact!(retained.sequence.initial.activations);
        compact!(retained.sequence.ticks);
        let tick = &retained.sequence.ticks[0];
        compact!(tick.encoded_inputs);
        compact!(tick.enabled_routes);
        compact!(tick.effective_weight_offsets);
        compact!(tick.structural_synapses);
        compact!(tick.candidates);
        compact!(retained.auxiliary.as_ref().unwrap().0);
        compact!(retained.speech_targets);
        compact!(retained.sequence.initial.dendrites.branches()[0].inputs);
        assert_eq!(window_clone_charge(&retained), Some(charge));
        // The unchanged doubled owned charge +4096 padding covers the small
        // Arc header/T allocation in addition to the initial clone allocations.
        assert!(
            charge
                >= (size_of::<PpoTrainingWindow>()
                    + 2 * size_of::<usize>()
                    + std::mem::align_of::<PpoTrainingWindow>()) as u64
        );
        assert!(original.sequence.ticks.capacity() > retained.sequence.ticks.capacity());
    }
    #[test]
    fn ppo_snapshot_cache_refuses_before_compact_clone_or_arc_allocation() {
        let original = window();
        let mut cache = PpoWindowSnapshotCache::new(1);
        assert!(!cache.retain_with_freezer(
            0,
            &original,
            |_| false,
            |_| panic!("must not clone or allocate Arc")
        ));
        assert!(!cache.retain_with_freezer(
            1,
            &original,
            |_| panic!("invalid index"),
            |_| panic!("invalid index")
        ));
        let mut disabled = PpoWindowSnapshotCache::new(MAX_WINDOWS + 1);
        assert!(!disabled.retain_with_freezer(
            0,
            &original,
            |_| panic!("disabled"),
            |_| panic!("disabled")
        ));
        cache.byte_limit = 0;
        assert!(!cache.retain_with_freezer(
            0,
            &original,
            |_| panic!("cap before guard"),
            |_| panic!("cap before allocation")
        ));
        assert_eq!(cache.charged_bytes, 0);
        assert!(cache
            .get_with_admission(0, |_| panic!("miss skips admission"))
            .unwrap()
            .is_none());
        assert!(cache
            .get_with_admission(1, |_| panic!("invalid index skips admission"))
            .unwrap()
            .is_none());
        cache.byte_limit = MAX_CHARGED_BYTES;
        let order = std::cell::Cell::new(0);
        assert!(cache.retain_with_freezer(
            0,
            &original,
            |_| {
                order.set(1);
                true
            },
            |w| {
                assert_eq!(order.get(), 1);
                order.set(2);
                Arc::new(w.clone())
            }
        ));
        assert_eq!(order.get(), 2);
        assert!(cache
            .get_with_admission(0, |_| Err(
                ScaffoldContractError::InvalidDecisionEvidence.into()
            ))
            .is_err());
        assert_eq!(cache.charged_bytes, window_clone_charge(&original).unwrap());
    }
}
