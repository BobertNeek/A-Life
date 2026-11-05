use super::*;

/// Cold-read a specifically pinned experimental fork. The native head alone
/// omits acquired state; consumers must retain/admit this whole fork bundle.
/// Recompute the one update on a disposable optimizer clone, never on the saved
/// frontier or a source parent. No filesystem checkpoint is overwritten.
pub fn verify_pc_donor_retained_training_fork(
    root: &Path,
    expected_manifest_sha256: &str,
    expected_experiment_sha256: &str,
) -> Result<Value> {
    verify_inner(
        root,
        expected_manifest_sha256,
        expected_experiment_sha256,
        0,
    )
}

fn verify_inner(
    root: &Path,
    expected_manifest_sha256: &str,
    expected_experiment_sha256: &str,
    depth: usize,
) -> Result<Value> {
    if depth > 32 {
        return Err("fork ancestry exceeds bounded verification depth".into());
    }
    let manifest: ForkManifest = serde_json::from_slice(&pinned(
        root.join("retained-fork.json"),
        expected_manifest_sha256,
    )?)?;
    if manifest.schema != 1
        || manifest.kind != "experimental-retained-fork-complete-PC-optimizer-transfer"
        || manifest.experiment_sha256 != expected_experiment_sha256
        || manifest.ppo.epochs != 1
        || manifest.training_seed.checked_add(1) != Some(manifest.recovery_seed)
        || fs::read(root.join("phase.txt"))? != b"sealed-experimental-retained-fork"
        || inventory(root)? != manifest.artifacts
    {
        return Err("fork manifest or complete artifact inventory differs".into());
    }
    if let Some(p) = &manifest.previous_fork {
        if p.directory.canonicalize()? == root.canonicalize()? {
            return Err("fork cannot be its own predecessor".into());
        }
        verify_inner(
            &p.directory,
            &p.manifest_sha256,
            expected_experiment_sha256,
            depth + 1,
        )?;
    }
    let experiment: RetainedInterpolationExperiment = decode_zstd(&pinned(
        root.join("experiment.json.zst"),
        expected_experiment_sha256,
    )?)?;
    if experiment.inputs().cloud_fraction != 0.25 {
        return Err("fork fraction changed".into());
    }
    let source_proof: Option<SavedBodyRoundingCompatibilityV1> =
        read(root.join("source-body-proof.json"))?;
    let PcDonorRetainedFork {
        mut trainer,
        mut value,
        mut retained,
        donor_actor,
        donor_value,
    } = scope(&source_proof, || {
        initialize_pc_donor_retained_fork(&experiment)
    })?;
    let pc = scope(&source_proof, || {
        CurrentTrainingCheckpointV1::decode(&experiment.inputs().pc_current)
    })?;
    let mut source_version = pc.handoff.policy_version;
    let mut collection_inputs = experiment.compiler_inputs().clone();
    let mut previous_seeds = None;
    if let Some(p) = &manifest.previous_fork {
        let (t, v, r, i, version, seeds) = previous_state(p)?;
        trainer = t;
        value = v;
        retained = r;
        collection_inputs = i;
        source_version = version;
        previous_seeds = Some(seeds);
    }
    let before_actor = trainer.checkpoint();
    let before_value = value.checkpoint();
    if manifest.policy_version
        != source_version
            .checked_add(1)
            .ok_or("fork version overflow")?
    {
        return Err("fork policy clock differs from explicit donor fork".into());
    }
    let baseline: CheckpointSeedsV1 = read(root.join("seed-exclusions.json"))?;
    if previous_seeds
        .as_ref()
        .is_some_and(|s| !covers(&baseline, s))
    {
        return Err("fork lost predecessor reservations".into());
    }
    let evidence: Vec<Value> = decode_zstd(&fs::read(root.join("seed-evidence.json.zst"))?)?;
    if baseline.historical_freshness != "unknown"
        || baseline.pending_range.is_some()
        || pc.seeds.excluded_ranges.iter().any(|[a, b]| {
            !baseline
                .excluded_ranges
                .iter()
                .any(|[x, y]| x <= a && b <= y)
        })
    {
        return Err("fork lost known parent exclusions/freshness boundary".into());
    }
    for s in [manifest.training_seed, manifest.recovery_seed] {
        check_seed(&baseline, &evidence, s)?;
    }
    let mut expected_seeds = baseline;
    expected_seeds.reserve(manifest.training_seed, manifest.recovery_seed)?;
    expected_seeds.consume(manifest.training_seed)?;
    expected_seeds.consume(manifest.recovery_seed)?;
    let seeds: CheckpointSeedsV1 = read(root.join("seed-state.json"))?;
    if seeds != expected_seeds {
        return Err("fork consumed seed history differs".into());
    }
    let genesis: Value = read(root.join("fork-genesis.json"))?;
    if genesis["donor_actor_sha256"] != hash(&serde_json::to_vec(&donor_actor)?)
        || genesis["donor_critic_sha256"] != hash(&serde_json::to_vec(&donor_value)?)
        || genesis["pc_sha256"] != experiment.inputs().expected_pc_current_sha256
        || genesis["cloud_tree_sha256"] != experiment.receipt().cloud_parent_sha256
        || genesis["exact_continuation"] != false
        || genesis["merged_optimizer"] != false
        || genesis["candidate_actor_sha256"] != hash(&serde_json::to_vec(&before_actor)?)
    {
        return Err("fork optimizer-transfer provenance differs from exact source parents".into());
    }
    let collection = root.join("collection");
    let asset = collection_inputs
        .n2048_candidate_asset()
        .ok_or("candidate missing")?;
    if fs::read(collection.join("collection-policy.alife-foundation"))?
        != asset.encode_canonical()?
        || serde_json::to_vec(&read::<PhenotypeCompilerInputs>(
            collection.join("collection-compiler-inputs.json"),
        )?)? != serde_json::to_vec(&collection_inputs)?
        || serde_json::to_vec(&read::<FoundationTrainerCheckpoint>(
            collection.join("collection-actor-checkpoint.json"),
        )?)? != serde_json::to_vec(&before_actor)?
        || serde_json::to_vec(&read::<PpoValueHeadCheckpoint>(
            collection.join("initial-value-checkpoint.json"),
        )?)? != serde_json::to_vec(&before_value)?
        || serde_json::to_vec(&read::<CpuRetainedNeuralInheritance>(
            root.join("initial-retained-neural.json"),
        )?)? != serde_json::to_vec(&retained)?
    {
        return Err(
            "fork initial heads or reset projection differ from native donor recipe".into(),
        );
    }
    let compatibility: Value = read(collection.join("body-compatibility.json"))?;
    let body_proof: Option<SavedBodyRoundingCompatibilityV1> =
        serde_json::from_value(compatibility["explicit_proof"].clone())?;
    let initial: CpuCanonicalCheckpoint = scope(&body_proof, || {
        read(collection.join("initial-checkpoint.json"))
    })?;
    if initial.world.deterministic_seed != manifest.training_seed
        || initial.residents.len() != 1
        || serde_json::to_vec(
            initial.residents[0]
                .inherited_neural_source
                .as_ref()
                .ok_or("initial inheritance missing")?,
        )? != serde_json::to_vec(&retained)?
    {
        return Err("fork initial body/seed/inheritance differs".into());
    }
    let host = restore_private(&collection, &mut trainer, &body_proof)?;
    let cp = host.checkpoint(&trainer)?;
    if cp.world.deterministic_seed != manifest.training_seed {
        return Err("fork collection seed differs".into());
    }
    let record = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .ok_or("fork collector body missing")?;
    let current = host.export_retained_neural_for_newborn(&trainer, record.organism_id())?;
    if serde_json::to_vec(&current)? != fs::read(root.join("collection-retained-neural.json"))? {
        return Err("fork current learned export differs from native collector".into());
    }
    let decisions: Vec<CpuCollectedDecision> =
        decode_zstd(&fs::read(root.join("training-replay.json.zst"))?)?;
    let windows = cpu_training_windows(
        &decisions,
        if record.lifecycle().is_alive() {
            None
        } else {
            Some(record)
        },
        &trainer,
        &value,
        record.phenotype(),
        manifest.policy_version,
        manifest.ppo,
        32,
        8,
    )?;
    let old_p = trainer.phenotype().clone();
    let update = train_cpu_residual_cohort(
        &mut trainer,
        &mut value,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        manifest.ppo,
        manifest.policy_version,
    )?;
    if update.completed_epochs != 1
        || update.stopped_for_kl
        || trainer.optimizer_step()
            != before_actor
                .optimizer_step
                .checked_add(1)
                .ok_or("Adam overflow")?
        || value.checkpoint().optimizer_step
            != before_value
                .optimizer_step
                .checked_add(1)
                .ok_or("critic overflow")?
    {
        return Err("fork native replay did not reproduce one actual update".into());
    }
    let trained = trainer.export_candidate(asset.manifest().training_stage())?;
    let (next_p, next_inputs) = body_inputs(host.world(), &trained)?;
    let next_retained = current.rebind_weight_only(&old_p, &next_p)?;
    trainer.rebind_for_next_cohort(next_p, trained.clone())?;
    let handoff: FoundationTrainingHandoffV1 = read(root.join("training-handoff.json"))?;
    let admitted = handoff.clone().admit()?;
    if handoff.continuation != FoundationContinuationKindV1::FreshCohort
        || handoff.origin != FoundationTrainingBackendV1::NativeCpu
        || handoff.policy_version != manifest.policy_version
        || serde_json::to_vec(&admitted.actor)? != serde_json::to_vec(&trainer.checkpoint())?
        || serde_json::to_vec(&admitted.value)? != serde_json::to_vec(&value.checkpoint())?
        || serde_json::to_vec(&handoff.compiler_inputs)? != serde_json::to_vec(&next_inputs)?
        || fs::read(root.join("trained.alife-foundation"))? != trained.encode_canonical()?
        || serde_json::to_vec(&read::<FoundationTrainerCheckpoint>(
            root.join("actor-checkpoint.json"),
        )?)? != serde_json::to_vec(&trainer.checkpoint())?
        || serde_json::to_vec(&read::<PpoValueHeadCheckpoint>(
            root.join("value-checkpoint.json"),
        )?)? != serde_json::to_vec(&value.checkpoint())?
        || serde_json::to_vec(&next_retained)? != fs::read(root.join("retained-neural.json"))?
    {
        return Err(
            "fork trained heads/retained layout fail exact native update reproduction".into(),
        );
    }
    let after = trainer.checkpoint();
    for i in 0..after.update_ages.len() {
        if after.update_ages[i]
            != before_actor.update_ages[i]
                .checked_add(u32::from(before_actor.stage_mask.is_trainable(i)))
                .ok_or("coordinate Adam age overflow")?
        {
            return Err("fork per-coordinate Adam age changed incorrectly".into());
        }
    }
    let recovery = root.join("recovered-newborn");
    let compatibility: Value = read(recovery.join("body-compatibility.json"))?;
    let proof: Option<SavedBodyRoundingCompatibilityV1> =
        serde_json::from_value(compatibility["explicit_proof"].clone())?;
    let initial: CpuCanonicalCheckpoint =
        scope(&proof, || read(recovery.join("initial-checkpoint.json")))?;
    if initial.world.deterministic_seed != manifest.recovery_seed
        || initial.residents.len() != 1
        || serde_json::to_vec(
            initial.residents[0]
                .inherited_neural_source
                .as_ref()
                .ok_or("recovered inheritance missing")?,
        )? != serde_json::to_vec(&next_retained)?
    {
        return Err("fork recovered newborn does not carry updated retained state".into());
    }
    let world = scope(&proof, || Ok(initial.world.restore_headless_world()?))?;
    trainer.bind_equivalent_assessment_body(&body_inputs(&world, &trained)?.0)?;
    let mut cold = scope(&proof, || {
        Ok(CpuCanonicalHost::from_checkpoint(
            initial.clone(),
            &trainer,
        )?)
    })?;
    if serde_json::to_vec(&cold.checkpoint(&trainer)?)? != serde_json::to_vec(&initial)? {
        return Err("fork updated newborn cold roundtrip differs".into());
    }
    cold.step(&trainer, 1.)?;
    if serde_json::to_vec(&cold.checkpoint(&trainer)?)?
        != read_cpu_private_checkpoint_bytes(&recovery, 1024 * 1024 * 1024)?
    {
        return Err("fork recovered actual next transition differs".into());
    }
    let report = json!({"passed":true,"kind":"experimental-PC-optimizer-transfer-fork-native-CPU-verification","not_exact_continuation":true,"not_merged_Adam":true,"not_promoted":true,"manifest_sha256":expected_manifest_sha256,"experiment_sha256":expected_experiment_sha256,"training_seed":manifest.training_seed,"recovery_validation_seed":manifest.recovery_seed,"historical_freshness":"unknown","actor_age_before":before_actor.optimizer_step,"actor_age_after":after.optimizer_step,"critic_age_before":before_value.optimizer_step,"critic_age_after":value.checkpoint().optimizer_step,"per_coordinate_integer_ages_exact":true,"actor_coordinates":after.weights.len(),"retained_live_slots":next_retained.topology.total_synapse_count,"retained_acquired_edges":next_retained.topology.v11_checkpoint.sparse_spans.iter().map(|s|s.edges.len()).sum::<usize>(),"source_actor_sha256":hash(&serde_json::to_vec(&donor_actor)?),"source_critic_sha256":hash(&serde_json::to_vec(&donor_value)?),"initial_actor_sha256":hash(&serde_json::to_vec(&before_actor)?),"updated_actor_sha256":hash(&serde_json::to_vec(&after)?),"updated_critic_sha256":hash(&serde_json::to_vec(&value.checkpoint())?),"collector_cold_restore_exact":true,"updated_native_head_admitted":true,"updated_retained_newborn_cold_restore_and_next_transition_exact":true,"native_update_recomputed_on_disposable_clone":true,"global_frontier_updates":1,"hardware_gpu_parity":false});
    Ok(report)
}
