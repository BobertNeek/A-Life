#![cfg(feature = "gpu-tests")]

use alife_core::*;
use alife_game_app::GpuLiveBrainRuntime;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::GpuCheckpointAssetStore;
use alife_world::{
    persistence::*, CreatureAppearanceGenome, HeadlessScenarioBuilder, WorldOrganismRecord,
};

#[test]
fn fixed_candidate_runs_and_reloads_through_game_runtime() {
    run_case(true);
}

#[test]
fn unchanged_builtin_control_runs_and_reloads_through_game_runtime() {
    run_case(false);
}

// A bounded experiment, not promotion or full founder acceptance. All scored
// outcomes below come from the archived game's ordinary joint-action loop.
#[test]
fn held_out_sampling_after_first_frame_imitation() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let control = FoundationWeightAsset::from_trained_weights(
        &baseline,
        source.weights().to_vec(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (phenotype, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&control).unwrap();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/sampling-probe-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    println!(
        "sampling_hardware={:?}; control={:?}; fixed_graph={:?}",
        backend.hardware_receipt(),
        control.digest(),
        phenotype.phenotype_hash()
    );
    let mut session = alife_runtime::GpuAuthoritativeSession::new(
        backend,
        alife_runtime::GpuSessionConsumerKind::Training,
    );
    let mut examples = Vec::new();
    for (case, position) in [
        Vec3f::new(0.5, 0.0, 0.0),
        Vec3f::new(-0.8, 0.0, 0.0),
        Vec3f::new(0.0, 0.0, 0.7),
        Vec3f::new(0.0, 0.0, -0.6),
        Vec3f::new(2.0, 0.0, 0.0),
        Vec3f::new(-4.0, 0.0, 0.0),
        Vec3f::new(0.0, 0.0, 3.0),
        Vec3f::new(0.0, 0.0, -2.5),
    ]
    .into_iter()
    .enumerate()
    {
        let demo =
            alife_training::record_founder_sampling_demonstration(93000 + case as u64, position)
                .unwrap();
        std::fs::write(
            root.join(format!("demonstration-{case}.json")),
            serde_json::to_vec_pretty(&demo).unwrap(),
        )
        .unwrap();
        let step = &demo.steps[0];
        let organism = step.observation.organism_id();
        let handle = session.insert_brain(organism, phenotype.clone()).unwrap();
        let memory = MemorySidecarState::new_profiled(
            organism,
            SensorProfileIdentity {
                profile_id: profile.into(),
                profile_schema_version: 1,
                sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
            },
            MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
        )
        .unwrap();
        let recall = memory.recall_frame(&step.observation).unwrap();
        let (frame, recall) = recall.finalize(step.observation.clone()).unwrap();
        let receptor = step
            .body_before
            .neural_receptor_frame(&demo.body_phenotype)
            .unwrap();
        let effects = NeuralReceptorEffects::from_frame(
            &receptor,
            &NeuralReceptorPhenotype::compile(&phenotype).unwrap(),
        )
        .unwrap();
        let upload = session
            .prepare_memory_context_upload(handle, &frame, &recall)
            .unwrap()
            .bind_neural_receptor_effects(effects)
            .unwrap();
        let input =
            alife_gpu_backend::GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload)
                .unwrap();
        let batch = alife_gpu_backend::GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
        let requested: Vec<_> = frame
            .candidates()
            .iter()
            .filter(|c| {
                phenotype
                    .candidate_decoder()
                    .families()
                    .iter()
                    .any(|f| f.family() == c.family && f.decoder_synapse_count() != 0)
            })
            .map(|c| c.candidate_index)
            .collect();
        assert!(requested.len() <= 8);
        let tick = session
            .tick_memory_batch_with_selector_diagnostics(&batch, &requested)
            .unwrap()
            .remove(0);
        let receipt = tick.selector_diagnostic.as_ref().unwrap();
        std::fs::write(
            root.join(format!("production-frame-{case}.json")),
            serde_json::to_vec_pretty(&(frame.clone(), receipt)).unwrap(),
        )
        .unwrap();
        let teacher = step.teacher_candidate_index;
        let mut pairs = Vec::new();
        for candidate in frame.candidates() {
            if candidate.candidate_index != teacher {
                pairs.push((teacher, candidate.candidate_index));
            }
        }
        let index = |family| {
            frame
                .candidates()
                .iter()
                .find(|c| c.family == family)
                .unwrap()
                .candidate_index
        };
        let approaching =
            frame.candidates()[usize::from(teacher)].family == CandidateActionFamily::Approach;
        // Independent locomotion and manipulation slots also need compatible
        // rankings. No claim about a learned inspection sequence is made here.
        pairs.push((
            index(CandidateActionFamily::Approach),
            index(CandidateActionFamily::Avoid),
        ));
        pairs.push(if approaching {
            (
                index(CandidateActionFamily::Contact),
                index(CandidateActionFamily::Ingest),
            )
        } else {
            (
                index(CandidateActionFamily::Ingest),
                index(CandidateActionFamily::Contact),
            )
        });
        pairs.sort_unstable();
        pairs.dedup();
        for (positive, negative) in pairs {
            examples.push(
                alife_training::ProductionReadoutExample::from_diagnostic(
                    &phenotype, &frame, receipt, positive, negative,
                )
                .unwrap(),
            );
        }
        // No teacher-forced recurrent history or fabricated learning update.
        // Each sample is a fresh first frame; the proposed action is discarded.
        session
            .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
            .unwrap();
        session.remove_brain(handle).unwrap();
    }
    drop(session);
    let mut trainer = alife_training::Nano512ReadoutTrainer::new_required(
        baseline,
        control.clone(),
        &examples,
        0.05,
    )
    .unwrap();
    let before = trainer.loss().unwrap();
    trainer.train_steps(256).unwrap();
    let after = trainer.loss().unwrap();
    let trained = trainer
        .export_candidate(TrainingStageManifest::new(2, 8, 1))
        .unwrap();
    std::fs::write(
        root.join("trained.alife-foundation"),
        trained.encode_canonical().unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("control.alife-foundation"),
        control.encode_canonical().unwrap(),
    )
    .unwrap();
    println!(
        "sampling_loss={before}->{after}; trained={:?}; evidence={}",
        trained.digest(),
        root.display()
    );
    drop(trainer);
    let held_out = [
        Vec3f::new(-3.0, 0.0, 1.5),
        Vec3f::new(0.7, 0.0, -0.2),
        Vec3f::new(2.7, 0.0, -1.0),
    ];
    let mut results = Vec::new();
    for (name, candidate) in [("control", &control), ("trained", &trained)] {
        for (case, position) in held_out.into_iter().enumerate() {
            results.push((
                name,
                case,
                sampling_world_outcome(
                    candidate,
                    95000 + case as u64,
                    position,
                    &root.join(format!("{name}-{case}")),
                ),
            ));
        }
    }
    println!("held_out_actual_consumption={results:?}");
    std::fs::write(
        root.join("outcomes.json"),
        serde_json::to_vec_pretty(&results).unwrap(),
    )
    .unwrap();
    assert!(
        results
            .iter()
            .filter(|(name, _, _)| *name == "trained")
            .all(|(_, _, consumed)| *consumed),
        "held-out autonomous sampling remains unproven"
    );
}

fn sampling_world_outcome(
    candidate: &FoundationWeightAsset,
    seed: u64,
    position: Vec3f,
    root: &std::path::Path,
) -> bool {
    std::fs::create_dir_all(root).unwrap();
    let organism = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", organism, Vec3f::ZERO)
        .food("object", position, 1.0)
        .build()
        .unwrap();
    let entity = world.organism_entity_ids()[0].1;
    let food = world.entity_id("object").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(
            FoundationId::N512_V1.raw(),
            1,
            FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap()
    .with_nano512_readout_candidate(candidate)
    .unwrap();
    let body = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(organism, entity, genome, body, Tick::ZERO).unwrap(),
        )
        .unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
        backend,
        world,
        seed,
        BrainScaleTier::Nano512,
        SensorProfile::GroundedObjectSlotsV1,
        alife_archive::LineageLibraryConfig::profile_default(root.join("lineage")),
        "sampling-probe",
        ArchiveLearnedCapturePolicy::GeneticOnly,
    )
    .unwrap();
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    let base = candidate_base_save(&mut runtime, &assets);
    runtime
        .attach_durable_checkpoint_boundary(root.join("world.json"), &assets, base)
        .unwrap();
    let started = std::time::Instant::now();
    let mut consumed = false;
    let mut evidence = Vec::new();
    while runtime.world_snapshot().tick().raw() < 16 && started.elapsed().as_secs() < 120 {
        let before = runtime.world_snapshot();
        match runtime.tick_outcome().unwrap() {
            alife_game_app::GpuLiveTickOutcome::NoProgress(_) => {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            alife_game_app::GpuLiveTickOutcome::Progressed(summaries) => {
                let after = runtime.world_snapshot();
                let before_energy = before
                    .organism_registry()
                    .get(organism)
                    .unwrap()
                    .biochemistry()
                    .body
                    .energy;
                let after_energy = after
                    .organism_registry()
                    .get(organism)
                    .unwrap()
                    .biochemistry()
                    .body
                    .energy;
                let p = after.entity(entity).unwrap().position;
                let distance = ((p.x - position.x).powi(2)
                    + (p.y - position.y).powi(2)
                    + (p.z - position.z).powi(2))
                .sqrt();
                consumed |= after.entity(food).unwrap().consumed;
                println!("case={}; tick={:?}; position={p:?}; distance={distance}; energy={before_energy}->{after_energy}; consumed={consumed}; summaries={summaries:?}", root.display(), after.tick());
                evidence.push(serde_json::json!({"tick": after.tick(), "position": p, "distance": distance, "energy_before": before_energy, "energy_after": after_energy, "consumed": consumed, "patches": runtime.sealed_patches()}));
                if consumed {
                    break;
                }
            }
        }
    }
    std::fs::write(
        root.join("ticks.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    consumed
}

fn run_case(use_candidate: bool) {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let mut weights = source.weights().to_vec();
    let changed = baseline.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)).unwrap();
    weights[changed] += 0.125;
    let candidate = FoundationWeightAsset::from_trained_weights(
        &baseline,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/live-candidate-{use_candidate}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let asset_path = root.join("untrained-admission-probe.alife-foundation");
    std::fs::write(&asset_path, candidate.encode_canonical().unwrap()).unwrap();
    let candidate =
        FoundationWeightAsset::decode_canonical(&std::fs::read(&asset_path).unwrap()).unwrap();
    let (expected, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&candidate).unwrap();
    let organism = OrganismId(940512);
    let seed = 940513;
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("candidate", organism, Vec3f::ZERO)
        .food("visible-object", Vec3f::new(2.0, 0.0, 0.0), 1.0)
        .build()
        .unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(
            FoundationId::N512_V1.raw(),
            1,
            FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap();
    let genome = if use_candidate {
        genome.with_nano512_readout_candidate(&candidate).unwrap()
    } else {
        genome
    };
    let body = genome.express().unwrap();
    let neural = if use_candidate {
        expected.clone()
    } else {
        N512FounderFoundationProjection::compile(&body, profile, &source)
            .unwrap()
            .compiled_phenotype()
            .clone()
    };
    let mut routes = neural
        .projections()
        .iter()
        .map(|projection| projection.route_index())
        .collect::<Vec<_>>();
    routes.sort_unstable();
    routes.dedup();
    let capacity = BrainCapacityClass::n512();
    let work = alife_gpu_backend::derive_executed_work(
        &neural,
        neural.microstep_count(),
        &routes,
        u32::from(capacity.execution().max_candidates()),
        u32::from(capacity.execution().max_memory_context_records()),
    )
    .unwrap();
    let cost = BrainAtpCostModel::production_v1();
    println!(
        "use_candidate={use_candidate}; full_work_upper_bound={work:?}; debit_upper_bound_q16={}",
        cost.q24_to_atp_q16_round_half_up(cost.neural_cost_q24(&work).unwrap())
            .unwrap()
    );
    let chemistry = BiochemistryState::new(&body, Tick::ZERO).unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(
                organism,
                world.organism_entity_ids()[0].1,
                genome,
                body,
                chemistry,
                Tick::ZERO,
            )
            .unwrap(),
        )
        .unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    println!(
        "hardware={:?}; candidate={:?}",
        backend.hardware_receipt(),
        candidate.digest()
    );
    let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
        backend,
        world,
        seed,
        BrainScaleTier::Nano512,
        profile,
        alife_archive::LineageLibraryConfig::profile_default(root.join("lineage")),
        "candidate-admission-probe",
        ArchiveLearnedCapturePolicy::GeneticOnly,
    )
    .expect("world-bound candidate must enter the same game birth path");
    assert!(runtime.archive_birth_manifest(organism).is_some());
    let asset_root = root.join("durable-assets");
    std::fs::create_dir_all(&asset_root).unwrap();
    let base = candidate_base_save(&mut runtime, &asset_root);
    runtime
        .attach_durable_checkpoint_boundary(root.join("live.json"), &asset_root, base)
        .unwrap();
    let store = GpuCheckpointAssetStore::new(root.join("checkpoint")).unwrap();
    ordinary_learning_tick(&mut runtime, organism, &store);
    let fast = runtime.active_fast_weights_for_test(organism).unwrap();
    let lifetime = runtime.active_lifetime_weights_for_test(organism).unwrap();
    println!(
        "use_candidate={use_candidate}; checkpoint_fast_nonzero={}; checkpoint_lifetime_nonzero={}",
        fast.iter().filter(|value| **value != 0.0).count(),
        lifetime.iter().filter(|value| **value != 0.0).count()
    );
    let write = runtime.checkpoint_brain(organism, &store).unwrap();
    if use_candidate {
        assert_eq!(write.save_state.phenotype_hash, expected.phenotype_hash());
    }
    let admitted_hash = write.save_state.phenotype_hash;
    let world = runtime.world_snapshot();
    let mut manifest = AssetManifest::empty();
    alife_runtime::merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries)
        .unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let mut restored = GpuLiveBrainRuntime::restore_with_checkpoints(
        backend,
        world,
        seed,
        BrainScaleTier::Nano512,
        &store,
        &manifest,
        &[write.save_state],
    )
    .unwrap();
    let restored_base = candidate_base_save(&mut restored, &asset_root);
    restored
        .attach_durable_checkpoint_boundary(root.join("restored.json"), &asset_root, restored_base)
        .unwrap();
    assert_eq!(
        restored
            .active_fast_weights_for_test(organism)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        fast.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(
        restored
            .active_lifetime_weights_for_test(organism)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        lifetime.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(
        restored
            .checkpoint_brain(organism, &store)
            .unwrap()
            .save_state
            .phenotype_hash,
        admitted_hash
    );
    ordinary_learning_tick(&mut restored, organism, &store);
}

fn ordinary_learning_tick(
    runtime: &mut GpuLiveBrainRuntime,
    organism: OrganismId,
    store: &GpuCheckpointAssetStore,
) {
    let started = std::time::Instant::now();
    let initial_tick = runtime.world_snapshot().tick();
    let mut previous_phase = SleepPhase::Awake;
    let mut pending = false;
    while runtime.world_snapshot().tick().raw() - initial_tick.raw() < 128 {
        assert!(
            started.elapsed().as_secs() < 180,
            "ordinary cognition did not resume within three minutes"
        );
        let before = runtime.world_snapshot().tick();
        let ticks = match runtime.tick_outcome().unwrap() {
            alife_game_app::GpuLiveTickOutcome::Progressed(ticks) => ticks,
            alife_game_app::GpuLiveTickOutcome::NoProgress(reason) => {
                assert_eq!(runtime.world_snapshot().tick(), before);
                assert_eq!(
                    reason,
                    alife_game_app::GpuLiveNoProgressReason::CheckpointPublicationPending
                );
                if !pending {
                    println!("tick={before:?}; no_progress={reason:?}");
                }
                pending = true;
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
        };
        pending = false;
        let atp = runtime.brain_atp_q16_for_test(organism).unwrap();
        assert_eq!(ticks.len(), 1);
        let summary = &ticks[0];
        let world = runtime.world_snapshot();
        let seal = world
            .organism_registry()
            .get(organism)
            .unwrap()
            .sleep_seal();
        if seal.phase != previous_phase {
            let record = world.organism_registry().get(organism).unwrap();
            let input = record.authoritative_sleep_input().unwrap();
            let recovery = ChemistryModulation::recovery_triggers(
                &input.homeostasis,
                HomeostaticParameters::reference(),
            )
            .unwrap();
            let sleep = runtime
                .checkpoint_brain(organism, store)
                .unwrap()
                .save_state
                .sleep;
            println!("phase_change={sleep:?}; body_input={input:?}; recovery={recovery:?}");
            previous_phase = seal.phase;
        }
        println!(
            "tick={:?}; sleep={seal:?}; atp_q16={atp}; patch={}; learning={}; action={:?}",
            summary.tick_after,
            summary.patch_sealed,
            summary.learning_updates,
            summary.selected_action_kind
        );
        if summary.patch_sealed {
            assert_eq!(summary.learning_updates, 1);
            return;
        }
    }
    panic!("128 normal scheduler ticks did not reach ordinary cognition");
}

fn candidate_base_save(
    runtime: &mut GpuLiveBrainRuntime,
    asset_root: &std::path::Path,
) -> PortableSaveFile {
    let world = runtime.world_snapshot();
    let creatures = world
        .organism_registry()
        .iter()
        .map(|record| {
            let organism_id = record.organism_id();
            let biochemistry = record.biochemistry().clone();
            let genetic_fixed_digest = PortableAssetDigest::for_bytes(
                &serde_json::to_vec(record.phenotype()).expect("canonical phenotype serializes"),
            )
            .0;
            CreatureSaveState {
                organism_id,
                genome_id: record.genome().id,
                brain_class: BrainScaleTier::Nano512,
                development_tick: biochemistry.development.last_update_tick,
                appearance: CreatureAppearanceGenome::default(),
                mind: CreatureMindSaveSummary {
                    tick: biochemistry.tick,
                    homeostasis: biochemistry.homeostasis,
                    memory_record_count: 0,
                    memory_source_ids: Vec::new(),
                    concept_count: 0,
                    edge_count: 0,
                    simplex_count: 0,
                    unresolved_gap_count: 0,
                    sleep_state_label: "awake".to_string(),
                    diagnostics: Vec::new(),
                },
                weights: WeightLayerSaveSummary {
                    generated_weight_asset_id: None,
                    genetic_fixed_digest,
                    genetic_layer_mutable: false,
                    lifetime_consolidated_entries: 0,
                    h_operational_entries: 0,
                    h_shadow_entries: 0,
                },
                learning: LearningTraceSaveSummary {
                    lifetime_learning_enabled: true,
                    lamarckian_mode_enabled: false,
                    last_consolidated_tick: None,
                },
                composite_genetics: None,
                lifetime_state_asset: None,
                gpu_brain: None,
            }
        })
        .collect();
    let mut config = RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let mut save = PortableSaveFile::from_headless_world(
        "candidate-admission-probe",
        &world,
        config,
        AssetManifest::empty(),
        creatures,
    )
    .expect("canonical player-loop base save");
    let store = GpuCheckpointAssetStore::new(asset_root).unwrap();
    for creature in &mut save.creatures {
        let write = runtime
            .checkpoint_brain(creature.organism_id, &store)
            .unwrap();
        alife_runtime::merge_gpu_checkpoint_manifest_entries(
            &mut save.assets,
            write.manifest_entries,
        )
        .unwrap();
        creature.gpu_brain = Some(write.save_state);
    }
    save.validate_with_asset_root(asset_root).unwrap();
    save
}
