//! Four actual cue interventions per matched native parent; no optimizer calls.
use super::*;

#[derive(Clone, Copy, Serialize)]
pub(super) struct Case {
    pub seed: u64,
    pub fixture: usize,
    pub padding: usize,
    pub stratum: &'static str,
}
pub(super) const CASES: [Case; 6] = [
    Case {
        seed: 10_210_057,
        fixture: 1,
        padding: 884,
        stratum: "known_food_positive",
    },
    Case {
        seed: 10_309_219,
        fixture: 2,
        padding: 915,
        stratum: "known_noun_positive",
    },
    Case {
        seed: 16_104_687,
        fixture: 0,
        padding: 1409,
        stratum: "fresh",
    },
    Case {
        seed: 16_309_347,
        fixture: 2,
        padding: 1450,
        stratum: "fresh",
    },
    Case {
        seed: 16_508_317,
        fixture: 4,
        padding: 1491,
        stratum: "fresh",
    },
    Case {
        seed: 16_610_541,
        fixture: 5,
        padding: 1532,
        stratum: "fresh",
    },
];

fn without_audio(mut cp: Value) -> Result<Value> {
    let world = cp
        .pointer_mut("/world/world")
        .and_then(Value::as_object_mut)
        .ok_or("missing portable world")?;
    if world.remove("next_utterance_id").is_none() || world.remove("audible_utterances").is_none() {
        return Err("missing exact audio-only intervention fields".into());
    }
    Ok(cp)
}
pub(super) fn before_audio(host: &CpuCanonicalHost, actor: &CpuFoundationTrainer) -> Result<Value> {
    Ok(serde_json::to_value(host.checkpoint(actor)?)?)
}
pub(super) fn after_audio(
    before: Value,
    host: &CpuCanonicalHost,
    actor: &CpuFoundationTrainer,
) -> Result<Value> {
    let after = serde_json::to_value(host.checkpoint(actor)?)?;
    let before_hash = identity(&before)?;
    let after_hash = identity(&after)?;
    let a = without_audio(before)?;
    let b = without_audio(after)?;
    if a != b {
        return Err(
            "speech intervention changed nonaudio physical/private/RNG checkpoint state".into(),
        );
    }
    Ok(
        json!({"before_audio_full_checkpoint_sha256":before_hash,"after_audio_full_checkpoint_sha256":after_hash,"all_nonaudio_physical_private_optimizer_binding_and_rng_fields_identical":true,"nonaudio_checkpoint_sha256":identity(&a)?,"only_excluded_fields":["world.world.next_utterance_id","world.world.audible_utterances"]}),
    )
}
pub(super) fn input(
    row: &CpuCollectedDecision,
    cue: Option<Lesson>,
    setup: Value,
) -> Result<Value> {
    let sensory = row.patch.pre_action().perception().sensory();
    let player = sensory
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .filter(|t| t.source_kind == UtteranceSourceKind::Player)
        .map(|t| t.token_id)
        .collect::<Vec<_>>();
    let expected = cue.map_or(vec![], |c| {
        c.words().into_iter().map(u32::from).collect::<Vec<_>>()
    });
    if player != expected {
        return Err("first actual heard player tokens differ from delivered intervention".into());
    }
    let heard = sensory
        .language_context
        .heard_tokens
        .iter()
        .take(2)
        .map(|t| t.as_ref().map(|h| h.token_id))
        .collect::<Vec<_>>();
    let broadcast =
        cue.is_some() && heard == expected.iter().copied().map(Some).collect::<Vec<_>>();
    let lanes = sensory.language_prior_neural_lanes();
    if broadcast {
        for (slot, word) in expected.iter().enumerate() {
            for bit in 0..8 {
                if (lanes[slot * 8 + bit] > 0.) != (word & (1 << bit) != 0)
                    || lanes[slot * 8 + bit] == 0.
                {
                    return Err(
                        "native heard word bit/confidence lanes do not match actual cue".into(),
                    );
                }
            }
        }
    }
    Ok(
        json!({"setup":setup,"delivered_tokens":expected,"actual_heard_player_tokens":player,"first_two_heard_token_slots":heard,"command_occupies_both_broadcast_token_slots":broadcast,"native_raw_language_lanes_first16":&lanes[..16],"encoded_inputs_sha256":identity(&row.replay_tick.encoded_inputs)?,"initial_neural_state_sha256":identity(&row.initial)?,"sampling_counter":row.sampling_counter}),
    )
}
pub(super) fn compare_inputs(
    actor: &CpuFoundationTrainer,
    first: &[(Option<Lesson>, CpuCollectedDecision)],
) -> Result<Value> {
    let targets = actor
        .phenotype()
        .sensor_encoder()
        .assignments()
        .iter()
        .filter(|a| {
            a.source_group() == SensorEncoderSourceGroup::HeardLanguage && a.source_index() < 16
        })
        .map(|a| a.target_neuron() as usize)
        .collect::<std::collections::BTreeSet<_>>();
    let mut pairs = vec![];
    for a in 0..first.len() {
        for b in a + 1..first.len() {
            let (ca, ra) = &first[a];
            let (cb, rb) = &first[b];
            let changed = ra
                .replay_tick
                .encoded_inputs
                .iter()
                .zip(&rb.replay_tick.encoded_inputs)
                .enumerate()
                .filter(|(_, (a, b))| a.to_bits() != b.to_bits())
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            pairs.push(json!({"cue_a":ca,"cue_b":cb,"changed_encoded_input_neurons":changed,"changed_direct_word_input_neurons":changed.iter().filter(|i|targets.contains(i)).copied().collect::<Vec<_>>(),"same_pre_forward_neural_initial":ra.initial==rb.initial,"same_replay_candidates":ra.replay_tick.candidates==rb.replay_tick.candidates,"same_effective_weight_offsets":ra.replay_tick.effective_weight_offsets==rb.replay_tick.effective_weight_offsets,"same_sampling_counter":ra.sampling_counter==rb.sampling_counter}));
        }
    }
    Ok(
        json!({"direct_compiled_first_two_word_targets":targets,"pairwise_actual_input_interventions":pairs,"zero_global_optimizer_updates":true}),
    )
}

pub fn assess_frozen_commands(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || c.rounds != 0
        || c.epochs != 0
        || c.decisions != 24
    {
        return Err("frozen causal assessment requires public V23, rounds0 epochs0 decisions24, no provider".into());
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if old["kind"] != "experimental-genuine-care-retention-v23" || old["config"]["seed"] != c.seed {
        return Err("not matching measured V23".into());
    }
    fs::create_dir(output)?;
    let started = Instant::now();
    let mut models = vec![];
    for (label, arm) in [
        ("Untreated", Arm::Untreated),
        ("CareOnly4", Arm::NoSpeech),
        ("MixedGrounded4", Arm::Grounded),
        ("MixedNoCommand4", Arm::NoSpeech),
    ] {
        let dir = source.join(format!("recovery-{label}"));
        let original_bytes = fs::read(dir.join("experimental-recovery.json"))?;
        let r = read_recovery(&dir)?;
        let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
        let asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
        let actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
        let source_actor = actor.checkpoint();
        let source_hash = identity(&source_actor)?;
        let recorded = old["models"]
            .as_array()
            .ok_or("no measured models")?
            .iter()
            .find(|v| v["condition"] == label)
            .ok_or("no matching measured actor")?;
        if recorded["actor_sha256"] != source_hash {
            return Err("source frozen actor differs from V23 receipt".into());
        }
        let source_private =
            identity(&CpuCanonicalHost::from_checkpoint(r.host, &actor)?.checkpoint(&actor)?)?;
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
            frozen_causal_v1: true,
        };
        eprintln!(
            "frozen-causal {label}: matched cue interventions and cold repeat, zero global updates"
        );
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{label}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan),
            },
            r.next_gradient,
        )?;
        if cold["native_private_and_optimizer_hashes"]["global_optimizer_updates"] != 0
            || actor.checkpoint() != source_actor
            || fs::read(dir.join("experimental-recovery.json"))? != original_bytes
        {
            return Err("frozen assessment changed source actor/optimizer/checkpoint".into());
        }
        models.push(json!({"condition":label,"arm":arm,"actor_step":actor.optimizer_step(),"source_actor_sha256":source_hash,"source_private_sha256":source_private,"source_recovery_file_sha256":hash(&original_bytes),"full_weights_moments_integer_ages_and_config_exact":true,"global_optimizer_updates":0,"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold}));
    }
    let result = json!({"kind":"experimental-frozen-command-causal-retention-v24","execution_domain":"native-cpu","config":c,"source_v23_results_sha256":identity(&old)?,"cases":CASES,"unique_actual_delivered_cues":["EatFood","GetFood","GetToy",null],"models":models,"global_optimizer_updates":0,"elapsed_seconds":started.elapsed().as_secs_f64(),"semantic_prior_enabled":false,"teacher_present":false,"limitations":["Two disclosed prior positive controls and four independent fresh seeds are separate strata; one founder and familiar words/geometries.","Each matched parent has four distinct actual cue episodes. Muted and noun/verb swapped contrasts reuse those real trajectories; no extra independent samples, invented transitions or hidden request labels.","Initial full physical/private/RNG setup is identical; only emitted audio and utterance counter may change. Later trajectory divergence is a consequence of the cue intervention.","Normal/swapped cues share cadence/presence. Muting also changes native prompted/spontaneous speech cooldown; no cooldown reset or output filtering.","Global actor weights, optimizer and ages stay frozen. Native private biology/learning remains enabled; paired induced sleep is not independent N12, spontaneous sleep or an awake-delay control.","All source actors retain old grounded training; current cue dependence does not establish when/how word meanings were acquired or broad comprehension. No new SLM/provider, Luna401 retry, source overwrite, live/default migration or promotion."]});
    write_json(&output.join("results.json"), &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_proof_preserves_full_private_setup_and_rejects_physical_changes() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let original = actor.checkpoint();
        let (mut h, s) = unseen_transfer::prepared_frozen_case(&actor, &asset, seed, 2).unwrap();
        unseen_transfer::place_case(h.world_mut(), s, CASES[2].fixture).unwrap();
        let before = before_audio(&h, &actor).unwrap();
        let id = h.world().organism_entity_ids()[0].0;
        let pos = speech_position(h.world()).unwrap();
        h.world_mut()
            .emit_player_tokens(
                Some(id),
                pos,
                Lesson::GetFood
                    .words()
                    .into_iter()
                    .map(LanguageTokenId::new)
                    .collect::<std::result::Result<_, _>>()
                    .unwrap(),
            )
            .unwrap();
        assert!(after_audio(before.clone(), &h, &actor).is_ok());
        let mut p = h.world().entity(s.food).unwrap().position;
        p.x += 0.01;
        h.world_mut().editor_move_object(s.food, p).unwrap();
        assert!(after_audio(before, &h, &actor).is_err());
        assert_eq!(actor.checkpoint(), original);
    }
}
