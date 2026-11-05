//! Failure replay labelled only by real native success in an input-matched sampler fork.
use super::*;
mod care_retention;
mod early_stop;
pub use care_retention::{compare_care_retention, preflight_care_retention};
pub use early_stop::compare_witnessed_early_stop;
struct Pair {
    failure: CpuCollectedDecision,
    target: u32,
    noun: usize,
}
fn action_mask(actor: &mut CpuFoundationTrainer) -> Result<usize> {
    let before = actor.checkpoint();
    let ids=actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(d) if d.head()==DecoderHeadKind::ActionCandidate).then_some(i as u32)).collect::<Vec<_>>();
    actor.set_stage_mask(StageTrainableMask::from_synapse_indices(
        actor.phenotype(),
        &ids,
    )?)?;
    let mut after = actor.checkpoint();
    after.stage_mask = before.stage_mask.clone();
    if after != before {
        return Err("action mask reset numeric/optimizer history".into());
    }
    Ok(ids.len())
}
fn emit(h: &mut CpuCanonicalHost, id: OrganismId, requested: Lesson, arm: Arm) -> Result<()> {
    if arm == Arm::NoSpeech {
        return Ok(());
    }
    let lesson = if arm == Arm::Shuffled {
        if requested == Lesson::GetFood {
            Lesson::GetToy
        } else {
            Lesson::GetFood
        }
    } else {
        requested
    };
    let pos = speech_position(h.world())?;
    h.world_mut().emit_player_tokens(
        Some(id),
        pos,
        lesson
            .words()
            .into_iter()
            .map(LanguageTokenId::new)
            .collect::<std::result::Result<_, _>>()?,
    )?;
    Ok(())
}
fn collect_pairs(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    traces: &mut impl Write,
) -> Result<(Vec<Pair>, Value)> {
    let mut pairs = vec![];
    let mut counts = [0usize; 2];
    let mut attempts = 0usize;
    let mut recorded_match = false;
    for case in 0..unseen_transfer::SEEDS.len() {
        let (mut base, scene) = unseen_transfer::prepared_case(actor, asset, c.seed, case)?;
        for _ in 0..2 {
            base.step(actor, 1.)?;
        }
        unseen_transfer::place_case(base.world_mut(), scene, case)?;
        let id = base.world().organism_entity_ids()[0].0;
        for (noun, goal) in [Lesson::GetFood, Lesson::GetToy].into_iter().enumerate() {
            let mut h = base.snapshot();
            for tick in 0..4 {
                if tick % 2 == 0 {
                    emit(&mut h, id, goal, arm)?;
                }
                let pre = h.snapshot();
                let before = owners(pre.world(), scene);
                let failure = h.step(actor, 1.)?.decisions.remove(0);
                let after = owners(h.world(), scene);
                let event = physical_event(&failure, scene, before, after)?;
                if arm == Arm::Grounded && case == 4 && noun == 0 && tick == 0 {
                    if identity(&failure)?
                        != "0b167b2922ad9da79d9973ca62e91957e4c116ecc35b135374405399aef6f889"
                    {
                        return Err("captured V19 failure did not reproduce".into());
                    }
                    recorded_match = true;
                }
                if event.as_ref().is_some_and(|e| e.lesson == goal) {
                    break;
                }
                let target = candidate_mask(
                    failure.patch.pre_action().perception(),
                    &[(HeadlessActionIds::GRAB, goal.target(scene))],
                );
                if target.count_ones() != 1 || target & failure.action.representative_mask != target
                {
                    continue;
                }
                let q = readout::single(actor, &failure);
                let ev = actor.evaluate_replay(&q)?;
                let (margin, _, _) = physical_margin(
                    &ev.candidate_logits[0],
                    failure.action.representative_mask,
                    target,
                )?;
                if margin <= 0. {
                    continue;
                }
                let admitted_before = pairs.len();
                let source_private = identity(&pre.checkpoint(actor)?)?;
                for attempt in 1..=32u32 {
                    attempts += 1;
                    let fork_seed = (unseen_transfer::SEEDS[case] as u32)
                        ^ 0x7265_706c
                        ^ attempt.wrapping_mul(0x9e37_79b9);
                    let mut fork = pre.fork_sampling_stream(fork_seed)?;
                    let success = fork.step(actor, 1.)?.decisions.remove(0);
                    let owned = owners(fork.world(), scene);
                    if physical_event(&success, scene, before, owned)?
                        .as_ref()
                        .is_none_or(|e| e.lesson != goal)
                    {
                        continue;
                    }
                    if u32::from(success.action.representative) != target.trailing_zeros() {
                        continue;
                    }
                    if failure.initial != success.initial
                        || failure.replay_tick != success.replay_tick
                        || failure.sampling_counter != success.sampling_counter
                    {
                        return Err("success witness changed neural inputs/state/candidates".into());
                    }
                    if identity(&pre.checkpoint(actor)?)? != source_private {
                        return Err("sampling exploration mutated source host".into());
                    }
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"witnessed_failure_replay_training","arm":arm,"case":case,"world_seed":unseen_transfer::SEEDS[case],"requested":goal,"scene":scene,"tick_after_cue":tick,"source_pre_private_sha256":source_private,"sampling_seed_success":fork_seed,"source_rng_counter_retained":true,"failure":failure,"success_witness":success,"failure_owners_before":before,"failure_owners_after":after,"success_owners_after":owned,"native_target_mask":target,"initial_physical_margin":margin,"same_native_neural_input_state_and_candidates":true,"failure_reward_not_replaced_by_success_reward":true})
                    )?;
                    pairs.push(Pair {
                        failure,
                        target,
                        noun,
                    });
                    counts[noun] += 1;
                    break;
                }
                if pairs.len() > admitted_before {
                    break;
                }
            }
        }
    }
    Ok((
        pairs,
        json!({"native_success_witness_pairs_by_noun":counts,"native_sampling_exploration_attempts":attempts,"recorded_v19_failure_exact":recorded_match,"retired_training_world_seeds":unseen_transfer::SEEDS,"max_source_ticks_per_case_command":4,"max_sampling_forks_per_failed_state":32,"no_forced_actions":true,"scripted_command_tutor_uses_ordinary_player_source":arm!=Arm::NoSpeech}),
    ))
}
fn gradient(actor: &CpuFoundationTrainer, pairs: &[Pair]) -> Result<(Vec<f32>, Value)> {
    let mut counts = [0usize; 2];
    for p in pairs {
        counts[p.noun] += 1;
    }
    let classes = counts.iter().filter(|n| **n > 0).count();
    if classes == 0 {
        return Err("no actual success-witnessed replay pairs".into());
    }
    let (mut sum, mut loss, mut active) = (vec![0.; actor.weights().len()], 0f64, 0usize);
    for p in pairs {
        let q = readout::single(actor, &p.failure);
        let ev = actor.evaluate_replay(&q)?;
        let (hinge, target, rival) = physical_margin(
            &ev.candidate_logits[0],
            p.failure.action.representative_mask,
            p.target,
        )?;
        if hinge > 0. {
            let scale = 1. / classes as f32 / counts[p.noun] as f32;
            let mut adj = vec![0.; ev.candidate_logits[0].len()];
            adj[target] -= scale;
            adj[rival] += scale;
            let g = actor.probe_replay_gradients(&q, &[adj])?;
            for (a, b) in sum.iter_mut().zip(g.gradients) {
                *a += b;
            }
            loss += f64::from(hinge * scale);
            active += 1;
        }
    }
    let mask = actor.checkpoint().stage_mask;
    let norm = sum
        .iter()
        .enumerate()
        .filter(|(i, _)| mask.is_trainable(*i))
        .map(|(_, g)| f64::from(*g).powi(2))
        .sum::<f64>()
        .sqrt();
    Ok((
        sum,
        json!({"success_witnessed_pairs_by_noun":counts,"active_corrective_rows":active,"native_margin_loss":loss,"trainable_native_gradient_l2":norm,"speech_and_other_nonaction_parameters_frozen":true}),
    ))
}
pub fn continue_witnessed_replay(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run(source, output, c, false)
}
pub fn preflight_witnessed_replay(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run(source, output, c, true)
}
fn run(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
    preflight_only: bool,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || c.rounds != 1
        || c.epochs != 8
        || c.decisions != 24
    {
        return Err(
            "witnessed replay requires scripted V18 source, rounds1 epochs8 decisions24".into(),
        );
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if old["kind"] != "experimental-report-choice-noun-margin-v18"
        || old["config"]["seed"] != c.seed
    {
        return Err("not matching public V18 source".into());
    }
    let recorded = old["arms"]
        .as_array()
        .ok_or("no source arms")?
        .iter()
        .find(|a| a["arm"] == "grounded")
        .ok_or("no grounded source")?;
    fs::create_dir(output)?;
    let started = Instant::now();
    let r = read_recovery(&source.join("recovery-Grounded"))?;
    if r.inputs.experimental_language_input()
        != Some(ExperimentalLanguageInputV1::LinearWordReadoutV3)
    {
        return Err("not public V3 source".into());
    }
    let phenotype = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
    let asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
    let original = CpuFoundationTrainer::from_checkpoint(phenotype, asset.clone(), r.actor)?;
    let common = identity(&original.checkpoint())?;
    if recorded["actor_sha256"] != common {
        return Err("source actor differs from measured".into());
    }
    let private =
        identity(&CpuCanonicalHost::from_checkpoint(r.host, &original)?.checkpoint(&original)?)?;
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("native-trace.jsonl.zst"))?,
        3,
    )?;
    let mut datasets = vec![];
    for arm in [Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        eprintln!("witnessed {arm:?}: collect native failure/success forks");
        let (pairs, receipt) = collect_pairs(&original, &asset, c, arm, &mut trace)?;
        let mut probe = CpuFoundationTrainer::from_checkpoint(
            original.phenotype().clone(),
            asset.clone(),
            original.checkpoint(),
        )?;
        action_mask(&mut probe)?;
        let (_, g) = gradient(&probe, &pairs)?;
        if g["trainable_native_gradient_l2"]
            .as_f64()
            .is_none_or(|n| n <= 1e-5)
            || g["active_corrective_rows"].as_u64() == Some(0)
        {
            return Err("no nonzero intended native corrective gradient before training".into());
        }
        if arm == Arm::Grounded
            && (g["success_witnessed_pairs_by_noun"][0] == 0
                || g["success_witnessed_pairs_by_noun"][1] == 0
                || receipt["recorded_v19_failure_exact"] != true)
        {
            return Err(
                "grounded preflight lacks both actual physical nouns or exact failure".into(),
            );
        }
        eprintln!("witnessed {arm:?} preflight {g}; collection {receipt}");
        datasets.push((arm, pairs, receipt, g));
    }
    trace.finish()?;
    let preflight = json!({"kind":"native-success-witnessed-replay-preflight-v1","source_actor_sha256":common,"source_optimizer_step":original.optimizer_step(),"source_actor_unmodified":identity(&original.checkpoint())?==common,"arms":datasets.iter().map(|(arm,pairs,receipt,g)|json!({"arm":arm,"pairs":pairs.len(),"collection":receipt,"corrective_gradient_before_training":g})).collect::<Vec<_>>(),"no_parameter_update_or_scored_assessment":true});
    write_json(&output.join("preflight.json"), &preflight)?;
    if preflight_only {
        write_json(&output.join("results.json"), &preflight)?;
        return Ok(preflight);
    }
    let mut arms = vec![];
    for arm in [Arm::Untreated, Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        let mut actor = CpuFoundationTrainer::from_checkpoint(
            original.phenotype().clone(),
            asset.clone(),
            original.checkpoint(),
        )?;
        let before = actor.checkpoint();
        let mut next = r.next_gradient.clone();
        let mut updates = vec![];
        let mut collections = vec![];
        let mut preflight = Value::Null;
        if arm != Arm::Untreated {
            let (_, pairs, receipt, g) = datasets
                .iter()
                .find(|(a, _, _, _)| *a == arm)
                .ok_or("no frozen dataset")?;
            action_mask(&mut actor)?;
            let masked = actor.checkpoint();
            preflight = g.clone();
            collections.push(receipt.clone());
            for epoch in 0..c.epochs {
                let (gradient, objective) = gradient(&actor, pairs)?;
                let frozen = actor.checkpoint();
                let norm = actor.apply_gradients(&gradient)?;
                preserve_frozen(&frozen, &actor.checkpoint())?;
                updates.push(json!({"epoch":epoch,"step":actor.optimizer_step(),"objective":objective,"gradient_norm":norm,"all_frozen_coordinates_exact":true}));
                next = gradient;
            }
            preserve_frozen(&masked, &actor.checkpoint())?;
        }
        let mut current_asset = asset.clone();
        if arm != Arm::Untreated {
            current_asset = export(&mut actor, c.seed)?;
        }
        let plan = AssessmentPlan {
            recovery_sleep: true,
            collect_utterance_timing: true,
            factorial_grounding: true,
            paired_request_focus: true,
            linear_readout: true,
            balanced_grounding_v1: false,
            balanced_grounding_v2: true,
            event_confirmed_speech_v1: true,
            unseen_report_transfer_v1: true,
            physical_before_speech_v1: true,
            captured_failure_replay_v1: true,
            replay_early_stop_v1: false,
            care_retention_v1: false,
            frozen_causal_v1: false,
        };
        eprintln!("witnessed {arm:?}: fresh native assessment and cold repeat");
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &current_asset,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan),
            },
            next,
        )?;
        arms.push(json!({"arm":arm,"common_source_actor_sha256":common,"source_private_sha256":private,"source_step":before.optimizer_step,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"collections":collections,"corrective_preflight":preflight,"updates":updates,"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold,"source_history_retained":true,"only_action_candidate_stage_mask_changed_on_disposable_trained_copy":arm!=Arm::Untreated}));
    }
    let result = json!({"kind":"experimental-success-witnessed-failure-replay-v21","execution_domain":"native-cpu","config":c,"source_report_sha256":identity(&old)?,"held_out_world_seeds":unseen_transfer::REPLAY_SEEDS,"independent_seed_count":6,"pre_post_are_paired_not_independent":true,"teacher_and_player_cue_ticks":INSTRUCTION_TICKS,"evaluation_ticks":EVALUATION_TICKS,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"semantic_prior_enabled":false,"teacher":"scripted training command tutor via ordinary spatial Player input; no learned provider","limitations":["Known V19 failure worlds/layouts are retired into training. Fresh seeds/IDs reuse the same six physical geometry tuples; one founder, familiar words, not independent training replicas.","Training explicitly forks only the sampling stream and native authority seals on disposable copies. Success labels require actual native ownership changes in a neural-input-matched fork; original failed rewards are never replaced.","Only ActionCandidate weights train through the native margin/Adam path; all other weights/moments/integer ages freeze. The stage mask changes explicitly on disposable trained copies.","Evaluation uses ordinary unmodified sampling/cooldown with no forks, forced actions, teacher/prior or output filtering. Native biology/private learning stays enabled.","Sleep conditions share the prepared seed parent; not independent N12, continuous post-command life, spontaneous sleep or an awake-delay causal control.","No comprehension/promotion, default/live brain change, original checkpoint overwrite or Luna401 retry."]});
    write_json(&output.join("results.json"), &result)?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_mask_preserves_source_history_and_freezes_speech_optimizer() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let mut actor = new_actor_profile(seed, &asset, true).unwrap();
        action_mask(&mut actor).unwrap();
        let before = actor.checkpoint();
        for (i, s) in actor.phenotype().synapses().iter().enumerate() {
            if matches!(s.kind(),CompiledSynapseKind::Decoder(d) if d.head()==DecoderHeadKind::SpeechPayload)
            {
                assert!(!before.stage_mask.is_trainable(i));
            }
        }
        actor
            .apply_gradients(&vec![1.; before.weights.len()])
            .unwrap();
        let after = actor.checkpoint();
        preserve_frozen(&before, &after).unwrap();
        assert_eq!(after.optimizer_step, before.optimizer_step + 1);
        assert!(before
            .weights
            .iter()
            .zip(&after.weights)
            .any(|(a, b)| a.to_bits() != b.to_bits()));
    }
    #[test]
    fn sampler_fork_keeps_source_and_real_neural_context_and_cold_authority() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let (mut source, scene) = unseen_transfer::prepared_case(&actor, &asset, seed, 0).unwrap();
        unseen_transfer::place_case(source.world_mut(), scene, 0).unwrap();
        let id = source.world().organism_entity_ids()[0].0;
        emit(&mut source, id, Lesson::GetFood, Arm::Grounded).unwrap();
        let before = source.checkpoint(&actor).unwrap();
        let mut fork = source.fork_sampling_stream(77).unwrap();
        let after = fork.checkpoint(&actor).unwrap();
        assert_eq!(
            identity(&source.checkpoint(&actor).unwrap()).unwrap(),
            identity(&before).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&before.residents).unwrap(),
            serde_json::to_value(&after.residents).unwrap()
        );
        assert_eq!(before.sampling_counter, after.sampling_counter);
        assert_eq!(after.sampling_seed, 77);
        let restored = CpuCanonicalHost::from_checkpoint(after, &actor).unwrap();
        assert_eq!(
            identity(&restored.checkpoint(&actor).unwrap()).unwrap(),
            identity(&fork.checkpoint(&actor).unwrap()).unwrap()
        );
        let a = source
            .snapshot()
            .step(&actor, 1.)
            .unwrap()
            .decisions
            .remove(0);
        let b = fork.step(&actor, 1.).unwrap().decisions.remove(0);
        assert_eq!(a.initial, b.initial);
        assert_eq!(a.replay_tick.encoded_inputs, b.replay_tick.encoded_inputs);
        assert_eq!(
            a.replay_tick.effective_weight_offsets,
            b.replay_tick.effective_weight_offsets
        );
        assert_eq!(a.replay_tick.candidates, b.replay_tick.candidates);
    }
}
