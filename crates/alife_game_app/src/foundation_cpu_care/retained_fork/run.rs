use super::*;

pub fn run_pc_donor_retained_training_fork(spec: &Path) -> Result<Value> {
    let req: Request = read(spec)?;
    let output = resolve_isolated_training_output(
        &req.output,
        [
            spec,
            req.prepared.as_path(),
            req.pc_current.as_path(),
            req.cloud_native_root.as_path(),
            req.cloud_archive.as_path(),
            req.seed_history.as_path(),
        ],
    )?;
    if output.exists() {
        return Err("existing fork prefix preserved; reconcile rather than rerun".into());
    }
    let Context {
        experiment_bytes,
        experiment,
        pc,
        source_proof,
        mut seed_history,
        seed_evidence,
    } = context(&req)?;
    let PcDonorRetainedFork {
        mut trainer,
        mut value,
        retained,
        donor_actor,
        donor_value,
    } = scope(&source_proof, || {
        initialize_pc_donor_retained_fork(&experiment)
    })?;
    let collection_inputs = experiment.compiler_inputs().clone();
    let asset = collection_inputs
        .n2048_candidate_asset()
        .ok_or("fork asset missing")?
        .clone();
    let source_actor = serde_json::to_vec(&donor_actor)?;
    let source_value = serde_json::to_vec(&donor_value)?;
    let before_actor = trainer.checkpoint();
    let before_value = value.checkpoint();
    let version = pc
        .handoff
        .policy_version
        .checked_add(1)
        .ok_or("fork policy overflow")?;
    if before_value
        .last_updated_policy_version
        .is_some_and(|v| v >= version)
    {
        return Err("fork version must follow donor critic update marker".into());
    }
    let (_, mut episode) = prepare_care_episode(CpuCareRequest {
        previous: None,
        output: output.join("collection"),
        seed: req.seed,
        decisions: req.decisions,
        temperature: 1.,
        lesson: FoundationTeacherLesson::EatHeldFood,
        food_need_condition: FoundationFoodNeedCondition::NewbornV1,
        food_offset: None,
        layout: None,
        seed_history: Some(req.seed_history.clone()),
        known_exclusions: true,
        record_diagnostic_replay: false,
        fixture: false,
    })?;
    // This domain performs its own full two-source and seed admission above.
    // It does not enter the fixture route or relabel an ordinary parent receipt.
    episode.independent_training = true;
    episode.validate_policy(&trainer)?;
    let saved = retained::saved_source_phenotype(&pc)?;
    let (game, compatibility, body_proof) =
        retained::newborn_for_view(&asset, pc.founder_seed_base, &saved, req.seed, &mut trainer)?;
    let (mut host, ()) = CpuCanonicalHost::new_prepared_cohort(
        game,
        &trainer,
        (req.seed as u32) ^ ((req.seed >> 32) as u32),
        |world| episode.prepare_world(world),
    )?;
    host.inherit_retained_neural(&trainer, &retained)?;
    let initial_cp = host.checkpoint(&trainer)?;
    let initial_cold = scope(&body_proof, || {
        Ok(CpuCanonicalHost::from_checkpoint(
            initial_cp.clone(),
            &trainer,
        )?)
    })?;
    if serde_json::to_vec(&initial_cold.checkpoint(&trainer)?)? != serde_json::to_vec(&initial_cp)?
    {
        return Err("fork initial retained state does not cold-roundtrip".into());
    }
    fs::create_dir(&output)?;
    let collection = output.join("collection");
    fs::create_dir(&collection)?;
    fs::write(
        output.join("phase.txt"),
        "collecting-experimental-retained-fork",
    )?;
    fs::write(output.join("request.json"), fs::read(spec)?)?;
    fs::write(output.join("experiment.json.zst"), &experiment_bytes)?;
    write(output.join("source-body-proof.json"), &source_proof)?;
    write(output.join("seed-exclusions.json"), &seed_history)?;
    write_zstd(output.join("seed-evidence.json.zst"), &seed_evidence)?;
    write(output.join("initial-retained-neural.json"), &retained)?;
    write(
        output.join("fork-genesis.json"),
        &json!({"kind":"experimental-two-parent-genetics-complete-PC-optimizer-transfer","exact_continuation":false,"merged_optimizer":false,"cloud_fraction":0.25,"pc_sha256":req.expected_pc_sha256,"cloud_tree_sha256":req.expected_cloud_tree_sha256,"cloud_transport_sha256":req.expected_cloud_archive_sha256,"experiment_sha256":req.expected_experiment_sha256,"PC_policy":pc.handoff.policy_version,"cloud_policy":experiment.receipt().cloud_native_cpu_binding.as_ref().map(|b|b.policy_version),"donor_actor_sha256":hash(&source_actor),"donor_critic_sha256":hash(&source_value),"donor_actor_age":before_actor.optimizer_step,"donor_critic_age":before_value.optimizer_step,"candidate_actor_sha256":hash(&serde_json::to_vec(&before_actor)?),"reset":"both fast/lifetime physical banks +0 only at new-fork start; original selectors/generations/topology/dendrites/predictor retained","personal_memory":"newborn; original lives remain archived in experiment","historical_freshness":"unknown","not_promoted":true}),
    )?;
    fs::write(
        collection.join("collection-policy.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    write(
        collection.join("collection-compiler-inputs.json"),
        &collection_inputs,
    )?;
    write(
        collection.join("collection-actor-checkpoint.json"),
        &before_actor,
    )?;
    write(
        collection.join("initial-value-checkpoint.json"),
        &before_value,
    )?;
    write(collection.join("body-compatibility.json"), &compatibility)?;
    write(collection.join("initial-checkpoint.json"), &initial_cp)?;
    seed_history.reserve(req.seed, req.recovery_seed)?;
    seed_history.consume(req.seed)?;
    write(output.join("seed-state.json"), &seed_history)?;
    let mut rows = BTreeMap::<u64, Vec<CpuCollectedDecision>>::new();
    let mut peak_injury = 0f32;
    let mut minimum_health = 1f32;
    let mut physiology = Vec::new();
    let mut failure = None;
    for _ in 0..(req.decisions + 1) * 256 {
        let step = (|| -> Result<CpuCanonicalStep> {
            episode.before_step(&mut host, &trainer)?;
            let step = host.step_with_preparation(&trainer, 1., |draft, sequence| {
                episode.prepare_draft(draft, sequence)
            })?;
            episode.after_step(&host, &trainer, &step)?;
            Ok(step)
        })();
        let step = match step {
            Ok(step) => step,
            Err(error) => {
                failure = Some(error.to_string());
                break;
            }
        };
        for row in step.decisions {
            rows.entry(row.organism_id.raw()).or_default().push(row);
        }
        for r in host.world().organism_registry().iter() {
            peak_injury = peak_injury.max(r.biochemistry().body.injury);
            minimum_health = minimum_health.min(r.biochemistry().body.health);
            physiology.push(json!({"tick":host.world().tick(),"organism":r.organism_id(),"alive":r.lifecycle().is_alive(),"body":r.biochemistry().body}));
        }
        if rows.values().map(Vec::len).sum::<usize>() > req.decisions
            || host
                .world()
                .organism_registry()
                .iter()
                .all(|r| !r.lifecycle().is_alive())
        {
            break;
        }
    }
    let cp = host.checkpoint(&trainer)?;
    write_cpu_private_checkpoint(&collection, &cp)?;
    write(collection.join("physiology-trace.json"), &physiology)?;
    episode.preserve_collection_recovery(&host, &trainer, &collection)?;
    if let Some(error) = failure {
        write_zstd(output.join("interrupted-replay.json.zst"), &rows)?;
        write(
            output.join("interrupted-error.json"),
            &json!({"error":error,"native_private_prefix_preserved":true}),
        )?;
        return Err(format!("fork collection failed; native prefix retained: {error}").into());
    }
    let decisions = rows
        .values()
        .next()
        .ok_or("fork collected no decisions; private prefix retained")?;
    if rows.len() != 1 {
        return Err("fork requires exactly one collected organism".into());
    }
    let record = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .ok_or("fork organism missing")?;
    let alive = record.lifecycle().is_alive();
    if alive && decisions.len() != req.decisions + 1 {
        return Err(
            "fork technical horizon reached without real bootstrap; private prefix retained".into(),
        );
    }
    if trainer.checkpoint() != before_actor || value.checkpoint() != before_value {
        return Err("fork global heads changed during collection".into());
    }
    let config = PpoConfig {
        epochs: 1,
        temperature: 1.,
        ..Default::default()
    };
    let windows = cpu_training_windows(
        decisions,
        if alive { None } else { Some(record) },
        &trainer,
        &value,
        record.phenotype(),
        version,
        config,
        32,
        8,
    )?;
    let training_rows = windows.iter().map(|w| w.actor.batch.len()).sum::<usize>();
    let episode_receipt =
        episode.finish_collection(&host, &trainer, &rows, training_rows, &collection)?;
    write_zstd(output.join("training-replay.json.zst"), decisions)?;
    let retained_current =
        host.export_retained_neural_for_newborn(&trainer, record.organism_id())?;
    write(
        output.join("collection-retained-neural.json"),
        &retained_current,
    )?;
    let old_p = trainer.phenotype().clone();
    fs::write(
        output.join("phase.txt"),
        "updating-experimental-retained-fork",
    )?;
    let update = train_cpu_residual_cohort(
        &mut trainer,
        &mut value,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        config,
        version,
    )?;
    if update.completed_epochs != 1
        || update.stopped_for_kl
        || trainer.optimizer_step()
            != before_actor
                .optimizer_step
                .checked_add(1)
                .ok_or("Adam age overflow")?
        || value.checkpoint().optimizer_step
            != before_value
                .optimizer_step
                .checked_add(1)
                .ok_or("critic age overflow")?
    {
        return Err(
            "fork did not complete exactly one native actor/value update; prefix retained".into(),
        );
    }
    let trained = trainer.export_candidate(asset.manifest().training_stage())?;
    let (next_p, next_inputs) = body_inputs(host.world(), &trained)?;
    let next_retained = retained_current.rebind_weight_only(&old_p, &next_p)?;
    trainer.rebind_for_next_cohort(next_p, trained.clone())?;
    let handoff = FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        version,
        next_inputs,
        trainer.checkpoint(),
        value.checkpoint(),
    )?;
    handoff.clone().admit()?;
    fs::write(
        output.join("trained.alife-foundation"),
        trained.encode_canonical()?,
    )?;
    write(output.join("training-handoff.json"), &handoff)?;
    write(output.join("actor-checkpoint.json"), &trainer.checkpoint())?;
    write(output.join("value-checkpoint.json"), &value.checkpoint())?;
    write(output.join("retained-neural.json"), &next_retained)?;
    // A separate training-validation seed, never an assessment holdout. This
    // exercises actual inherited state under the updated head on a fresh body.
    seed_history.consume(req.recovery_seed)?;
    write(output.join("seed-state.json"), &seed_history)?;
    let (game, recovery_compat, recovery_proof) = retained::newborn_for_view(
        &trained,
        pc.founder_seed_base,
        &saved,
        req.recovery_seed,
        &mut trainer,
    )?;
    let mut next_host = CpuCanonicalHost::new_cohort(
        game,
        &trainer,
        (req.recovery_seed as u32) ^ ((req.recovery_seed >> 32) as u32),
    )?;
    next_host.inherit_retained_neural(&trainer, &next_retained)?;
    let next_cp = next_host.checkpoint(&trainer)?;
    let mut cold = scope(&recovery_proof, || {
        Ok(CpuCanonicalHost::from_checkpoint(
            next_cp.clone(),
            &trainer,
        )?)
    })?;
    if serde_json::to_vec(&cold.checkpoint(&trainer)?)? != serde_json::to_vec(&next_cp)? {
        return Err("updated retained newborn cold recovery differs".into());
    }
    let first = next_host.step(&trainer, 1.)?;
    let second = cold.step(&trainer, 1.)?;
    if serde_json::to_vec(&first.decisions)? != serde_json::to_vec(&second.decisions)?
        || serde_json::to_vec(&next_host.checkpoint(&trainer)?)?
            != serde_json::to_vec(&cold.checkpoint(&trainer)?)?
    {
        return Err("updated cold recovered next action/world state differs".into());
    }
    let recovered = output.join("recovered-newborn");
    fs::create_dir(&recovered)?;
    write_cpu_private_checkpoint(&recovered, &next_host.checkpoint(&trainer)?)?;
    write(recovered.join("initial-checkpoint.json"), &next_cp)?;
    write(recovered.join("body-compatibility.json"), &recovery_compat)?;
    let after = trainer.checkpoint();
    for i in 0..after.weights.len() {
        let expected =
            before_actor.update_ages[i] + u32::from(before_actor.stage_mask.is_trainable(i));
        if after.update_ages[i] != expected {
            return Err("fork exact coordinate Adam age differs".into());
        }
    }
    write(
        output.join("fork-update.json"),
        &json!({"native_update":update,"training_rows":training_rows,"captured_decisions":decisions.len(),"bootstrap_decisions":usize::from(alive),"initial_actor_age":before_actor.optimizer_step,"actor_age":after.optimizer_step,"initial_critic_age":before_value.optimizer_step,"critic_age":value.checkpoint().optimizer_step,"exact_coordinate_age_increments":true,"changed_genetic_coordinates":before_actor.weights.iter().zip(&after.weights).filter(|(a,b)|a.to_bits()!=b.to_bits()).count(),"episode":episode_receipt.metadata,"alive":alive,"peak_injury":peak_injury,"minimum_health":minimum_health,"stop_semantics":"full requested decision horizon unless natural death or actual technical limit; no injury/health censoring","retained_export":"current CPU active canonical fast/lifetime values duplicated into both newborn banks; original exact two-bank inheritance evidence retained in collector and full parents in experiment","updated_head_native_admission":true,"updated_retained_newborn_cold_roundtrip_and_next_action_exact":true,"global_training_updates":1,"replay_verification_updates":"may recompute update on disposable clone only; not additional training frontier steps","historical_freshness":"unknown","promoted":false,"hardware_gpu_parity":false}),
    )?;
    // Full original source bytes and both donor histories remain untouched.
    pinned(&req.pc_current, &req.expected_pc_sha256)?;
    pinned(&req.cloud_archive, &req.expected_cloud_archive_sha256)?;
    if inspect_native_cpu_archive(&req.cloud_native_root)?.tree_sha256
        != req.expected_cloud_tree_sha256
        || pc.handoff.actor != donor_actor
        || pc.handoff.value != donor_value
    {
        return Err("fork changed original parent or optimizer history".into());
    }
    fs::write(
        output.join("phase.txt"),
        "sealed-experimental-retained-fork",
    )?;
    let manifest = ForkManifest {
        schema: 1,
        kind: "experimental-retained-fork-complete-PC-optimizer-transfer".into(),
        experiment_sha256: req.expected_experiment_sha256.clone(),
        training_seed: req.seed,
        recovery_seed: req.recovery_seed,
        policy_version: version,
        ppo: config,
        artifacts: inventory(&output)?,
    };
    write(output.join("retained-fork.json"), &manifest)?;
    let manifest_sha = hash(&fs::read(output.join("retained-fork.json"))?);
    let verification = verify_pc_donor_retained_training_fork(
        &output,
        &manifest_sha,
        &req.expected_experiment_sha256,
    )?;
    Ok(
        json!({"output":output,"manifest_sha256":manifest_sha,"experiment_sha256":req.expected_experiment_sha256,"verification":verification,"parents_unchanged":true,"not_promoted":true}),
    )
}
