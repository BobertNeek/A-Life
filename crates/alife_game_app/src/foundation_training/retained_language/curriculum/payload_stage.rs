//! Native speech-only refinement on recorded real experiences, with corpus-wide
//! positive/silence budgets. No classifier, forced motor or output substitution.
use super::*;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
};

#[derive(Deserialize)]
struct Captured {
    arm: Arm,
    round: usize,
    layout: usize,
    requested: Goal,
    row: CpuCollectedDecision,
    speech_target: ReplaySpeechTarget,
}

fn corpus_path(source: &Path) -> std::path::PathBuf {
    let retained = source.join("retained-training-corpus.jsonl.zst");
    if retained.is_file() {
        retained
    } else {
        source.join("native-trace.jsonl.zst")
    }
}

fn corpus(source: &Path, arm: Arm) -> Result<Vec<Episode>> {
    let mut episodes = BTreeMap::<(usize, usize, usize), Episode>::new();
    let input = BufReader::new(zstd::stream::read::Decoder::new(fs::File::open(
        corpus_path(source),
    )?)?);
    for line in input.lines() {
        let row: Captured = serde_json::from_str(&line?)?;
        if row.arm != arm {
            continue;
        }
        let episode = episodes
            .entry((row.round, row.layout, row.requested.index()))
            .or_insert_with(|| Episode {
                rows: Vec::new(),
                labels: Vec::new(),
                speech: Vec::new(),
            });
        episode.rows.push(row.row);
        episode.labels.push(ImitationTarget {
            representative: None,
            motors: [None; 6],
        });
        episode.speech.push(Some(row.speech_target));
    }
    if episodes.is_empty() {
        return Err("missing actual native payload corpus".into());
    }
    Ok(episodes.into_values().collect())
}

fn mass(episodes: &[Episode]) -> [f32; 2] {
    let mut total = [0.; 2];
    for target in episodes.iter().flat_map(|e| e.speech.iter().flatten()) {
        total[usize::from(target.token.is_none())] += target.weight * target.len() as f32;
    }
    total
}

fn balanced_gradient(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
) -> Result<(Vec<f32>, Value)> {
    weighted_gradient(actor, episodes, [0.5, 0.5])
}

pub(super) fn weighted_gradient(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
    class_budget: [f32; 2],
) -> Result<(Vec<f32>, Value)> {
    let total = mass(episodes);
    if total.iter().any(|m| *m <= 0.) {
        return Err("global balancing requires genuine positive and silence labels".into());
    }
    let mut gradient = vec![0.; actor.weights().len()];
    let mut objective = [0f64; 2];
    let mut budgets = [0f64; 2];
    for episode in episodes {
        let q = sequence(actor, episode, episode.rows.len(), 0);
        let zero = episode
            .rows
            .iter()
            .map(|r| vec![0.; r.replay_tick.candidates.len()])
            .collect::<Vec<_>>();
        for class in 0..2 {
            let targets = episode
                .speech
                .iter()
                .map(|t| t.filter(|t| usize::from(t.token.is_none()) == class))
                .collect::<Vec<_>>();
            let amount = targets
                .iter()
                .flatten()
                .map(|t| t.weight * t.len() as f32)
                .sum::<f32>();
            if amount == 0. {
                continue;
            }
            // The native probe normalizes this single class locally. Weight
            // by its actual share of the corpus and the explicit GLOBAL budget.
            let scale = class_budget[class] * amount / total[class];
            let probe = actor.probe_replay_gradients_with_speech(&q, &zero, &targets)?;
            objective[class] += probe.objective * f64::from(scale);
            budgets[class] += f64::from(scale);
            for (dst, src) in gradient.iter_mut().zip(probe.gradients) {
                *dst += src * scale;
            }
        }
    }
    let mask = actor.checkpoint().stage_mask;
    for (i, value) in gradient.iter_mut().enumerate() {
        if !mask.is_trainable(i) {
            *value = 0.;
        }
    }
    Ok((
        gradient,
        json!({"positive_loss":objective[0],"silence_loss":objective[1],"positive_budget":budgets[0],"silence_budget":budgets[1],"positive_presence_output_budget":budgets[0]/18.,"silence_presence_output_budget":budgets[1]/2.,"positive_token_mass":total[0],"silence_token_mass":total[1]}),
    ))
}

pub fn refine_language_payload(
    source: &Path,
    output: &Path,
    config: LanguagePilotConfig,
) -> Result<Value> {
    refine(source, output, config, false)
}

pub fn refine_language_shared_gates(
    source: &Path,
    output: &Path,
    config: LanguagePilotConfig,
) -> Result<Value> {
    refine(source, output, config, true)
}

fn refine(
    source: &Path,
    output: &Path,
    config: LanguagePilotConfig,
    shared_gates: bool,
) -> Result<Value> {
    if !config.experimental_word_bridge
        || !(1..=64).contains(&config.epochs)
        || config.rounds != 1
        || !(24..=64).contains(&config.decisions)
    {
        return Err(
            "payload stage requires explicit bridge, rounds=1, epochs 1..64, decisions 24..64"
                .into(),
        );
    }
    let previous: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if !matches!(
        previous["kind"].as_str(),
        Some(
            "experimental-request-contingent-language-v4"
                | "experimental-global-payload-balance-v5"
                | "experimental-shared-speech-gates-v6"
        )
    ) || previous["config"]["seed"] != config.seed
    {
        return Err("payload stage only accepts matching measured disposable curriculum".into());
    }
    fs::create_dir(output)?;
    let started = Instant::now();
    let mut arms = Vec::new();
    let corpus_source = corpus_path(source);
    let corpus_sha = hash(&fs::read(&corpus_source)?);
    if previous
        .get("real_training_corpus_sha256")
        .and_then(Value::as_str)
        .is_some_and(|expected| expected != corpus_sha)
    {
        return Err("retained actual training corpus differs from the recorded source".into());
    }
    fs::copy(
        &corpus_source,
        output.join("retained-training-corpus.jsonl.zst"),
    )?;
    for arm in [Arm::Grounded, Arm::Shuffled] {
        let saved = read_recovery(&source.join(format!("recovery-{arm:?}")))?;
        if saved.kind != "disposable-retained-language-pilot-v1"
            || saved.inputs.experimental_language_input().is_none()
        {
            return Err("not an experimental public-founder recovery".into());
        }
        let p = PhenotypeCompiler::compile_validated(&saved.inputs, &BrainCapacityClass::n2048())?;
        let mut asset = FoundationWeightAsset::decode_canonical(&saved.foundation)?;
        let mut actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), saved.actor)?;
        let life = CpuCanonicalHost::from_checkpoint(saved.host, &actor)?;
        let prior = previous["arms"]
            .as_array()
            .ok_or("missing source arms")?
            .iter()
            .find(|v| v["arm"] == json!(arm))
            .ok_or("missing source arm")?;
        let source_actor = identity(&actor.checkpoint())?;
        if source_actor != prior["actor_sha256"] {
            return Err("payload actor differs from measured source".into());
        }
        let source_private = identity(&life.checkpoint(&actor)?)?;
        let start_step = actor.optimizer_step();
        let ids=actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload).then_some(i as u32)).collect::<Vec<_>>();
        actor.set_stage_mask(StageTrainableMask::from_synapse_indices(
            actor.phenotype(),
            &ids,
        )?)?;
        let episodes = corpus(source, arm)?;
        let mut updates = Vec::new();
        let mut next = None;
        eprintln!(
            "native payload {arm:?}: {} real episodes, mass {:?}",
            episodes.len(),
            mass(&episodes)
        );
        for epoch in 0..config.epochs {
            let (gradient, objective) = if shared_gates {
                // Presence/stop appear in both classes: 18 positive outputs
                // versus two silence outputs. Equal per-gate weight needs
                // class budgets 18/20 and 2/20, not equal class totals.
                weighted_gradient(&actor, &episodes, [0.9, 0.1])?
            } else {
                balanced_gradient(&actor, &episodes)?
            };
            let before = actor.checkpoint();
            let norm = actor.apply_gradients(&gradient)?;
            preserve_frozen(&before, &actor.checkpoint())?;
            updates.push(json!({"epoch":epoch,"step":actor.optimizer_step(),"objective":objective,"gradient_norm":norm,"all_other_weights_moments_and_ages_exact":true}));
            next = Some(gradient);
            if epoch % 8 == 0 {
                eprintln!("native payload {arm:?}: epoch {epoch}");
            }
        }
        asset = export(&mut actor, config.seed)?;
        eprintln!("native payload {arm:?}: cold world/sleep assessment");
        let continuity = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config,
                arm,
                curriculum: Some(AssessmentPlan {
                    recovery_sleep: true,
                    collect_utterance_timing: false,
                    factorial_grounding: false,
                    paired_request_focus: false,
                    linear_readout: false,
                    balanced_grounding_v1: false,
                    balanced_grounding_v2: false,
                    event_confirmed_speech_v1: false,
                    unseen_report_transfer_v1: false,
                    physical_before_speech_v1: false,
                    captured_failure_replay_v1: false,
                    replay_early_stop_v1: false,
                    care_retention_v1: false,
                }),
            },
            next.ok_or("no payload update")?,
        )?;
        let evaluation =
            continuity["native_private_and_optimizer_hashes"]["assessment"]["evaluation"].clone();
        arms.push(json!({"arm":arm,"source_actor_sha256":source_actor,"source_private_sha256":source_private,"source_step":start_step,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"speech_only_coordinates":ids.len(),"updates":updates,"evaluation":evaluation,"continuity":continuity}));
    }
    let report = json!({"kind":if shared_gates{"experimental-shared-speech-gates-v6"}else{"experimental-global-payload-balance-v5"},"execution_domain":"native-cpu","config":config,"source_report_sha256":identity(&previous)?,"real_training_corpus_sha256":corpus_sha,"semantic_prior_enabled":false,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"limitations":["Native payload-only stage on actual saved experiences; recurrent/action coordinates and all their optimizer history remain exact.","No additional world transitions/reward or forced assessment action. Corpus contexts remain fixed under conditional replay.","Both source treatment histories retain truthful activity narration; shuffled differs in request alignment.","Explicit native RecoveryProtocol sleep, not spontaneous natural sleep.","Fresh cohorts test retained foundation learning; this is not same-individual online optimizer rebase.","Real SLM delivery remains unavailable; no model output is claimed."]});
    write_json(&output.join("results.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_gate_budget_removes_native_silence_bias_at_zero_presence() {
        let seed = 847_503;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let first = new_actor_profile(seed, &asset, true).unwrap();
        let mut weights = first.weights().to_vec();
        for (i, s) in first.phenotype().synapses().iter().enumerate() {
            if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)
            {
                weights[i] = 0.;
            }
        }
        let asset = FoundationWeightAsset::from_trained_weights(
            first.phenotype(),
            weights,
            TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let mut actor = new_actor_profile(seed, &asset, true).unwrap();
        let ids=actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload).then_some(i as u32)).collect::<Vec<_>>();
        actor
            .set_stage_mask(
                StageTrainableMask::from_synapse_indices(actor.phenotype(), &ids).unwrap(),
            )
            .unwrap();
        let (mut host, _) = base_host(&actor, &asset, seed + 70_000, seed).unwrap();
        let id = host.world().organism_entity_ids()[0].0;
        let pos = speech_position(host.world()).unwrap();
        host.world_mut()
            .emit_player_tokens(
                Some(id),
                pos,
                vec![
                    LanguageTokenId::new(8).unwrap(),
                    LanguageTokenId::new(1).unwrap(),
                ],
            )
            .unwrap();
        let row = host.step(&actor, 1.).unwrap().decisions.remove(0);
        // Contradictory labels on the same real frame isolate objective bias;
        // this is a gradient fixture, never a grounded-learning claim.
        let episodes = [true, false].map(|positive| Episode {
            rows: vec![row.clone()],
            labels: vec![ImitationTarget {
                representative: None,
                motors: [None; 6],
            }],
            speech: vec![Some(ReplaySpeechTarget {
                token: positive.then_some(8),
                continuation: [0; 5],
                act: SpeechActKind::Declare,
                weight: 1.,
            })],
        });
        let (equal, _) = balanced_gradient(&actor, &episodes).unwrap();
        let (shared, objective) = weighted_gradient(&actor, &episodes, [0.9, 0.1]).unwrap();
        let target = actor.phenotype().candidate_decoder().motor_start() + 160 + 16;
        let gate=actor.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)&&s.target()==target).map(|(i,_)|i).collect::<Vec<_>>();
        assert!(gate.iter().any(|&i| equal[i].abs() > 1e-5));
        assert!(gate.iter().all(|&i| shared[i].abs() < 1e-6));
        assert!(
            (objective["positive_presence_output_budget"]
                .as_f64()
                .unwrap()
                - objective["silence_presence_output_budget"]
                    .as_f64()
                    .unwrap())
            .abs()
                < 1e-7
        );
    }

    #[test]
    fn duplicating_silence_does_not_change_global_native_payload_objective() {
        let seed = 847_501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let mut actor = new_actor_profile(seed, &asset, true).unwrap();
        let ids=actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload).then_some(i as u32)).collect::<Vec<_>>();
        actor
            .set_stage_mask(
                StageTrainableMask::from_synapse_indices(actor.phenotype(), &ids).unwrap(),
            )
            .unwrap();
        let (mut host, _) = base_host(&actor, &asset, seed + 70_000, seed).unwrap();
        let row = host.step(&actor, 1.).unwrap().decisions.remove(0);
        // A normalization fixture on an actual native frame, not a claimed
        // physical completion or a behavior/learning success.
        let make = |positive: bool| Episode {
            rows: vec![row.clone()],
            labels: vec![ImitationTarget {
                representative: None,
                motors: [None; 6],
            }],
            speech: vec![Some(ReplaySpeechTarget {
                token: positive.then_some(8),
                continuation: if positive { [1, 0, 0, 0, 0] } else { [0; 5] },
                act: SpeechActKind::Declare,
                weight: if positive { 1. } else { 0.25 },
            })],
        };
        let (one, one_loss) = balanced_gradient(&actor, &[make(true), make(false)]).unwrap();
        let mut repeated = vec![make(true)];
        repeated.extend((0..9).map(|_| make(false)));
        let (many, many_loss) = balanced_gradient(&actor, &repeated).unwrap();
        for (a, b) in one.iter().zip(many) {
            assert!((a - b).abs() < 1e-5);
        }
        for field in [
            "positive_loss",
            "silence_loss",
            "positive_budget",
            "silence_budget",
        ] {
            assert!(
                (one_loss[field].as_f64().unwrap() - many_loss[field].as_f64().unwrap()).abs()
                    < 1e-6
            );
        }
        assert!(balanced_gradient(&actor, &[make(false)]).is_err());
    }
}
