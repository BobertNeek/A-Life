//! Eight detached forwards on two immutable native captures. No world step,
//! gradient, optimizer, admission override, or hypothetical world evidence.
use super::super::token_probe::speech;
use super::bounded_tape_probe::{source_actor, write_compressed};
use super::*;
use alife_runtime::canonical_preparation::*;
use alife_training::{TrainingReplayEvaluation, TrainingReplayTick};
use std::path::PathBuf;

#[derive(Deserialize)]
struct SavedCase {
    record: Value,
    full_native_row: CpuCollectedDecision,
    full_host_before: CpuCanonicalCheckpoint,
    full_host_after: CpuCanonicalCheckpoint,
}
fn read_compressed<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_reader(zstd::stream::read::Decoder::new(
        fs::File::open(path)?,
    )?)?)
}
fn exact_floats(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
}
fn delta(a: &[f32], b: &[f32]) -> Value {
    assert_eq!(a.len(), b.len());
    let d = a
        .iter()
        .zip(b)
        .map(|(a, b)| f64::from(*a) - f64::from(*b))
        .collect::<Vec<_>>();
    json!({"changed":d.iter().filter(|x|**x!=0.).count(),"l2":d.iter().map(|x|x*x).sum::<f64>().sqrt(),"max_abs":d.iter().map(|x|x.abs()).fold(0.,f64::max)})
}
fn protect(preregistration: &Path) -> Result<usize> {
    let v: Value = serde_json::from_slice(&fs::read(preregistration)?)?;
    let files = v["protected_files"]
        .as_object()
        .ok_or("no preservation map")?;
    for (path, expected) in files {
        assert_eq!(hash(&fs::read(path)?), expected.as_str().ok_or("bad hash")?);
    }
    Ok(files.len())
}

// Native input.rs arithmetic, in the same f32 operation/order, with just one
// explicitly detached source override. This never constructs an admitted frame.
// Both rows must match encode_cpu_frame AND the captured vector bit for bit
// with None before the override is allowed to be used in a neural forward.
fn encoder_arithmetic(
    p: &BrainPhenotype,
    frame: &PerceptionFrame,
    effects: &NeuralReceptorEffects,
    holding: Option<f32>,
) -> Vec<f32> {
    let mut sensory = frame.sensory().channels.as_flat_array();
    if let Some(value) = holding {
        sensory[34] = value;
    }
    let body = frame.body().neural_projection(frame.sensor_profile());
    let t = body.pose.translation;
    let r = body.pose.rotation;
    let v = body.velocity.linear;
    let a = body.velocity.angular;
    let body = [
        t.x, t.y, t.z, r.x, r.y, r.z, r.w, v.x, v.y, v.z, a.x, a.y, a.z,
    ];
    let home = frame
        .homeostasis()
        .drives
        .to_array()
        .into_iter()
        .chain(frame.homeostasis().hormones.to_array())
        .collect::<Vec<_>>();
    let language = frame.sensory().language_prior_neural_lanes();
    let mut encoded = vec![0.; p.neuron_count() as usize];
    for assignment in p.sensor_encoder().assignments() {
        let index = assignment.source_index() as usize;
        let source = match assignment.source_group() {
            SensorEncoderSourceGroup::SensoryChannel => sensory[index] * effects.attention_gain,
            SensorEncoderSourceGroup::Body => body[index],
            SensorEncoderSourceGroup::Homeostasis => home[index] * effects.interoceptive_gain,
            SensorEncoderSourceGroup::HeardLanguage => language[index],
            SensorEncoderSourceGroup::SemanticPrior => language[128 + index],
        };
        let (min, max) = assignment.clamp_range();
        encoded[assignment.target_neuron() as usize] +=
            (source * assignment.scale() + assignment.bias()).clamp(min, max);
    }
    let gate =
        (effects.structural_growth_gate + effects.sleep_gate + effects.consolidation_gate) / 3.;
    for value in &mut encoded {
        *value = (*value * effects.regional_excitability + (gate - 0.5) * 0.05).clamp(-1., 1.);
    }
    encoded
}

fn reconstruct(actor: &CpuFoundationTrainer, case: &SavedCase, index: usize) -> Result<Value> {
    let p = actor.phenotype();
    let row = &case.full_native_row;
    let frame = row.patch.pre_action().perception();
    let host = CpuCanonicalHost::from_checkpoint(case.full_host_before.clone(), actor)?;
    assert_eq!(
        identity(&host.checkpoint(actor)?)?,
        identity(&case.full_host_before)?
    );
    let record = host
        .world()
        .organism_registry()
        .get(row.organism_id)
        .ok_or("missing captured organism")?;
    let resident = case
        .full_host_before
        .residents
        .iter()
        .find(|r| r.neural.organism_id == row.organism_id)
        .ok_or("missing captured resident")?;
    assert!(row.initial == resident.neural.neural);
    let neural = CpuOrganismState::from_checkpoint(p, resident.neural.clone())?;
    assert!(exact_floats(
        &neural.effective_offsets(p)?,
        &row.replay_tick.effective_weight_offsets
    ));
    assert_eq!(
        case.full_host_before.sampling_counter,
        row.sampling_counter.ok_or("no sampler")?
    );
    no_teacher_or_prior(row)?;
    let receptors = record
        .biochemistry()
        .neural_receptor_frame(record.phenotype())?;
    let effects =
        NeuralReceptorEffects::from_frame(&receptors, &NeuralReceptorPhenotype::compile(p)?)?;
    assert_eq!(effects.source_tick, frame.tick());
    assert_eq!(
        effects.projection_gain.to_bits(),
        row.replay_tick.projection_gain.to_bits()
    );
    assert_eq!(
        effects.local_threshold_shift.to_bits(),
        row.replay_tick.local_threshold_shift.to_bits()
    );
    assert_eq!(&record.biochemistry().homeostasis, frame.homeostasis());
    let mut perception_world = host.world().clone();
    let draft = perception_world.perception_frame_draft(
        row.organism_id,
        frame.tick(),
        p.sensor_profile(),
        record.biochemistry().homeostasis,
    )?;
    let memory = MemorySidecarState::restore_portable(
        resident.memory.profile,
        resident.memory_compaction,
        resident.memory.clone(),
        resident.staged_memory.clone(),
    )?;
    let topology = TopologySidecar::restore_portable(resident.topology.clone())?;
    let prepared = prepare_canonical_frame(
        &CanonicalPreparationInput {
            draft,
            sequence_id: ExperienceSequenceId(resident.next_sequence),
            homeostasis: record.biochemistry().homeostasis,
            hysteresis: resident.hysteresis,
            policy: attention_selection_policy_for(p),
            neural_receptors: receptors.clone(),
            receptor_effects: effects,
        },
        &memory,
        &topology,
        &resident.predictor,
        false,
    )
    .result?;
    assert!(
        prepared.frame == *frame,
        "canonical frame not reproduced for row {index}"
    );
    // These captures have no sleep-grown sparse edges. Do not silently guess a
    // structural context in any later dataset.
    assert!(row.replay_tick.structural_synapses.is_empty());
    assert!(resident
        .neural_topology
        .sparse_spans
        .iter()
        .all(|s| s.edges.is_empty()));
    let native = encode_cpu_frame(
        p,
        &prepared.frame,
        &prepared.memory_recall,
        &effects,
        row.replay_tick.enabled_routes.clone(),
        row.replay_tick.structural_synapses.clone(),
    )?;
    assert!(exact_floats(
        &native.encoded_inputs,
        &row.replay_tick.encoded_inputs
    ));
    assert_eq!(native.candidates, row.replay_tick.candidates);
    let reconstructed = encoder_arithmetic(p, frame, &effects, None);
    assert!(exact_floats(&reconstructed, &native.encoded_inputs));
    let raw = frame.sensory().channels.as_flat_array()[34];
    assert!(raw == 0. || raw == 1.);
    let changed = encoder_arithmetic(p, frame, &effects, Some(1. - raw));
    let assignments = p.sensor_encoder().assignments().iter().filter(|a| {
        a.source_group() == SensorEncoderSourceGroup::SensoryChannel && a.source_index() == 34
    }).map(|a| json!({"target":a.target_neuron(),"scale":a.scale(),"bias":a.bias(),"clamp":a.clamp_range()})).collect::<Vec<_>>();
    let differences = reconstructed
        .iter()
        .zip(&changed)
        .enumerate()
        .filter(|(_, (a, b))| a.to_bits() != b.to_bits())
        .map(|(i, (a, b))| json!({"target":i,"baseline":a,"holding_toggle":b}))
        .collect::<Vec<_>>();
    assert!(!differences.is_empty());
    assert!(differences
        .iter()
        .all(|d| assignments.iter().any(|a| a["target"] == d["target"])));
    // Return evidence plus the detached vector; the canonical frame/receipt is
    // never edited or re-admitted under the counterfactual.
    Ok(
        json!({"row":index,"native_row_sha256":identity(row)?,"frame_sha256":identity(frame)?,"memory_recall_receipt_sha256":identity(prepared.memory_recall.receipt())?,"receptors":receptors,"receptor_effects":effects,"normal_admission_and_canonical_preparation_exact":true,"captured_inputs_and_native_encoder_bit_exact":true,"initial_neural_and_effective_offsets_exact":true,"native_encoder_default_microsteps":native.microstep_count,"captured_activity_microsteps":row.replay_tick.microstep_count,"holding_raw":raw,"holding_toggled":1.-raw,"holding_assignments":assignments,"affected_encoder_targets":differences,"detached_holding_encoded_inputs":changed}),
    )
}

fn forward(
    actor: &CpuFoundationTrainer,
    q: &TrainingSequence,
    out: &Path,
    count: &mut usize,
    row: usize,
    condition: &str,
) -> Result<TrainingReplayEvaluation> {
    if *count >= 8 {
        return Err("refusing neural forward 9".into());
    }
    *count += 1;
    write_json(
        &out.join(format!("forward-attempt-{:02}.json", *count)),
        &json!({"reserved_forward":count,"maximum":8,"row":row,"condition":condition,"sequence_sha256":identity(q)?}),
    )?;
    let ev = actor.evaluate_replay(q)?;
    write_compressed(
        &out.join(format!("forward-{:02}.json.zst", *count)),
        &json!({"row":row,"condition":condition,"detached_replay_sequence":q,"candidate_logits":ev.candidate_logits,"final_activations":ev.final_activations,"final_activity_ema":ev.final_activity_ema,"final_metabolic_load":ev.final_metabolic_load}),
    )?;
    Ok(ev)
}
fn summarize(
    actor: &CpuFoundationTrainer,
    case: &SavedCase,
    ev: &TrainingReplayEvaluation,
    baseline: &TrainingReplayEvaluation,
    condition: &str,
) -> Result<Value> {
    let row = &case.full_native_row;
    let source = actor.phenotype().candidate_decoder().motor_start() as usize + 128;
    let features = &ev.final_activations[0][source..source + 32];
    let raw = speech(
        actor,
        &ev.final_activations[0],
        &row.replay_tick.effective_weight_offsets,
    );
    let base_raw = speech(
        actor,
        &baseline.final_activations[0],
        &row.replay_tick.effective_weight_offsets,
    );
    let payload = decode_cpu_speech(
        actor,
        &ev.final_activations[0],
        &row.replay_tick.effective_weight_offsets,
    )?;
    let gates=raw.iter().map(|r|json!({"presence_raw":r[16],"stop_raw":r[17],"presence_tanh":r[16].tanh(),"stop_tanh":r[17].tanh(),"presence_positive":r[16]>0.,"stop_positive":r[17]>0.})).collect::<Vec<_>>();
    let logits=row.patch.pre_action().perception().candidates().iter().map(|c|json!({"candidate_index":c.candidate_index,"action_id":c.action_id,"kind":c.kind,"target":c.target,"logit":ev.candidate_logits[0][c.candidate_index as usize]})).collect::<Vec<_>>();
    Ok(
        json!({"condition":condition,"speech_source_neurons":[source,source+32],"speech_source_features":features,"speech_source_delta":delta(features,&baseline.final_activations[0][source..source+32]),"raw_speech_outputs_six_steps":raw,"speech_raw_delta_per_step":raw.iter().zip(&base_raw).map(|(a,b)|delta(a,b)).collect::<Vec<_>>(),"gates":gates,"conditional_native_payload":payload,"action_logits":logits,"action_logit_delta":delta(&ev.candidate_logits[0],&baseline.candidate_logits[0]),"neural_activation_delta":delta(&ev.final_activations[0],&baseline.final_activations[0]),"not_a_new_world_outcome":true}),
    )
}

#[test]
#[ignore = "explicit detached V28 rows10/14; exactly eight forwards, zero world/gradient/Adam"]
fn isolate_holding_and_candidate_memory() -> Result<()> {
    let source = PathBuf::from(std::env::var("ALIFE_DETACHED_SOURCE")?);
    let v26 = PathBuf::from(std::env::var("ALIFE_DETACHED_V26")?);
    let rows = PathBuf::from(std::env::var("ALIFE_DETACHED_ROWS")?);
    let out = PathBuf::from(std::env::var("ALIFE_DETACHED_OUT")?);
    let prereg = PathBuf::from(std::env::var("ALIFE_DETACHED_PREREG")?);
    let protected = protect(&prereg)?;
    let (compiled, asset) = source_actor(&source, "MixedGrounded4")?;
    let original: FoundationTrainerCheckpoint =
        read_compressed(&v26.join("actor-Untouched.json.zst"))?;
    let actor = CpuFoundationTrainer::from_checkpoint(
        compiled.phenotype().clone(),
        asset,
        original.clone(),
    )?;
    assert_eq!(actor.optimizer_step(), 436);
    assert_eq!(actor.checkpoint(), original);
    let actor_hash = identity(&original)?;
    fs::create_dir(&out)?;
    let start = Instant::now();
    let mut cases = vec![];
    let mut proofs = vec![];
    for index in [10, 14] {
        let case: SavedCase =
            read_compressed(&rows.join(format!("native-decision-{index:02}.json.zst")))?;
        let proof = reconstruct(&actor, &case, index)?;
        write_json(&out.join(format!("reconstruction-{index}.json")), &proof)?;
        cases.push(case);
        proofs.push(proof);
    }
    write_json(
        &out.join("preflight-complete.json"),
        &json!({"both_canonical_frames_and_encoder_inputs_exact":true,"neural_forwards_so_far":0,"actor_sha256":actor_hash,"protected_files_exact":protected}),
    )?;
    let mut calls = 0;
    let mut baselines = vec![];
    let mut results = vec![];
    // Both baselines precede every intervention. A failure stops, retaining its
    // reserved call and outputs rather than silently spending another forward.
    for (i, case) in cases.iter().enumerate() {
        let row = &case.full_native_row;
        let ev = forward(
            &actor,
            &readout::single(&actor, row),
            &out,
            &mut calls,
            [10, 14][i],
            "baseline",
        )?;
        assert!(exact_floats(&ev.final_activations[0], &row.final_features));
        let after = &case
            .full_host_after
            .residents
            .iter()
            .find(|r| r.neural.organism_id == row.organism_id)
            .ok_or("no post resident")?
            .neural
            .neural;
        assert!(exact_floats(&ev.final_activity_ema[0], &after.activity_ema));
        assert!(exact_floats(
            &ev.final_metabolic_load[0],
            &after.metabolic_load
        ));
        let raw = speech(
            &actor,
            &ev.final_activations[0],
            &row.replay_tick.effective_weight_offsets,
        );
        let recorded: Vec<Vec<f32>> =
            serde_json::from_value(case.record["raw_speech_outputs_six_steps"].clone())?;
        assert!(
            raw.iter().zip(&recorded).all(|(a, b)| exact_floats(a, b))
                && raw.len() == recorded.len()
        );
        assert_eq!(
            serde_json::to_value(decode_cpu_speech(
                &actor,
                &ev.final_activations[0],
                &row.replay_tick.effective_weight_offsets
            )?)?,
            case.record["conditional_native_payload"]
        );
        assert_eq!(
            sample_cpu_joint_action(
                actor.phenotype(),
                row.patch.pre_action().perception(),
                &ev.candidate_logits[0],
                case.full_host_before.sampling_seed,
                row.sampling_counter.ok_or("no sampler")?,
                row.action.temperature
            )?,
            row.action
        );
        results.push(json!({"row":([10,14][i]),"recorded_outcome_unchanged":case.record,"baseline_exact":true,"conditions":[summarize(&actor,case,&ev,&ev,"baseline")?]}));
        baselines.push(ev);
    }
    write_json(
        &out.join("both-baselines-exact.json"),
        &json!({"forwards":calls,"activations_ema_metabolic_raw_speech_payload_and_sampled_joint_action_exact":true,"action_logit_arrays":"reconstructed; original raw arrays were not captured"}),
    )?;
    for (i, case) in cases.iter().enumerate() {
        for (condition, holding, memory) in [
            ("holding_toggle", true, false),
            ("candidate_memory_zero", false, true),
            ("both", true, true),
        ] {
            let original_tick = &case.full_native_row.replay_tick;
            let mut q = readout::single(&actor, &case.full_native_row);
            if holding {
                q.ticks[0].encoded_inputs =
                    serde_json::from_value(proofs[i]["detached_holding_encoded_inputs"].clone())?;
            }
            let mut memory_changes = vec![];
            if memory {
                for (candidate, c) in q.ticks[0].candidates.iter_mut().enumerate() {
                    for (slot, v) in c.decoder_inputs.iter_mut().enumerate().take(36).skip(24) {
                        if *v != 0. {
                            memory_changes.push(
                                json!({"candidate":candidate,"slot":slot,"before":v,"after":0.}),
                            );
                        }
                        *v = 0.;
                    }
                }
            }
            // Whitelist the detached differences; all neural initial/private,
            // route, receptor and effective-weight inputs remain identical.
            let mut normalized: TrainingReplayTick = q.ticks[0].clone();
            normalized.encoded_inputs = original_tick.encoded_inputs.clone();
            for (c, o) in normalized
                .candidates
                .iter_mut()
                .zip(&original_tick.candidates)
            {
                c.decoder_inputs[24..36].copy_from_slice(&o.decoder_inputs[24..36]);
            }
            assert_eq!(&normalized, original_tick);
            assert_eq!(q.initial, case.full_native_row.initial);
            let ev = forward(&actor, &q, &out, &mut calls, [10, 14][i], condition)?;
            let mut receipt = summarize(&actor, case, &ev, &baselines[i], condition)?;
            receipt["candidate_memory_changed_lanes"] = json!(memory_changes);
            results[i]["conditions"]
                .as_array_mut()
                .ok_or("no conditions")?
                .push(receipt);
            assert_eq!(actor.checkpoint(), original);
        }
    }
    assert_eq!(calls, 8);
    assert_eq!(actor.checkpoint(), original);
    assert_eq!(protect(&prereg)?, protected);
    // Reloaded hosts/checkpoints, frames and sealed patches have only been read.
    for case in &cases {
        let host = CpuCanonicalHost::from_checkpoint(case.full_host_before.clone(), &actor)?;
        assert_eq!(
            identity(&host.checkpoint(&actor)?)?,
            identity(&case.full_host_before)?
        );
    }
    write_json(
        &out.join("results.json"),
        &json!({"kind":"experimental-detached-native-speech-inputs-v30","elapsed_seconds":start.elapsed().as_secs_f64(),"neural_forwards":calls,"world_decisions":0,"gradient_probes":0,"native_or_shadow_optimizer_updates":0,"provider_or_jules_calls":0,"actor_optimizer_and_private_state_exact":true,"actor_sha256":actor_hash,"protected_prior_files_exact":protected,"reconstruction":proofs,"rows":results,"limitations":["two recorded states from one seed; fixed cue/history","candidate-memory zero only removes direct current action-decoder access, not historical neural effects","conditional payloads are not new utterances; no counterfactual world action/outcome was executed"]}),
    )
}
