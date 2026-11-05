//! Fixed four-update curriculum comparison. Every world decision has a receipt;
//! controls are loaded, never trained or replayed for duplicate scored results.
use super::bounded_tape_probe::{source_actor, write_compressed};
use super::witnessed_gate_retention::{component, Example};
use super::*;
use std::{io::BufRead, path::PathBuf};

const TEST: &str = "foundation_training::retained_language::curriculum::factorial::balanced::matched_gate_curriculum::compare_matched_factual_curriculum";
const ARMS: [&str; 4] = ["Untouched", "ZeroGradient4", "Balanced4", "MatchedFactual4"];
const FLOOR: u64 = 482_148_352;
const RECEIPTS: u64 = 384 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
struct Common {
    seed: u64,
    fixture: usize,
    seen: bool,
    scene: Scene,
    host: CpuCanonicalCheckpoint,
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_reader(zstd::stream::read::Decoder::new(
        fs::File::open(path)?,
    )?)?)
}
fn out() -> Result<PathBuf> {
    Ok(PathBuf::from(std::env::var("ALIFE_MATCHED_OUT")?))
}
fn old() -> Result<PathBuf> {
    Ok(PathBuf::from(std::env::var("ALIFE_MATCHED_V26")?))
}
fn source() -> Result<PathBuf> {
    Ok(PathBuf::from(std::env::var("ALIFE_MATCHED_SOURCE")?))
}
fn size(path: &Path) -> Result<u64> {
    let mut total = 0;
    for e in fs::read_dir(path)? {
        let p = e?.path();
        total += if p.is_dir() {
            size(&p)?
        } else {
            p.metadata()?.len()
        };
    }
    Ok(total)
}
fn storage() -> Result<()> {
    let v = Command::new("df")
        .args(["-B1", "--output=avail", "/workspace"])
        .output()?;
    assert!(v.status.success());
    let free = String::from_utf8(v.stdout)?
        .lines()
        .last()
        .ok_or("no disk result")?
        .trim()
        .parse::<u64>()?;
    if free < FLOOR + 8 * 1024 * 1024 || size(&out()?)? + 8 * 1024 * 1024 > RECEIPTS {
        return Err("declared storage floor/receipt budget reached".into());
    }
    Ok(())
}
fn reserve(kind: &str, maximum: usize, v: Value) -> Result<usize> {
    storage()?;
    let dir = out()?;
    let prefix = format!("{kind}-attempt-");
    let n = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .count()
        + 1;
    if n > maximum {
        return Err(format!("refusing {kind} attempt {n}, maximum {maximum}").into());
    }
    write_json(
        &dir.join(format!("{prefix}{n:04}.json")),
        &json!({"number":n,"maximum":maximum,"evidence":v}),
    )?;
    Ok(n)
}
fn actor(arm: &str) -> Result<(CpuFoundationTrainer, FoundationWeightAsset)> {
    let (base, asset) = source_actor(&source()?, "MixedGrounded4")?;
    let path = if arm == "MatchedFactual4" {
        out()?.join("actor-MatchedFactual4.json.zst")
    } else {
        old()?.join(format!("actor-{arm}.json.zst"))
    };
    let cp: FoundationTrainerCheckpoint = read(&path)?;
    let a =
        CpuFoundationTrainer::from_checkpoint(base.phenotype().clone(), asset.clone(), cp.clone())?;
    assert_eq!(a.checkpoint(), cp);
    assert_eq!(
        a.optimizer_step(),
        if arm == "Untouched" { 436 } else { 440 }
    );
    Ok((a, asset))
}
fn known_rows() -> Result<Vec<Value>> {
    let f = std::io::BufReader::new(zstd::stream::read::Decoder::new(fs::File::open(
        old()?.join("positive-report-collection.jsonl.zst"),
    )?)?);
    f.lines().map(|l| Ok(serde_json::from_str(&l?)?)).collect()
}
fn known_parent_hash() -> Result<String> {
    let f = std::io::BufReader::new(zstd::stream::read::Decoder::new(fs::File::open(
        source()?.join("recovery-MixedGrounded4/reloaded-evaluation.jsonl.zst"),
    )?)?);
    for l in f.lines() {
        let v: Value = serde_json::from_str(&l?)?;
        if v["phase"] == "balanced_readout_evaluation"
            && v["world_seed"] == 10_309_219u64
            && v["post_sleep"] == false
            && v["requested"] == "EatFood"
        {
            return Ok(
                v["causal_input_evidence"]["setup"]["before_audio_full_checkpoint_sha256"]
                    .as_str()
                    .ok_or("no exact supported parent")?
                    .to_string(),
            );
        }
    }
    Err("exact known positive provenance unavailable".into())
}
fn save_host(name: &str, h: &CpuCanonicalHost, a: &CpuFoundationTrainer) -> Result<Value> {
    storage()?;
    let cp = h.checkpoint(a)?;
    let path = out()?.join(name);
    write_compressed(&path, &cp)?;
    let recovered: CpuCanonicalCheckpoint = read(&path)?;
    let cold = CpuCanonicalHost::from_checkpoint(recovered, a)?;
    assert_eq!(identity(&cold.checkpoint(a)?)?, identity(&cp)?);
    Ok(json!({"file":name,"sha256":identity(&cp)?}))
}
fn child(mode: &str, case: usize, arm: &str, post: bool) -> Result<()> {
    let label = format!("{mode}-{case}-{arm}-{post}");
    write_json(
        &out()?.join(format!("child-attempt-{label}.json")),
        &json!({"mode":mode,"case":case,"arm":arm,"post_sleep":post,"no_retry":true}),
    )?;
    let status = Command::new(std::env::current_exe()?)
        .args([TEST, "--exact", "--ignored", "--nocapture"])
        .env("ALIFE_MATCHED_MODE", mode)
        .env("ALIFE_MATCHED_CASE", case.to_string())
        .env("ALIFE_MATCHED_ARM", arm)
        .env("ALIFE_MATCHED_POST", if post { "1" } else { "0" })
        .status()?;
    if !status.success() {
        return Err(format!("child failed: {label}; no retry or dependent continuation").into());
    }
    Ok(())
}
fn prepare() -> Result<()> {
    let (a, asset) = actor("Untouched")?;
    assert_eq!(
        identity(&a.checkpoint())?,
        "a64296c15537030f3dd7fbe967472947bd1afb3da42e1a9103bd4217dc9ec58f"
    );
    let mut cases = vec![];
    for (case, (seed, fixture, padding, seen)) in [
        (10_309_219, 2, 915, true),
        (23_110_769, 2, 2113, false),
        (23_311_891, 4, 2174, false),
    ]
    .into_iter()
    .enumerate()
    {
        reserve(
            "biological-preparation",
            3,
            json!({"case":case,"seed":seed,"total_tick_cap_including_two_awake_warmups":600}),
        )?;
        let (mut h, s) = unseen_transfer::prepared_instance_capped(
            &a,
            &asset,
            847501,
            fixture,
            seed,
            padding,
            Some(598),
        )?;
        let biological_ticks = h.world().tick().raw();
        assert!(biological_ticks <= 598);
        for warmup in 0..2 {
            reserve(
                "awake-preparation",
                6,
                json!({"case":case,"warmup":warmup,"pre_sha256":identity(&h.checkpoint(&a)?)?}),
            )?;
            let step = h.step(&a, 1.)?;
            assert_eq!(step.decisions.len(), 1);
            write_compressed(
                &out()?.join(format!("preparation-{case}-{warmup}.json.zst")),
                &step.decisions,
            )?;
        }
        assert!(h.world().tick().raw() <= 600);
        let cp = h.checkpoint(&a)?;
        let c = Common {
            seed,
            fixture,
            seen,
            scene: s,
            host: cp,
        };
        if seen {
            let mut placed = h.snapshot();
            unseen_transfer::place_case(placed.world_mut(), s, fixture)?;
            assert_eq!(
                identity(&frozen_causal::before_audio(&placed, &a)?)?,
                known_parent_hash()?,
                "known positive parent differs from actual supported provenance"
            );
            write_json(
                &out()?.join("known-parent-exact.json"),
                &json!({"seed":seed,"fixture":fixture,"padding":padding,"founder":847501,"parent_matches_original_full_checkpoint_before_audio":true,"recorded_positive_rows":known_rows()?.iter().filter(|r|!r["target"].is_null()&&!r["actual_emitted"].as_array().is_none_or(|v|v.is_empty())).map(|r|r["prior_native_row_sha256"].clone()).collect::<Vec<_>>(),"biology_ticks":biological_ticks,"awake_preparation_decisions":2}),
            )?;
        }
        write_compressed(&out()?.join(format!("common-{case}.json.zst")), &c)?;
        cases.push(json!({"case":case,"seed":seed,"seen":seen,"biological_ticks":biological_ticks,"awake_preparation_decisions":2,"common_sha256":identity(&c.host)?}));
    }
    write_json(
        &out()?.join("preparation-complete.json"),
        &json!({"cases":cases,"total_awake_preparation_decisions":6,"source_actor_exact":true}),
    )
}
fn train() -> Result<()> {
    let (mut a, _) = actor("Untouched")?;
    let initial = a.checkpoint();
    let target = a.phenotype().candidate_decoder().motor_start() as usize + 160;
    let ids=a.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|(matches!(s.kind(),CompiledSynapseKind::Decoder(d)if d.head()==DecoderHeadKind::SpeechPayload)&&[target+16,target+17].contains(&(s.target()as usize))).then_some(i)).collect::<Vec<_>>();
    assert_eq!(ids.len(), 64);
    assert_eq!(
        ids,
        (0..initial.weights.len())
            .filter(|i| initial.stage_mask.is_trainable(*i))
            .collect::<Vec<_>>()
    );
    let mut values: Vec<Value> = read(&old()?.join("grounded-gate-corpus.json.zst"))?;
    let v28 = PathBuf::from(std::env::var("ALIFE_MATCHED_V28")?);
    for index in [1, 5, 10, 14] {
        let e: Value = read(&v28.join(format!("native-decision-{index:02}.json.zst")))?;
        let positive = e["record"]["label"] == "supported_recent";
        assert!(positive || e["record"]["label"] == "unsupported_observed_prefix");
        assert_eq!(e["record"]["actual_matching_declare"], true);
        let t = ReplaySpeechTarget {
            token: positive.then_some(9),
            continuation: if positive { [1, 0, 0, 0, 0] } else { [0; 5] },
            act: SpeechActKind::Declare,
            weight: 1.,
        };
        values.push(json!({"row":e["full_native_row"],"target":t,"positive":positive,"provenance":e["record"]}));
    }
    let data: Vec<Example> = serde_json::from_value(json!(values))?;
    assert_eq!(data.len(), 9);
    write_compressed(
        &out()?.join("exact-nine-factual-captures.json.zst"),
        &values,
    )?;
    let mut updates = vec![];
    for epoch in 1..=4 {
        let before = a.checkpoint();
        let (p, pl) = component(&a, &data, &ids, true)?;
        let (n, nl) = component(&a, &data, &ids, false)?;
        let g = p
            .iter()
            .zip(&n)
            .map(|(p, n)| 0.5 * p + 0.5 * n)
            .collect::<Vec<_>>();
        assert!(g
            .iter()
            .enumerate()
            .all(|(i, g)| ids.contains(&i) || *g == 0.));
        assert!(g.iter().any(|v| *v != 0.));
        write_compressed(
            &out()?.join(format!("gradient-MatchedFactual4-{epoch}.json.zst")),
            &g,
        )?;
        reserve(
            "native-update",
            4,
            json!({"epoch":epoch,"source_step":a.optimizer_step(),"before_sha256":identity(&before)?,"positive_mass":0.5,"absence_mass":0.5,"positive_count":4,"absence_count":5,"positive_native_gate_factor":9,"positive_gate_loss":pl,"absence_gate_loss":nl}),
        )?;
        let norm = a.apply_gradients(&g)?;
        let after = a.checkpoint();
        preserve_frozen(&before, &after)?;
        assert_eq!(after.optimizer_step, before.optimizer_step + 1);
        assert_eq!(after.config, initial.config);
        for i in &ids {
            assert_eq!(after.update_ages[*i], before.update_ages[*i] + 1);
        }
        let changed = ids
            .iter()
            .filter(|i| after.weights[**i].to_bits() != before.weights[**i].to_bits())
            .count();
        assert!(changed > 0);
        write_compressed(
            &out()?.join(format!("actor-MatchedFactual4-after-{epoch}.json.zst")),
            &after,
        )?;
        updates.push(json!({"epoch":epoch,"optimizer_step":after.optimizer_step,"gradient_norm":norm,"positive_gate_loss":pl,"absence_gate_loss":nl,"changed_gate_weights":changed,"all_other_weights_moments_and_integer_ages_exact":true,"checkpoint_sha256":identity(&after)?}));
    }
    preserve_frozen(&initial, &a.checkpoint())?;
    write_compressed(
        &out()?.join("actor-MatchedFactual4.json.zst"),
        &a.checkpoint(),
    )?;
    let (recovered, _) = actor("MatchedFactual4")?;
    assert_eq!(recovered.checkpoint(), a.checkpoint());
    write_json(
        &out()?.join("four-updates-complete.json"),
        &json!({"updates":updates,"full_optimizer_recovery_exact":true,"initial_sha256":identity(&initial)?,"final_sha256":identity(&a.checkpoint())?}),
    )
}
fn phase_name(case: usize, arm: &str, post: bool) -> String {
    format!("phase-{case}-{arm}-{post}")
}
fn evaluate(case: usize, arm: &str, post: bool) -> Result<()> {
    let (a, _) = actor(arm)?;
    let before = a.checkpoint();
    let common: Common = read(&out()?.join(format!("common-{case}.json.zst")))?;
    let name = phase_name(case, arm, post);
    let cp: CpuCanonicalCheckpoint = read(&out()?.join(format!("{name}-bound-start.json.zst")))?;
    let mut parent = CpuCanonicalHost::from_checkpoint(cp.clone(), &a)?;
    assert_eq!(identity(&parent.checkpoint(&a)?)?, identity(&cp)?);
    let id = parent.world().organism_entity_ids()[0].0;
    let mut sleep_ticks = 0;
    let mut consolidations = 0;
    if post {
        reserve(
            "sleep-request",
            12,
            json!({"case":case,"arm":arm,"pre_sha256":identity(&cp)?,"world_tick_cap":600,"expected_extra_awake_decisions":0}),
        )?;
        parent.request_recovery_sleep(id)?;
        let mut trace = zstd::stream::write::Encoder::new(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(out()?.join(format!("{name}-sleep.jsonl.zst")))?,
            3,
        )?;
        for t in 0..600 {
            storage()?;
            write_json(
                &out()?.join(format!("{name}-sleep-attempt-{t:03}.json")),
                &json!({"case":case,"arm":arm,"transition":t,"pre_tick":parent.world().tick().raw(),"maximum":600,"scored_slot_available":fs::read_dir(out()?)?.filter_map(|e|e.ok()).filter(|e|e.file_name().to_string_lossy().starts_with("scored-attempt-")).count()<704}),
            )?;
            if fs::read_dir(out()?)?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("scored-attempt-")
                })
                .count()
                >= 704
            {
                return Err("no scored slot remains for an unexpected sleep decision".into());
            }
            let step = parent.step(&a, 1.)?;
            sleep_ticks += 1;
            consolidations += step.sleep_consolidations;
            writeln!(
                trace,
                "{}",
                json!({"tick":parent.world().tick().raw(),"decisions":step.decisions,"sleeping":step.sleeping_organisms,"consolidations":step.sleep_consolidations})
            )?;
            if !step.decisions.is_empty() {
                reserve(
                    "scored",
                    704,
                    json!({"unexpected_sleep_awake_decisions":step.decisions.len(),"case":case,"arm":arm,"abort_without_additional_trials":true}),
                )?;
                trace.finish()?;
                return Err("unexpected awake decision during sleep; counted and aborting".into());
            }
            if consolidations > 0
                && parent
                    .world()
                    .organism_registry()
                    .get(id)
                    .is_some_and(|r| r.sleep_phase() == SleepPhase::Awake)
            {
                break;
            }
        }
        trace.finish()?;
        if consolidations == 0
            || parent
                .world()
                .organism_registry()
                .get(id)
                .is_none_or(|r| r.sleep_phase() != SleepPhase::Awake)
        {
            return Err("sleep cap reached without recovery".into());
        }
    }
    unseen_transfer::place_case(parent.world_mut(), common.scene, common.fixture)?;
    if common.seen && !post && arm == "Untouched" {
        assert_eq!(
            identity(&frozen_causal::before_audio(&parent, &a)?)?,
            known_parent_hash()?
        );
    }
    let expected = known_rows()?;
    let mut checkpoints = vec![];
    let mut episodes = vec![];
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out()?.join(format!("{name}-scored.jsonl.zst")))?,
        3,
    )?;
    let requests = if common.seen {
        vec![
            Some(Lesson::EatFood),
            Some(Lesson::GetFood),
            Some(Lesson::GetToy),
        ]
    } else {
        vec![
            Some(Lesson::EatFood),
            Some(Lesson::GetFood),
            Some(Lesson::GetToy),
            None,
        ]
    };
    for (trial, requested) in requests.into_iter().enumerate() {
        let mut h = parent.snapshot();
        let mut events = Vec::<Event>::new();
        checkpoints.push(save_host(
            &format!("{name}-trial-{trial}-start.json.zst"),
            &h,
            &a,
        )?);
        let mut rows = vec![];
        for tick in 0..8 {
            let cue = if tick % 2 == 0 {
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
                    true
                } else {
                    false
                }
            } else {
                false
            };
            let owned_before = owners(h.world(), common.scene);
            let n = reserve(
                "scored",
                704,
                json!({"case":case,"seed":common.seed,"seen":common.seen,"arm":arm,"post_sleep":post,"requested":requested,"tick":tick,"pre_sha256":identity(&h.checkpoint(&a)?)?}),
            )?;
            let step = h.step(&a, 1.)?;
            assert_eq!(step.decisions.len(), 1);
            let row = &step.decisions[0];
            no_teacher_or_prior(row)?;
            if tick == 0 {
                assert!(
                    [common.scene.food, common.scene.toy, common.scene.distractor]
                        .into_iter()
                        .all(|target| visible_target(row.patch.pre_action().perception(), target)),
                    "declared visible physical fixture failed; no reroll"
                );
            }
            if cue {
                let tokens = row
                    .patch
                    .pre_action()
                    .perception()
                    .sensory()
                    .language_context
                    .heard_tokens
                    .iter()
                    .take(2)
                    .map(|h| h.as_ref().map(|t| t.token_id))
                    .collect::<Vec<_>>();
                assert_eq!(
                    tokens,
                    requested
                        .ok_or("missing delivered request")?
                        .words()
                        .into_iter()
                        .map(|w| Some(u32::from(w)))
                        .collect::<Vec<_>>()
                );
            }
            if requested.is_none() {
                assert!(row
                    .patch
                    .pre_action()
                    .perception()
                    .sensory()
                    .language_context
                    .heard_tokens
                    .iter()
                    .flatten()
                    .all(|t| t.source_kind != UtteranceSourceKind::Player));
            }
            let owned_after = owners(h.world(), common.scene);
            let current = physical_event(row, common.scene, owned_before, owned_after)?;
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
            if common.seen && !post && arm == "Untouched" {
                if let Some(old) = expected.iter().find(|e| {
                    serde_json::to_value(requested)
                        .ok()
                        .is_some_and(|r| r == e["requested"])
                        && e["step"] == tick
                }) {
                    assert_eq!(identity(row)?, old["prior_native_row_sha256"]);
                    assert_eq!(serde_json::to_value(&emitted)?, old["actual_emitted"]);
                }
            }
            let r = h
                .world()
                .organism_registry()
                .get(id)
                .ok_or("missing scored organism")?;
            let declare = row.patch.selected_bundle().is_some_and(|b| {
                b.channels.iter().any(|c| {
                    c.channel == MotorChannel::Vocal && c.payload.values.get(1) == Some(&0)
                })
            });
            let raw = super::super::token_probe::speech(
                &a,
                &row.final_features,
                &row.replay_tick.effective_weight_offsets,
            );
            let payload = decode_cpu_speech(
                &a,
                &row.final_features,
                &row.replay_tick.effective_weight_offsets,
            )?;
            let metrics = json!({"scored_decision":n,"tick":row.patch.pre_action().tick.raw(),"step":tick,"prior_actual_events":events,"current_actual_event":current,"owned_before":owned_before,"owned_after":owned_after,"food_consumed":consequences(row,common.scene).0,"actual_emitted":emitted,"native_declare_selected":declare,"alive":r.lifecycle().is_alive(),"energy":r.biochemistry().body.energy,"health":r.biochemistry().body.health,"injury":r.biochemistry().body.injury,"pain":r.biochemistry().homeostasis.drives.pain,"native_row_sha256":identity(row)?,"raw_speech_outputs_six_steps":raw,"conditional_native_payload_not_emission":payload});
            writeln!(
                trace,
                "{}",
                json!({"case":case,"seed":common.seed,"seen":common.seen,"arm":arm,"post_sleep":post,"requested":requested,"metrics":metrics,"full_native_row":row})
            )?;
            trace.flush()?;
            rows.push(metrics);
            if let Some(e) = current {
                events.push(e);
            }
            assert_eq!(a.checkpoint(), before);
        }
        checkpoints.push(save_host(
            &format!("{name}-trial-{trial}-final.json.zst"),
            &h,
            &a,
        )?);
        episodes.push(json!({"requested":requested,"rows":rows}));
    }
    trace.finish()?;
    assert_eq!(a.checkpoint(), before);
    write_json(
        &out()?.join(format!("{name}-results.json")),
        &json!({"case":case,"seed":common.seed,"seen":common.seen,"arm":arm,"post_sleep":post,"actor_sha256":identity(&before)?,"optimizer_step":a.optimizer_step(),"sleep_world_ticks":sleep_ticks,"consolidations":consolidations,"extra_awake_sleep_decisions":0,"episodes":episodes,"checkpoints":checkpoints,"actor_and_optimizer_unchanged_during_assessment":true}),
    )
}
fn verify(case: usize, arm: &str, post: bool) -> Result<()> {
    let (a, _) = actor(arm)?;
    let before = a.checkpoint();
    let name = phase_name(case, arm, post);
    let result: Value =
        serde_json::from_slice(&fs::read(out()?.join(format!("{name}-results.json")))?)?;
    let mut count = 0;
    for item in result["checkpoints"]
        .as_array()
        .ok_or("missing checkpoint list")?
    {
        let cp: CpuCanonicalCheckpoint =
            read(&out()?.join(item["file"].as_str().ok_or("bad checkpoint filename")?))?;
        assert_eq!(identity(&cp)?, item["sha256"]);
        let h = CpuCanonicalHost::from_checkpoint(cp, &a)?;
        assert_eq!(identity(&h.checkpoint(&a)?)?, item["sha256"]);
        count += 1;
    }
    assert_eq!(a.checkpoint(), before);
    write_json(
        &out()?.join(format!("{name}-fresh-recovery.json")),
        &json!({"checkpoints_exact_in_fresh_process":count,"actor_and_full_optimizer_exact":true,"new_world_decisions":0}),
    )
}
fn run_phase(case: usize, arm: &str, post: bool) -> Result<()> {
    let (s, _) = actor("Untouched")?;
    let (a, _) = actor(arm)?;
    let common: Common = read(&out()?.join(format!("common-{case}.json.zst")))?;
    let h = CpuCanonicalHost::from_checkpoint(common.host, &s)?;
    let (branch, proof) = h.fork_experimental_speech_gate_policy(&s, &a)?;
    let name = phase_name(case, arm, post);
    write_json(&out()?.join(format!("{name}-policy-branch.json")), &proof)?;
    save_host(&format!("{name}-bound-start.json.zst"), &branch, &a)?;
    child("evaluate", case, arm, post)?;
    child("verify", case, arm, post)
}

#[test]
#[ignore = "explicit four-update V32 trial; 704 scored decisions; no retries"]
fn compare_matched_factual_curriculum() -> Result<()> {
    if let Ok(mode) = std::env::var("ALIFE_MATCHED_MODE") {
        let case = std::env::var("ALIFE_MATCHED_CASE")?.parse()?;
        let arm = std::env::var("ALIFE_MATCHED_ARM")?;
        let post = std::env::var("ALIFE_MATCHED_POST")? == "1";
        return match mode.as_str() {
            "evaluate" => evaluate(case, &arm, post),
            "verify" => verify(case, &arm, post),
            _ => Err("unknown isolated child mode".into()),
        };
    }
    let stage = std::env::var("ALIFE_MATCHED_STAGE")?;
    if stage == "setup" {
        fs::create_dir(out()?)?;
        storage()?;
        prepare()?;
        // Spend the first24 of the704 slots on the exact known Untouched control,
        // before any new update. It is retained and never repeated later.
        run_phase(0, "Untouched", false)?;
        let control: Value = serde_json::from_slice(&fs::read(
            out()?.join("phase-0-Untouched-false-results.json"),
        )?)?;
        let eat = &control["episodes"][0]["rows"];
        assert!(eat
            .as_array()
            .ok_or("no known Eat rows")?
            .iter()
            .any(|r| r["food_consumed"] == true));
        assert!(eat
            .as_array()
            .ok_or("no known Eat rows")?
            .iter()
            .any(|r| r["actual_emitted"]
                .as_array()
                .is_some_and(|u| u.iter().any(|u| u["tokens"] == json!([8, 1])))));
        write_json(
            &out()?.join("known-positive-control-passed.json"),
            &json!({"exact_original_native_prefix_hashes_and_emissions":true,"genuine_consumption_and_matching_declaration":true,"scored_decisions_spent":24,"new_optimizer_updates_so_far":0,"fresh_process_recovery_passed":true}),
        )?;
        return Ok(());
    }
    if stage != "continue" {
        return Err("stage must be setup or continue".into());
    }
    let admitted: Value = serde_json::from_slice(&fs::read(
        out()?.join("known-positive-control-passed.json"),
    )?)?;
    assert_eq!(admitted["scored_decisions_spent"], 24);
    assert_eq!(
        fs::read_dir(out()?)?
            .filter_map(|e| e.ok())
            .filter(|e| e
                .file_name()
                .to_string_lossy()
                .starts_with("scored-attempt-"))
            .count(),
        24
    );
    assert!(!out()?.join("four-updates-complete.json").exists());
    assert!(fs::read_dir(out()?)?.filter_map(|e| e.ok()).all(|e| !e
        .file_name()
        .to_string_lossy()
        .starts_with("native-update-attempt-")));
    train()?;
    for case in 0..3 {
        for arm in ARMS {
            for post in [false, true] {
                if case == 0 && arm == "Untouched" && !post {
                    continue;
                }
                run_phase(case, arm, post)?;
            }
        }
    }
    let count = |prefix: &str| -> Result<usize> {
        Ok(fs::read_dir(out()?)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .count())
    };
    assert_eq!(count("scored-attempt-")?, 704);
    assert_eq!(count("native-update-attempt-")?, 4);
    assert_eq!(count("awake-preparation-attempt-")?, 6);
    assert_eq!(count("sleep-request-attempt-")?, 12);
    write_json(
        &out()?.join("native-comparison-complete.json"),
        &json!({"scored_decisions":704,"held_out":512,"seen_retention":192,"awake_preparation_decisions":6,"unexpected_awake_sleep_decisions":0,"sleep_requests":12,"new_native_adam_updates":4,"controls_retrained":false,"all_phase_cold_recoveries_passed":true,"no_promotion":true}),
    )
}
