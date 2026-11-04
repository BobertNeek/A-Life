use super::*;
use alife_core::*;
use alife_training::*;
use alife_world::persistence::*;

fn sha(v: &impl serde::Serialize) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(v).unwrap()))
}

pub(super) fn game(
    seed: u64,
    id: u64,
    asset: &FoundationWeightAsset,
) -> alife_world::CanonicalNewGame {
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1).unwrap();
    config.founder_seed_base = 5124;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    alife_world::create_canonical_training_new_life(&config, asset, OrganismId(id)).unwrap()
}

// Deliberately metadata-only private closure: exercises typed lineage/identity
// admission without a GPU. The native asset loader must reject this fixture.
// The hardware test below creates the complete actual private asset closure.
pub(super) fn metadata_life(
    game: &alife_world::CanonicalNewGame,
    handoff: &FoundationTrainingHandoffV1,
    stream: u32,
) -> CurrentLifeCheckpointV1 {
    let mut life = CurrentLifeCheckpointV1 {
        world_json: String::new(),
        sleep_journal_json: "{}".into(),
        assets_base64: Default::default(),
    };
    let mut manifest = AssetManifest::empty();
    let mut asset = |name: &str, bytes: Vec<u8>| {
        let digest = PortableAssetDigest::for_bytes(&bytes);
        manifest.entries.push(AssetManifestEntry {
            asset_id: name.into(),
            kind: AssetKind::Other,
            relative_path: format!("{name}.json"),
            digest: digest.clone(),
            presence: AssetPresence::Required,
            schema_version: 1,
            size_bytes: Some(bytes.len() as u64),
            provenance: Some("disposable metadata unit fixture; not hardware state".into()),
        });
        life.insert_asset(format!("{name}.json"), &bytes);
        GpuBrainAssetRef {
            asset_id: name.into(),
            digest,
        }
    };
    let p = handoff.clone().admit().unwrap().phenotype;
    let phenotype = asset("phenotype", serde_json::to_vec(&p).unwrap());
    let inputs = asset(
        "inputs",
        serde_json::to_vec(&handoff.compiler_inputs).unwrap(),
    );
    let sampler = alife_gpu_backend::GpuTrainingSamplingConfig {
        seed: stream,
        counter: 0,
        temperature: 0.9,
        demonstrator: None,
    };
    let exact = asset(
        "exact",
        serde_json::to_vec(
            &serde_json::json!({"private_training_sampling":serde_json::to_vec(&sampler).unwrap()}),
        )
        .unwrap(),
    );
    let placeholder = asset("placeholder", b"{}".to_vec());
    let id = game.creatures[0].organism_id;
    let profile = SensorProfileIdentity {
        profile_id: p.sensor_profile().into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        id,
        profile,
        MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let topology =
        TopologySidecar::new_profiled(id, profile, TopologicalMapConfig::default()).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../alife_world/tests/fixtures/p34/tiny_save.json"
    ))
    .unwrap();
    let policy = BrainActivityPolicyV1::production_v1();
    let brain = GpuBrainSaveState {
        schema_version: GPU_BRAIN_SAVE_STATE_LEGACY_SCHEMA_VERSION,
        organism_id: id,
        phenotype_hash: p.phenotype_hash(),
        capacity_class_id: p.brain_class_id(),
        sensor_profile: profile,
        immutable_phenotype: phenotype,
        phenotype_compiler_inputs: inputs,
        live_structural_topology: None,
        legacy_nano512_compatibility_receipt: None,
        active_weight_generation: 1,
        active_weight_bank: 0,
        active_eligibility_bank: 0,
        learning_transaction_generation: 1,
        lifetime_weights: placeholder.clone(),
        fast_weights: placeholder.clone(),
        eligibility: placeholder.clone(),
        replay_journal: placeholder.clone(),
        replay_journal_generation: 1,
        replay_journal_cursor: 0,
        replay_journal_event_count: 0,
        activation_state: placeholder.clone(),
        neuron_homeostasis: placeholder.clone(),
        checkpoint_tick: Tick::ZERO,
        exact_cognitive_state: Some(exact),
        last_learning_replay_key: None,
        pending_eligibility: None,
        pending_experience_transaction: None,
        memory: MemorySidecarSaveState::from_sidecar(&memory, placeholder.clone(), None, None)
            .unwrap(),
        topology: TopologySidecarSaveSummary::from_sidecar(&topology, placeholder.clone()).unwrap(),
        tracked_objects: alife_world::TrackedObjectRegistry::new(game.world.seed(), 1024)
            .unwrap()
            .save_state(id)
            .unwrap(),
        language_grounding: Default::default(),
        life_statistics: None,
        sleep: SleepState::awake_at(Tick::ZERO),
        sleep_assets: GpuSleepAssetState::default(),
        backend_provenance: serde_json::from_value(
            fixture["creatures"][0]["gpu_brain"]["backend_provenance"].clone(),
        )
        .unwrap(),
        runtime_profile_id: 1,
        runtime_profile_digest: [31, 32, 33, 34],
        activity_policy_version: policy.policy_version,
        activity_policy_digest: policy.policy_digest,
        throttle_replay: ThrottleReplaySaveState::bootstrap(placeholder).unwrap(),
    };
    brain.validate().unwrap();
    let mut save = PortableSaveFile::from_headless_world(
        "new-life-unit-metadata",
        &game.world,
        alife_world::RuntimeConfig::deterministic_default(
            game.world.seed(),
            BrainScaleTier::Standard2048,
        ),
        manifest,
        game.creatures.clone(),
    )
    .unwrap();
    save.creatures[0].gpu_brain = Some(brain);
    life.world_json = serde_json::to_string(&save).unwrap();
    life
}

pub(super) fn terminal() -> CurrentTrainingCheckpointV1 {
    let asset =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let first_game = game(5124, 1, &asset);
    let record = first_game.world.organism_registry().iter().next().unwrap();
    let capacity = BrainCapacityClass::n2048();
    let genome = &record.phenotype().brain_genome;
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let base = PhenotypeCompiler::compile(
        genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        base.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let mut game = game(5124, 1, &asset);
    let (_, base_inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    let transition =
        N2048LivePolicyTransition::adopt_existing_policy(&base_inputs, 11891, asset.clone())
            .unwrap();
    let p = transition.next_phenotype();
    let n = p.synapses().len();
    let mut actor = FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: p.phenotype_hash(),
        source_foundation_digest: asset.digest(),
        optimizer_step: 25664,
        config: AdamWConfig::default(),
        stage_mask: StageTrainableMask::from_synapse_indices(p, &(1..n as u32).collect::<Vec<_>>())
            .unwrap(),
        weights: p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        first_moment: vec![0.01; n],
        second_moment: vec![0.02; n],
        update_ages: vec![25664; n],
    };
    actor.first_moment[0] = -0.;
    actor.second_moment[0] = f32::from_bits(1);
    actor.update_ages[0] = 2;
    actor.update_ages[1] = 25660;
    let width = p.neuron_count() as usize + 1;
    let mut parameters = vec![0.; width * 3];
    parameters[0] = -0.;
    parameters[width] = 0.01;
    parameters[width * 2] = 0.02;
    let value = PpoValueHeadCheckpoint {
        feature_count: p.neuron_count(),
        optimizer_step: 28780,
        parameters,
        last_updated_policy_version: Some(11891),
    };
    let handoff = FoundationTrainingHandoffV1::new_same_individual(
        FoundationTrainingBackendV1::NativeGpu,
        11891,
        transition.next_inputs().clone(),
        actor,
        value,
    )
    .unwrap();
    // A retained preterminal body with depleted physical reserves must never
    // become the new body's physiology.
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let mut chemistry = *record.biochemistry();
    chemistry.body.set_energy(0.001).unwrap();
    let depleted = alife_world::WorldOrganismRecord::new(
        record.organism_id(),
        record.world_entity_id(),
        record.genome().clone(),
        record.phenotype().clone(),
        chemistry,
        record.birth_tick(),
    )
    .unwrap();
    game.world.replace_organism_record_exact(depleted).unwrap();
    let life = metadata_life(&game, &handoff, 77);
    let tag = SameIndividualCurrentV1 {
        schema: 1,
        kind: "same-individual-ppo-v1".into(),
        counting_scope: "retained-same-individual-ppo-since-first-tag".into(),
        progress: SameIndividualProgressV1 {
            session_nonce: 6000,
            organism_id: 1,
            session_start_world_tick: 0,
            world_tick: 0,
            trained_rows: 0,
            completed_updates: 0,
            sampling_temperature: 0.9,
        },
        baseline_current_sha256: "ab".repeat(32),
        baseline_policy_version: 11891,
        baseline_actor_optimizer_step: 25664,
        baseline_value_optimizer_step: 28780,
        baseline_trained_rows: 0,
        baseline_completed_updates: 0,
        total_trained_rows: 0,
        total_completed_updates: 0,
        compiler_inputs_sha256: sha(&handoff.compiler_inputs),
        actor_sha256: sha(&handoff.actor),
        value_sha256: sha(&handoff.value),
        private_life_sha256: sha(&life),
        adopted_foundation_digest: Some(asset.digest()),
        sampling_state: alife_gpu_backend::GpuTrainingSamplingConfig {
            seed: 77,
            counter: 0,
            temperature: 0.9,
            demonstrator: None,
        },
    };
    let source = CurrentTrainingCheckpointV1 {
        schema_version: 1,
        seed: 5124,
        founder_seed_base: 5124,
        seeds: CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[5124, 5124], [6000, 6000], [8000, 8010]],
            pending_range: Some([8000, 8010]),
        },
        handoff,
        life: Some(life),
        same_individual: Some(tag),
        terminal_disposition: None,
        training_attempt_origin: None,
        new_life_origin: None,
    };
    source.with_terminal_disposition(&"a1".repeat(32),&"b2".repeat(32),6000,serde_json::json!({"kind":"continuous-terminal-v1","terminal":true,"session_nonce":6000,"source_world_seed":5124,"organism_id":1,"world_tick":4,"terminal_death_tick":4})).unwrap()
}

#[test]
fn new_life_carries_every_head_bit_and_preserves_death_and_reservations() {
    let source = terminal();
    let original = source.encode().unwrap();
    let (game, _) = prepare(&source, 8000, 2).unwrap();
    let next = source
        .seal_new_life(
            &original,
            8000,
            2,
            metadata_life(&game, &source.handoff, 88),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&source.handoff).unwrap(),
        serde_json::to_vec(&next.handoff).unwrap()
    );
    assert_eq!(next.handoff.actor.optimizer_step, 25664);
    assert_eq!(next.handoff.value.optimizer_step, 28780);
    assert_eq!(next.seeds.pending_range, Some([8002, 8010]));
    assert_eq!(next.seeds.excluded_ranges, source.seeds.excluded_ranges);
    assert_eq!(next.seeds.historical_freshness, "unknown");
    assert_eq!(source.encode().unwrap(), original);
    assert!(source.require_living_continuation().is_err());
    assert!(next.require_living_continuation().is_ok());
    assert_eq!(
        next.new_life_origin.as_ref().unwrap().lives[0].source_terminal,
        source.terminal_disposition.clone().unwrap()
    );
    assert_eq!(
        next.same_individual
            .as_ref()
            .unwrap()
            .total_completed_updates,
        0
    );
    let restored = CurrentTrainingCheckpointV1::decode(&next.encode().unwrap()).unwrap();
    assert_eq!(restored.encode().unwrap(), next.encode().unwrap());
    let old = PortableSaveFile::from_json_str(&source.life.as_ref().unwrap().world_json)
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert!(
        old.organism_registry()
            .iter()
            .next()
            .unwrap()
            .biochemistry()
            .body
            .energy
            < 0.01
    );
    assert!(
        game.world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .biochemistry()
            .body
            .energy
            > 0.5
    );
    assert_eq!(game.world.tick(), Tick::ZERO);
    assert_eq!(
        game.world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .organism_id(),
        OrganismId(2)
    );
    assert!(
        admit_current_training_checkpoint(next).is_err(),
        "metadata placeholders must never be admitted as native private assets"
    );
}

#[test]
fn new_life_rejects_collisions_partial_tampered_and_incompatible_sources() {
    let source = terminal();
    let bytes = source.encode().unwrap();
    for (seed, id) in [(5124, 2), (6000, 2), (8001, 2), (8000, 1), (u64::MAX, 2)] {
        assert!(prepare(&source, seed, id).is_err());
    }
    let mut changed = source.clone();
    changed.handoff.actor.first_moment[1] += 0.01;
    assert!(prepare(&changed, 8000, 2).is_err());
    let mut changed = source.clone();
    changed.handoff.value.parameters[0] = 0.;
    assert!(prepare(&changed, 8000, 2).is_err());
    let mut changed = source.clone();
    changed
        .life
        .as_mut()
        .unwrap()
        .assets_base64
        .remove("inputs.json");
    assert!(prepare(&changed, 8000, 2).is_err());
    let (game, _) = prepare(&source, 8000, 2).unwrap();
    let life = metadata_life(&game, &source.handoff, 88);
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(source
        .seal_new_life(&corrupt, 8000, 2, life.clone())
        .is_err());
    let mut next = source.seal_new_life(&bytes, 8000, 2, life).unwrap();
    next.handoff.actor.update_ages[1] -= 1;
    assert!(next.encode().is_err());
    let mut changed = source.clone();
    changed.founder_seed_base += 1; // Reseal disposition only to test the actual constructor's independent genotype check.
    changed.terminal_disposition = None;
    let tag = source.terminal_disposition.as_ref().unwrap();
    let changed = changed
        .with_terminal_disposition(
            &tag.baseline_current_sha256,
            &tag.request_sha256,
            6000,
            tag.terminal_event.clone(),
        )
        .unwrap();
    assert!(prepare(&changed, 8000, 2).is_err());
}

#[cfg(feature = "cpu-training")]
#[test]
fn new_life_newborn_cpu_component_training_and_recovery_smoke() {
    use alife_training::cpu::*;
    let source = terminal();
    let (game, _) = prepare(&source, 8000, 2).unwrap();
    // CPU collectors currently admit ordinary founder compiler identities,
    // not the GPU live-policy hash. This is a component smoke of the same
    // newborn factory and native learner, NOT end-to-end GPU admission.
    // Only this disposable test's actor hash is bound to that ordinary graph.
    let record = game.world.organism_registry().iter().next().unwrap();
    let capacity = BrainCapacityClass::n2048();
    let genome = &record.phenotype().brain_genome;
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let asset = source
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .unwrap()
        .clone();
    let (phenotype, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    let mut actor = source.handoff.actor.clone();
    actor.phenotype_hash = phenotype.phenotype_hash();
    let mut trainer = CpuFoundationTrainer::from_checkpoint(phenotype, asset, actor).unwrap();
    let mut value = CpuValueHead::from_checkpoint(source.handoff.value.clone()).unwrap();
    assert_eq!(trainer.checkpoint().weights, source.handoff.actor.weights);
    assert_eq!(
        trainer.checkpoint().update_ages,
        source.handoff.actor.update_ages
    );
    let mut host = CpuCanonicalHost::new_cohort(game, &trainer, 88).unwrap();
    let mut rows = Vec::new();
    for _ in 0..5 {
        rows.extend(host.step(&trainer, 0.9).unwrap().decisions);
    }
    assert_eq!(rows.len(), 5);
    let saved = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let mut restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&saved).unwrap(), &trainer)
            .unwrap();
    assert_eq!(
        serde_json::to_vec(&host.step(&trainer, 0.9).unwrap().decisions).unwrap(),
        serde_json::to_vec(&restored.step(&trainer, 0.9).unwrap().decisions).unwrap()
    );
    let body = host
        .world()
        .organism_registry()
        .get(OrganismId(2))
        .unwrap()
        .phenotype();
    let config = PpoConfig {
        temperature: 0.9,
        epochs: 1,
        ..Default::default()
    };
    let windows =
        cpu_training_windows(&rows, None, &trainer, &value, body, 11892, config, 4, 0).unwrap();
    let mut trainer2 = CpuFoundationTrainer::from_checkpoint(
        trainer.phenotype().clone(),
        source
            .handoff
            .compiler_inputs
            .n2048_candidate_asset()
            .unwrap()
            .clone(),
        trainer.checkpoint(),
    )
    .unwrap();
    let mut value2 = CpuValueHead::from_checkpoint(value.checkpoint()).unwrap();
    let a = train_cpu_residual_cohort(
        &mut trainer,
        &mut value,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        config,
        11892,
    )
    .unwrap();
    train_cpu_residual_cohort(
        &mut trainer2,
        &mut value2,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        config,
        11892,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&trainer.checkpoint()).unwrap(),
        serde_json::to_vec(&trainer2.checkpoint()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&value.checkpoint()).unwrap(),
        serde_json::to_vec(&value2.checkpoint()).unwrap()
    );
    assert!(a.actor_optimizer_step > 25664);
    assert!(a.value_optimizer_step > 28780);
    eprintln!("new-life newborn/native-CPU component smoke: 5 real decisions; exact next private step and full next actor/value Adam states; policy 11891 -> 11892, ages {}/{}; no feeding efficacy claim",a.actor_optimizer_step,a.value_optimizer_step);
}

fn gpu_qualification_request(
    now: u64,
    collection: Option<&str>,
    finalization: Option<&str>,
) -> Result<crate::ResidentContinuousRequest> {
    let collection_cutoff_unix_seconds: u64 = collection
        .ok_or("set ALIFE_NEW_LIFE_GPU_COLLECTION_CUTOFF to the admitted absolute Unix cutoff")?
        .parse()?;
    let finalization_deadline_unix_seconds: u64 = finalization
        .ok_or(
            "set ALIFE_NEW_LIFE_GPU_FINALIZATION_DEADLINE to the admitted absolute Unix deadline",
        )?
        .parse()?;
    let request = crate::ResidentContinuousRequest {
        schema: 1,
        seed_range: [8002, 8010],
        total_trained_decisions: 4,
        decisions_per_update: 4,
        snapshot_every_updates: 1,
        care_assistance: false,
        sampling_temperature: 0.9,
        collection_cutoff_unix_seconds,
        finalization_deadline_unix_seconds,
        max_temporary_bytes: 2 * 1024 * 1024 * 1024,
    };
    crate::validate_resident_continuous_request(&request, now)?;
    Ok(request)
}

#[test]
fn new_life_gpu_qualification_requires_both_explicit_deadlines() {
    assert!(gpu_qualification_request(100, None, None).is_err());
    assert!(gpu_qualification_request(100, Some("200"), None).is_err());
    assert!(gpu_qualification_request(100, None, Some("300")).is_err());
    assert!(gpu_qualification_request(100, Some("invalid"), Some("300")).is_err());
}

#[test]
fn new_life_gpu_qualification_rejects_expired_and_reversed_windows() {
    for (collection, finalization) in [
        ("100", "300"),
        ("99", "300"),
        ("300", "300"),
        ("300", "200"),
    ] {
        assert!(gpu_qualification_request(100, Some(collection), Some(finalization)).is_err());
    }
}

#[test]
fn new_life_gpu_qualification_preserves_absolute_window_through_slow_setup() {
    let request = gpu_qualification_request(100, Some("1000"), Some("1600")).unwrap();
    // Setup consumes real time; it never moves the owner's absolute boundaries.
    crate::validate_resident_continuous_request(&request, 999).unwrap();
    assert!(crate::validate_resident_continuous_request(&request, 1000).is_err());
    assert_eq!(request.collection_cutoff_unix_seconds, 1000);
    assert_eq!(request.finalization_deadline_unix_seconds, 1600);
    assert_eq!(request.total_trained_decisions, 4);
}

#[cfg(feature = "gpu-tests")]
#[test]
#[ignore = "requires an admitted native hardware GPU; creates only disposable public fixture state"]
fn new_life_gpu_initialization_and_continuous_recovery() {
    gpu_initialization_and_continuous_recovery(false);
}

#[cfg(feature = "gpu-tests")]
#[test]
#[ignore = "requires an admitted native hardware GPU; creates only disposable acquired-neural fixture state"]
fn retained_neural_gpu_initialization_and_continuous_recovery() {
    gpu_initialization_and_continuous_recovery(true);
}

#[cfg(feature = "gpu-tests")]
fn gpu_initialization_and_continuous_recovery(retained_neural: bool) {
    let started = std::time::Instant::now();
    let now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
    let collection = std::env::var("ALIFE_NEW_LIFE_GPU_COLLECTION_CUTOFF").ok();
    let finalization = std::env::var("ALIFE_NEW_LIFE_GPU_FINALIZATION_DEADLINE").ok();
    // No invented host-speed budget, hidden extension or rolling deadline.
    // Missing/expired settings fail before any device or fixture construction.
    let request = gpu_qualification_request(now(), collection.as_deref(), finalization.as_deref())
        .expect("native GPU qualification requires an explicit admitted absolute window");
    let root = std::env::temp_dir().join(format!(
        "alife-new-life-gpu-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let phase = |name: &str| {
        eprintln!(
            "{}",
            serde_json::json!({
                "kind":"new-life-gpu-qualification-phase-v1", "phase":name,
                "fixture":root,"unix_seconds":now(),"elapsed_seconds":started.elapsed().as_secs_f64(),
                "collection_cutoff_unix_seconds":request.collection_cutoff_unix_seconds,
                "finalization_deadline_unix_seconds":request.finalization_deadline_unix_seconds,
            })
        )
    };
    phase("fixture-created");
    let source_dir = root.join("source");
    fs::create_dir(&source_dir).unwrap();
    let mut source = terminal();
    let event = source.terminal_disposition.take().unwrap();
    // Build a fully native source on this GPU. No metadata placeholders enter
    // the admission/public command exercised below.
    let birth = source
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .unwrap();
    let source_game = game(5124, 1, birth);
    let terminal_probe = terminal();
    let (_, transition) = prepare(&terminal_probe, 8000, 2).unwrap();
    let sampling = source.same_individual.as_ref().unwrap().sampling_state;
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let life = initialize(
        &source,
        source_game,
        &transition,
        sampling,
        backend,
        &source_dir,
    )
    .unwrap();
    let life = if retained_neural {
        neural_inheritance::tests::native_learned_fixture(
            life,
            &source.handoff.clone().admit().unwrap().phenotype,
        )
    } else {
        life
    };
    phase("source-native-initialized");
    let save = PortableSaveFile::from_json_str(&life.world_json).unwrap();
    let tag = source.same_individual.as_mut().unwrap();
    tag.private_life_sha256 = sha(&life);
    if save.creatures[0].composite_genetics.is_some() {
        tag.adopted_foundation_digest = None;
    }
    source.life = Some(life);
    let source = source
        .with_terminal_disposition(
            &event.baseline_current_sha256,
            &event.request_sha256,
            event.session_nonce,
            event.terminal_event,
        )
        .unwrap();
    let source_path = source_dir.join("terminal.alife-checkpoint");
    let original = source.encode().unwrap();
    write_new(&source_path, &original).unwrap();
    read_current_training_checkpoint(&source_path).unwrap();
    let output = root.join("new-life");
    if retained_neural {
        check_new_individual_retained_neural(&source_path, 8000, 2).unwrap();
        begin_new_individual_training_life_with_retained_neural(&source_path, 8000, 2, &output)
            .unwrap();
    } else {
        begin_new_individual_training_life(&source_path, 8000, 2, &output).unwrap();
    }
    phase("new-individual-native-initialized");
    let checkpoint =
        read_current_training_checkpoint(&output.join("store/current.alife-checkpoint")).unwrap();
    assert_eq!(
        serde_json::to_vec(&checkpoint.handoff).unwrap(),
        serde_json::to_vec(&source.handoff).unwrap()
    );
    let newborn =
        PortableSaveFile::from_json_str(&checkpoint.life.as_ref().unwrap().world_json).unwrap();
    let brain = newborn.creatures[0].gpu_brain.as_ref().unwrap();
    assert_eq!(brain.memory.summary.record_count, 0);
    assert_eq!(brain.replay_journal_event_count, 0);
    assert!(brain.pending_eligibility.is_none());
    assert_eq!(brain.checkpoint_tick, Tick::ZERO);
    if retained_neural {
        neural_inheritance::verify(
            &neural_inheritance::RetainedNeural::read(&source).unwrap(),
            checkpoint.life.as_ref().unwrap(),
        )
        .unwrap();
        assert!(checkpoint
            .new_life_origin
            .as_ref()
            .unwrap()
            .lives
            .last()
            .unwrap()
            .neural_inheritance
            .is_some());
    }
    assert!(begin_new_individual_training_life(&source_path, 8000, 2, &output).is_err());
    crate::validate_resident_continuous_request(&request, now())
        .expect("admitted collection window expired during fixture setup; fixture retained");
    phase("continuous-producer-starting");
    if let Err(error) = crate::run_resident_continuous_care(&output.join("store"), request.clone())
    {
        phase("continuous-producer-failed-before-recovery");
        panic!(
            "native qualification producer failed; fixture retained at {}: {error}",
            root.display()
        );
    }
    phase("continuous-producer-finished");
    let recovery = crate::recover_resident_serial_checkpoint(&output.join("store")).unwrap();
    phase("continuous-recovery-finished");
    let recovered =
        read_current_training_checkpoint(Path::new(recovery["candidate"].as_str().unwrap()))
            .unwrap();
    assert!(recovered.handoff.policy_version > source.handoff.policy_version);
    assert!(recovered.handoff.actor.optimizer_step > 25664);
    assert!(recovered.handoff.value.optimizer_step > 28780);
    assert_eq!(recovered.new_life_origin, checkpoint.new_life_origin);
    assert_eq!(fs::read(&source_path).unwrap(), original);
    assert_eq!(
        fs::read(output.join("terminal-source.alife-checkpoint")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(output.join("new-life-baseline.alife-checkpoint")).unwrap(),
        checkpoint.encode().unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_life_rejects_private_structural_changes_outside_global_head_coordinates() {
    let source = terminal();
    let admitted = source.handoff.clone().admit().unwrap();
    let mut topology =
        alife_gpu_backend::GpuV11Checkpoint::canonical_for_phenotype(&admitted.phenotype).unwrap();
    require_canonical_private_topology(&topology, &admitted.phenotype).unwrap();
    topology
        .sparse_spans
        .push(alife_gpu_backend::GpuV11SparseSpan {
            target: 1,
            edges: vec![alife_gpu_backend::GpuV11SparseEdge {
                source: 0,
                target: 1,
                route: 0,
                weight: 0.2,
            }],
        });
    assert!(require_canonical_private_topology(&topology, &admitted.phenotype).is_err());
}
