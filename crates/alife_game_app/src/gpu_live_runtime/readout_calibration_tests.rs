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
    // Grounded contact uses the object's actual radius (0.2), not the
    // privileged report's contact radius. Both distances remain ingestible.
    let radii = if held { [0.15, -1.1] } else { [0.1, -1.0] };
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

fn assert_sensory_coverage(
    features: &CandidateFeatureVector,
    expected_bearing: [f32; 2],
    expected_contact: f32,
) -> serde_json::Value {
    for (actual, expected) in features.0[..2].iter().copied().zip(expected_bearing) {
        assert!(
            actual.is_finite() && (actual - expected).abs() < 1e-5,
            "captured horizontal bearing {:?} does not match {expected_bearing:?}",
            &features.0[..2]
        );
    }
    assert_eq!(
        features.0[18], expected_contact,
        "captured physical contact coverage missing"
    );
    serde_json::json!({"bearing":[features.0[0],features.0[1]],
        "distance":features.0[2],"contact":features.0[18]})
}

fn assert_pair_coverage(
    frame: &PerceptionFrame,
    ids: [u16; 2],
    near_bearing: [f32; 2],
) -> serde_json::Value {
    let near = &frame.candidates()[usize::from(ids[0])];
    let far = &frame.candidates()[usize::from(ids[1])];
    assert!(near.features.0[2] < far.features.0[2]);
    serde_json::json!({
        "near":assert_sensory_coverage(&near.features,near_bearing,1.0),
        "far":assert_sensory_coverage(&far.features,[-near_bearing[0],-near_bearing[1]],0.0),
    })
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
        &serde_json::json!({"curriculum_revision":3,"bearing_plane":"XZ, Y-up",
        "calibration_object_radius":0.2,"calibration_distances":[0.1,1.0],
        "held_out_distances":[0.15,1.1],"coverage_gate":"captured cardinal bearings and contact 1/0 before optimizer",
        "training_frames":16,
        "competence_frames":8,"calibration_frames":8,"weighted_pairs":128,
        "competence_pairs":64,"calibration_pairs":64,"steps":STEPS,"optimizer_rate":RATE,
        "held_out_calibration_cases":4,"margin_limit":MARGIN_LIMIT,
        "calibration_role":"diagnostic_only","primary_gate":"actual feeding",
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
    let mut competence_coverage = Vec::new();
    let mut handoff_coverage = Vec::new();
    let mut calibration_coverage = Vec::new();
    // Match distance/contact across opposite bearings in each competence pair.
    for (case, p) in [
        [0.5, 0.0],
        [-0.5, 0.0],
        [0.0, 0.7],
        [0.0, -0.7],
        [2.0, 0.0],
        [-2.0, 0.0],
        [0.0, 3.0],
        [0.0, -3.0],
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
        assert_eq!(
            approaching,
            case >= 4,
            "competence label must follow matched contact class"
        );
        let expected_bearing = [[0.0, 1.0], [0.0, -1.0], [1.0, 0.0], [-1.0, 0.0]][case % 4];
        competence_coverage.push(assert_sensory_coverage(
            &frame.candidates()[usize::from(index(CandidateActionFamily::Ingest))].features,
            expected_bearing,
            if approaching { 0.0 } else { 1.0 },
        ));
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
            if approaching {
                (
                    index(CandidateActionFamily::Approach),
                    index(CandidateActionFamily::Ingest),
                )
            } else {
                (
                    index(CandidateActionFamily::Ingest),
                    index(CandidateActionFamily::Approach),
                )
            },
        ];
        let contact = frame.candidates()[usize::from(index(CandidateActionFamily::Ingest))]
            .features
            .0[18];
        let ingest = index(CandidateActionFamily::Ingest);
        let approach = index(CandidateActionFamily::Approach);
        let expected_handoff = if contact == 1.0 {
            (ingest, approach)
        } else {
            assert_eq!(contact, 0.0);
            (approach, ingest)
        };
        assert!(
            pairs.contains(&expected_handoff),
            "every competence frame must teach the contact-derived global handoff"
        );
        handoff_coverage.push(serde_json::json!({
            "case":case,"contact":contact,"preferred":expected_handoff.0,
            "rejected":expected_handoff.1,"pass":true,
        }));
        save(&root, &format!("competence-{case}-pairs.json"), &pairs);
        for (a, b) in pairs {
            let example =
                ProductionReadoutExample::from_diagnostic(&phenotype, &frame, &receipt, a, b)
                    .unwrap();
            examples.extend([example.clone(), example]);
        }
    }
    assert_eq!(examples.len(), 64);
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
        let expected_bearing =
            [[0.0, 1.0], [1.0, 0.0], [0.0, -1.0], [-1.0, 0.0]][case as usize / 2];
        calibration_coverage.push(assert_pair_coverage(&frame, [a, b], expected_bearing));
        for (positive, negative) in [(a, b), (b, a)] {
            let example = ProductionReadoutExample::from_diagnostic(
                &phenotype, &frame, &receipt, positive, negative,
            )
            .unwrap();
            examples.extend([example.clone(), example.clone(), example.clone(), example]);
        }
    }
    assert_eq!(examples.len(), 128);
    drop(session);
    assert_eq!(competence_coverage.len(), 8);
    assert_eq!(handoff_coverage.len(), 8);
    assert_eq!(calibration_coverage.len(), 8);
    save(
        &root,
        "training-coverage-gate.json",
        &serde_json::json!({
            "pass":true,"competence":competence_coverage,"calibration":calibration_coverage,
            "global_handoff":handoff_coverage,
            "source":"actual frames bound to the saved fresh GPU receipts; checked before optimizer creation",
        }),
    );
    let mut trainer =
        Nano512ReadoutTrainer::new_required(baseline, initial.clone(), &examples, RATE).unwrap();
    let loss_before = trainer.loss().unwrap();
    trainer.train_steps(STEPS).unwrap();
    let loss_after = trainer.loss().unwrap();
    let trained = trainer
        .export_candidate(TrainingStageManifest::new(4, 16, 1))
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
            let (frame, receipt, ids) = capture_pair(
                &mut session,
                p,
                &world,
                &root,
                &format!("held-{case}-{name}.json"),
            );
            let diagonal = std::f32::consts::FRAC_1_SQRT_2;
            let expected_bearing = if case < 2 {
                [diagonal, diagonal]
            } else {
                [-diagonal, -diagonal]
            };
            let coverage = assert_pair_coverage(&frame, ids, expected_bearing);
            save(
                &root,
                &format!("held-{case}-{name}-coverage.json"),
                &coverage,
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
        &serde_json::json!({"cases":held,"pass":calibration_pass,"role":"diagnostic_only",
        "feeding_gate":"not yet run","live_meal_gate":"not yet run"}),
    );
    println!("calibration_evidence={}; calibration_pass={calibration_pass}; loss={loss_before}->{loss_after}",root.display());
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

#[test]
#[ignore = "one behavioral assessment; requires ALIFE_BEHAVIOR_CANDIDATE from corrected run 23928"]
fn saved_calibration_candidate_behavior_once() {
    let bytes = std::fs::read(std::env::var("ALIFE_BEHAVIOR_CANDIDATE").unwrap()).unwrap();
    let candidate = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        candidate.digest().bytes(),
        &[
            153, 191, 252, 5, 245, 201, 169, 20, 96, 179, 146, 80, 193, 153, 202, 153, 100, 125,
            83, 63, 187, 111, 59, 222, 179, 83, 92, 147, 205, 82, 191, 157,
        ]
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/saved-candidate-behavior-{}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve previous behavioral evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("candidate.alife-foundation"), bytes).unwrap();
    let configured = Nano512ActionCreditCandidateV2::new(
        &candidate,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    save(&root, "configured-candidate.json", &configured);
    save(&root, "neural-phenotype.json", &phenotype);
    save(&root, "compiler-inputs.json", &inputs);
    save(
        &root,
        "bounds.json",
        &serde_json::json!({
            "assessment":"distinct behavioral diagnostic, not a calibration rerun",
            "source_calibration_run":23928,"source_calibration_gate":"FAIL at unchanged 0.02",
            "candidate_digest":candidate.digest(),"optimizer_steps":0,
            "positions":[[-3.0,0.0,1.5],[0.7,0.0,-0.2],[2.7,0.0,-1.0]],
            "feeding_case_cap":3,"feeding_tick_cap_per_case":16,
            "feeding_wall_seconds_per_case":90,"first_meal_tick_cap":4,
            "first_meal_wall_seconds":75,"stop_on_first_feeding_failure":true,
            "default_promoted":false,
        }),
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
        let consumed = feeding_case(
            &candidate,
            95000 + case as u64,
            position,
            &root.join(format!("feeding-{case}")),
        );
        feeding.push(consumed);
        save(
            &root,
            &format!("feeding-gate-{case}.json"),
            &serde_json::json!({
                "completed_cases":feeding,"case":case,"consumed":consumed,
                "first_meal_diagnostic":"not yet run","optimizer_steps":0,
            }),
        );
        println!(
            "behavior_case={case}; consumed={consumed}; evidence={}",
            root.display()
        );
        assert!(
            consumed,
            "behavioral feeding failure; stop and inspect before any tuning"
        );
    }
    first_meal(&candidate, &root.join("first-meal"));
    save(
        &root,
        "completion.json",
        &serde_json::json!({
            "feeding_cases":feeding,"first_meal_plus_next_decision_observed":true,
            "source_calibration_gate":"still FAIL","optimizer_steps":0,
            "learned_avoidance":"not established by this bounded diagnostic",
        }),
    );
}

#[test]
#[ignore = "one paired preference assessment; requires saved revision-3 ALIFE_PREFERENCE_CANDIDATE"]
fn saved_handoff_candidate_preference_once() {
    saved_handoff_preference(
        ActionCandidateCreditProfileV1::SignedConsequences,
        "handoff-preference",
    );
}

#[test]
#[ignore = "one paired assessment of SignedChoiceReadouts; saved ALIFE_PREFERENCE_CANDIDATE"]
fn signed_choice_readouts_preference_once() {
    saved_handoff_preference(
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
        "signed-choice-preference",
    );
}

fn saved_handoff_preference(profile: ActionCandidateCreditProfileV1, evidence_prefix: &str) {
    let bytes = std::fs::read(std::env::var("ALIFE_PREFERENCE_CANDIDATE").unwrap()).unwrap();
    let candidate = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        candidate.digest().bytes(),
        &[
            40, 161, 51, 234, 158, 204, 236, 70, 61, 99, 211, 218, 78, 154, 50, 208, 109, 194, 193,
            113, 108, 113, 39, 239, 166, 78, 186, 38, 133, 91, 128, 250,
        ],
        "assessment must use the unchanged saved revision-3 candidate"
    );
    let configured = Nano512ActionCreditCandidateV2::new(&candidate, profile).unwrap();
    assert_eq!(
        configured.asset().unwrap().encode_canonical().unwrap(),
        bytes
    );
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    for (synapse, weight) in phenotype.synapses().iter().zip(candidate.weights()) {
        assert_eq!(synapse.genetic_weight().to_bits(), weight.to_bits());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/{evidence_prefix}-{}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve prior evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("candidate.alife-foundation"), bytes).unwrap();
    save(&root, "configured-candidate.json", &configured);
    save(&root, "neural-phenotype.json", &phenotype);
    save(&root, "compiler-inputs.json", &inputs);
    save(
        &root,
        "bounds.json",
        &serde_json::json!({
            "candidate_source":"readout-calibration-16188","optimizer_steps":0,
            "profile":profile,"outer_wall_seconds":360,
            "lives":2,"ticks_per_life":32,"wall_seconds_per_life":120,
            "assignments":["cyan nutritious, amber harmful","amber nutritious, cyan harmful"],
            "runtime":"existing run_food_life; identical initial candidate and physiology",
            "stop_on_crash_or_invariant_failure":true,"automatic_repeat":false,
            "late_choice_gate":"existing assert_choices after both lives; ticks 17-32",
            "genetic_feeding_competence":"prior revision-3 held-out 3/3 PASS",
            "calibration_diagnostic":"prior revision-3 FAIL preserved",
            "default_promoted":false,
        }),
    );
    let cyan_nutritious = run_food_life(
        &candidate,
        &phenotype,
        true,
        &root.join("cyan-nutritious"),
        Some(&configured),
    );
    let amber_nutritious = run_food_life(
        &candidate,
        &phenotype,
        false,
        &root.join("amber-nutritious"),
        Some(&configured),
    );
    save(
        &root,
        "outcomes.json",
        &[&cyan_nutritious, &amber_nutritious],
    );
    if profile == ActionCandidateCreditProfileV1::SignedChoiceReadouts {
        for life in ["cyan-nutritious", "amber-nutritious"] {
            save_choice_credit_rows(&phenotype, &root.join(life));
        }
    }
    println!(
        "paired_preference_evidence={}; cyan={cyan_nutritious:?}; amber={amber_nutritious:?}",
        root.display()
    );
    assert_choices(&cyan_nutritious, &amber_nutritious);
}

fn save_choice_credit_rows(phenotype: &BrainPhenotype, root: &Path) {
    let mut rows = Vec::new();
    for tick in 1..=32 {
        let path = root.join(format!("tick-{tick:02}.json"));
        if !path.exists() {
            break;
        }
        let row: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        if row["sealed"] != true {
            rows.push(serde_json::json!({"tick":tick,"sealed":false}));
            continue;
        }
        let family: CandidateActionFamily =
            serde_json::from_value(row["receptor_family"].clone()).unwrap();
        let lanes: [f32; 8] = serde_json::from_value(row["credit_lanes"].clone()).unwrap();
        let mut heads = Vec::new();
        for head in [
            DecoderHeadKind::ActionCandidate,
            DecoderHeadKind::MemoryContext,
        ] {
            let receptor = phenotype.synapses().iter().find_map(|s| match s.kind() {
                CompiledSynapseKind::Decoder(c) if c.head() == head && c.family() == family => {
                    Some(&phenotype.plasticity_receptors()[usize::from(s.receptor_index())])
                }
                _ => None,
            });
            let factor = receptor.map(|r| {
                let weights = *r.receptor_profile().weights();
                assert_eq!(weights, [0.0, -1.0, 1.0, -0.5, 0.2, 0.0, 0.5, -0.5]);
                weights.iter().zip(lanes).map(|(w, l)| w * l).sum::<f32>()
                    / weights.iter().map(|w| w.abs()).sum::<f32>()
            });
            heads.push(serde_json::json!({"head":head,"receptor":receptor,"third_factor":factor}));
        }
        rows.push(
            serde_json::json!({"tick":tick,"sealed":true,"family":family,
            "target":row["patch"]["outcome"]["physical"]["target_entity"],
            "contact":row["patch"]["outcome"]["physical"]["contact"],
            "credit_lanes":lanes,"heads":heads}),
        );
    }
    save(
        root,
        "choice-credit-rows.json",
        &serde_json::json!({"rows":rows,
        "source":"existing run_food_life sealed-outcome credit lanes and compiled dispatch phenotype; no injected credit"}),
    );
}

fn feeding_case(asset: &FoundationWeightAsset, seed: u64, position: Vec3f, root: &Path) -> bool {
    feeding_case_configured(asset, seed, position, root, None)
}

fn feeding_case_configured(
    asset: &FoundationWeightAsset,
    seed: u64,
    position: Vec3f,
    root: &Path,
    configured: Option<&Nano512ActionCreditCandidateV2>,
) -> bool {
    std::fs::create_dir_all(root).unwrap();
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", ORGANISM, Vec3f::ZERO)
        .food("object", position, 1.0)
        .build()
        .unwrap();
    let expected_phenotype = if let Some(configured) = configured {
        assert_eq!(configured.asset().unwrap(), *asset);
        let genome = founder(seed, asset)
            .with_nano512_action_credit_candidate(configured.clone())
            .unwrap();
        let body = genome.express().unwrap();
        let entity = world.organism_entity_ids()[0].1;
        world
            .register_organism_record(
                WorldOrganismRecord::newborn(ORGANISM, entity, genome.clone(), body, Tick::ZERO)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            world.organism_registry().get(ORGANISM).unwrap().genome(),
            &genome
        );
        save(root, "configured-candidate.json", configured);
        Some(
            PhenotypeCompiler::compile_nano512_action_credit_candidate(configured)
                .unwrap()
                .0,
        )
    } else {
        register(&mut world, asset);
        None
    };
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
        if let Some(expected) = &expected_phenotype {
            assert_eq!(
                runtime
                    .sealed_patches()
                    .last()
                    .unwrap()
                    .decision()
                    .neural_evidence()
                    .unwrap()
                    .phenotype_hash,
                expected.phenotype_hash()
            );
        }
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

#[test]
#[ignore = "one 5% inherited AC trial: three feeding cases, then paired preference; no optimizer"]
fn scaled_choice_readouts_once() {
    let bytes = std::fs::read(std::env::var("ALIFE_PREFERENCE_CANDIDATE").unwrap()).unwrap();
    let source = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        source.digest().bytes(),
        &[
            40, 161, 51, 234, 158, 204, 236, 70, 61, 99, 211, 218, 78, 154, 50, 208, 109, 194, 193,
            113, 108, 113, 39, 239, 166, 78, 186, 38, 133, 91, 128, 250,
        ]
    );
    let factor = 0.05_f32;
    let profile = ActionCandidateCreditProfileV1::SignedChoiceReadouts;
    let source_configured = Nano512ActionCreditCandidateV2::new(&source, profile).unwrap();
    let (before, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&source_configured).unwrap();
    let mut weights = source.weights().to_vec();
    let mut ac_coordinates = Vec::new();
    for (i, synapse) in before.synapses().iter().enumerate() {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)
        {
            weights[i] *= factor;
            ac_coordinates.push(i);
        }
    }
    let builtin =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let baseline = PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
        SensorProfile::GroundedObjectSlotsV1,
        &builtin,
    )
    .unwrap()
    .into_runtime_parts()
    .0;
    let candidate = FoundationWeightAsset::from_nano512_readout_candidate(
        &baseline,
        weights,
        source.manifest().training_stage(),
    )
    .unwrap();
    let configured = Nano512ActionCreditCandidateV2::new(&candidate, profile).unwrap();
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    assert_eq!(configured.asset().unwrap(), candidate);
    assert_ne!(candidate.digest(), source.digest());
    let mut changed_coordinates = Vec::new();
    for (i, (old, new)) in before
        .synapses()
        .iter()
        .zip(phenotype.synapses())
        .enumerate()
    {
        let expected = if ac_coordinates.contains(&i) {
            source.weights()[i] * factor
        } else {
            source.weights()[i]
        };
        assert_eq!(candidate.weights()[i].to_bits(), expected.to_bits());
        assert_eq!(new.genetic_weight().to_bits(), expected.to_bits());
        let mut old_row = serde_json::to_value(old).unwrap();
        old_row["genetic_weight"] = serde_json::json!(expected);
        assert_eq!(old_row, serde_json::to_value(new).unwrap());
        if old.genetic_weight().to_bits() != new.genetic_weight().to_bits() {
            changed_coordinates.push(i);
        }
    }
    assert_eq!(ac_coordinates.len(), 984);
    assert!(!changed_coordinates.is_empty());
    // Gene values, their two derived identities, and the embedded canonical
    // asset change together. Verify the embedded bytes before normalizing them.
    let mut old = serde_json::to_value(&before).unwrap();
    let new = serde_json::to_value(&phenotype).unwrap();
    assert_eq!(
        old["foundation_abi_selection"]["contract"]["source"]["canonical_asset"],
        serde_json::json!(source.encode_canonical().unwrap())
    );
    assert_eq!(
        new["foundation_abi_selection"]["contract"]["source"]["canonical_asset"],
        serde_json::json!(candidate.encode_canonical().unwrap())
    );
    old["foundation_abi_selection"]["contract"]["source"]["canonical_asset"] =
        new["foundation_abi_selection"]["contract"]["source"]["canonical_asset"].clone();
    for key in ["synapses", "phenotype_hash", "compiler_inputs_digest"] {
        old[key] = new[key].clone();
    }
    assert_eq!(old, new);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/scaled-choice-{}",
        std::process::id(),
    ));
    assert!(!root.exists(), "preserve previous scaling evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("source.alife-foundation"), bytes).unwrap();
    let encoded = candidate.encode_canonical().unwrap();
    assert_eq!(
        FoundationWeightAsset::decode_canonical(&encoded).unwrap(),
        candidate
    );
    std::fs::write(root.join("candidate.alife-foundation"), encoded).unwrap();
    save(&root, "configured-candidate.json", &configured);
    save(&root, "neural-phenotype.json", &phenotype);
    save(&root, "compiler-inputs.json", &inputs);
    save(
        &root,
        "scaling.json",
        &serde_json::json!({
            "source_digest":source.digest(), "candidate_digest":candidate.digest(),
            "factor":factor, "factor_bits":factor.to_bits(), "ac_coordinates":ac_coordinates,
            "changed_coordinates":changed_coordinates, "all_coordinates_verified":true,
            "non_gene_phenotype_fields_unchanged":true, "optimizer_steps":0,
            "embedded_assets_verified":"each equals its respective canonical asset",
            "source_phenotype_hash":before.phenotype_hash(), "candidate_phenotype_hash":phenotype.phenotype_hash(),
            "training_stage":"retained source provenance; this is arithmetic scaling, not further training",
        }),
    );
    save(
        &root,
        "bounds.json",
        &serde_json::json!({
            "profile":profile, "feeding_cases":3, "feeding_ticks_per_case":16,
            "feeding_seconds_per_case":90, "feeding_outer_seconds":360,
            "preference_lives":2, "preference_ticks_per_life":32,
            "preference_seconds_per_life":120, "preference_outer_seconds":360,
            "outer_seconds":720, "selector_capture_ticks":"1-4 and 17-32 of paired lives",
            "selector_details":"Ingest, Approach, Avoid, Contact; at most eight candidates",
            "selector_final_scores":"all candidates",
            "stop_on_first_feeding_failure":true, "automatic_repeat":false, "default_promoted":false,
            "fresh_arithmetic":"preserves original decisions; not eight teacher successes",
            "scale_status":"experimental choice, not a proven optimum",
        }),
    );
    let feeding_started = Instant::now();
    let mut feeding = Vec::new();
    for (case, position) in [
        Vec3f::new(-3.0, 0.0, 1.5),
        Vec3f::new(0.7, 0.0, -0.2),
        Vec3f::new(2.7, 0.0, -1.0),
    ]
    .into_iter()
    .enumerate()
    {
        let consumed = feeding_case_configured(
            &candidate,
            95000 + case as u64,
            position,
            &root.join(format!("feeding-{case}")),
            Some(&configured),
        );
        feeding.push(consumed);
        save(
            &root,
            &format!("feeding-gate-{case}.json"),
            &serde_json::json!({
                "completed_cases":feeding, "candidate_digest":candidate.digest(), "profile":profile,
                "phenotype_hash":phenotype.phenotype_hash(), "preference":"not yet run",
            }),
        );
        assert!(
            consumed,
            "scaled founder feeding failed; stop before preference or retuning"
        );
        assert!(feeding_started.elapsed().as_secs() < 360);
    }
    let preference_started = Instant::now();
    let mut outcomes = Vec::new();
    for (cyan_nutritious, life) in [(true, "cyan-nutritious"), (false, "amber-nutritious")] {
        let life_root = root.join(life);
        SELECTOR_CAPTURE_ROOT.with(|capture| *capture.borrow_mut() = Some(life_root.clone()));
        CHOICE_TRIAL_SELECTOR_CAPTURE.with(|capture| capture.set(true));
        let outcome = run_food_life(
            &candidate,
            &phenotype,
            cyan_nutritious,
            &life_root,
            Some(&configured),
        );
        CHOICE_TRIAL_SELECTOR_CAPTURE.with(|capture| capture.set(false));
        SELECTOR_CAPTURE_ROOT.with(|capture| *capture.borrow_mut() = None);
        save_choice_credit_rows(&phenotype, &life_root);
        outcomes.push(outcome);
        save(
            &root,
            &format!("preference-outcomes-{life}.json"),
            &outcomes,
        );
        assert!(preference_started.elapsed().as_secs() < 360);
    }
    save(&root, "preference-outcomes.json", &outcomes);
    assert_choices(&outcomes[0], &outcomes[1]);
    save(
        &root,
        "completion.json",
        &serde_json::json!({
            "feeding_cases":feeding, "preference_gate":"PASS", "default_promoted":false,
        }),
    );
}

#[test]
#[ignore = "one short live profile check; saved ALIFE_PREFERENCE_CANDIDATE, no optimizer"]
fn signed_choice_readouts_first_meal_once() {
    let bytes = std::fs::read(std::env::var("ALIFE_PREFERENCE_CANDIDATE").unwrap()).unwrap();
    let asset = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        asset.digest().bytes(),
        &[
            40, 161, 51, 234, 158, 204, 236, 70, 61, 99, 211, 218, 78, 154, 50, 208, 109, 194, 193,
            113, 108, 113, 39, 239, 166, 78, 186, 38, 133, 91, 128, 250,
        ]
    );
    let configured = Nano512ActionCreditCandidateV2::new(
        &asset,
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    assert_eq!(
        configured.asset().unwrap().encode_canonical().unwrap(),
        bytes
    );
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/signed-choice-profile-{}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve prior evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("candidate.alife-foundation"), bytes).unwrap();
    save(&root, "neural-phenotype.json", &phenotype);
    save(&root, "compiler-inputs.json", &inputs);
    save(
        &root,
        "bounds.json",
        &serde_json::json!({
            "candidate_source":"readout-calibration-16188","profile":"SignedChoiceReadouts",
            "optimizer_steps":0,"lives":1,"tick_cap":4,"wall_seconds":75,
            "outer_wall_seconds":120,"stop_after":"first meal plus next decision",
            "preference_assessment":false,"default_promoted":false,
        }),
    );
    first_meal_with_configured(&configured, &root);
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("receipt.json")).unwrap()).unwrap();
    let mut credit_receipts = Vec::new();
    let mut harmful_meals = 0;
    for tick in 1..=receipt["ticks"].as_u64().unwrap() {
        let outcome: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("tick-{tick}-outcome.json"))).unwrap(),
        )
        .unwrap();
        let patch: ExperiencePatch = serde_json::from_value(outcome["patch"].clone()).unwrap();
        assert_eq!(
            patch.decision().neural_evidence().unwrap().phenotype_hash,
            phenotype.phenotype_hash()
        );
        let selector: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("tick-{tick:02}-selector.json"))).unwrap(),
        )
        .unwrap();
        let receptors: NeuralReceptorFrame =
            serde_json::from_value(selector["neural_receptors"].clone()).unwrap();
        let credit = OutcomeCreditPacket::from_sealed_patch(&patch)
            .unwrap()
            .with_biochemical_receptors(&receptors)
            .unwrap();
        let lanes = *credit.modulator().frame().lanes();
        let harmful = patch.outcome().physical.contact == PhysicalContactKind::Consumed
            && patch.outcome().pain_delta.raw() > 0.0;
        harmful_meals += usize::from(harmful);
        for head in [
            DecoderHeadKind::ActionCandidate,
            DecoderHeadKind::MemoryContext,
        ] {
            let synapse = phenotype
                .synapses()
                .iter()
                .find(|s| {
                    matches!(s.kind(), CompiledSynapseKind::Decoder(c)
                if c.head() == head && c.family() == CandidateActionFamily::Ingest)
                })
                .unwrap();
            let receptor = &phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())];
            let weights = *receptor.receptor_profile().weights();
            assert_eq!(weights, [0.0, -1.0, 1.0, -0.5, 0.2, 0.0, 0.5, -0.5]);
            let factor = weights.iter().zip(lanes).map(|(w, l)| w * l).sum::<f32>()
                / weights.iter().map(|w| w.abs()).sum::<f32>();
            credit_receipts.push(serde_json::json!({"tick":tick,"head":head,
                "credit_lanes":lanes,"receptor":receptor,"third_factor":factor,
                "actual_harmful_meal":harmful,"pass":!harmful || factor < 0.0}));
        }
    }
    save(
        &root,
        "profile-credit-gate.json",
        &serde_json::json!({
            "harmful_meals":harmful_meals,"receipts":credit_receipts,
            "source":"sealed physical outcomes and receptor frames bound to saved GPU dispatches",
        }),
    );
    assert!(
        harmful_meals > 0,
        "short check did not expose harmful meal credit"
    );
    assert!(credit_receipts.iter().all(|r| r["pass"] == true));
}

fn first_meal(asset: &FoundationWeightAsset, root: &Path) {
    let configured = Nano512ActionCreditCandidateV2::new(
        asset,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    first_meal_with_configured(&configured, root);
}

fn first_meal_with_configured(configured: &Nano512ActionCreditCandidateV2, root: &Path) {
    let asset = configured.asset().unwrap();
    let (mut runtime, _) =
        super::super::founder_consequence_tests::paired_food_runtime_with_action_credit(
            &asset,
            true,
            &root.join("world"),
            true,
            Some(configured),
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
