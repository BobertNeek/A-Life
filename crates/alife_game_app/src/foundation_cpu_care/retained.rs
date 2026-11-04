//! Retained-neural newborn assessment, separate from policy-only continuation.
//! Source Current admission and #248 inheritance validate the full saved state.
use super::*;
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::CurrentTrainingCheckpointV1;
use alife_world::*;
use std::{fs, path::Path, time::Instant};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec(value)?)?;
    Ok(())
}

fn body_scope<T>(
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
    action: impl FnOnce() -> Result<T>,
) -> Result<T> {
    if let Some(proof) = proof {
        let (result, used) = with_saved_body_rounding_compatibility(proof.clone(), action)?;
        let result = result?;
        if !used {
            return Err("declared body compatibility proof was not used".into());
        }
        Ok(result)
    } else {
        action()
    }
}

fn newborn(
    source: &CurrentTrainingCheckpointV1,
    seed: u64,
    founder: &mut CpuFoundationTrainer,
) -> Result<(
    CanonicalNewGame,
    Value,
    Option<SavedBodyRoundingCompatibilityV1>,
)> {
    let asset = source
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("source asset missing")?;
    let saved = saved_source_phenotype(source)?;
    newborn_for_view(asset, source.founder_seed_base, &saved, seed, founder)
}

pub(super) fn saved_source_phenotype(
    source: &CurrentTrainingCheckpointV1,
) -> Result<CreaturePhenotype> {
    let raw: Value = serde_json::from_str(
        &source
            .life
            .as_ref()
            .ok_or("source private life missing")?
            .world_json,
    )?;
    Ok(serde_json::from_value(
        raw["world"]["organism_records"][0]["phenotype"].clone(),
    )?)
}

fn newborn_for_view(
    asset: &FoundationWeightAsset,
    body_seed: u64,
    saved: &CreaturePhenotype,
    seed: u64,
    founder: &mut CpuFoundationTrainer,
) -> Result<(
    CanonicalNewGame,
    Value,
    Option<SavedBodyRoundingCompatibilityV1>,
)> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = body_seed;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game = create_canonical_new_game_with_n2048_candidate(&config, asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("newborn missing")?
        .clone();
    let mut phenotype = record.phenotype().clone();
    let generated_turnover = phenotype.body.metabolic_turnover;
    let saved_turnover = saved.body.metabolic_turnover;
    let mut body = saved.body.clone();
    body.metabolic_turnover = generated_turnover;
    if body != phenotype.body
        || generated_turnover
            .to_bits()
            .abs_diff(saved_turnover.to_bits())
            > 1
    {
        return Err(
            "newborn physical phenotype differs beyond bounded metabolic-turnover compatibility"
                .into(),
        );
    }
    phenotype.body.metabolic_turnover = saved_turnover;
    let proof = if saved_turnover.to_bits() == generated_turnover.to_bits() {
        None
    } else {
        Some(
            SavedBodyRoundingCompatibilityV1::propose_for_saved_phenotype(
                record.organism_id(),
                record.genome(),
                &phenotype,
            )?,
        )
    };
    let replacement = body_scope(&proof, || {
        Ok(WorldOrganismRecord::newborn(
            record.organism_id(),
            record.world_entity_id(),
            record.genome().clone(),
            phenotype,
            Tick::ZERO,
        )?)
    })?;
    game.world.replace_organism_record_exact(replacement)?;
    let genome = &record.phenotype().brain_genome;
    let development = foundation_construction_development(
        genome,
        &BrainCapacityClass::n2048(),
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let (body_p, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        asset.clone(),
    )?;
    founder.bind_equivalent_assessment_body(&body_p)?;
    Ok((
        game,
        json!({"field":"body.metabolic_turnover","saved_bits":saved_turnover.to_bits(),"native_generated_bits":generated_turnover.to_bits(),"ulp_difference":saved_turnover.to_bits().abs_diff(generated_turnover.to_bits()),"saved_bits_preserved":true,"other_physical_fields_exact":true,"neural_execution_fields_exact":true,"identity_only_body_bridge":true,"explicit_proof":proof}),
        proof,
    ))
}

pub fn run_retained_cpu_battery(
    current: &Path,
    output: &Path,
    expected_sha256: &str,
    inspect_only: bool,
) -> Result<Value> {
    let source_bytes = fs::read(current)?;
    let source_sha = hash(&source_bytes);
    let executable_sha = hash(&fs::read(std::env::current_exe()?)?);
    if source_sha != expected_sha256 {
        return Err("retained Current differs from explicitly pinned source SHA256".into());
    }
    let (source, source_compatibility) = match crate::read_current_training_checkpoint(current) {
        Ok(source) => (source, None),
        Err(strict_error) => {
            let proposal=crate::propose_current_body_rounding_compatibility(current,expected_sha256).map_err(|e|format!("strict native source admission: {strict_error}; bounded rounding proposal: {e}"))?;
            let source = crate::read_current_training_checkpoint_with_body_rounding(
                current,
                expected_sha256,
                &proposal.proof,
            )?;
            (source, Some(proposal))
        }
    };
    if source.seeds.historical_freshness != "unknown" || source.founder_seed_base != 539363617 {
        return Err("unexpected body seed or historical-freshness declaration".into());
    }
    let source_proof = source_compatibility.as_ref().map(|p| p.proof.clone());
    let retained = body_scope(&source_proof, || {
        crate::foundation_training_cycle::retained_neural_cpu_projection(&source)
    })?;
    let admitted = source.handoff.clone().admit()?;
    let mut founder = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    let value = CpuValueHead::from_checkpoint(admitted.value)?;
    let actor_before = serde_json::to_vec(&founder.checkpoint())?;
    let value_before = serde_json::to_vec(&value.checkpoint())?;
    let mapping = retained.canonical_to_live(founder.phenotype())?;
    let probe = retained.forward_mapping_probe(&founder)?;
    let (game, compatibility, body_proof) = newborn(&source, 202611185878, &mut founder)?;
    let mut initial = CpuCanonicalHost::new_cohort(game, &founder, 1)?;
    initial.inherit_retained_neural(&founder, &retained)?;
    let initial_cp = initial.checkpoint(&founder)?;
    let initial_bytes = serde_json::to_vec(&initial_cp)?;
    let restored = body_scope(&body_proof, || {
        Ok(CpuCanonicalHost::from_checkpoint(
            serde_json::from_slice(&initial_bytes)?,
            &founder,
        )?)
    })?;
    if serde_json::to_vec(&restored.checkpoint(&founder)?)? != initial_bytes {
        return Err("actual retained newborn checkpoint roundtrip changed state".into());
    }
    let qualification = json!({"kind":"retained-neural-newborn-cpu-assessment","source_sha256":source_sha,"executable_sha256":executable_sha,"policy":source.handoff.policy_version,"actor_adam":source.handoff.actor.optimizer_step,"value_adam":source.handoff.value.optimizer_step,"canonical_coordinates":mapping.len(),"live_synapses":retained.topology.total_synapse_count,"acquired_edges":retained.topology.v11_checkpoint.sparse_spans.iter().map(|s|s.edges.len()).sum::<usize>(),"source_full_native_admission":true,"source_both_live_banks_preserved":true,"full_actor_sha256":hash(&actor_before),"full_value_sha256":hash(&value_before),"source_neural_sha256":hash(&serde_json::to_vec(&retained)?),"initial_checkpoint_sha256":hash(&initial_bytes),"actual_checkpoint_roundtrip_exact":true,"forward_probe":probe,"body_compatibility":compatibility,"gpu_parity_demonstrated":false,"prior_reference_executable_comparison_qualified":false,"historical_freshness":"unknown","claim_scope":"within-source retained-neural newborn CPU benchmark; no strict comparison with earlier executables or claim of unseen-seed generalization","personal_memory":"fresh newborn; source personal archive remains intact","offline_actor_optimizer_updates":0,"offline_value_optimizer_updates":0});
    let mut qualification = qualification;
    qualification["source_rounding_compatibility"] = json!(source_compatibility);
    if inspect_only {
        if fs::read(current)? != source_bytes {
            return Err("source changed during inspection".into());
        }
        return Ok(qualification);
    }
    if output.exists() {
        return Err(
            "assessment output already exists; preserve it and inspect rather than restart".into(),
        );
    }
    fs::create_dir_all(output)?;
    write(output.join("qualification.json"), &qualification)?;
    write(output.join("source-seeds.json"), &source.seeds)?;
    write(
        output.join("external-seed-exclusions.json"),
        &json!({"assessment_only":[202611185878u64,202611185897u64],"cloud_training_consumed_through":202611185957u64,"new_exact_cloud_exclusions":[[202611185943u64,202611185957u64]],"independent_pc_cloud_reserved":[[2026110411643001u64,2026110411643004u64]],"prior_cloud_ranges":"remain excluded; lower endpoints not reconstructed here","new_training_allocations":0}),
    )?;
    let mut results = Vec::new();
    for (label, lesson, condition) in [
        (
            "held",
            FoundationTeacherLesson::EatHeldFood,
            FoundationFoodNeedCondition::NewbornV1,
        ),
        (
            "reachable",
            FoundationTeacherLesson::EatReachableFood,
            FoundationFoodNeedCondition::NewbornV1,
        ),
        (
            "feeding_newborn",
            FoundationTeacherLesson::Feeding,
            FoundationFoodNeedCondition::NewbornV1,
        ),
        (
            "feeding_aged",
            FoundationTeacherLesson::Feeding,
            FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
        ),
        (
            "obstacles",
            FoundationTeacherLesson::ObstacleNavigation,
            FoundationFoodNeedCondition::NewbornV1,
        ),
        (
            "relocated_aged",
            FoundationTeacherLesson::RelocatedFood,
            FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
        ),
    ] {
        for seed in 202611185878..=202611185897 {
            if source.seeds.contains(seed) {
                return Err("assessment seed overlaps retained parent's known exclusion".into());
            }
            let trial = output.join(format!("{label}-{seed}"));
            let result = run_trial(
                &source,
                Some(&retained),
                &mut founder,
                &value,
                &trial,
                seed,
                lesson,
                condition,
                128,
            );
            let mut record = match result {
                Ok(v) => v,
                Err(e) => {
                    json!({"condition":label,"seed":seed,"complete":false,"lesson_completed":false,"error":e.to_string(),"preserve_all_prefix_artifacts":true})
                }
            };
            record["condition"] = json!(label);
            println!(
                "{}",
                json!({"condition":label,"seed":seed,"complete":record["complete"],"lesson_completed":record["lesson_completed"],"error":record["error"]})
            );
            results.push(record);
            write(output.join("results.json"), &results)?;
        }
    }
    if serde_json::to_vec(&founder.checkpoint())? != actor_before
        || serde_json::to_vec(&value.checkpoint())? != value_before
        || fs::read(current)? != source_bytes
    {
        return Err("assessment changed source or complete offline optimizer state".into());
    }
    let result = json!({"qualification":qualification,"trials":results,"source_unchanged":true,"full_actor_value_unchanged":true});
    write(output.join("battery.json"), &result)?;
    Ok(result)
}

/// Small within-parent control: the actual retained overlay versus the native
/// fresh-neural constructor. This is not an older-parent or GPU comparison.
pub fn run_retained_cpu_fresh_controls(
    current: &Path,
    output: &Path,
    expected_sha256: &str,
) -> Result<Value> {
    let source_bytes = fs::read(current)?;
    if hash(&source_bytes) != expected_sha256 {
        return Err("control source differs from its explicit SHA256 pin".into());
    }
    let (source, source_proof) = match crate::read_current_training_checkpoint(current) {
        Ok(source) => (source, None),
        Err(_) => {
            let proposal =
                crate::propose_current_body_rounding_compatibility(current, expected_sha256)?;
            let source = crate::read_current_training_checkpoint_with_body_rounding(
                current,
                expected_sha256,
                &proposal.proof,
            )?;
            (source, Some(proposal.proof))
        }
    };
    let retained = body_scope(&source_proof, || {
        crate::foundation_training_cycle::retained_neural_cpu_projection(&source)
    })?;
    let admitted = source.handoff.clone().admit()?;
    let mut founder = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    let value = CpuValueHead::from_checkpoint(admitted.value)?;
    retained.canonical_to_live(founder.phenotype())?;
    let actor_before = serde_json::to_vec(&founder.checkpoint())?;
    let value_before = serde_json::to_vec(&value.checkpoint())?;
    if output.exists() {
        return Err("control output already exists; preserve and inspect".into());
    }
    for seed in 202611185878..=202611185879 {
        if source.seeds.contains(seed) {
            return Err("assessment-only control seed overlaps known PC training history".into());
        }
    }
    fs::create_dir_all(output)?;
    write(
        output.join("control-intent.json"),
        &json!({
            "kind":"within-parent-retained-versus-native-fresh-neural",
            "policy":source.handoff.policy_version,"source_sha256":expected_sha256,
            "actor_adam":source.handoff.actor.optimizer_step,"value_adam":source.handoff.value.optimizer_step,
            "assessment_only_seeds":[202611185878u64,202611185879u64],"scored_decisions":128,"bootstrap_decisions":1,
            "retained":"exact source somatic weights, sparse structure, native/saved dendrites and predictor; new personal memory and body",
            "native_fresh":"same actual global actor/value and complete Adam; native fresh somatic/topology/predictor state, including native default dendrites",
            "temperature":1,"teacher_and_semantic_prior":false,"waking_plasticity":"ordinary in both views",
            "offline_optimizer_updates":0,"new_training_allocations":0,"gpu_parity":false,
            "limits":"Eight diagnostic trials on reused assessment seeds; no older-parent, GPU, independent native waking-update or generalization claim. World semantics belong to this executable."
        }),
    )?;
    let mut results = Vec::new();
    for (condition_label, lesson, condition) in [
        (
            "held",
            FoundationTeacherLesson::EatHeldFood,
            FoundationFoodNeedCondition::NewbornV1,
        ),
        (
            "feeding_aged",
            FoundationTeacherLesson::Feeding,
            FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
        ),
    ] {
        for seed in 202611185878..=202611185879 {
            for (view, projection) in [("retained", Some(&retained)), ("native_fresh", None)] {
                let trial = output.join(format!("{condition_label}-{seed}-{view}"));
                let mut record = match run_trial(
                    &source,
                    projection,
                    &mut founder,
                    &value,
                    &trial,
                    seed,
                    lesson,
                    condition,
                    128,
                ) {
                    Ok(record) => record,
                    Err(error) => {
                        json!({"complete":false,"lesson_completed":false,"error":error.to_string(),"preserve_all_prefix_artifacts":true})
                    }
                };
                record["view"] = json!(view);
                record["condition"] = json!(condition_label);
                record["seed"] = json!(seed);
                println!(
                    "{}",
                    json!({"condition":condition_label,"seed":seed,"view":view,"complete":record["complete"],"lesson_completed":record["lesson_completed"],"error":record["error"]})
                );
                results.push(record);
                write(output.join("results.json"), &results)?;
            }
        }
    }
    if fs::read(current)? != source_bytes
        || serde_json::to_vec(&founder.checkpoint())? != actor_before
        || serde_json::to_vec(&value.checkpoint())? != value_before
    {
        return Err(
            "fresh-neural controls changed source or complete offline optimizer history".into(),
        );
    }
    let result = json!({"trials":results,"source_unchanged":true,"complete_actor_value_unchanged":true,"new_training_allocations":0,"comparative_parent_verdict_qualified":false});
    write(output.join("comparison.json"), &result)?;
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn run_trial(
    source: &CurrentTrainingCheckpointV1,
    retained: Option<&CpuRetainedNeuralInheritance>,
    founder: &mut CpuFoundationTrainer,
    value: &CpuValueHead,
    output: &Path,
    seed: u64,
    lesson: FoundationTeacherLesson,
    condition: FoundationFoodNeedCondition,
    scored_limit: usize,
) -> Result<Value> {
    let asset = source
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("source asset missing")?;
    let saved = saved_source_phenotype(source)?;
    run_view_trial(
        &AssessmentView {
            asset,
            compiler_inputs: &source.handoff.compiler_inputs,
            body_seed: source.founder_seed_base,
            saved_body: &saved,
            identity: json!({"kind":"native-source-parent","policy":source.handoff.policy_version}),
            archive_heads: true,
        },
        retained,
        founder,
        Some(value),
        output,
        &AssessmentCase {
            seed,
            lesson,
            condition,
            scored_limit,
        },
    )
}

pub(super) struct AssessmentView<'a> {
    pub asset: &'a FoundationWeightAsset,
    pub compiler_inputs: &'a PhenotypeCompilerInputs,
    pub body_seed: u64,
    pub saved_body: &'a CreaturePhenotype,
    pub identity: Value,
    /// Parent controls archive their real heads. Inference-only candidates
    /// export neither the evaluator's internal optimizer container nor a critic.
    pub archive_heads: bool,
}
pub(super) struct AssessmentCase {
    pub seed: u64,
    pub lesson: FoundationTeacherLesson,
    pub condition: FoundationFoodNeedCondition,
    pub scored_limit: usize,
}

pub(super) fn run_view_trial(
    view: &AssessmentView<'_>,
    retained: Option<&CpuRetainedNeuralInheritance>,
    founder: &mut CpuFoundationTrainer,
    value: Option<&CpuValueHead>,
    output: &Path,
    case: &AssessmentCase,
) -> Result<Value> {
    let AssessmentCase {
        seed,
        lesson,
        condition,
        scored_limit,
    } = *case;
    let started = Instant::now();
    let (_, mut episode) = prepare_care_episode(CpuCareRequest {
        previous: None,
        output: output.into(),
        seed,
        decisions: scored_limit,
        temperature: 1.,
        lesson,
        food_need_condition: condition,
        seed_history: None,
        known_exclusions: false,
        record_diagnostic_replay: false,
        fixture: true,
        layout: None,
        food_offset: None,
    })?;
    episode.assessment = true;
    episode.validate_policy(founder)?;
    let (game, compatibility, _) =
        newborn_for_view(view.asset, view.body_seed, view.saved_body, seed, founder)?;
    let (mut host, ()) = CpuCanonicalHost::new_prepared_cohort(
        game,
        founder,
        (seed as u32) ^ ((seed >> 32) as u32),
        |world| episode.prepare_world(world),
    )?;
    if let Some(retained) = retained {
        host.inherit_retained_neural(founder, retained)?;
    }
    fs::create_dir(output)?;
    fs::write(
        output.join("collection-policy.alife-foundation"),
        view.asset.encode_canonical()?,
    )?;
    write(
        output.join("collection-compiler-inputs.json"),
        view.compiler_inputs,
    )?;
    write(output.join("assessment-model.json"), &view.identity)?;
    if view.archive_heads {
        write(
            output.join("collection-actor-checkpoint.json"),
            &founder.checkpoint(),
        )?;
        write(
            output.join("initial-value-checkpoint.json"),
            &value
                .ok_or("parent assessment lacks its actual value head")?
                .checkpoint(),
        )?;
    }
    write(
        output.join("initial-checkpoint.json"),
        &host.checkpoint(founder)?,
    )?;
    let mut rows = BTreeMap::<u64, Vec<CpuCollectedDecision>>::new();
    let mut failure = None;
    for _ in 0..(scored_limit + 1) * 256 {
        let step = (|| -> Result<CpuCanonicalStep> {
            episode.before_step(&mut host, founder)?;
            let step = host.step_with_preparation(founder, 1., |draft, sequence| {
                episode.prepare_draft(draft, sequence)
            })?;
            episode.after_step(&host, founder, &step)?;
            Ok(step)
        })();
        match step {
            Ok(step) => {
                for row in step.decisions {
                    rows.entry(row.organism_id.raw()).or_default().push(row);
                }
            }
            Err(e) => {
                failure = Some(e.to_string());
                break;
            }
        }
        if rows.values().map(Vec::len).sum::<usize>() > scored_limit
            || host
                .world()
                .organism_registry()
                .iter()
                .all(|r| !r.lifecycle().is_alive())
        {
            break;
        }
    }
    let decisions = rows.values().map(Vec::len).sum::<usize>();
    let scored = decisions.min(scored_limit);
    let alive = host
        .world()
        .organism_registry()
        .iter()
        .all(|r| r.lifecycle().is_alive());
    let checkpoint = host.checkpoint(founder)?;
    write_cpu_private_checkpoint(output, &checkpoint)?;
    episode.preserve_collection_recovery(&host, founder, output)?;
    let mut receipt = episode.finish_collection(&host, founder, &rows, scored, output)?;
    if !view.archive_heads {
        receipt.artifacts.retain(|name| {
            !matches!(
                name.as_str(),
                "collection-actor-checkpoint.json" | "initial-value-checkpoint.json"
            )
        });
        receipt.metadata["persistent_learning"] = json!("fresh-private-life-with-ordinary-waking-plasticity; genetic-weights-frozen; candidate-has-no-optimizer-or-critic; not-resumable");
        receipt.metadata["inference_only_candidate"] = json!(true);
        write(output.join("care-episode.json"), &receipt.metadata)?;
    }
    let organisms=host.world().organism_registry().iter().map(|r|json!({"organism":r.organism_id(),"alive":r.lifecycle().is_alive(),"biology":r.biochemistry()})).collect::<Vec<_>>();
    let complete = decisions == scored_limit + 1 && alive && failure.is_none();
    let result = json!({"lesson":lesson,"food_need_condition":condition,"seed":seed,"body_seed":view.body_seed,"policy":view.identity["policy"],"model":view.identity,"complete":complete,"lesson_completed":receipt.metadata["lesson_completed"],"scored_decisions":scored,"captured_decisions":decisions,"bootstrap_decisions":decisions.saturating_sub(scored_limit),"error":failure,"alive":alive,"organisms":organisms,"seconds":started.elapsed().as_secs_f64(),"body_compatibility":compatibility,"episode":receipt.metadata,"teacher_present":false,"semantic_prior_enabled":false,"private_waking_learning":"ordinary","offline_actor_optimizer_updates":0,"offline_value_optimizer_updates":0});
    write(output.join("retained-assessment.json"), &result)?;
    Ok(result)
}
