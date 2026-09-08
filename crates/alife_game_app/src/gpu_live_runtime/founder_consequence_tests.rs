//! Manual first-consequence experiment. The failed 32-tick probe remains intact.

use super::*;
use alife_core::*;
use alife_gpu_backend::GpuRuntimeProfile;
use alife_runtime::GpuCheckpointAssetStore;
use alife_world::{
    persistence::*, CreatureAppearanceGenome, HeadlessScenarioBuilder, WorldOrganismRecord,
};

#[derive(Debug)]
struct PrefixStep {
    patch: ExperiencePatch,
    receptors: NeuralReceptorFrame,
    organism_before: serde_json::Value,
    checkpoint: alife_runtime::GpuBrainCheckpointWrite,
    assets: BTreeMap<String, Vec<u8>>,
    fast_before: Vec<u32>,
    fast_after: Vec<u32>,
    activity: GpuActivityRuntimeSnapshot,
}

#[test]
#[ignore = "bounded manual causal probe; requires retained ALIFE_FOUNDER_CANDIDATE"]
fn first_food_consequence_with_exact_pressure_prefix() {
    let candidate = FoundationWeightAsset::decode_canonical(
        &std::fs::read(std::env::var("ALIFE_FOUNDER_CANDIDATE").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        candidate.digest().bytes(),
        &[
            14, 148, 152, 50, 97, 32, 184, 207, 71, 123, 153, 74, 88, 1, 122, 247, 18, 136, 157,
            226, 80, 112, 127, 203, 223, 222, 92, 5, 5, 204, 232, 246,
        ]
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/first-consequence-{}",
        std::process::id(),
    ));
    assert!(!root.exists(), "evidence directory must be fresh");
    std::fs::create_dir_all(&root).unwrap();
    let recorded = first_meal_life(&candidate, true, &root.join("cyan-nutritious"), None);
    let pressures = recorded
        .iter()
        .map(|step| step.activity.pressure.unwrap())
        .collect();
    let replayed = first_meal_life(
        &candidate,
        false,
        &root.join("amber-nutritious"),
        Some(pressures),
    );
    // Both complete raw traces are on disk before the cross-life assertions.
    assert_eq!(
        recorded.len(),
        2,
        "recording did not reach the expected first meal"
    );
    assert_eq!(
        replayed.len(),
        2,
        "replay did not reach the expected first meal"
    );
    for (index, (a, b)) in recorded.iter().zip(&replayed).enumerate() {
        assert!(
            a.assets == b.assets,
            "incoming causal checkpoint assets differ at step {index}"
        );
        assert_eq!(a.organism_before, b.organism_before);
        assert_eq!(a.fast_before, b.fast_before);
        assert_eq!(a.receptors, b.receptors);
        assert_eq!(a.activity.pressure, b.activity.pressure);
        assert_eq!(a.activity.throttle, b.activity.throttle);
        assert_eq!(a.activity.work, b.activity.work);
        assert_eq!(a.activity.brain_atp_q16, b.activity.brain_atp_q16);
        assert_eq!(
            a.activity.next_sequence_cursor,
            b.activity.next_sequence_cursor
        );
        assert_eq!(
            a.activity.last_world_atp_tick,
            b.activity.last_world_atp_tick
        );
        assert_eq!(
            a.patch.pre_action().perception(),
            b.patch.pre_action().perception()
        );
        assert_eq!(
            a.patch.decision().neural_evidence().unwrap(),
            b.patch.decision().neural_evidence().unwrap()
        );
        let aj = serde_json::to_value(&a.patch).unwrap();
        let bj = serde_json::to_value(&b.patch).unwrap();
        // Sealing attaches the observed successor target to both pre_action and
        // decision. Its target_state and derived prediction error are outcomes,
        // so compare the actual pre-action fields explicitly, retaining all JSON.
        for field in [
            "abi_version",
            "organism_id",
            "sequence_id",
            "tick",
            "genome_id",
            "genome_schema_version",
            "development_state",
            "brain_evidence",
            "cognitive_context",
        ] {
            assert_eq!(
                aj["pre_action"][field], bj["pre_action"][field],
                "pre_action.{field}"
            );
        }
        for field in [
            "abi_version",
            "action_abi_version",
            "organism_id",
            "sequence_id",
            "decision_tick",
            "confidence",
            "evidence",
            "cognitive_work",
            "episodic_key",
            "selected_action",
            "selected_bundle",
        ] {
            assert_eq!(
                aj["decision"][field], bj["decision"][field],
                "decision.{field}"
            );
        }
        let a_prediction = a.patch.prediction_target().unwrap();
        let b_prediction = b.patch.prediction_target().unwrap();
        assert_eq!(a_prediction.source_state, b_prediction.source_state);
        assert_eq!(a_prediction.source_digest, b_prediction.source_digest);
        assert_eq!(a_prediction.motor_condition, b_prediction.motor_condition);
        if index == 0 {
            assert_eq!(a.patch, b.patch);
            assert_eq!(a.fast_after, b.fast_after);
            assert_eq!(a.checkpoint.save_state, b.checkpoint.save_state);
        }
    }
    let a = &recorded[1];
    let b = &replayed[1];
    assert_eq!(
        a.patch.outcome().physical.contact,
        PhysicalContactKind::Consumed
    );
    assert_eq!(
        b.patch.outcome().physical.contact,
        PhysicalContactKind::Consumed
    );
    assert_eq!(
        a.patch.outcome().physical.target_entity,
        b.patch.outcome().physical.target_entity
    );
    let energy_a = a
        .patch
        .outcome()
        .measured_physiology
        .as_ref()
        .unwrap()
        .energy_delta
        .raw();
    let energy_b = b
        .patch
        .outcome()
        .measured_physiology
        .as_ref()
        .unwrap()
        .energy_delta
        .raw();
    assert_ne!(energy_a.to_bits(), energy_b.to_bits());
    assert_ne!(
        a.fast_after, b.fast_after,
        "matched incoming state and work did not produce differing acquired fast state"
    );
    println!("first_consequence: matched_prefix=true; energy={energy_a} vs {energy_b}; changed_fast_entries={}; evidence={}",
        a.fast_after.iter().zip(&b.fast_after).filter(|(x,y)| x != y).count(), root.display());
}

fn first_meal_life(
    candidate: &FoundationWeightAsset,
    cyan_nutritious: bool,
    root: &Path,
    pressure_replay: Option<Vec<GpuPressureSample>>,
) -> Vec<PrefixStep> {
    let (mut runtime, assets) = paired_food_runtime(candidate, cyan_nutritious, root, false);
    let organism = OrganismId(1);
    let replaying = pressure_replay.is_some();
    if let Some(samples) = pressure_replay {
        runtime.install_recorded_pressure_replay(samples).unwrap();
    }
    let store = GpuCheckpointAssetStore::new(&assets).unwrap();
    let mut steps = Vec::new();
    let mut polls = 0_u32;
    let started = Instant::now();
    while runtime.world_snapshot().tick().raw() < 2 && started.elapsed().as_secs() < 120 {
        let before = runtime.world_snapshot();
        let record = before.organism_registry().get(organism).unwrap();
        let organism_before = serde_json::to_value(record).unwrap();
        let receptors = record
            .biochemistry()
            .neural_receptor_frame(record.phenotype())
            .unwrap();
        let fast_before = runtime
            .active_fast_weights_for_test(organism)
            .unwrap()
            .into_iter()
            .map(f32::to_bits)
            .collect::<Vec<_>>();
        let checkpoint = runtime.checkpoint_brain(organism, &store).unwrap();
        let incoming_assets: BTreeMap<_, _> = checkpoint
            .manifest_entries
            .iter()
            .map(|entry| {
                (
                    entry.provenance.clone().unwrap(),
                    std::fs::read(assets.join(&entry.relative_path)).unwrap(),
                )
            })
            .collect();
        assert_eq!(
            incoming_assets.len(),
            checkpoint.manifest_entries.len(),
            "duplicate checkpoint provenance"
        );
        match runtime.tick_outcome().unwrap() {
            GpuLiveTickOutcome::NoProgress(reason) => {
                polls += 1;
                std::fs::write(root.join("no-progress.json"), serde_json::to_vec_pretty(
                    &serde_json::json!({"count":polls,"world_tick":before.tick(),"reason":format!("{reason:?}")})
                ).unwrap()).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            GpuLiveTickOutcome::Progressed(summaries) => assert!(summaries[0].patch_sealed),
        }
        let patch = runtime.sealed_patches().last().unwrap().clone();
        let fast_after = runtime
            .active_fast_weights_for_test(organism)
            .unwrap()
            .into_iter()
            .map(f32::to_bits)
            .collect::<Vec<_>>();
        let activity = runtime.evidence_activity_snapshot(organism).unwrap();
        let learning = *runtime.last_learning_receipts().first().unwrap();
        let credit = OutcomeCreditPacket::from_sealed_patch(&patch)
            .unwrap()
            .with_biochemical_receptors(&receptors)
            .unwrap();
        let changed = fast_before
            .iter()
            .zip(&fast_after)
            .filter(|(a, b)| a != b)
            .count();
        let actual_work = activity.work.as_ref().unwrap();
        let effective_pressure = activity.pressure.unwrap();
        let backend = runtime.backend.backend();
        let own_heap_used = backend.admission_receipt().logical_committed_bytes;
        let own_heap_capacity = backend.runtime_budget().logical_neural_heap_budget_bytes;
        let own_pressure = GpuPressureSample::try_new(
            backend.activity_policy(),
            GpuPressureSampleInput {
                identity: effective_pressure.dispatch_identity(),
                source_dispatch_generation: effective_pressure.source_dispatch_generation,
                source_frame_digest: effective_pressure.source_frame_digest,
                completed_gpu_time_ns: effective_pressure.completed_gpu_time_ns,
                queue_depth: 0,
                logical_heap_used: own_heap_used,
                logical_heap_capacity: own_heap_capacity,
                brain_atp_remaining_q16: actual_work.atp_before_q16,
                brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
            },
        )
        .unwrap();
        let row = serde_json::json!({
            "tick": runtime.world_snapshot().tick(), "replaying": replaying, "no_progress_polls": polls,
            "organism_before": organism_before, "receptors": receptors, "patch": patch,
            "checkpoint_before": checkpoint.save_state, "checkpoint_manifest": checkpoint.manifest_entries,
            "fast_before": fast_before, "fast_after": fast_after,
            "effective_pressure": effective_pressure, "throttle": activity.throttle, "work": activity.work,
            "actual_completed_gpu_time_ns": activity.next_completed_gpu_time_ns,
            "own_heap_used": own_heap_used, "own_heap_capacity": own_heap_capacity,
            "brain_atp_after_q16": activity.brain_atp_q16, "credit_lanes": credit.modulator().frame().lanes(),
            "learning": {"dispatch":learning.dispatch_generation,"sequence":learning.sequence_id,
                "input_fast":learning.input_fast_generation,"output_fast":learning.output_fast_generation,
                "eligibility":learning.output_eligibility_generation,"replay":learning.replay_journal_generation,
                "changed":changed,"max_abs_delta":learning.max_abs_delta},
        });
        std::fs::write(
            root.join(format!("tick-{}.json", steps.len() + 1)),
            serde_json::to_vec_pretty(&row).unwrap(),
        )
        .unwrap();
        assert_eq!(
            effective_pressure, own_pressure,
            "replay pressure must retain this resident's actual ATP and heap"
        );
        // Replay may replace measured duration, never the resident's ATP.
        // MAX is exactly 65535, so Q16 fraction equals this actual pre-work ATP.
        assert_eq!(
            effective_pressure.brain_atp_fraction_q16,
            actual_work.atp_before_q16
        );
        assert_eq!(changed as u32, learning.fast_weights_changed);
        assert_eq!(
            learning.dispatch_generation,
            patch
                .decision()
                .neural_evidence()
                .unwrap()
                .dispatch_generation
        );
        let consumed = patch.outcome().physical.contact == PhysicalContactKind::Consumed;
        steps.push(PrefixStep {
            patch,
            receptors,
            organism_before,
            checkpoint,
            assets: incoming_assets,
            fast_before,
            fast_after,
            activity,
        });
        if consumed {
            break;
        }
    }
    if replaying {
        assert_eq!(runtime.recorded_pressure_replay_remaining(), 0);
    }
    println!(
        "first_meal_life: cyan_nutritious={cyan_nutritious}; ticks={}; seconds={}; polls={polls}",
        runtime.world_snapshot().tick().raw(),
        started.elapsed().as_secs_f32()
    );
    steps
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

pub(super) fn paired_food_runtime(
    candidate: &FoundationWeightAsset,
    cyan_nutritious: bool,
    root: &Path,
    innate_nociception: bool,
) -> (GpuLiveBrainRuntime, PathBuf) {
    paired_food_runtime_with_action_credit(
        candidate,
        cyan_nutritious,
        root,
        innate_nociception,
        None,
    )
}

pub(super) fn paired_food_runtime_with_action_credit(
    candidate: &FoundationWeightAsset,
    cyan_nutritious: bool,
    root: &Path,
    innate_nociception: bool,
    action_credit: Option<&Nano512ActionCreditCandidateV2>,
) -> (GpuLiveBrainRuntime, PathBuf) {
    std::fs::create_dir_all(root).unwrap();
    let seed = 96001;
    let organism = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", organism, Vec3f::ZERO)
        .terrain_zone(
            1,
            "feeding-area",
            alife_world::TerrainZoneKind::Meadow,
            Vec3f::ZERO,
            4.0,
            0.0,
            0.0,
        )
        .build()
        .unwrap();
    let mut foods = Vec::new();
    for (cyan, label, x, color) in [
        (true, "cyan", -0.5, [0.0, 1.0, 1.0]),
        (false, "amber", 0.5, [1.0, 0.55, 0.1]),
    ] {
        let nutritious = cyan == cyan_nutritious;
        let food = world
            .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                label: label.to_string(),
                kind: alife_world::WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(x, 0.0, 0.0),
                nutrition: if nutritious { 1.0 } else { 0.0 },
                hazard_pain: if nutritious { 0.0 } else { 0.15 },
                radius: 0.2,
                token_id: None,
            })
            .unwrap();
        world
            .set_grounded_physical_properties(
                food,
                alife_world::GroundedPhysicalProperties {
                    velocity: Vec3f::ZERO,
                    color,
                    material: [0.5; 3],
                    shape: [0.5; 3],
                    chemical: [0.0; 3],
                    surface_temperature: 0.0,
                    terrain: [0.5; 2],
                },
            )
            .unwrap();
        world
            .track_resource_lifecycle(food, alife_world::EcologyZoneId(1), 1, 1000)
            .unwrap();
        foods.push(food);
    }
    let entity = world.organism_entity_ids()[0].1;
    let mut genome = CreatureGenome::early_mammal_founder(
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
    if let Some(configured) = action_credit {
        assert_eq!(configured.asset().unwrap(), *candidate);
        genome = genome
            .with_nano512_action_credit_candidate(configured.clone())
            .unwrap();
    }
    if innate_nociception {
        let graph = genome.chemistry.graph.expressed();
        let pain = graph
            .receptors()
            .iter()
            .find(|r| r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Pain))
            .unwrap()
            .source;
        let index = graph
            .emitters()
            .iter()
            .position(|e| e.source == BiochemicalSourceLocus::Damage && e.target == pain)
            .unwrap();
        for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .with_emitter_expression_floor(side, index, 1.0)
                .unwrap();
        }
        std::fs::write(
            root.join("genome.json"),
            serde_json::to_vec_pretty(&genome).unwrap(),
        )
        .unwrap();
    }
    let body = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(organism, entity, genome, body, Tick::ZERO).unwrap(),
        )
        .unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    println!(
        "paired_hardware={:?}; cyan_nutritious={cyan_nutritious}; candidate={:?}; foods={foods:?}",
        backend.hardware_receipt(),
        candidate.digest()
    );
    let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
        backend,
        world,
        seed,
        BrainScaleTier::Nano512,
        SensorProfile::GroundedObjectSlotsV1,
        alife_archive::LineageLibraryConfig::profile_default(root.join("lineage")),
        "paired-food-consequence-probe",
        ArchiveLearnedCapturePolicy::GeneticOnly,
    )
    .unwrap();
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    let base = candidate_base_save(&mut runtime, &assets);
    runtime
        .attach_durable_checkpoint_boundary(root.join("world.json"), &assets, base)
        .unwrap();
    (runtime, assets)
}
