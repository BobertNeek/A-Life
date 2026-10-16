//! CPU-only coverage of the production preflight, including release builds.

use super::*;
use alife_core::{
    ActionCandidate, ActionTarget, BodySnapshot, BrainActivityPolicyV1, BrainDispatchIdentity,
    BrainGenome, CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef,
    DevelopmentState, DurationTicks, GpuPressureSample, GpuPressureSampleInput,
    HomeostaticSnapshot, MemoryBank, MemoryBankConfig, NeuralReceptorEffects, NormalizedScalar,
    OrganismId, PerceptionFrameDraft, PhenotypeCompiler, Pose, SensorProfile,
    SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot, Tick, Vec3f,
    Velocity,
};

struct Fixture {
    phenotype: BrainPhenotype,
    slots: Vec<GpuBrainSlot>,
    frames: Vec<PerceptionFrame>,
    decisions: Vec<NeuralThrottleDecision>,
    memories: Vec<GpuMemoryContextUpload>,
}

impl Fixture {
    fn new(capacity: BrainCapacityClass) -> Self {
        let genome = BrainGenome::scaffold(41, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let profile = SensorProfile::PrivilegedAffordanceV1;
        let phenotype =
            PhenotypeCompiler::compile(&genome, &capacity, &development, profile).unwrap();
        let mut plan = crate::GpuClassBucketPlan::new(capacity, 2).unwrap();
        let mut slots = Vec::new();
        let mut frames = Vec::new();
        let mut decisions = Vec::new();
        let mut memories = Vec::new();
        for row in 0..2 {
            let slot = plan.insert_phenotype(row, row + 7, &phenotype).unwrap();
            let owner = OrganismId(u64::from(row) + 1);
            let tick = Tick(100);
            let candidates = [
                (ActionKind::Idle, CandidateActionFamily::Idle),
                (ActionKind::Inspect, CandidateActionFamily::Inspect),
            ]
            .into_iter()
            .enumerate()
            .map(|(index, (kind, family))| {
                let mut features = CandidateFeatureVector::zero();
                features.0[0] = (row as f32 + 1.0) * (index as f32 - 0.5);
                ActionCandidate::new(
                    index as u16,
                    kind.canonical_id(),
                    kind,
                    family,
                    CandidateObservationRef::None,
                    ActionTarget::NONE,
                    features,
                    Confidence::new(0.8).unwrap(),
                    NormalizedScalar::new(0.1).unwrap(),
                    DurationTicks::new(1),
                    DurationTicks::new(1),
                )
                .unwrap()
            })
            .collect();
            let draft = PerceptionFrameDraft::new(
                owner,
                tick,
                profile,
                SensorySnapshot::new(
                    owner,
                    tick,
                    Vec3f::ZERO,
                    SensoryChannels::ZERO,
                    Default::default(),
                )
                .unwrap(),
                BodySnapshot {
                    pose: Pose::IDENTITY,
                    velocity: Velocity::ZERO,
                },
                HomeostaticSnapshot::baseline(tick),
                candidates,
                SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
                Vec::new(),
            )
            .unwrap();
            let bank = MemoryBank::new(
                MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
            )
            .unwrap();
            let (frame, recall) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();
            let perception = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
            let memory = GpuMemoryContextUpload::try_from_finalized(
                &frame,
                &recall,
                perception.frame_binding,
                &slot,
            )
            .unwrap()
            .bind_neural_receptor_effects(NeuralReceptorEffects {
                source_tick: tick,
                source_chemistry_version: 1,
                interoceptive_gain: 0.5,
                regional_excitability: 0.5,
                projection_gain: 0.5,
                local_threshold_shift: 0.25,
                attention_gain: 0.5,
                plasticity_appetitive: 0.0,
                plasticity_aversive: 0.0,
                structural_growth_gate: 0.0,
                sleep_gate: 0.0,
                consolidation_gate: 0.0,
            })
            .unwrap();
            let identity = BrainDispatchIdentity {
                organism_id_raw: owner.raw(),
                tick: tick.raw(),
                class_id_raw: capacity.id().raw(),
                handle_slot: slot.record().slot,
                handle_generation: slot.record().slot_generation,
                sequence_cursor: 1,
                dispatch_generation: 1,
                frame_digest: frame.frame_digest().0,
            };
            let policy = BrainActivityPolicyV1::production_v1();
            let pressure = GpuPressureSample::try_new(
                &policy,
                GpuPressureSampleInput {
                    identity,
                    source_dispatch_generation: 0,
                    source_frame_digest: [0; 4],
                    completed_gpu_time_ns: 0,
                    queue_depth: 0,
                    logical_heap_used: 0,
                    logical_heap_capacity: 1,
                    brain_atp_remaining_q16: 65_535,
                    brain_atp_capacity_q16: 65_535,
                },
            )
            .unwrap();
            decisions.push(
                NeuralThrottleDecision::derive(
                    &policy,
                    &phenotype,
                    capacity.execution(),
                    identity,
                    pressure,
                )
                .unwrap(),
            );
            slots.push(slot);
            frames.push(frame);
            memories.push(memory);
        }
        Self {
            phenotype,
            slots,
            frames,
            decisions,
            memories,
        }
    }

    fn build(&self, with_memory: bool) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
        self.build_with_capacity(with_memory, GPU_REQUIRED_MAX_BUFFER_WORDS)
    }

    fn build_with_capacity(
        &self,
        with_memory: bool,
        payload_capacity: usize,
    ) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
        let entries = (0..2)
            .map(|row| GpuBatchEntryView {
                frame: &self.frames[row],
                slot: &self.slots[row],
                phenotype: &self.phenotype,
                activity: &self.decisions[row],
                memory_upload: with_memory.then_some(&self.memories[row]),
                active_eligibility_generation: 3 + row as u64,
            })
            .collect::<Vec<_>>();
        let sides = BTreeMap::from([((0, 7), 0), ((1, 8), 1)]);
        GpuActiveBatchUpload::try_from_views(
            &entries,
            37,
            11,
            &sides,
            GPU_REQUIRED_MAX_BUFFER_WORDS,
            payload_capacity,
            NonZeroU64::new(1).unwrap(),
            13,
        )
    }
}

#[test]
fn batch_projection_matches_checked_baseline_for_all_promoted_classes() {
    eprintln!(
        "batch projection debug_assertions={}",
        cfg!(debug_assertions)
    );
    let mut digests = Vec::new();
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let fixture = Fixture::new(capacity);
        for with_memory in [false, true] {
            let batch = fixture.build(with_memory).unwrap();
            // This covers every GPU word and host binding, not just selected
            // offsets. Baselines were captured with the original full checks
            // on main 69f00c05ec6774c3655ae87c5d4baa5df3861929.
            digests.push(
                blake3::hash(format!("{batch:?}").as_bytes())
                    .to_hex()
                    .to_string(),
            );
        }
    }
    assert_eq!(
        digests,
        [
            "10b9a14f3948013905023884e3a5d6f4a67757996603e5d173a9ede583ae4896",
            "0bb9bd483b0c70d3d38c7905b802b4f0ae0e3fae5017264bd90fc319d710db69",
            "93a476623ae92a2dd4819147a25ec78a3fe9d7a353fb56e70063183c7a183d62",
            "69f6e079ce9da85783d143dece5d20deef2ee758535138e44452145af28c5953",
            "750327964e007fe06646e10df61763f18b6a565e7dd1f5e1bfd213981f3861fa",
            "ac9502c10648a77eeca6c74548056b0e26f4a931fe45733f662fb46fe4aef5da",
        ]
    );
}

#[test]
fn batch_projection_rejects_malformed_caller_owned_memory_without_mutating_it() {
    let mut fixture = Fixture::new(BrainCapacityClass::n512());
    let valid = fixture.memories[1].clone();
    let mutations: [fn(&mut GpuMemoryContextUpload); 9] = [
        |memory| memory.header.slot_generation += 1,
        |memory| memory.perception_binding.tick = Tick(101),
        |memory| memory.records[0].target_latent[0] = f32::NAN,
        |memory| memory.records[0].family_value[0] = 0.5,
        |memory| memory.records[0].candidate_index += 1,
        |memory| {
            memory.records.pop();
        },
        |memory| memory.cognitive_records[0].values[0] = f32::INFINITY,
        |memory| {
            memory.cognitive_records.pop();
        },
        |memory| memory.neural_receptor_effects.as_mut().unwrap().source_tick = Tick(99),
    ];
    for mutate in mutations {
        fixture.memories[1] = valid.clone();
        mutate(&mut fixture.memories[1]);
        let before = format!("{:?}", fixture.memories);
        assert!(fixture.build(true).is_err());
        assert_eq!(format!("{:?}", fixture.memories), before);
    }
    fixture.memories[1] = valid;
    fixture.build(true).unwrap();
}

#[test]
fn batch_projection_retains_constructor_and_checked_rebase_failures() {
    let fixture = Fixture::new(BrainCapacityClass::n512());
    let frame = &fixture.frames[0];
    let slot = &fixture.slots[0];
    assert_eq!(
        GpuPerceptionUpload::try_from_frame(frame, slot, 2).unwrap_err(),
        GpuClosedLoopError::MalformedUpload,
    );
    let valid = GpuPerceptionUpload::try_from_frame(frame, slot, 1).unwrap();
    for (dispatch, payload) in [(u32::MAX, 0), (0, u32::MAX)] {
        let mut upload = valid.clone();
        assert_eq!(
            upload.rebase(dispatch, payload),
            Err(GpuClosedLoopError::ArithmeticOverflow)
        );
        assert_eq!(upload, valid);
    }
    let mut upload = valid;
    upload.rebase(128, 37).unwrap();
    assert_eq!(
        upload.rebase(128, 37),
        Err(GpuClosedLoopError::InvalidOffsetDomain)
    );
    upload.validate_against(frame, slot).unwrap();
}

#[test]
fn batch_projection_preserves_memory_error_priority_over_payload_capacity() {
    let mut fixture = Fixture::new(BrainCapacityClass::n512());
    // The prefix fits but the first complete row cannot fit this payload heap.
    assert_eq!(
        fixture.build_with_capacity(true, 37).unwrap_err(),
        GpuClosedLoopError::CapacityExceeded,
    );
    fixture.memories[0].header.slot_generation += 1;
    // Keep invalid caller-input classification ahead of capacity failure: some
    // sessions treat backend-unavailable capacity errors as fail-stop causes.
    assert_eq!(
        fixture.build_with_capacity(true, 37).unwrap_err(),
        GpuClosedLoopError::MalformedUpload,
    );
}
