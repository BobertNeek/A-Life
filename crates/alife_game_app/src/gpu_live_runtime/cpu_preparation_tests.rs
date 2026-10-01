//! CPU-only contracts for the production runner. No device, provider or neural
//! dispatch is created: device commit atomicity needs the separate GPU suite.
use super::*;
use alife_core::{HysteresisState, StableFocusIdentity, TrackedObjectId};
use alife_world::HeadlessScenarioBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "terrain_hint_tests.rs"]
mod terrain_hint_tests;

struct OwnerFixture {
    input: CapturedCpuPreparation,
    memory: MemorySidecarState,
    topology: TopologySidecar,
    predictor: GroundedSuccessorPredictor,
}

impl OwnerFixture {
    fn new(owner: u64, learned: bool) -> Self {
        Self::profiled(
            owner,
            learned,
            SensorProfile::GroundedObjectSlotsV1,
            BrainScaleTier::Nano512,
        )
    }

    fn terrain(owner: u64, learned: bool) -> Self {
        Self::profiled(
            owner,
            learned,
            SensorProfile::GroundedTerrainVisionV1,
            BrainScaleTier::Standard2048,
        )
    }

    fn profiled(
        owner: u64,
        learned: bool,
        sensor_profile: SensorProfile,
        brain_class: BrainScaleTier,
    ) -> Self {
        let organism = OrganismId(owner);
        // Keep the established object-slot scenario; TerrainVision only sees
        // the forward cone, so both distinct foods must lie ahead of its gaze.
        let food_b = if sensor_profile == SensorProfile::GroundedTerrainVisionV1 {
            Vec3f::new(2.0, 0.0, -1.0)
        } else {
            Vec3f::new(-2.0, 0.0, 0.0)
        };
        let mut world = HeadlessScenarioBuilder::new(77_112)
            .agent("agent", organism, Vec3f::ZERO)
            .food("food-a", Vec3f::new(4.0, 0.0, 0.0), 0.8)
            .food("food-b", food_b, 0.8)
            .build()
            .unwrap();
        if brain_class == BrainScaleTier::Nano512 {
            super::super::tests::register_sealing_test_organism(&mut world, organism);
        } else {
            let (asset, _) = terrain_reference_foundation();
            let manifest = asset.manifest();
            let identity = FoundationGeneticIdentity::new(
                manifest.foundation_id().raw(),
                manifest.foundation_version().raw() as u16,
                manifest.compatibility_family_id().raw(),
                BrainCapacityClass::N2048_ID,
            )
            .unwrap();
            let genome = alife_core::CreatureGenome::early_mammal_founder(9_308, identity).unwrap();
            let phenotype = genome.express().unwrap();
            let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
            let entity = world.organism_entity_ids()[0].1;
            world
                .register_organism_record(
                    WorldOrganismRecord::new(
                        organism,
                        entity,
                        genome,
                        phenotype,
                        biology,
                        Tick::ZERO,
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        let draft = world
            .perception_frame_draft(
                organism,
                Tick::ZERO,
                sensor_profile,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        assert_eq!(draft.grounded_object_slots().len(), 2);
        let object_b = draft
            .grounded_object_slots()
            .iter()
            .find(|s| {
                if sensor_profile == SensorProfile::GroundedTerrainVisionV1 {
                    s.bearing[0] < 0.0
                } else {
                    s.bearing[1] < 0.0
                }
            })
            .unwrap()
            .tracked_object_id;
        let phenotype = if brain_class == BrainScaleTier::Nano512 {
            GpuLiveBrainRuntime::compile_birth(&world, brain_class, sensor_profile, organism)
                .unwrap()
                .0
        } else {
            terrain_reference_foundation().1
        };
        let canonical = world.organism_registry().get(organism).unwrap();
        let mut neural_receptors = canonical
            .biochemistry()
            .neural_receptor_frame(canonical.phenotype())
            .unwrap();
        // Retimed immutable perception lets earlier sealed episodes be recalled.
        let tick = Tick::new(1_000);
        neural_receptors.source_tick = tick;
        let mut receptor_effects = NeuralReceptorEffects::from_frame(
            &neural_receptors,
            &NeuralReceptorPhenotype::compile(&phenotype).unwrap(),
        )
        .unwrap();
        receptor_effects.regional_excitability = 1.5;
        receptor_effects.attention_gain = 1.5;
        receptor_effects.projection_gain = 1.0;
        receptor_effects.local_threshold_shift = 0.0;
        let homeostasis = HomeostaticSnapshot::baseline(tick);
        let mut sensory = draft.sensory().clone();
        sensory.tick = tick;
        let draft = PerceptionFrameDraft::new(
            organism,
            tick,
            draft.sensor_profile(),
            sensory,
            draft.body(),
            homeostasis,
            draft.candidates().to_vec(),
            alife_core::SensorProfileProvenance::new(
                draft.sensor_profile(),
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            draft.grounded_object_slots().to_vec(),
        )
        .unwrap();
        let memory =
            GpuLiveBrainRuntime::new_memory_sidecar(organism, draft.sensor_profile()).unwrap();
        let topology = TopologySidecar::new_profiled(
            organism,
            draft.profile_provenance().identity(),
            TopologicalMapConfig::default(),
        )
        .unwrap();
        let mut predictor =
            GroundedSuccessorPredictor::with_learning_rate(0.1 + (owner % 5) as f32 * 0.01)
                .unwrap();
        let sequence_id = ExperienceSequenceId(1_001 + owner);
        if learned {
            let candidate = &draft.candidates()[0];
            let command = candidate
                .to_command(organism, candidate.sensor_confidence)
                .unwrap();
            let bundle = alife_core::arbitrate_gpu_selected_command_into_factorized_bundle(
                organism,
                sequence_id,
                tick,
                Vec::new(),
                &command,
                None,
                false,
            )
            .unwrap();
            let source = grounded_semantic_state_from_draft(&draft).unwrap();
            let target = SemanticStateVector::new(vec![0.8; source.len()]).unwrap();
            let receipt = PredictionTargetReceipt::for_successor(
                organism,
                sequence_id,
                candidate.action_id,
                tick,
                draft.base_digest().0,
                source,
                JointMotorCondition::from_bundle(&bundle).unwrap(),
                target,
            )
            .unwrap();
            predictor.observe(&receipt).unwrap();
            assert!(predictor.has_acquired_state());
        }
        Self {
            input: CapturedCpuPreparation {
                draft,
                sequence_id,
                homeostasis,
                hysteresis: HysteresisState {
                    previous_identity: Some(StableFocusIdentity::TrackedObject(object_b)),
                    retained_ticks: (owner % 7) as u16 + 2,
                    ..HysteresisState::default()
                },
                policy: AttentionSelectionPolicy {
                    focal_capacity: 1,
                    protected_minimum: 1,
                    requested_focal_count: 1,
                    switch_cost: NormalizedScalar(0.01),
                    hysteresis_margin: NormalizedScalar(0.01),
                },
                neural_receptors,
                receptor_effects,
            },
            memory,
            topology,
            predictor,
        }
    }

    fn job<'a>(&'a self, slot: &'a GpuBrainSlot) -> CpuPreparationJob<'a> {
        CpuPreparationJob {
            input: &self.input,
            slot: Some(slot),
            memory: Some(&self.memory),
            topology: Some(&self.topology),
            predictor: Some(&self.predictor),
        }
    }

    fn snapshot(&self) -> serde_json::Value {
        // Memory sidecars intentionally are not Serialize. Their full portable
        // active bank and compaction checkpoint are the durable typed snapshot.
        serde_json::json!({ "input_draft": self.input.draft,
            "receptors": self.input.neural_receptors, "effects": self.input.receptor_effects,
            "hysteresis": self.input.hysteresis, "bank": self.memory.export_active_bank().unwrap(),
            "compaction": self.memory.compaction_checkpoint(), "topology": self.topology,
            "predictor": self.predictor })
    }
}

fn terrain_reference_foundation() -> (FoundationWeightAsset, alife_core::BrainPhenotype) {
    // Same native CPU construction as initial_n2048_care_asset. Builtins reject
    // TerrainVision; do not relabel an object-slot asset or use obsolete metadata.
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(77_112, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar(1.0));
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap(),
        phenotype
    );
    assert_eq!(
        phenotype.sensor_profile(),
        SensorProfile::GroundedTerrainVisionV1
    );
    (asset, phenotype)
}

fn terrain_slots(count: usize) -> Vec<GpuBrainSlot> {
    let (_, phenotype) = terrain_reference_foundation();
    let mut bucket =
        alife_gpu_backend::GpuClassBucketPlan::new(BrainCapacityClass::n2048(), count as u32)
            .unwrap();
    (0..count)
        .map(|index| {
            let slot = bucket
                .insert_phenotype(index as u32, 1, &phenotype)
                .unwrap();
            assert_eq!(
                slot.record().class_id,
                u32::from(BrainCapacityClass::N2048_ID.raw())
            );
            slot
        })
        .collect()
}

fn canonical_slots(count: usize) -> Vec<GpuBrainSlot> {
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))
    .unwrap();
    let manifest = asset.manifest();
    let identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(),
        manifest.capacity_class_id(),
    )
    .unwrap();
    let extension = alife_core::CognitiveChannelExtensionV1::try_new_v1(
        identity,
        alife_core::ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    let candidate =
        alife_core::Nano512ActionCreditCandidateV2::new_with_cognitive_extension(&asset, extension)
            .unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&candidate).unwrap();
    let mut bucket =
        alife_gpu_backend::GpuClassBucketPlan::new(BrainCapacityClass::n512(), count as u32)
            .unwrap();
    (0..count)
        .map(|index| {
            let slot = bucket
                .insert_phenotype(index as u32, 1, &phenotype)
                .unwrap();
            assert_eq!(slot.decoder_input_stride(), 54);
            slot
        })
        .collect()
}

fn topology_fixture(
    input: &CapturedCpuPreparation,
    object: TrackedObjectId,
    concept_score: f32,
    gap_score: f32,
) -> TopologySidecar {
    let topology = TopologySidecar::new_profiled(
        input.draft.organism_id(),
        input.draft.profile_provenance().identity(),
        TopologicalMapConfig::default(),
    )
    .unwrap();
    let mut concept = alife_core::ConceptCell::new(
        alife_core::ConceptCellId(1),
        alife_core::ConceptBindings {
            objects: vec![object],
            ..Default::default()
        },
    )
    .unwrap();
    concept.confidence = Confidence(1.0);
    concept.salience = NormalizedScalar(concept_score);
    let gap = alife_core::UnresolvedGap {
        id: alife_core::UnresolvedGapId(1),
        source_concepts: vec![concept.id],
        contradiction_type: alife_core::ContradictionType::PredictionError,
        prediction_error: NormalizedScalar(1.0),
        curiosity_voltage: NormalizedScalar(1.0),
        salience: NormalizedScalar(gap_score),
        first_tick: Tick::ZERO,
        last_tick: Tick::ZERO,
        confidence: Confidence(1.0),
        status: alife_core::GapResolutionStatus::Open,
    };
    let mut encoded = serde_json::to_value(topology).unwrap();
    encoded["map"]["concepts"] = serde_json::to_value(vec![concept]).unwrap();
    encoded["map"]["unresolved_gaps"] = serde_json::to_value(vec![gap]).unwrap();
    encoded["map"]["next_concept_id"] = serde_json::json!(2);
    encoded["map"]["next_gap_id"] = serde_json::json!(2);
    let mut topology: TopologySidecar = serde_json::from_value(encoded).unwrap();
    topology.decay_edges(0).unwrap();
    topology.validate_contract().unwrap();
    topology
}

fn assert_same_trace(actual: &CpuPreparationOutcome, expected: &CpuPreparationOutcome) {
    // Full PartialEq includes final keys/receipts, routed candidate order,
    // cognitive projection, receptor binding and literal upload words.
    assert_eq!(actual.result, expected.result);
    assert_eq!(actual.stage, expected.stage);
    assert_eq!(actual.hysteresis, expected.hysteresis);
}

fn visits(count: usize) -> PreparationFaults {
    PreparationFaults {
        visits: (0..count).map(|_| AtomicUsize::new(0)).collect(),
        ..Default::default()
    }
}

fn assert_once(faults: &PreparationFaults) {
    assert!(faults.visits.iter().all(|v| v.load(Ordering::Relaxed) == 1));
}

#[test]
fn worker_policy_bounds_workers_after_serial_hint_capture() {
    for rows in [8, 16, 50, 1_000] {
        for available in [2, 4, 64] {
            assert_eq!(preparation_worker_count(rows, available), 2);
        }
    }
    for rows in 0..8 {
        assert_eq!(preparation_worker_count(rows, 64), 1);
    }
    for available in [0, 1] {
        assert_eq!(preparation_worker_count(50, available), 1);
    }
}

#[test]
fn legacy_serial_and_two_workers_preserve_learned_owner_traces_across_mixed_skips() {
    // Dispatched owners are deliberately noncontiguous in canonical order.
    // Sleep, checkpoint hold, and retained-learning owners supply no CPU job;
    // scheduler decisions themselves belong to the live integration tests.
    let owners = [
        (1, true),
        (2, false),
        (7, true),
        (9, false),
        (14, true),
        (21, true),
        (25, true),
        (31, true),
        (47, true),
        (90, true),
        (93, true),
    ];
    let fixtures: Vec<_> = owners
        .iter()
        .map(|(id, _)| OwnerFixture::new(*id, id % 2 == 1))
        .collect();
    let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
    let slots = canonical_slots(fixtures.len());
    let jobs: Vec<_> = fixtures
        .iter()
        .zip(&slots)
        .zip(&owners)
        .filter(|(_, (_, awake))| *awake)
        .map(|((fixture, slot), _)| fixture.job(slot))
        .collect();
    assert_eq!(jobs.len(), 9);
    let serial = prepare_cpu_rows(&jobs, 1, false).unwrap();
    let faults = visits(jobs.len());
    let parallel = prepare_cpu_rows_inner(&jobs, 2, false, &faults).unwrap();
    assert_once(&faults);
    for ((job, serial), parallel) in jobs.iter().zip(&serial).zip(&parallel) {
        assert!(serial.result.is_ok());
        assert_same_trace(serial, &legacy_serial_reference(job));
        assert_same_trace(parallel, serial);
        let prepared = parallel.result.as_ref().unwrap();
        assert_eq!(prepared.frame.organism_id(), job.input.draft.organism_id());
        let projection = prepared
            .memory_recall
            .cognitive_context()
            .unwrap()
            .cognitive_projection
            .as_ref()
            .unwrap();
        assert!(projection
            .candidates
            .iter()
            .all(|candidate| candidate.forecast_available
                == job.predictor.unwrap().has_acquired_state()));

        assert_eq!(
            parallel.hysteresis.unwrap().previous_identity,
            job.input.hysteresis.previous_identity
        );
    }
    assert_eq!(
        fixtures
            .iter()
            .map(OwnerFixture::snapshot)
            .collect::<Vec<_>>(),
        before
    );
    // Reversed row order must preserve each owner's full identity-bound result.
    let reversed: Vec<_> = jobs
        .iter()
        .rev()
        .map(|job| CpuPreparationJob {
            input: job.input,
            slot: job.slot,
            memory: job.memory,
            topology: job.topology,
            predictor: job.predictor,
        })
        .collect();
    let reordered = prepare_cpu_rows(&reversed, 2, false).unwrap();
    for (actual, expected) in reordered.iter().zip(serial.iter().rev()) {
        assert_same_trace(actual, expected);
    }
}

#[test]
fn object_bound_concepts_and_gaps_keep_selective_focus_after_extraction() {
    let mut fixture = OwnerFixture::new(1, true);
    let object_a = fixture
        .input
        .draft
        .grounded_object_slots()
        .iter()
        .find(|s| s.bearing[1] > 0.0)
        .unwrap()
        .tracked_object_id;
    let object_b = fixture
        .input
        .draft
        .grounded_object_slots()
        .iter()
        .find(|s| s.bearing[1] < 0.0)
        .unwrap()
        .tracked_object_id;
    let slots = canonical_slots(1);
    for (object, concept, gap, expected) in [
        (object_a, 0.0, 0.0, object_b),
        (object_a, 0.01, 0.01, object_b),
        (object_a, 1.0, 0.0, object_a),
        (object_a, 0.0, 1.0, object_a),
        (object_a, 1.0, 1.0, object_a),
        (TrackedObjectId(999_999), 1.0, 1.0, object_b),
    ] {
        fixture.topology = topology_fixture(&fixture.input, object, concept, gap);
        let job = fixture.job(&slots[0]);
        let result = prepare_cpu_row(&job, false);
        assert!(result.result.is_ok());
        assert_same_trace(&result, &legacy_serial_reference(&job));
        assert_eq!(
            result.hysteresis.unwrap().previous_identity,
            Some(StableFocusIdentity::TrackedObject(expected))
        );
    }
}

#[test]
fn one_owner_early_or_late_failure_does_not_corrupt_other_rows_or_hysteresis() {
    let fixtures: Vec<_> = (0..8)
        .map(|index| OwnerFixture::new(3 + index * 7, true))
        .collect();
    let slots = canonical_slots(fixtures.len());
    let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
    let good_jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
    let expected = prepare_cpu_rows(&good_jobs, 1, false).unwrap();
    // Exercise a bad job in each half, both before attention and after attention.
    for failing in [1, 6] {
        for late in [false, true] {
            let mut jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
            if late {
                jobs[failing].slot = None;
            } else {
                jobs[failing].memory = None;
            }
            let faults = visits(jobs.len());
            let actual = prepare_cpu_rows_inner(&jobs, 2, false, &faults).unwrap();
            let serial = prepare_cpu_rows(&jobs, 1, false).unwrap();
            assert_once(&faults);
            for (index, row) in actual.iter().enumerate() {
                assert_same_trace(row, &serial[index]);
                if index != failing {
                    assert_same_trace(row, &expected[index]);
                }
            }
            let failed = &actual[failing];
            assert_eq!(
                failed.result,
                Err(ScaffoldContractError::BrainOwnershipMismatch)
            );
            assert_eq!(
                failed.stage,
                if late {
                    "GPU memory upload"
                } else {
                    "baseline recall"
                }
            );
            assert_eq!(
                failed.hysteresis,
                if late {
                    expected[failing].hysteresis
                } else {
                    None
                }
            );
        }
    }
    assert_eq!(
        fixtures
            .iter()
            .map(OwnerFixture::snapshot)
            .collect::<Vec<_>>(),
        before
    );
}

#[test]
fn caller_and_child_panics_fail_without_partial_results_and_always_join() {
    let fixtures: Vec<_> = (0..8)
        .map(|index| OwnerFixture::new(1 + index * 9, true))
        .collect();
    let slots = canonical_slots(fixtures.len());
    let jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
    let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
    for panic_row in [0, 4] {
        let mut faults = visits(jobs.len());
        faults.panic_row = Some(panic_row);
        let result = prepare_cpu_rows_inner(&jobs, 2, false, &faults);
        assert!(matches!(
            result,
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        ));
        assert_eq!(faults.visits[panic_row].load(Ordering::Relaxed), 1);
        let completed_half = if panic_row == 0 { 4..8 } else { 0..4 };
        for index in completed_half {
            assert_eq!(faults.visits[index].load(Ordering::Relaxed), 1);
        }
        assert!(faults.visits.iter().all(|v| v.load(Ordering::Relaxed) <= 1));
        // Seeing every row in the healthy child when caller panics establishes
        // that the runner joined it before returning its whole-batch error.
        assert_eq!(
            fixtures
                .iter()
                .map(OwnerFixture::snapshot)
                .collect::<Vec<_>>(),
            before
        );
    }
}

#[test]
fn spawn_failure_falls_back_once_using_the_captured_semantic_hint() {
    let mut fixtures: Vec<_> = (0..8)
        .map(|index| OwnerFixture::new(1 + index * 3, true))
        .collect();
    let hint = |code| alife_core::SemanticContextRef {
        feature_flags: alife_core::ContextFeatureFlags(0),
        confidence: Confidence(0.1),
        compressed_codes: vec![alife_core::CompressedSemanticCode {
            codebook_id: 1,
            code,
            salience: NormalizedScalar(1.0),
        }],
        salience: Vec::new(),
    };
    let old_hint = hint(17);
    let new_hint = hint(29);
    for fixture in &mut fixtures {
        fixture.input.draft = fixture
            .input
            .draft
            .clone()
            .with_semantic_context(Some(old_hint.clone()))
            .unwrap();
    }
    // A later asynchronous reply is queued only after the immutable captures.
    // Workers have no receiver and cannot consume or request a second hint.
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    reply_tx.send(new_hint.clone()).unwrap();
    let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
    let slots = canonical_slots(fixtures.len());
    let jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
    let expected = prepare_cpu_rows(&jobs, 1, false).unwrap();
    let mut faults = visits(jobs.len());
    faults.spawn_failure = true;
    let fallback = prepare_cpu_rows_inner(&jobs, 2, false, &faults).unwrap();
    assert_once(&faults);
    for (actual, expected) in fallback.iter().zip(&expected) {
        assert_same_trace(actual, expected);
        assert_eq!(
            actual
                .result
                .as_ref()
                .unwrap()
                .frame
                .sensory()
                .semantic_context,
            Some(old_hint.clone())
        );
    }
    assert_eq!(reply_rx.try_recv().unwrap(), new_hint);
    assert_eq!(
        fixtures
            .iter()
            .map(OwnerFixture::snapshot)
            .collect::<Vec<_>>(),
        before
    );
}

// Frozen serial suffix from staged_tick.rs at 1c53224a, starting at baseline
// recall. Only owner lookup, hysteresis publication, timers, and backend upload
// admission are replaced with immutable fixture inputs and CPU upload builders.
// Keep this independently spelled out: calling prepare_cpu_row here would only
// test scheduling, and could conceal an extraction-order regression.
fn legacy_serial_reference(job: &CpuPreparationJob<'_>) -> CpuPreparationOutcome {
    let input = job.input;
    let draft = &input.draft;
    let memory = job.memory.unwrap();
    let topology = job.topology.unwrap();
    let predictor = job.predictor.unwrap();
    let slot = job.slot.unwrap();
    let sequence_id = input.sequence_id;
    let tick_before = draft.tick();
    let receptor_effects = input.receptor_effects;
    let mut preparation_stage = "baseline recall";
    let mut hysteresis = None;
    let result = (|| {
        sequence_id.validate()?;
        let prepared_recall = memory.recall_frame(draft)?;
        let baseline_context = cognitive_context_for_recall(
            draft.organism_id(),
            sequence_id,
            &prepared_recall,
            topology,
        )?;
        preparation_stage = "baseline attention evidence";
        let baseline_prepared = prepared_recall.with_cognitive_context(baseline_context.clone())?;
        let memory_evidence = baseline_prepared.attention_evidence_for_draft(draft)?;
        let mut peripheral_summaries =
            grounded_peripheral_summaries(draft.grounded_object_slots())?;
        let topology_evidence = topology_evidence_for_draft(draft, topology)?;
        let body_need = input
            .homeostasis
            .drives
            .to_array()
            .iter()
            .copied()
            .fold(0.0, f32::max);
        apply_predecision_attention_evidence(
            &mut peripheral_summaries,
            body_need,
            &memory_evidence,
            &baseline_context,
            &topology_evidence,
            receptor_effects,
        )?;
        preparation_stage = "attention selection";
        for summary in &mut peripheral_summaries {
            if let alife_core::StableFocusIdentity::TrackedObject(id) = summary.identity {
                summary.salience.novelty = NormalizedScalar::new(
                    1.0 - memory.bank().object_familiarity(
                        draft.organism_id(),
                        id,
                        memory.profile(),
                    ),
                )?;
            }
        }
        let attention = select_focal_targets(
            draft.organism_id(),
            sequence_id,
            tick_before,
            &peripheral_summaries,
            input.hysteresis,
            input.policy,
        )?;
        hysteresis = Some(attention.hysteresis);
        preparation_stage = "focal routing";
        let routed_draft = route_focal_candidates(draft.clone(), &attention)?;
        let novelty = attention
            .focal_targets
            .first()
            .and_then(|id| match id {
                alife_core::StableFocusIdentity::TrackedObject(object) => Some(
                    1.0 - memory.bank().object_familiarity(
                        draft.organism_id(),
                        *object,
                        memory.profile(),
                    ),
                ),
                _ => None,
            })
            .unwrap_or(0.0);
        let routed_draft = routed_draft.with_remembered_novelty(novelty)?;
        preparation_stage = "routed recall";
        let routed_recall = memory.recall_frame(&routed_draft)?;
        let cognitive_context = cognitive_context_for_recall(
            draft.organism_id(),
            sequence_id,
            &routed_recall,
            topology,
        )?;
        let cognitive_context = cognitive_context_with_attention(cognitive_context, attention)?;
        preparation_stage = "cognitive projection";
        let cognitive_projection = cognitive_projection_for_draft(
            &routed_draft,
            &routed_recall,
            sequence_id,
            predictor,
            &topology_evidence,
        )?;
        let cognitive_context =
            cognitive_context_with_projection(cognitive_context, cognitive_projection)?;
        preparation_stage = "routed finalization";
        let prepared_recall = routed_recall.with_cognitive_context(cognitive_context)?;
        let (frame, memory_recall) = prepared_recall.finalize(routed_draft)?;
        memory_recall.validate_for_frame(&frame)?;
        preparation_stage = "GPU memory upload";
        let perception = alife_gpu_backend::GpuPerceptionUpload::try_from_frame(&frame, slot, 0)
            .map_err(upload_error)?;
        let memory_upload = GpuMemoryContextUpload::try_from_finalized(
            &frame,
            &memory_recall,
            perception.frame_binding,
            slot,
        )
        .map_err(upload_error)?
        .bind_neural_receptor_effects(receptor_effects)
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        Ok(PreparedCpuFrame {
            frame,
            memory_recall,
            memory_upload,
        })
    })();
    CpuPreparationOutcome {
        result,
        stage: preparation_stage,
        hysteresis,
        timing: CpuPreparationTiming::default(),
    }
}

// A narrowed version of the established candidate_memory_retrieval fixture:
// distinct grounded objects, sealed neural decision, measured outcome channels.
// This is fixture history, never a live CPU neural execution fallback.
fn seed_memory(fixture: &mut OwnerFixture, records: usize) {
    use alife_core::{
        ActionCandidate, ActionId, ActionKind, ActionTarget, BrainClassSpec, CandidateActionFamily,
        DriveDelta, DurationTicks, EndocrineDelta, ExperiencePatchBuilder, HomeostaticDelta,
        LobeKind, PhenotypeHash, PhysicalActionOutcome, PhysicalContactKind,
        SensorProfileProvenance, SensorySnapshot,
    };
    let organism = fixture.input.draft.organism_id();
    let empty =
        GpuLiveBrainRuntime::new_memory_sidecar(organism, SensorProfile::GroundedObjectSlotsV1)
            .unwrap();
    for index in 0..records {
        let sequence = ExperienceSequenceId(index as u64 + 1);
        let tick = Tick::new(index as u64 * 2 + 2);
        let mut object = fixture.input.draft.grounded_object_slots()[index % 2];
        object.slot_index = 0;
        if index >= 2 {
            object.tracked_object_id = TrackedObjectId(1_000 + index as u64);
        }
        let candidate = ActionCandidate::new(
            0,
            ActionId(200),
            ActionKind::Interact,
            CandidateActionFamily::Ingest,
            CandidateObservationRef::ObjectSlot(0),
            ActionTarget::new(
                Some(WorldEntityId(9_000 + index as u64)),
                Some(Vec3f::new(0.4, 0.0, 0.0)),
            ),
            object.candidate_features().unwrap(),
            Confidence(0.9),
            NormalizedScalar(0.2),
            DurationTicks::new(1),
            DurationTicks::new(2),
        )
        .unwrap();
        let draft = PerceptionFrameDraft::new(
            organism,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            SensorySnapshot::new(
                organism,
                tick,
                Vec3f::ZERO,
                fixture.input.draft.sensory().channels,
                Default::default(),
            )
            .unwrap(),
            fixture.input.draft.body(),
            HomeostaticSnapshot::baseline(tick),
            vec![candidate],
            SensorProfileProvenance::new(
                SensorProfile::GroundedObjectSlotsV1,
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            vec![object],
        )
        .unwrap();
        let (frame, finalized) = empty.recall_frame(&draft).unwrap().finalize(draft).unwrap();
        let decision = DecisionSnapshot::from_neural_selection(
            sequence,
            PhenotypeHash([1, 2, 3, 4]),
            sequence.raw(),
            (sequence.raw() & 1) as u8,
            &frame,
            NeuralActionSelection {
                candidate_index: 0,
                logit: 0.7,
                confidence: Confidence(0.8),
                active_tiles: 8,
                active_synapses: 64,
            },
            frame.candidates()[0]
                .to_command(organism, Confidence(0.8))
                .unwrap(),
        )
        .unwrap()
        .with_finalized_memory_recall(&frame, &finalized, 0)
        .unwrap();
        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let genome = BrainGenome::scaffold(321, spec.id);
        let development = DevelopmentState::new(genome.id, tick, NormalizedScalar(0.5))
            .with_enabled_lobes([
                LobeKind::PerceptualIntegration,
                LobeKind::TemporalPredictive,
                LobeKind::ActionPlanning,
            ]);
        let pre_action = PreActionSnapshot::from_neural_frame(
            sequence,
            spec.id,
            PhenotypeHash([1, 2, 3, 4]),
            genome.id,
            genome.schema_version,
            development,
            frame,
        )
        .unwrap();
        let outcome = PostActionOutcome::new(
            organism,
            sequence,
            Tick::new(tick.raw() + 1),
            false,
            PhysicalActionOutcome {
                contact: PhysicalContactKind::Consumed,
                target_entity: Some(WorldEntityId(9_000 + index as u64)),
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar(0.1),
            },
            HomeostaticDelta {
                drives: DriveDelta {
                    pain: 0.7,
                    fear: 0.7,
                    brain_atp: -0.1,
                    ..DriveDelta::zero()
                },
                hormones: EndocrineDelta::zero(),
            },
            SignedValence(-0.8),
            NormalizedScalar(0.5),
            NormalizedScalar(0.7),
            SignedValence(-0.1),
            NormalizedScalar(0.7),
        )
        .unwrap();
        let patch = ExperiencePatchBuilder::new(sequence)
            .record_pre_action(pre_action)
            .unwrap()
            .record_decision(decision)
            .unwrap()
            .record_outcome(outcome)
            .unwrap()
            .seal()
            .unwrap();
        fixture.memory.observe_sealed_patch(&patch).unwrap();
    }
    assert_eq!(
        fixture.memory.export_active_bank().unwrap().records.len(),
        records
    );
}

fn with_candidate_count(fixture: &mut OwnerFixture, count: usize) {
    let draft = &fixture.input.draft;
    let candidates = (0..count)
        .map(|index| {
            let mut candidate = draft.candidates()[index % draft.candidates().len()];
            candidate.candidate_index = index as u16;
            candidate.action_id = alife_core::ActionId(1_000 + index as u32);
            candidate
        })
        .collect();
    fixture.input.draft = PerceptionFrameDraft::new(
        draft.organism_id(),
        draft.tick(),
        draft.sensor_profile(),
        draft.sensory().clone(),
        draft.body(),
        *draft.homeostasis(),
        candidates,
        draft.profile_provenance(),
        draft.grounded_object_slots().to_vec(),
    )
    .unwrap();
}

#[test]
fn retained_zero_sixty_four_and_full_banks_preserve_complete_preparation_trace() {
    let slots = canonical_slots(8);
    for records in [0, 64, 256] {
        let mut fixtures: Vec<_> = (0..8)
            .map(|index| OwnerFixture::new(1 + index * 11, true))
            .collect();
        for fixture in &mut fixtures {
            seed_memory(fixture, records);
            with_candidate_count(fixture, 26);
        }
        let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
        let jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
        let serial = prepare_cpu_rows(&jobs, 1, false).unwrap();
        let parallel = prepare_cpu_rows(&jobs, 2, false).unwrap();
        for ((job, expected), actual) in jobs.iter().zip(&serial).zip(&parallel) {
            assert!(expected.result.is_ok());
            assert_same_trace(expected, &legacy_serial_reference(job));
            assert_same_trace(actual, expected);
        }
        assert_eq!(
            fixtures
                .iter()
                .map(OwnerFixture::snapshot)
                .collect::<Vec<_>>(),
            before
        );
    }
}

#[test]
fn preparation_panics_roll_back_the_real_staged_tick_wrapper() {
    struct TestAuthority {
        world: HeadlessWorld,
        residents: BTreeMap<u64, ResidentCognition>,
    }
    impl LiveAuthorityOwner for TestAuthority {
        fn world_and_residents(
            &mut self,
        ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>) {
            (&mut self.world, &mut self.residents)
        }
    }
    let fixtures: Vec<_> = (0..8)
        .map(|index| OwnerFixture::new(1 + index * 3, true))
        .collect();
    let slots = canonical_slots(fixtures.len());
    let jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
    let mut authority = TestAuthority {
        world: HeadlessScenarioBuilder::new(8)
            .food("food", Vec3f::ZERO, 0.8)
            .build()
            .unwrap(),
        residents: BTreeMap::new(),
    };
    let initial = authority.world.canonical_signature_digest().unwrap();
    for panic_row in [0, 4] {
        let mut faults = visits(jobs.len());
        faults.panic_row = Some(panic_row);
        let continuation_visits = AtomicUsize::new(0);
        let (result, _) = tick_with_sleep_progress_inner(&mut authority, false, |owner| {
            owner.world.try_advance_tick().unwrap();
            assert_ne!(owner.world.canonical_signature_digest().unwrap(), initial);
            let prepared = prepare_cpu_rows_inner(&jobs, 2, false, &faults)?;
            // Explicit test boundary marking where dispatch could continue.
            // This counter is not a simulated GPU commit or learning receipt.
            continuation_visits.fetch_add(1, Ordering::Relaxed);
            Ok::<_, ScaffoldContractError>(prepared.len())
        });
        assert_eq!(result, Err(ScaffoldContractError::InvalidDecisionEvidence));
        assert_eq!(continuation_visits.load(Ordering::Relaxed), 0);
        assert_eq!(authority.world.tick(), Tick::ZERO);
        assert_eq!(
            authority.world.canonical_signature_digest().unwrap(),
            initial
        );
    }
}

#[test]
#[ignore = "CPU release timing evidence; run explicitly with --release --ignored --nocapture"]
fn production_runner_cpu_timing_evidence() {
    for owners in [8, 16, 32, 50] {
        let mut fixtures: Vec<_> = (0..owners)
            .map(|index| OwnerFixture::new(1 + index as u64 * 7, true))
            .collect();
        for fixture in &mut fixtures {
            seed_memory(fixture, 64);
            with_candidate_count(fixture, 26);
        }
        let slots = canonical_slots(owners);
        let jobs: Vec<_> = fixtures.iter().zip(&slots).map(|(f, s)| f.job(s)).collect();
        let serial = prepare_cpu_rows(&jobs, 1, false).unwrap();
        assert!(serial.iter().all(|row| row.result.is_ok()));
        let counts = visits(owners);
        let parallel = prepare_cpu_rows_inner(&jobs, 2, false, &counts).unwrap();
        assert_once(&counts);
        for (actual, expected) in parallel.iter().zip(&serial) {
            assert_same_trace(actual, expected);
        }
        let mut samples = [Vec::new(), Vec::new()];
        for sample in 0..9 {
            let order = if sample % 2 == 0 { [1, 2] } else { [2, 1] };
            for workers in order {
                let started = Instant::now();
                let result = prepare_cpu_rows(&jobs, workers, false).unwrap();
                let elapsed = elapsed_ns(started);
                for (actual, expected) in result.iter().zip(&serial) {
                    assert_same_trace(actual, expected);
                }
                samples[workers - 1].push(elapsed);
            }
        }
        let mut medians = [0; 2];
        for (index, sample) in samples.iter().enumerate() {
            let mut sorted = sample.clone();
            sorted.sort_unstable();
            medians[index] = sorted[sorted.len() / 2];
        }
        println!(
            "{}",
            serde_json::json!({ "runner": "production_prepare_cpu_rows", "owners": owners,
            "candidates": 26, "retained_records": 64, "samples_ns": samples,
            "median_ns": medians, "full_trace_equal": true, "row_visits_once": true,
            "device_created": false })
        );
    }
}

#[test]
fn preparation_is_read_only_and_existing_learning_guard_rejects_second_commit() {
    let fixture = OwnerFixture::new(811, true);
    let slot = canonical_slots(1).pop().unwrap();
    let jobs = [fixture.job(&slot)];
    let phenotype = alife_core::PhenotypeHash([1, 2, 3, 4]);
    let owner = fixture.input.draft.organism_id();
    let mut guard = alife_core::LearningSequenceGuard::new(owner, phenotype);
    let initial = guard.clone();
    let before = fixture.snapshot();
    let mut faults = visits(1);
    faults.spawn_failure = true;
    let prepared = prepare_cpu_rows_inner(&jobs, 2, false, &faults).unwrap();
    assert_once(&faults);
    assert_eq!(guard, initial);
    assert_eq!(fixture.snapshot(), before);
    let context = prepared[0]
        .result
        .as_ref()
        .unwrap()
        .memory_recall
        .cognitive_context()
        .unwrap();
    let key = alife_core::OutcomeCreditReplayKey {
        organism_id: owner,
        phenotype_hash: phenotype,
        sequence_id: context.sequence_id,
    };
    // Exercise the actual unchanged guard used by backend host learning commit.
    // These are CPU guard transitions, not device inference or learning proof.
    let first = guard.validate_next(key).unwrap();
    let stale = guard.validate_next(key).unwrap();
    guard.commit_validated(first).unwrap();
    let committed = guard.clone();
    assert_eq!(
        guard.commit_validated(stale),
        Err(ScaffoldContractError::LearningReplayRejected)
    );
    assert_eq!(
        guard.validate_next(key),
        Err(ScaffoldContractError::LearningReplayRejected)
    );
    assert_eq!(guard, committed);
}
