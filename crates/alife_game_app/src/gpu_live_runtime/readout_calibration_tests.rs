//! One opt-in inherited calibration experiment. No normal runtime behavior changes.
use super::*;
use alife_core::*;
use alife_gpu_backend::{GpuRuntimeProfile, GpuSelectorDiagnosticReceipt};
use alife_training::{Nano512ReadoutTrainer, ProductionReadoutExample};
use alife_world::{HeadlessScenarioBuilder, WorldOrganismRecord};

const ORGANISM: OrganismId = OrganismId(1);
const STEPS: u32 = 256;
const RATE: f32 = 0.05;
const MARGIN_LIMIT: f32 = 0.02;

fn save(root: &Path, name: &str, value: &impl serde::Serialize) {
    write_capture(&root.join(name), &serde_json::to_value(value).unwrap());
}

fn world_evidence(world: &HeadlessWorld) -> serde_json::Value {
    // Reuse the world's canonical serializable projection. This is evidence,
    // not a complete creature checkpoint; live cases use the durable boundary.
    let snapshot = PortableSaveFile::from_headless_world(
        "calibration-evidence",
        world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    serde_json::json!({"world":snapshot.world,
        "signature":world.canonical_signature_digest().unwrap(),
        "organism":world.organism_registry().get(ORGANISM).unwrap()})
}

fn founder(seed: u64, asset: &FoundationWeightAsset) -> CreatureGenome {
    let configured = Nano512ActionCreditCandidateV2::new(
        asset,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
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
    .with_nano512_action_credit_candidate(configured)
    .unwrap();
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
    genome
}

fn register(world: &mut HeadlessWorld, asset: &FoundationWeightAsset) {
    let genome = founder(world.seed(), asset);
    let entity = world.organism_entity_ids()[0].1;
    let body = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(ORGANISM, entity, genome, body, Tick::ZERO).unwrap(),
        )
        .unwrap();
}

fn paired_world(
    asset: &FoundationWeightAsset,
    seed: u64,
    angle: f32,
    swap: bool,
    held: bool,
) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", ORGANISM, Vec3f::ZERO)
        .build()
        .unwrap();
    let colors = [[0.0, 1.0, 1.0], [1.0, 0.55, 0.1]];
    let radii = if held { [0.6, -1.1] } else { [0.5, -1.0] };
    for (i, radius) in radii.into_iter().enumerate() {
        let food = world
            .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                label: format!("unknown-{i}"),
                kind: alife_world::WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(radius * angle.cos(), 0.0, radius * angle.sin()),
                nutrition: 1.0,
                hazard_pain: 0.0,
                radius: 0.2,
                token_id: None,
            })
            .unwrap();
        world
            .set_grounded_physical_properties(
                food,
                alife_world::GroundedPhysicalProperties {
                    velocity: Vec3f::ZERO,
                    color: colors[i ^ usize::from(swap)],
                    material: [0.5; 3],
                    shape: [0.5; 3],
                    chemical: [0.0; 3],
                    surface_temperature: 0.0,
                    terrain: [0.5; 2],
                },
            )
            .unwrap();
    }
    register(&mut world, asset);
    world
}

// These are fresh production GPU dispatches made here, not loaded diagnostics.
// V1 and V2 differ in outcome credit, which is never applied to these first frames.
fn capture(
    session: &mut GpuAuthoritativeSession,
    phenotype: &BrainPhenotype,
    draft: PerceptionFrameDraft,
    chemistry: &BiochemistryState,
    body: &CreaturePhenotype,
    root: &Path,
    name: &str,
) -> (PerceptionFrame, GpuSelectorDiagnosticReceipt) {
    let handle = session.insert_brain(ORGANISM, phenotype.clone()).unwrap();
    let memory = MemorySidecarState::new_profiled(
        ORGANISM,
        SensorProfileIdentity {
            profile_id: SensorProfile::GroundedObjectSlotsV1.into(),
            profile_schema_version: 1,
            sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
        },
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let (frame, recall) = memory
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let receptors = chemistry.neural_receptor_frame(body).unwrap();
    let effects = NeuralReceptorEffects::from_frame(
        &receptors,
        &NeuralReceptorPhenotype::compile(phenotype).unwrap(),
    )
    .unwrap();
    let upload = session
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap()
        .bind_neural_receptor_effects(effects)
        .unwrap();
    let input = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
    // Two food frames have ten action-bearing rows. Only Ingest rows are needed
    // there; single-food competence frames request all five decoder families.
    let ingest_count = frame
        .candidates()
        .iter()
        .filter(|c| c.family == CandidateActionFamily::Ingest)
        .count();
    let requested = frame
        .candidates()
        .iter()
        .filter(|c| {
            if ingest_count == 2 {
                c.family == CandidateActionFamily::Ingest
            } else {
                phenotype
                    .candidate_decoder()
                    .families()
                    .iter()
                    .any(|f| f.family() == c.family && f.decoder_synapse_count() != 0)
            }
        })
        .map(|c| c.candidate_index)
        .collect::<Vec<_>>();
    assert!(requested.len() <= 8);
    let mut ticks = session
        .tick_memory_batch_with_selector_diagnostics(&batch, &requested)
        .unwrap();
    let tick = ticks.remove(0);
    let receipt = tick.selector_diagnostic.unwrap();
    save(
        root,
        name,
        &serde_json::json!({"frame": frame, "selector": receipt,
        "receptors": receptors, "body": body, "chemistry": chemistry,
        "provenance": "fresh production GPU dispatch in this process; zero acquired state; no outcome applied"}),
    );
    session
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();
    session.remove_brain(handle).unwrap();
    (frame, receipt)
}

fn capture_pair(
    session: &mut GpuAuthoritativeSession,
    phenotype: &BrainPhenotype,
    world: &HeadlessWorld,
    root: &Path,
    name: &str,
) -> (PerceptionFrame, GpuSelectorDiagnosticReceipt, [u16; 2]) {
    // Sensing updates tracking caches. Give each readout the same fresh world.
    let mut world = world.clone();
    let record = world.organism_registry().get(ORGANISM).unwrap().clone();
    let draft = world
        .perception_frame_draft(
            ORGANISM,
            Tick::ZERO,
            SensorProfile::GroundedObjectSlotsV1,
            record.biochemistry().homeostasis,
        )
        .unwrap();
    let ids = draft
        .candidates()
        .iter()
        .filter(|c| c.family == CandidateActionFamily::Ingest)
        .map(|c| c.candidate_index)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    let mut legality = Vec::new();
    // Prove both are edible independently in copies of the same grounded world.
    // These commands never alter the source observation or any GPU state.
    for index in &ids {
        let mut copy = world.clone();
        let command = draft.candidates()[usize::from(*index)]
            .to_command(ORGANISM, Confidence::new(1.0).unwrap())
            .unwrap();
        let outcome = copy
            .apply_registered_neural_command(
                &command,
                world.organism_entity_ids()[0].1,
                Tick(1),
                None,
                false,
            )
            .unwrap();
        legality.push(outcome.action_result.execution.clone());
        assert!(outcome.action_result.execution.succeeded);
        assert_eq!(
            outcome.action_result.execution.physical.contact,
            PhysicalContactKind::Consumed
        );
    }
    save(root, &format!("{name}-world.json"), &world_evidence(&world));
    save(root, &format!("{name}-legality.json"), &legality);
    let (frame, receipt) = capture(
        session,
        phenotype,
        draft,
        record.biochemistry(),
        record.phenotype(),
        root,
        name,
    );
    (frame, receipt, [ids[0], ids[1]])
}

fn margin(receipt: &GpuSelectorDiagnosticReceipt, ids: [u16; 2]) -> f32 {
    (receipt.candidates[usize::from(ids[0])].final_logit.unwrap()
        - receipt.candidates[usize::from(ids[1])].final_logit.unwrap())
    .abs()
}

#[test]
#[ignore = "one bounded calibration candidate; requires retained ALIFE_FOUNDER_CANDIDATE"]
fn inherited_readout_calibration_once() {
    let bytes = std::fs::read(std::env::var("ALIFE_FOUNDER_CANDIDATE").unwrap()).unwrap();
    let initial = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        initial.digest().bytes(),
        &[
            14, 148, 152, 50, 97, 32, 184, 207, 71, 123, 153, 74, 88, 1, 122, 247, 18, 136, 157,
            226, 80, 112, 127, 203, 223, 222, 92, 5, 5, 204, 232, 246
        ]
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/readout-calibration-{}",
        std::process::id()
    ));
    assert!(!root.exists());
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("initial.alife-foundation"), bytes).unwrap();
    save(
        &root,
        "bounds.json",
        &serde_json::json!({"training_frames":16,
        "competence_frames":8,"calibration_frames":8,"weighted_pairs":96,
        "competence_pairs":48,"calibration_pairs":48,"steps":STEPS,"optimizer_rate":RATE,
        "held_out_calibration_cases":4,"margin_limit":MARGIN_LIMIT,
        "held_out_feeding_positions":[[-3.0,0.0,1.5],[0.7,0.0,-0.2],[2.7,0.0,-1.0]],
        "feeding_tick_cap_per_case":16,"first_meal_tick_cap":4,"default_promoted":false}),
    );
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let builtin = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &builtin)
            .unwrap()
            .into_runtime_parts();
    let (phenotype, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&initial).unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    save(&root, "hardware.json", &backend.hardware_receipt());
    let mut session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
    let mut examples = Vec::new();
    for (case, p) in [
        [0.5, 0.0],
        [-0.8, 0.0],
        [0.0, 0.7],
        [0.0, -0.6],
        [2.0, 0.0],
        [-4.0, 0.0],
        [0.0, 3.0],
        [0.0, -2.5],
    ]
    .into_iter()
    .enumerate()
    {
        let demo = alife_training::record_founder_sampling_demonstration(
            93000 + case as u64,
            Vec3f::new(p[0], 0.0, p[1]),
        )
        .unwrap();
        save(
            &root,
            &format!("competence-{case}-demonstration.json"),
            &demo,
        );
        let step = &demo.steps[0];
        let (frame, receipt) = capture(
            &mut session,
            &phenotype,
            step.observation.clone(),
            &step.body_before,
            &demo.body_phenotype,
            &root,
            &format!("competence-{case}.json"),
        );
        let index = |family| {
            frame
                .candidates()
                .iter()
                .find(|c| c.family == family)
                .unwrap()
                .candidate_index
        };
        let approaching = frame.candidates()[usize::from(step.teacher_candidate_index)].family
            == CandidateActionFamily::Approach;
        let pairs = [
            (
                index(CandidateActionFamily::Approach),
                index(CandidateActionFamily::Avoid),
            ),
            if approaching {
                (
                    index(CandidateActionFamily::Contact),
                    index(CandidateActionFamily::Ingest),
                )
            } else {
                (
                    index(CandidateActionFamily::Ingest),
                    index(CandidateActionFamily::Contact),
                )
            },
            (
                step.teacher_candidate_index,
                index(CandidateActionFamily::Rest),
            ),
        ];
        save(&root, &format!("competence-{case}-pairs.json"), &pairs);
        for (a, b) in pairs {
            let example =
                ProductionReadoutExample::from_diagnostic(&phenotype, &frame, &receipt, a, b)
                    .unwrap();
            examples.extend([example.clone(), example]);
        }
    }
    assert_eq!(examples.len(), 48);
    for case in 0..8 {
        let world = paired_world(
            &initial,
            97000 + case,
            (case / 2) as f32 * std::f32::consts::FRAC_PI_2,
            case % 2 != 0,
            false,
        );
        let (frame, receipt, [a, b]) = capture_pair(
            &mut session,
            &phenotype,
            &world,
            &root,
            &format!("calibration-{case}.json"),
        );
        for (positive, negative) in [(a, b), (b, a)] {
            let example = ProductionReadoutExample::from_diagnostic(
                &phenotype, &frame, &receipt, positive, negative,
            )
            .unwrap();
            examples.extend([example.clone(), example.clone(), example]);
        }
    }
    assert_eq!(examples.len(), 96);
    drop(session);
    let mut trainer =
        Nano512ReadoutTrainer::new_required(baseline, initial.clone(), &examples, RATE).unwrap();
    let loss_before = trainer.loss().unwrap();
    trainer.train_steps(STEPS).unwrap();
    let loss_after = trainer.loss().unwrap();
    let trained = trainer
        .export_candidate(TrainingStageManifest::new(3, 16, 1))
        .unwrap();
    std::fs::write(
        root.join("calibrated.alife-foundation"),
        trained.encode_canonical().unwrap(),
    )
    .unwrap();
    drop(trainer);
    let (after, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&trained).unwrap();
    for (i, (old, new)) in phenotype
        .synapses()
        .iter()
        .zip(after.synapses())
        .enumerate()
    {
        assert_eq!(old.alpha().to_bits(), new.alpha().to_bits());
        assert_eq!(old.receptor_index(), new.receptor_index());
        if !matches!(old.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate)
        {
            assert_eq!(
                initial.weights()[i].to_bits(),
                trained.weights()[i].to_bits()
            );
        }
    }
    assert_eq!(
        phenotype.plasticity_receptors(),
        after.plasticity_receptors()
    );
    save(&root, "initial-phenotype.json", &phenotype);
    save(&root, "calibrated-phenotype.json", &after);
    save(
        &root,
        "training.json",
        &serde_json::json!({"loss_before":loss_before,"loss_after":loss_after,
        "initial_digest":initial.digest(),"candidate_digest":trained.digest(),"frozen_coordinates_verified":true}),
    );
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let mut session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
    let mut held = Vec::new();
    for case in 0..4 {
        let angle = std::f32::consts::FRAC_PI_4 + (case / 2) as f32 * std::f32::consts::PI;
        // Identical body/world inputs for both neural readouts.
        let world = paired_world(&initial, 98000 + case, angle, case % 2 != 0, true);
        let mut scores = Vec::new();
        for (name, p) in [("initial", &phenotype), ("calibrated", &after)] {
            let (_, receipt, ids) = capture_pair(
                &mut session,
                p,
                &world,
                &root,
                &format!("held-{case}-{name}.json"),
            );
            scores.push(margin(&receipt, ids));
        }
        held.push(serde_json::json!({"case":case,"initial_margin":scores[0],"calibrated_margin":scores[1],"pass":scores[1]<=MARGIN_LIMIT}));
    }
    drop(session);
    let calibration_pass = held.iter().all(|r| r["pass"] == true);
    save(
        &root,
        "calibration-gate.json",
        &serde_json::json!({"cases":held,"pass":calibration_pass,
        "feeding_gate":"not yet run","live_meal_gate":"not yet run"}),
    );
    println!("calibration_evidence={}; calibration_pass={calibration_pass}; loss={loss_before}->{loss_after}",root.display());
    assert!(
        calibration_pass,
        "calibration failed; stop before competence/live tests or retuning"
    );
    let mut feeding = Vec::new();
    for (case, position) in [
        Vec3f::new(-3.0, 0.0, 1.5),
        Vec3f::new(0.7, 0.0, -0.2),
        Vec3f::new(2.7, 0.0, -1.0),
    ]
    .into_iter()
    .enumerate()
    {
        let result = feeding_case(
            &trained,
            95000 + case as u64,
            position,
            &root.join(format!("feeding-{case}")),
        );
        feeding.push(result);
        save(&root, &format!("feeding-gate-{case}.json"), &feeding);
        assert!(
            result,
            "feeding competence failed; stop before next case or retuning"
        );
    }
    first_meal(&trained, &root.join("first-meal"));
}

fn feeding_case(asset: &FoundationWeightAsset, seed: u64, position: Vec3f, root: &Path) -> bool {
    std::fs::create_dir_all(root).unwrap();
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", ORGANISM, Vec3f::ZERO)
        .food("object", position, 1.0)
        .build()
        .unwrap();
    register(&mut world, asset);
    let food = world.entity_id("object").unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    save(root, "hardware.json", &backend.hardware_receipt());
    let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
        backend,
        world,
        seed,
        BrainScaleTier::Nano512,
        SensorProfile::GroundedObjectSlotsV1,
        LineageLibraryConfig::profile_default(root.join("lineage")),
        "readout-calibration-feeding",
        ArchiveLearnedCapturePolicy::GeneticOnly,
    )
    .unwrap();
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    let base = super::super::founder_consequence_tests::candidate_base_save(&mut runtime, &assets);
    runtime
        .attach_durable_checkpoint_boundary(root.join("world.json"), &assets, base)
        .unwrap();
    let started = Instant::now();
    let mut polls = 0;
    while runtime.world_snapshot().tick().raw() < 16 && started.elapsed().as_secs() < 90 {
        match runtime.tick_outcome().unwrap() {
            GpuLiveTickOutcome::NoProgress(_) => {
                polls += 1;
                assert!(polls <= 400);
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            GpuLiveTickOutcome::Progressed(_) => {}
        }
        let world = runtime.world_snapshot();
        save(
            root,
            &format!("tick-{}.json", world.tick().raw()),
            &serde_json::json!({"state":world_evidence(&world),"patches":runtime.sealed_patches()}),
        );
        if world.entity(food).unwrap().consumed {
            return true;
        }
    }
    false
}

fn first_meal(asset: &FoundationWeightAsset, root: &Path) {
    let configured = Nano512ActionCreditCandidateV2::new(
        asset,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    let (mut runtime, _) =
        super::super::founder_consequence_tests::paired_food_runtime_with_action_credit(
            asset,
            true,
            &root.join("world"),
            true,
            Some(&configured),
        );
    save(root, "configured-candidate.json", &configured);
    SELECTOR_CAPTURE_ROOT.with(|capture| *capture.borrow_mut() = Some(root.to_path_buf()));
    let started = Instant::now();
    let mut polls = 0;
    let mut first = None;
    while runtime.world_snapshot().tick().raw() < 4 && started.elapsed().as_secs() < 75 {
        match runtime.tick_outcome().unwrap() {
            GpuLiveTickOutcome::NoProgress(_) => {
                polls += 1;
                assert!(polls <= 400);
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            GpuLiveTickOutcome::Progressed(rows) => assert!(rows[0].patch_sealed),
        }
        let tick = runtime.world_snapshot().tick();
        let patch = runtime.sealed_patches().last().unwrap();
        save(
            root,
            &format!("tick-{}-outcome.json", tick.raw()),
            &serde_json::json!({"patch":patch,
            "memory_updates":runtime.last_memory_update_receipts(),"memory_errors":format!("{:?}",runtime.last_memory_observation_errors())}),
        );
        let consumed = patch.outcome().physical.contact == PhysicalContactKind::Consumed;
        if consumed && first.is_none() {
            first = Some(tick.raw());
        }
        let handle = *runtime.handles.get(&ORGANISM.raw()).unwrap();
        capture_gpu_banks(
            &mut runtime.backend,
            handle,
            tick,
            &root.join(format!("tick-{}-after-gpu.json", tick.raw())),
        );
        if first.is_some_and(|meal| tick.raw() > meal) {
            break;
        }
    }
    SELECTOR_CAPTURE_ROOT.with(|capture| *capture.borrow_mut() = None);
    save(
        root,
        "receipt.json",
        &serde_json::json!({"first_meal_tick":first,"ticks":runtime.world_snapshot().tick(),
        "no_progress_polls":polls,"learned_avoidance":"requires interpretation; no preference claim"}),
    );
    assert!(
        first.is_some_and(|meal| runtime.world_snapshot().tick().raw() > meal),
        "meal plus next decision not observed within four ticks"
    );
}
