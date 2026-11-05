//! Four native gate-only updates; real witnessed labels and matched recovery.
use super::super::token_probe::speech;
use super::bounded_tape_probe::{source_actor, write_compressed};
use super::*;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};

const TEST: &str = "foundation_training::retained_language::curriculum::factorial::balanced::witnessed_gate_retention::compare_four_gate_updates";
const ARMS: [&str; 4] = ["Untouched", "ZeroGradient4", "PositiveOnly4", "Balanced4"];
const CASES: [(u64, usize, usize); 2] = [(19_310_729, 2, 1833), (19_508_423, 4, 1874)];
#[derive(Clone, Serialize, Deserialize)]
struct Example {
    row: CpuCollectedDecision,
    target: ReplaySpeechTarget,
    positive: bool,
    provenance: Value,
}
#[derive(Clone, Serialize, Deserialize)]
struct CommonCase {
    seed: u64,
    fixture: usize,
    scene: Scene,
    host: CpuCanonicalCheckpoint,
}
fn read_compressed<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_reader(zstd::stream::read::Decoder::new(
        fs::File::open(path)?,
    )?)?)
}
fn gate_mask(actor: &mut CpuFoundationTrainer) -> Result<Vec<usize>> {
    let before = actor.checkpoint();
    let target = actor.phenotype().candidate_decoder().motor_start() as usize + 160;
    let ids=actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|(matches!(s.kind(),CompiledSynapseKind::Decoder(d)if d.head()==DecoderHeadKind::SpeechPayload)&&[target+16,target+17].contains(&(s.target()as usize))).then_some(i)).collect::<Vec<_>>();
    assert_eq!(ids.len(), 64);
    actor.set_stage_mask(StageTrainableMask::from_synapse_indices(
        actor.phenotype(),
        &ids.iter().map(|i| *i as u32).collect::<Vec<_>>(),
    )?)?;
    let mut after = actor.checkpoint();
    after.stage_mask = before.stage_mask.clone();
    assert_eq!(after, before, "gate mask reset inherited state");
    Ok(ids)
}
fn collect_examples(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    source: &Path,
    witness: &Path,
    out: &Path,
) -> Result<Vec<Example>> {
    let mut expected = [vec![], vec![], vec![]];
    let reader = BufReader::new(zstd::stream::read::Decoder::new(fs::File::open(
        source.join("recovery-MixedGrounded4/reloaded-evaluation.jsonl.zst"),
    )?)?);
    for line in reader.lines() {
        let e: Value = serde_json::from_str(&line?)?;
        if e["phase"] != "balanced_readout_evaluation"
            || e["world_seed"] != 10_309_219u64
            || e["post_sleep"] != false
        {
            continue;
        }
        if let Ok(l) = serde_json::from_value::<Lesson>(e["requested"].clone()) {
            expected[l.index()].push(e);
        }
    }
    let (mut base, s) = unseen_transfer::prepared_frozen_case(actor, asset, 847501, 1)?;
    for _ in 0..2 {
        base.step(actor, 1.)?;
    }
    unseen_transfer::place_case(base.world_mut(), s, 2)?;
    let id = base.world().organism_entity_ids()[0].0;
    let mut data = vec![];
    let mut decisions = 0;
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out.join("positive-report-collection.jsonl.zst"))?,
        3,
    )?;
    for lesson in Lesson::ALL {
        let mut h = base.snapshot();
        let (mut pending, mut left) = (None::<Event>, 0usize);
        assert_eq!(
            identity(&frozen_causal::before_audio(&h, actor)?)?,
            expected[lesson.index()][0]["causal_input_evidence"]["setup"]
                ["before_audio_full_checkpoint_sha256"]
        );
        let last = if lesson == Lesson::EatFood { 4 } else { 1 };
        for (tick, prior) in expected[lesson.index()].iter().enumerate().take(last + 1) {
            if tick % 2 == 0 {
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
            }
            let before = owners(h.world(), s);
            let row = h.step(actor, 1.)?.decisions.remove(0);
            decisions += 1;
            let after = owners(h.world(), s);
            no_teacher_or_prior(&row)?;
            assert_eq!(identity(&row)?, prior["native_full_row_sha256"]);
            let target = confirmed_speech(pending.as_ref(), left, Some(lesson));
            let event = physical_event(&row, s, before, after)?;
            let emitted = h
                .world()
                .audible_utterances()
                .into_iter()
                .filter(|u| {
                    u.source_kind == UtteranceSourceKind::Creature
                        && u.speaker_id == Some(id)
                        && u.emitted_tick == row.patch.pre_action().tick
                })
                .collect::<Vec<_>>();
            assert_eq!(serde_json::to_value(&emitted)?, prior["emitted"]);
            writeln!(
                trace,
                "{}",
                json!({"requested":lesson,"step":tick,"row":row,"owned_before":before,"owned_after":after,"pending":pending,"actual_event":event,"target":target,"actual_emitted":emitted,"prior_native_row_sha256":prior["native_full_row_sha256"]})
            )?;
            if tick == last {
                let t = target.ok_or("no genuine positive activity report target")?;
                assert!(pending.as_ref().is_some_and(
                    |p| p.lesson == lesson && p.tick < row.patch.pre_action().tick.raw()
                ));
                assert!(prior["emitted"]
                    .as_array()
                    .ok_or("no prior utterances")?
                    .iter()
                    .any(|u| u["tokens"] == json!(lesson.words())));
                assert!(row.patch.selected_bundle().is_some_and(|b| b
                    .channels
                    .iter()
                    .any(
                        |c| c.channel == MotorChannel::Vocal && c.payload.values.get(1) == Some(&0)
                    )));
                data.push(Example{row,target:t,positive:true,provenance:json!({"world_seed":10_309_219u64,"requested":lesson,"step":tick,"actual_prior_event":pending,"actually_spoken_matching_declare":true,"native_row_sha256":prior["native_full_row_sha256"]})});
            } else {
                if target.is_some() {
                    left -= 1;
                }
                if let Some(e) = event {
                    pending = Some(e);
                    left = 3;
                }
            }
        }
    }
    trace.finish()?;
    assert_eq!(decisions, 9);
    let old: Value = serde_json::from_slice(&fs::read(witness.join("results.json"))?)?;
    assert_eq!(
        old["kind"],
        "experimental-two-witness-native-tape-gradient-v25"
    );
    assert_eq!(
        old["models"]
            .as_array()
            .ok_or("no witness models")?
            .iter()
            .find(|m| m["condition"] == "MixedGrounded4")
            .ok_or("no measured witness actor")?["source_actor_sha256"],
        identity(&actor.checkpoint())?
    );
    for file in ["native-decision-10.json.zst", "native-decision-12.json.zst"] {
        let e: Value = read_compressed(&witness.join(file))?;
        assert_eq!(e["condition"], "MixedGrounded4");
        assert_eq!(e["seed"], 16_309_347u64);
        assert_eq!(e["step"], 1);
        let lesson: Lesson = serde_json::from_value(e["requested"].clone())?;
        let row: CpuCollectedDecision = serde_json::from_value(e["full_native_row"].clone())?;
        let prior = old["records"]
            .as_array()
            .ok_or("no witness records")?
            .iter()
            .find(|r| {
                r["condition"] == "MixedGrounded4"
                    && r["world_seed"] == 16_309_347u64
                    && r["requested"] == e["requested"]
                    && r["step"] == 1
            })
            .ok_or("missing actual witness")?;
        assert_eq!(identity(&row)?, prior["native_full_row_sha256"]);
        assert!(prior["prior_actual_event"].is_null());
        assert_eq!(prior["owned_before"], json!([null, null]));
        assert_eq!(prior["owned_after"], json!([null, null]));
        assert_eq!(prior["actual_food_consumption"], false);
        assert!(prior["actual_emitted"]
            .as_array()
            .ok_or("no witness utterance")?
            .iter()
            .any(|u| u["tokens"] == json!(lesson.words())));
        data.push(Example{row,target:ReplaySpeechTarget{token:None,continuation:[0;5],act:SpeechActKind::Declare,weight:1.},positive:false,provenance:json!({"world_seed":16_309_347u64,"requested":lesson,"step":1,"actual_unsupported_declare":true,"no_prior_or_current_activity":true,"native_row_sha256":prior["native_full_row_sha256"]})});
    }
    assert_eq!(data.iter().filter(|e| e.positive).count(), 3);
    assert_eq!(data.iter().filter(|e| !e.positive).count(), 2);
    write_compressed(&out.join("grounded-gate-corpus.json.zst"), &data)?;
    Ok(data)
}
fn component(
    actor: &CpuFoundationTrainer,
    data: &[Example],
    ids: &[usize],
    positive: bool,
) -> Result<(Vec<f32>, f64)> {
    let samples = data
        .iter()
        .filter(|e| e.positive == positive)
        .collect::<Vec<_>>();
    let mut sum = vec![0.; actor.weights().len()];
    let mut loss = 0.;
    for e in &samples {
        let q = readout::single(actor, &e.row);
        let ev = actor.evaluate_replay(&q)?;
        let raw = speech(
            actor,
            &ev.final_activations[0],
            &q.ticks[0].effective_weight_offsets,
        );
        let g = actor.probe_replay_gradients_with_speech(
            &q,
            &[vec![0.; q.ticks[0].candidates.len()]],
            &[Some(e.target)],
        )?;
        let target_start = actor.phenotype().candidate_decoder().motor_start() as usize + 160;
        let source_start = target_start - 32;
        let steps = e.target.len();
        let factor = if positive { 9. } else { 1. };
        for (step, outputs) in raw.iter().enumerate().take(steps) {
            for output in [16usize, 17] {
                loss += f64::from(outputs[output] - e.target.step_logits(step)[output]).powi(2)
                    / (steps * 2 * samples.len()) as f64;
            }
        }
        for &i in ids {
            let s = actor.phenotype().synapses()[i];
            let output = s.target() as usize - target_start;
            let lane = s.source() as usize - source_start;
            let mut analytic = 0.;
            for step in 0..steps {
                let input = if step > 0 && lane >= 18 {
                    raw[step - 1][lane].tanh()
                } else {
                    ev.final_activations[0][s.source() as usize]
                };
                analytic += 2. * (raw[step][output] - e.target.step_logits(step)[output]) * input
                    / (steps * 2) as f32;
            }
            assert!(
                (analytic - g.gradients[i] * factor).abs() < 1e-4,
                "gate normalization differs from native BPTT"
            );
            sum[i] += g.gradients[i] * factor / samples.len() as f32;
        }
    }
    Ok((sum, loss))
}
fn objective(
    actor: &CpuFoundationTrainer,
    data: &[Example],
    ids: &[usize],
    arm: &str,
) -> Result<(Vec<f32>, Value)> {
    if arm == "ZeroGradient4" {
        return Ok((
            vec![0.; actor.weights().len()],
            json!({"loss":0.,"zero_gradient_momentum_and_decay_control":true}),
        ));
    }
    let (p, pl) = component(actor, data, ids, true)?;
    let (n, nl) = if arm == "Balanced4" {
        component(actor, data, ids, false)?
    } else {
        (vec![0.; p.len()], 0.)
    };
    let g = p
        .iter()
        .zip(&n)
        .map(|(a, b)| 0.5 * a + 0.5 * b)
        .collect::<Vec<_>>();
    Ok((
        g,
        json!({"positive_gate_loss":pl,"absence_gate_loss":nl,"positive_mass":0.5,"absence_mass":if arm=="Balanced4"{0.5}else{0.},"positive_native_normalization_factor":9.,"same_positive_component_in_both_supervised_arms":true,"only_presence_and_stop_coordinates_have_gradient":true}),
    ))
}
fn assess(
    actor: &CpuFoundationTrainer,
    cases: &[CommonCase],
    traces: &mut impl Write,
    private_out: Option<&Path>,
) -> Result<Value> {
    let mut finals = vec![];
    let mut decisions = 0;
    for (case, c) in cases.iter().enumerate() {
        let base = CpuCanonicalHost::from_checkpoint(c.host.clone(), actor)?;
        assert_eq!(identity(&base.checkpoint(actor)?)?, identity(&c.host)?);
        let id = base.world().organism_entity_ids()[0].0;
        for post_sleep in [false, true] {
            let mut parent = base.snapshot();
            let (mut sleeping, mut consolidations) = (0usize, 0usize);
            if post_sleep {
                parent.request_recovery_sleep(id)?;
                for _ in 0..600 {
                    let step = parent.step(actor, 1.)?;
                    sleeping += step.sleeping_organisms;
                    consolidations += step.sleep_consolidations;
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"balanced_recovery_sleep","case":case,"world_seed":c.seed,"decisions":step.decisions,"sleeping":step.sleeping_organisms,"consolidations":step.sleep_consolidations})
                    )?;
                    if consolidations > 0
                        && parent.world().organism_registry().get(id).is_some_and(|r| {
                            r.sleep_phase() == alife_core::sleep::SleepPhase::Awake
                        })
                    {
                        break;
                    }
                }
                if consolidations == 0
                    || parent
                        .world()
                        .organism_registry()
                        .get(id)
                        .is_none_or(|r| r.sleep_phase() != alife_core::sleep::SleepPhase::Awake)
                {
                    return Err("gate assessment did not consolidate and wake".into());
                }
            }
            unseen_transfer::place_case(parent.world_mut(), c.scene, c.fixture)?;
            let parent_hash = identity(&parent.checkpoint(actor)?)?;
            for requested in [
                Some(Lesson::EatFood),
                Some(Lesson::GetFood),
                Some(Lesson::GetToy),
                None,
            ] {
                let mut h = parent.snapshot();
                let (mut pending, mut left) = (None::<Event>, 0usize);
                for tick in 0..24 {
                    if tick < 8 && tick % 2 == 0 {
                        if let Some(l) = requested {
                            let pos = speech_position(h.world())?;
                            h.world_mut().emit_player_tokens(
                                Some(id),
                                pos,
                                l.words()
                                    .into_iter()
                                    .map(LanguageTokenId::new)
                                    .collect::<std::result::Result<_, _>>()?,
                            )?;
                        }
                    }
                    let before = owners(h.world(), c.scene);
                    let row = h.step(actor, 1.)?.decisions.remove(0);
                    let after = owners(h.world(), c.scene);
                    decisions += 1;
                    no_teacher_or_prior(&row)?;
                    if tick == 0 {
                        assert!([c.scene.food, c.scene.toy, c.scene.distractor]
                            .into_iter()
                            .all(|t| visible_target(row.patch.pre_action().perception(), t)));
                    }
                    let target = confirmed_speech(pending.as_ref(), left, requested);
                    let event = physical_event(&row, c.scene, before, after)?;
                    let emitted = h
                        .world()
                        .audible_utterances()
                        .into_iter()
                        .filter(|u| {
                            u.source_kind == UtteranceSourceKind::Creature
                                && u.speaker_id == Some(id)
                                && u.emitted_tick == row.patch.pre_action().tick
                        })
                        .collect::<Vec<_>>();
                    let organism = h
                        .world()
                        .organism_registry()
                        .get(id)
                        .ok_or("no assessed organism")?;
                    let bio = organism.biochemistry();
                    let trace_row = if tick == 0 {
                        serde_json::to_value(&row)?
                    } else {
                        json!({"organism_id":row.organism_id,"sampling_counter":row.sampling_counter,"action":row.action,"patch":row.patch,"biological_reward":row.biological_reward,"brain_work":row.brain_work})
                    };
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"balanced_readout_evaluation","case":case,"world_seed":c.seed,"post_sleep":post_sleep,"requested":requested,"scene":c.scene,"row":trace_row,"native_full_row_sha256":identity(&row)?,"state_capture":"full-first-row-and-full-state-digest-v1","owned_before":before,"owned_after":after,"actual_food_consumption":consequences(&row,c.scene).0,"grounded_speech_target":target,"narration_evidence":pending,"completed":event,"emitted":emitted,"speech_opportunity_protocol":"event-confirmed-cue-window-v1","physical_safety_after_native_tick":{"alive":organism.lifecycle().is_alive(),"health":bio.body.health,"injury":bio.body.injury,"energy":bio.body.energy,"pain":bio.homeostasis.drives.pain}})
                    )?;
                    if target.is_some() {
                        left -= 1;
                    }
                    if let Some(e) = event {
                        pending = Some(e);
                        left = 3;
                    }
                }
                let checkpoint = h.checkpoint(actor)?;
                let private_file = private_out.map(|dir| {
                    dir.join(format!(
                        "{}-{}-{}.json.zst",
                        c.seed,
                        post_sleep,
                        requested.map_or(3, Lesson::index)
                    ))
                });
                if let Some(file) = &private_file {
                    write_compressed(file, &checkpoint)?;
                }
                finals.push(json!({"world_seed":c.seed,"post_sleep":post_sleep,"requested":requested,"paired_initial_private_sha256":parent_hash,"final_full_private_sha256":identity(&checkpoint)?,"private_checkpoint_file":private_file,"sleeping_ticks":sleeping,"consolidations":consolidations}));
            }
        }
    }
    assert_eq!(decisions, 384);
    Ok(json!({"scored_decisions":decisions,"final_states":finals}))
}
fn evaluate_saved(source: &Path, out: &Path, cold: bool) -> Result<Value> {
    let (base, asset) = source_actor(source, "MixedGrounded4")?;
    let cases: Vec<CommonCase> = read_compressed(&out.join("common-assessment-starts.json.zst"))?;
    let data: Vec<Example> = read_compressed(&out.join("grounded-gate-corpus.json.zst"))?;
    let resumed = std::env::var_os("ALIFE_GATE_RESUME_EVAL").is_some();
    let prefix = match (resumed, cold) {
        (true, true) => "resumed-cold",
        (true, false) => "resumed-hot",
        (false, true) => "cold",
        (false, false) => "hot",
    };
    let mut results = vec![];
    for arm in ARMS {
        eprintln!(
            "gate-retention {} {arm}: matched two-seed pre/post-sleep assessment",
            if cold { "cold" } else { "hot" }
        );
        let cp: FoundationTrainerCheckpoint =
            read_compressed(&out.join(format!("actor-{arm}.json.zst")))?;
        let actor = CpuFoundationTrainer::from_checkpoint(
            base.phenotype().clone(),
            asset.clone(),
            cp.clone(),
        )?;
        let before = identity(&cp)?;
        let mut rebased = vec![];
        let mut proofs = vec![];
        for c in &cases {
            let original = CpuCanonicalHost::from_checkpoint(c.host.clone(), &base)?;
            let before_host = original.checkpoint(&base)?;
            let (branch, receipt) = original.fork_experimental_speech_gate_policy(&base, &actor)?;
            let after = branch.checkpoint(&actor)?;
            let mut normalized = after.clone();
            normalized.founder_weight_digest = before_host.founder_weight_digest;
            assert_eq!(
                serde_json::to_vec(&normalized)?,
                serde_json::to_vec(&before_host)?
            );
            assert_eq!(
                identity(&original.checkpoint(&base)?)?,
                identity(&before_host)?
            );
            let mut invalid = actor.checkpoint();
            let bad=base.phenotype().synapses().iter().enumerate().find(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(d)if d.head()==DecoderHeadKind::SpeechPayload)&&s.target()as usize==base.phenotype().candidate_decoder().motor_start()as usize+168).ok_or("no lexical guard coordinate")?.0;
            invalid.weights[bad] += 0.001;
            let invalid = CpuFoundationTrainer::from_checkpoint(
                base.phenotype().clone(),
                asset.clone(),
                invalid,
            )?;
            assert!(original
                .fork_experimental_speech_gate_policy(&base, &invalid)
                .is_err());
            if !receipt.changed_weight_indices.is_empty() {
                let normal = CpuCanonicalHost::from_checkpoint(c.host.clone(), &actor)?;
                assert!(
                    normal.checkpoint(&actor).is_err(),
                    "normal admission must retain the policy guard"
                );
            }
            proofs.push(json!({"world_seed":c.seed,"original_host_sha256":identity(&before_host)?,"branch_host_sha256":identity(&after)?,"only_policy_binding_may_differ":true,"all_world_private_and_rng_fields_exact":true,"non_gate_weight_change_rejected":true,"normal_policy_guard_retained":true,"source_host_unchanged":true,"experimental_branch_receipt":receipt}));
            rebased.push(CommonCase {
                host: after,
                ..c.clone()
            });
        }
        let reused = resumed && !cold && arm == "Untouched";
        let (assessment, trace_hash) = if reused {
            let mut reader = zstd::stream::read::Decoder::new(fs::File::open(
                out.join("hot-Untouched.jsonl.zst"),
            )?)?;
            let mut hash = HashedTrace {
                inner: std::io::sink(),
                digest: Sha256::new(),
            };
            std::io::copy(&mut reader, &mut hash)?;
            (
                json!({"scored_decisions":384,"final_states":null,"original_complete_trace_reused_without_repeat":true,"old_hot_final_private_hashes_were_not_written_before_guard_failure":true}),
                format!("{:x}", hash.digest.finalize()),
            )
        } else {
            let file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(out.join(format!("{prefix}-{arm}.jsonl.zst")))?;
            let mut traces = HashedTrace {
                inner: zstd::stream::write::Encoder::new(file, 3)?,
                digest: Sha256::new(),
            };
            let private_dir = if resumed && cold && arm == "Untouched" {
                let p = out.join("resumed-cold-Untouched-private");
                fs::create_dir(&p)?;
                Some(p)
            } else {
                None
            };
            let assessment = assess(&actor, &rebased, &mut traces, private_dir.as_deref())?;
            let hash = format!("{:x}", traces.digest.finalize());
            traces.inner.finish()?;
            (assessment, hash)
        };
        assert_eq!(actor.checkpoint(), cp);
        let mut masked =
            CpuFoundationTrainer::from_checkpoint(base.phenotype().clone(), asset.clone(), cp)?;
        let ids = gate_mask(&mut masked)?;
        assert_eq!(actor.checkpoint(), masked.checkpoint());
        let (g, objective) = objective(
            &actor,
            &data,
            &ids,
            if arm == "Untouched" {
                "PositiveOnly4"
            } else {
                arm
            },
        )?;
        let record = json!({"condition":arm,"actor_sha256":before,"optimizer_step":actor.optimizer_step(),"native_trace_sha256":trace_hash,"assessment":assessment,"next_gradient_sha256":identity(&g)?,"next_gradient_objective":objective,"zero_optimizer_updates_during_assessment_or_cold_restore":true,"experimental_policy_branch_proofs":proofs});
        write_json(&out.join(format!("{prefix}-summary-{arm}.json")), &record)?;
        results.push(record);
    }
    Ok(json!(results))
}
fn recover_saved_updates(source: &Path, out: &Path) -> Result<Value> {
    let (base, asset) = source_actor(source, "MixedGrounded4")?;
    let data: Vec<Example> = read_compressed(&out.join("grounded-gate-corpus.json.zst"))?;
    let initial: FoundationTrainerCheckpoint =
        read_compressed(&out.join("actor-Untouched.json.zst"))?;
    let mut masked = CpuFoundationTrainer::from_checkpoint(
        base.phenotype().clone(),
        asset.clone(),
        initial.clone(),
    )?;
    let ids = gate_mask(&mut masked)?;
    assert_eq!(masked.checkpoint(), initial);
    let mut models = vec![];
    for arm in ARMS {
        let mut before = initial.clone();
        let mut updates = vec![];
        for epoch in 0..if arm == "Untouched" { 0 } else { 4 } {
            let after: FoundationTrainerCheckpoint =
                read_compressed(&out.join(format!("actor-{arm}-after-{}.json.zst", epoch + 1)))?;
            let saved: Vec<f32> =
                read_compressed(&out.join(format!("gradient-{arm}-{}.json.zst", epoch + 1)))?;
            let actor = CpuFoundationTrainer::from_checkpoint(
                base.phenotype().clone(),
                asset.clone(),
                before.clone(),
            )?;
            let (g, objective) = objective(&actor, &data, &ids, arm)?;
            assert_eq!(g, saved, "saved native update gradient changed");
            preserve_frozen(&before, &after)?;
            assert_eq!(after.optimizer_step, before.optimizer_step + 1);
            assert_eq!(after.config, before.config);
            assert_eq!(after.stage_mask, before.stage_mask);
            let norm = ids.iter().map(|i| g[*i] * g[*i]).sum::<f32>().sqrt();
            let c = before.config;
            let scale = if norm > c.gradient_clip {
                c.gradient_clip / norm
            } else {
                1.
            };
            for &i in &ids {
                let grad = g[i] * scale;
                let age = before.update_ages[i] + 1;
                let m = c.beta1 * before.first_moment[i] + (1. - c.beta1) * grad;
                let v = c.beta2 * before.second_moment[i] + (1. - c.beta2) * grad * grad;
                let w = before.weights[i]
                    - c.learning_rate
                        * ((m / (1. - c.beta1.powf(age as f32)))
                            / ((v / (1. - c.beta2.powf(age as f32))).sqrt() + c.epsilon)
                            + c.weight_decay * before.weights[i]);
                assert_eq!(after.update_ages[i], age);
                assert_eq!(after.first_moment[i].to_bits(), m.to_bits());
                assert_eq!(after.second_moment[i].to_bits(), v.to_bits());
                assert_eq!(
                    after.weights[i].to_bits(),
                    w.to_bits(),
                    "saved gate differs from native Adam arithmetic"
                );
            }
            updates.push(json!({"epoch":epoch,"optimizer_step":after.optimizer_step,"gradient_norm":norm,"objective":objective,"changed_gate_weights":ids.iter().filter(|i|before.weights[**i].to_bits()!=after.weights[**i].to_bits()).count(),"gate_weight_delta_l2":ids.iter().map(|i|f64::from(after.weights[*i]-before.weights[*i]).powi(2)).sum::<f64>().sqrt(),"all_other_weights_moments_integer_ages_exact":true,"saved_gradient_recomputed_exact_without_update":true,"native_adam_weights_moments_and_ages_verified_by_read_only_arithmetic":true,"checkpoint_sha256":identity(&after)?}));
            before = after;
        }
        let final_cp: FoundationTrainerCheckpoint =
            read_compressed(&out.join(format!("actor-{arm}.json.zst")))?;
        assert_eq!(before, final_cp);
        preserve_frozen(&initial, &final_cp)?;
        models.push(json!({"condition":arm,"initial_actor_sha256":identity(&initial)?,"actor_sha256":identity(&final_cp)?,"source_optimizer_step":initial.optimizer_step,"actor_step":final_cp.optimizer_step,"updates":updates,"only_trainable_coordinates":ids,"all_frozen_history_exact":true,"gate_weight_change_from_source_l2":ids.iter().map(|i|f64::from(final_cp.weights[*i]-initial.weights[*i]).powi(2)).sum::<f64>().sqrt()}));
    }
    Ok(json!(models))
}
fn resume_saved_assessment(source: &Path, out: &Path) -> Result<()> {
    let start = Instant::now();
    let models = recover_saved_updates(source, out)?;
    write_json(&out.join("recovered-update-receipts.json"), &models)?;
    let hot = evaluate_saved(source, out, false)?;
    write_json(&out.join("resumed-hot-results.json"), &hot)?;
    let status = Command::new(std::env::current_exe()?)
        .args([TEST, "--exact", "--ignored", "--nocapture"])
        .env("ALIFE_GATE_COLD_ONLY", "1")
        .status()?;
    assert!(status.success());
    let hot: Value = serde_json::from_slice(&fs::read(out.join("resumed-hot-results.json"))?)?;
    let cold: Value = serde_json::from_slice(&fs::read(out.join("resumed-cold-results.json"))?)?;
    for (h, c) in hot
        .as_array()
        .ok_or("no hot")?
        .iter()
        .zip(cold.as_array().ok_or("no cold")?)
    {
        for key in [
            "condition",
            "actor_sha256",
            "optimizer_step",
            "native_trace_sha256",
            "next_gradient_sha256",
            "next_gradient_objective",
            "experimental_policy_branch_proofs",
        ] {
            assert_eq!(h[key], c[key], "hot/cold {key} differs");
        }
        if h["condition"] != "Untouched" {
            assert_eq!(h["assessment"], c["assessment"]);
        }
    }
    let (base, asset) = source_actor(source, "MixedGrounded4")?;
    let cp = read_compressed(&out.join("actor-Untouched.json.zst"))?;
    let actor = CpuFoundationTrainer::from_checkpoint(base.phenotype().clone(), asset, cp)?;
    let baseline = cold
        .as_array()
        .ok_or("no cold")?
        .iter()
        .find(|r| r["condition"] == "Untouched")
        .ok_or("no baseline")?;
    let mut verified = 0;
    for item in baseline["assessment"]["final_states"]
        .as_array()
        .ok_or("no baseline states")?
    {
        let path = PathBuf::from(
            item["private_checkpoint_file"]
                .as_str()
                .ok_or("no saved baseline state")?,
        );
        let cp: CpuCanonicalCheckpoint = read_compressed(&path)?;
        assert_eq!(identity(&cp)?, item["final_full_private_sha256"]);
        let h = CpuCanonicalHost::from_checkpoint(cp, &actor)?;
        assert_eq!(
            identity(&h.checkpoint(&actor)?)?,
            item["final_full_private_sha256"]
        );
        verified += 1;
    }
    assert_eq!(verified, 16);
    write_json(
        &out.join("results.json"),
        &json!({"kind":"experimental-witnessed-speech-gate-retention-v26","execution_domain":"native-cpu","models":models,"assessment":cold,"hot_assessment":hot,"fresh_cases":CASES,"positive_rows":3,"absence_rows":2,"positive_collection_native_decisions":9,"native_updates_per_trained_arm":4,"optimizer_updates_during_resume":0,"only_64_presence_stop_coordinates_trainable":true,"all_complete_native_hot_cold_traces_and_optimizer_and_next_gradients_exact":true,"trained_arm_final_private_hot_cold_hashes_exact":true,"untouched_final_cold_private_states_exact_after_fresh_process_load":verified,"old_untouched_hot_final_private_hashes_not_available":true,"elapsed_resume_seconds":start.elapsed().as_secs_f64(),"teacher_or_prior_in_evaluation":false,"limitations":["Two fresh seeds, familiar layouts and one founder; phases paired, not independent.","Three genuine positive reports, two witnessed unsupported declarations; other missing labels not negative.","Original evaluation stopped at policy-binding guard after Untouched. All trained actors and update vectors were already saved. Resume verifies their native Adam arithmetic and gradients without another update.","Explicit experimental readout-only simulator fork changes only policy binding; all world/private/RNG fields are preserved, source host unchanged. No accepted-life migration.","The original complete Untouched hot trace is reused without repetition; its old final private hashes were not written. Full native cold trace matches, and16 cold final states load exactly in the parent process. Trained arms have full hot/cold final private equality.","Induced RecoveryProtocol sleep has no awake-delay control. No SLM or broad comprehension claim, accepted replacement or promotion."]}),
    )?;
    Ok(())
}
#[test]
#[ignore = "explicit four-update disposable gate retention test; SOURCE/WITNESSES/new OUT required"]
fn compare_four_gate_updates() -> Result<()> {
    let source = PathBuf::from(std::env::var("ALIFE_GATE_SOURCE")?);
    let out = PathBuf::from(std::env::var("ALIFE_GATE_OUT")?);
    if std::env::var_os("ALIFE_GATE_COLD_ONLY").is_some() {
        let r = evaluate_saved(&source, &out, true)?;
        let name = if std::env::var_os("ALIFE_GATE_RESUME_EVAL").is_some() {
            "resumed-cold-results.json"
        } else {
            "cold-results.json"
        };
        write_json(&out.join(name), &r)?;
        return Ok(());
    }
    if std::env::var_os("ALIFE_GATE_RESUME_EVAL").is_some() {
        return resume_saved_assessment(&source, &out);
    }
    let witness = PathBuf::from(std::env::var("ALIFE_GATE_WITNESSES")?);
    let measured: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    assert_eq!(
        measured["kind"],
        "experimental-frozen-command-causal-retention-v24"
    );
    fs::create_dir(&out)?;
    let start = Instant::now();
    let (mut base, asset) = source_actor(&source, "MixedGrounded4")?;
    let original = base.checkpoint();
    let data = collect_examples(&base, &asset, &source, &witness, &out)?;
    let mut common = vec![];
    for (seed, fixture, padding) in CASES {
        assert!(![10_309_219, 16_309_347, 16_508_317].contains(&seed));
        let (mut h, scene) =
            unseen_transfer::prepared_instance(&base, &asset, 847501, fixture, seed, padding)?;
        for _ in 0..2 {
            h.step(&base, 1.)?;
        }
        common.push(CommonCase {
            seed,
            fixture,
            scene,
            host: h.checkpoint(&base)?,
        });
    }
    write_compressed(&out.join("common-assessment-starts.json.zst"), &common)?;
    let ids = gate_mask(&mut base)?;
    let initial = base.checkpoint();
    let mut models = vec![];
    let (preflight, details) = objective(&base, &data, &ids, "Balanced4")?;
    let norm = preflight
        .iter()
        .map(|g| f64::from(*g).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(norm > 0.);
    write_json(
        &out.join("gradient-preflight.json"),
        &json!({"gradient_l2":norm,"objective":details,"native_gradient_matches_analytic_gate_loss":true,"all_other_coordinates_zero":preflight.iter().enumerate().all(|(i,g)|ids.contains(&i)||*g==0.),"no_optimizer_update":true}),
    )?;
    for arm in ARMS {
        let mut actor = CpuFoundationTrainer::from_checkpoint(
            base.phenotype().clone(),
            asset.clone(),
            initial.clone(),
        )?;
        let mut updates = vec![];
        for epoch in 0..if arm == "Untouched" { 0 } else { 4 } {
            let before = actor.checkpoint();
            let (g, objective) = objective(&actor, &data, &ids, arm)?;
            let norm = actor.apply_gradients(&g)?;
            let after = actor.checkpoint();
            preserve_frozen(&before, &after)?;
            assert_eq!(after.optimizer_step, before.optimizer_step + 1);
            for &i in &ids {
                assert_eq!(after.update_ages[i], before.update_ages[i] + 1);
            }
            let changed = ids
                .iter()
                .filter(|i| before.weights[**i].to_bits() != after.weights[**i].to_bits())
                .count();
            let delta = ids
                .iter()
                .map(|i| f64::from(after.weights[*i] - before.weights[*i]).powi(2))
                .sum::<f64>()
                .sqrt();
            write_compressed(
                &out.join(format!("actor-{arm}-after-{}.json.zst", epoch + 1)),
                &after,
            )?;
            write_compressed(
                &out.join(format!("gradient-{arm}-{}.json.zst", epoch + 1)),
                &g,
            )?;
            updates.push(json!({"epoch":epoch,"optimizer_step":actor.optimizer_step(),"gradient_norm":norm,"objective":objective,"changed_gate_weights":changed,"gate_weight_delta_l2":delta,"all_other_weights_moments_integer_ages_exact":true,"checkpoint_sha256":identity(&after)?}));
        }
        let cp = actor.checkpoint();
        preserve_frozen(&initial, &cp)?;
        write_compressed(&out.join(format!("actor-{arm}.json.zst")), &cp)?;
        models.push(json!({"condition":arm,"initial_actor_sha256":identity(&initial)?,"actor_sha256":identity(&cp)?,"source_optimizer_step":initial.optimizer_step,"actor_step":cp.optimizer_step,"updates":updates,"only_trainable_coordinates":ids,"all_frozen_history_exact":true,"gate_weight_change_from_source_l2":ids.iter().map(|i|f64::from(cp.weights[*i]-initial.weights[*i]).powi(2)).sum::<f64>().sqrt()}));
    }
    let hot = evaluate_saved(&source, &out, false)?;
    write_json(&out.join("hot-results.json"), &hot)?;
    let status = Command::new(std::env::current_exe()?)
        .args([TEST, "--exact", "--ignored", "--nocapture"])
        .env("ALIFE_GATE_COLD_ONLY", "1")
        .status()?;
    assert!(status.success());
    let hot: Value = serde_json::from_slice(&fs::read(out.join("hot-results.json"))?)?;
    let cold: Value = serde_json::from_slice(&fs::read(out.join("cold-results.json"))?)?;
    assert_eq!(
        hot, cold,
        "cold traces/private states/optimizer/next gradient differ"
    );
    let mut normalized = base.checkpoint();
    normalized.stage_mask = original.stage_mask.clone();
    assert_eq!(normalized, original);
    write_json(
        &out.join("results.json"),
        &json!({"kind":"experimental-witnessed-speech-gate-retention-v26","execution_domain":"native-cpu","models":models,"assessment":hot,"fresh_cases":CASES,"positive_rows":3,"absence_rows":2,"positive_collection_native_decisions":9,"native_updates_per_trained_arm":4,"global_source_unchanged":true,"only_64_presence_stop_coordinates_trainable":true,"cold_all_traces_private_states_optimizer_and_next_gradients_exact":true,"elapsed_seconds":start.elapsed().as_secs_f64(),"teacher_or_prior_in_evaluation":false,"limitations":["Two fresh seeds with familiar layouts and one source founder; phases paired, not independent samples.","Three genuine positive reports from one prior seed, two witnessed premature declarations from the earlier diagnostic; no absence label for other missing targets.","Only gate coordinates change. Native private learning remains enabled; indirect physical changes must be scored.","Zero-gradient four-update arm measures inherited Adam momentum/decay separately; positive-only and balanced arms share identical positive component and update count.","RecoveryProtocol sleep before evaluation is induced and has no awake-delay control. No SLM contribution, accepted-state replacement or promotion."]}),
    )?;
    Ok(())
}
