//! Diagnostics on captured real simulation rows; never a scored learning result.
use super::*;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
};

#[derive(Deserialize)]
struct Label {
    representative: Option<u32>,
    motors: [Option<u32>; 6],
}
#[derive(Deserialize)]
struct Captured {
    arm: Arm,
    round: usize,
    layout: usize,
    requested: Goal,
    row: CpuCollectedDecision,
    label: Label,
    speech_target: ReplaySpeechTarget,
}

fn corpus(path: &Path, arm: Arm) -> Result<Vec<Episode>> {
    let mut episodes = BTreeMap::<(usize, usize, usize), Episode>::new();
    for line in BufReader::new(zstd::stream::read::Decoder::new(fs::File::open(path)?)?).lines() {
        let r: Captured = serde_json::from_str(&line?)?;
        if r.arm != arm {
            continue;
        }
        let e = episodes
            .entry((r.round, r.layout, r.requested.index()))
            .or_insert_with(|| Episode {
                rows: vec![],
                labels: vec![],
                speech: vec![],
            });
        e.rows.push(r.row);
        e.labels.push(ImitationTarget {
            representative: r.label.representative,
            motors: r.label.motors,
        });
        e.speech.push(Some(r.speech_target));
    }
    Ok(episodes.into_values().collect())
}
fn gates(actor: &CpuFoundationTrainer, a: &[f32], offset: &[f32]) -> [f32; 2] {
    let mut out = [0.; 2];
    let start = actor.phenotype().candidate_decoder().motor_start() + 160;
    for (i, s) in actor.phenotype().synapses().iter().enumerate() {
        if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)
            && (start + 16..start + 18).contains(&s.target())
        {
            out[(s.target() - start - 16) as usize] +=
                a[s.source() as usize] * (actor.weights()[i] + offset[i]);
        }
    }
    out
}
fn measurements(actor: &CpuFoundationTrainer, episodes: &[Episode]) -> Result<Value> {
    let mut records = vec![];
    let (mut selected, mut quiet_selected, mut positive, mut enabled) = (0, 0, 0, 0);
    let (mut actual, mut replay, mut single) = (vec![], vec![], vec![]);
    for e in episodes {
        let q = sequence(actor, e, e.rows.len(), 0);
        let evaluated = actor.evaluate_replay(&q)?;
        for (i, r) in e.rows.iter().enumerate() {
            let t = e.speech[i].unwrap();
            let vocal = r.action.motor_candidates[3].is_some();
            selected += usize::from(vocal);
            quiet_selected += usize::from(vocal && t.token.is_none());
            enabled += usize::from(r.action.motor_masks[3] != 0);
            if t.token.is_none() {
                continue;
            }
            positive += 1;
            let mut one = q.clone();
            one.initial = r.initial.clone();
            one.ticks = vec![r.replay_tick.clone()];
            let one = actor.evaluate_replay(&one)?;
            let observed = gates(
                actor,
                &r.final_features,
                &r.replay_tick.effective_weight_offsets,
            );
            let full = gates(
                actor,
                &evaluated.final_activations[i],
                &r.replay_tick.effective_weight_offsets,
            );
            let detached = gates(
                actor,
                &one.final_activations[0],
                &r.replay_tick.effective_weight_offsets,
            );
            actual.push(observed[0]);
            replay.push(full[0]);
            single.push(detached[0]);
            records.push(json!({"tick":r.patch.pre_action().tick.raw(),"captured_feature_readout":observed,"full_replay":full,"one_step_replay":detached,"positive_token":t.token,"selected_vocalize":vocal}));
        }
    }
    let range = |v: &Vec<f32>| {
        [
            v.iter().copied().fold(f32::INFINITY, f32::min),
            v.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        ]
    };
    Ok(
        json!({"positive_rows":positive,"collected_selected_vocalize":selected,"collected_quiet_selected_vocalize":quiet_selected,"vocal_conditional_mask_enabled_rows":enabled,"captured_feature_presence_range":range(&actual),"full_replay_presence_range":range(&replay),"one_step_presence_range":range(&single),"rows":records}),
    )
}
fn speech_gradient(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
    class: usize,
) -> Result<Vec<f32>> {
    let mut g = vec![0.; actor.weights().len()];
    let mass = episodes
        .iter()
        .flat_map(|e| e.speech.iter().flatten())
        .filter(|t| usize::from(t.token.is_none()) == class)
        .map(|t| t.weight * t.len() as f32)
        .sum::<f32>();
    for e in episodes {
        let q = sequence(actor, e, e.rows.len(), 0);
        let targets = e
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
        let zero = q
            .ticks
            .iter()
            .map(|t| vec![0.; t.candidates.len()])
            .collect::<Vec<_>>();
        let p = actor.probe_replay_gradients_with_speech(&q, &zero, &targets)?;
        for (dst, src) in g.iter_mut().zip(p.gradients) {
            *dst += src * amount / mass;
        }
    }
    Ok(g)
}
fn positive_loss(actor: &CpuFoundationTrainer, episodes: &[Episode]) -> Result<f64> {
    let mass = episodes
        .iter()
        .flat_map(|e| e.speech.iter().flatten())
        .filter(|t| t.token.is_some())
        .map(|t| t.weight * t.len() as f32)
        .sum::<f32>();
    let mut loss = 0.;
    for e in episodes {
        let q = sequence(actor, e, e.rows.len(), 0);
        let targets = e
            .speech
            .iter()
            .map(|t| t.filter(|t| t.token.is_some()))
            .collect::<Vec<_>>();
        let amount = targets
            .iter()
            .flatten()
            .map(|t| t.weight * t.len() as f32)
            .sum::<f32>();
        if amount == 0. {
            continue;
        }
        let zero = q
            .ticks
            .iter()
            .map(|t| vec![0.; t.candidates.len()])
            .collect::<Vec<_>>();
        loss += actor
            .probe_replay_gradients_with_speech(&q, &zero, &targets)?
            .objective
            * f64::from(amount / mass);
    }
    Ok(loss)
}
fn gate_finite_difference(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    episodes: &[Episode],
) -> Result<Value> {
    let e = episodes
        .iter()
        .find(|e| e.speech.iter().flatten().any(|t| t.token.is_some()))
        .ok_or("missing real positive")?;
    let q = sequence(actor, e, e.rows.len(), 0);
    let targets = e
        .speech
        .iter()
        .map(|t| t.filter(|t| t.token.is_some()))
        .collect::<Vec<_>>();
    let zero = q
        .ticks
        .iter()
        .map(|t| vec![0.; t.candidates.len()])
        .collect::<Vec<_>>();
    let base = actor.probe_replay_gradients_with_speech(&q, &zero, &targets)?;
    let target = actor.phenotype().candidate_decoder().motor_start() + 176;
    let id=actor.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::SpeechPayload)&&s.target()==target).max_by(|(i,_),(j,_)|base.gradients[*i].abs().total_cmp(&base.gradients[*j].abs())).map(|(i,_)|i).ok_or("no presence coordinate")?;
    let mut loss = vec![];
    let epsilon = 0.001;
    for change in [-epsilon, epsilon] {
        let mut cp = actor.checkpoint();
        cp.weights[id] += change;
        let a =
            CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset.clone(), cp)?;
        loss.push(
            a.probe_replay_gradients_with_speech(&q, &zero, &targets)?
                .objective,
        );
    }
    let finite = (loss[1] - loss[0]) / (2. * f64::from(epsilon));
    let analytic = f64::from(base.gradients[id]);
    if (finite - analytic).abs() > 0.002 {
        return Err("real positive presence finite difference mismatch".into());
    }
    Ok(
        json!({"coordinate":id,"native_analytic_gradient":analytic,"central_finite_difference":finite,"epsilon":epsilon,"actual_positive_labels_only":true}),
    )
}
#[test]
#[ignore = "explicit saved public V2 simulation gate diagnosis; no model request"]
fn diagnose_actual_v2_gate_targets_and_native_gradients() {
    let dir = std::path::PathBuf::from(std::env::var("ALIFE_GATE_DIAGNOSTIC_DIR").unwrap());
    fs::create_dir(&dir).unwrap();
    let source = std::path::PathBuf::from(
        std::env::var("ALIFE_GATE_SOURCE")
            .unwrap_or_else(|_| "/workspace/language-embodied-run".into()),
    );
    let mut arms = vec![];
    for arm in [Arm::Grounded, Arm::Shuffled] {
        let r = read_recovery(&source.join(format!("recovery-{arm:?}"))).unwrap();
        let p =
            PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048()).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(&r.foundation).unwrap();
        let final_actor = CpuFoundationTrainer::from_checkpoint(p, asset, r.actor).unwrap();
        let r = read_recovery(
            &Path::new("/workspace/language-shared-gate-continued")
                .join(format!("recovery-{arm:?}")),
        )
        .unwrap();
        let p =
            PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048()).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(&r.foundation).unwrap();
        let old = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor).unwrap();
        let (mut initial, _) = old
            .fork_embodied_language_input_experiment(&r.inputs)
            .unwrap();
        let ids = initial
            .phenotype()
            .synapses()
            .iter()
            .enumerate()
            .filter_map(|(i, s)| match s.kind() {
                CompiledSynapseKind::Recurrent => Some(i as u32),
                CompiledSynapseKind::Decoder(c)
                    if matches!(
                        c.head(),
                        DecoderHeadKind::ActionCandidate | DecoderHeadKind::SpeechPayload
                    ) =>
                {
                    Some(i as u32)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        initial
            .set_stage_mask(
                StageTrainableMask::from_synapse_indices(initial.phenotype(), &ids).unwrap(),
            )
            .unwrap();
        let e = corpus(&source.join("native-trace.jsonl.zst"), arm).unwrap();
        let pos = speech_gradient(&initial, &e, 0).unwrap();
        let quiet = speech_gradient(&initial, &e, 1).unwrap();
        let (action, _) = gradients_with_speech(&initial, &e, false).unwrap();
        let norm = |g: &Vec<f32>, head: Option<DecoderHeadKind>, gate: bool| -> f64 {
            initial
                .phenotype()
                .synapses()
                .iter()
                .enumerate()
                .filter(|(_, s)| match s.kind() {
                    CompiledSynapseKind::Recurrent => head.is_none(),
                    CompiledSynapseKind::Decoder(c) => {
                        head == Some(c.head()) && (!gate || c.motor_index() == 16)
                    }
                })
                .map(|(i, _)| f64::from(g[i]).powi(2))
                .sum::<f64>()
                .sqrt()
        };
        let presence_target = initial.phenotype().candidate_decoder().motor_start() + 176;
        let presence_ids=initial.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::SpeechPayload)&&s.target()==presence_target).map(|(i,_)|i).collect::<Vec<_>>();
        let dot = presence_ids
            .iter()
            .map(|&i| f64::from(pos[i]) * f64::from(quiet[i]))
            .sum::<f64>();
        let squared = presence_ids
            .iter()
            .map(|&i| f64::from(pos[i]).powi(2))
            .sum::<f64>();
        let quiet_squared = presence_ids
            .iter()
            .map(|&i| f64::from(quiet[i]).powi(2))
            .sum::<f64>();
        let baseline_positive_loss = positive_loss(&initial, &e).unwrap();
        let mut interventions = vec![];
        for (name, direction) in [
            (
                "previous_balanced_presence_direction",
                presence_ids
                    .iter()
                    .map(|&i| 0.9 * pos[i] + 0.1 * quiet[i])
                    .collect::<Vec<_>>(),
            ),
            (
                "positive_presence_direction",
                presence_ids.iter().map(|&i| pos[i]).collect::<Vec<_>>(),
            ),
        ] {
            let norm = direction.iter().map(|x| x * x).sum::<f32>().sqrt();
            let mut cp = initial.checkpoint();
            for (&id, g) in presence_ids.iter().zip(direction) {
                cp.weights[id] -= 0.01 * g / norm;
            }
            let perturbed = CpuFoundationTrainer::from_checkpoint(
                initial.phenotype().clone(),
                asset.clone(),
                cp,
            )
            .unwrap();
            interventions.push(json!({"kind":name,"presence_weight_l2_perturbation":0.01,"positive_loss_before":baseline_positive_loss,"positive_loss_after":positive_loss(&perturbed,&e).unwrap(),"no_optimizer_commit_or_behavior_claim":true}));
        }
        let gate_direction = json!({"positive_quiet_dot":dot,"positive_quiet_cosine":dot/(squared*quiet_squared).sqrt(),"positive_dot_previous_09_01_gate_gradient":0.9*squared+0.1*dot,"negative_value_means_previous_gate_descent_increases_positive_loss_locally":true,"coordinate_count":presence_ids.len()});
        arms.push(json!({"arm":arm,"initial_actor_sha256":identity(&initial.checkpoint()).unwrap(),"presence_gradient_interaction":gate_direction,"direct_presence_weight_interventions":interventions,"presence_finite_difference":gate_finite_difference(&initial,&asset,&e).unwrap(),"initial":measurements(&initial,&e).unwrap(),"trained":measurements(&final_actor,&e).unwrap(),"gradient_norms":{"positive_speech_decoder":norm(&pos,Some(DecoderHeadKind::SpeechPayload),false),"quiet_speech_decoder":norm(&quiet,Some(DecoderHeadKind::SpeechPayload),false),"positive_recurrent":norm(&pos,None,false),"quiet_recurrent":norm(&quiet,None,false),"action_recurrent":norm(&action,None,false),"action_speech_decoder":norm(&action,Some(DecoderHeadKind::SpeechPayload),false),"positive_presence_decoder":norm(&pos,Some(DecoderHeadKind::SpeechPayload),true),"quiet_presence_decoder":norm(&quiet,Some(DecoderHeadKind::SpeechPayload),true)}}));
    }
    write_json(&dir.join("gate-diagnostic.json"),&json!({"kind":"actual-v2-native-gate-gradient-diagnostic","not_behavior_or_learning_claim":true,"arms":arms})).unwrap();
}
