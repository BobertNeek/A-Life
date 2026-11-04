//! Read-only real failure-row intervention; no world action or retained update.
use super::*;
use std::io::{BufRead, BufReader};
#[test]
#[ignore = "explicit disposable native priority diagnostic; no provider request"]
fn diagnose_physical_priority_on_captured_premature_row() {
    let source = std::env::var("ALIFE_PHYSICAL_PRIORITY_SOURCE").unwrap();
    let output = std::env::var("ALIFE_PHYSICAL_PRIORITY_OUT").unwrap();
    let source = Path::new(&source);
    let saved: Value =
        serde_json::from_slice(&fs::read(source.join("results.json")).unwrap()).unwrap();
    assert_eq!(
        saved["kind"],
        "experimental-unseen-frozen-grounding-transfer-v19"
    );
    let recovery = read_recovery(&source.join("recovery-Grounded")).unwrap();
    let p = PhenotypeCompiler::compile_validated(&recovery.inputs, &BrainCapacityClass::n2048())
        .unwrap();
    let asset = FoundationWeightAsset::decode_canonical(&recovery.foundation).unwrap();
    let actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), recovery.actor).unwrap();
    let original = identity(&actor.checkpoint()).unwrap();
    let decoded = zstd::stream::read::Decoder::new(
        fs::File::open(source.join("recovery-Grounded/reloaded-evaluation.jsonl.zst")).unwrap(),
    )
    .unwrap();
    let event = BufReader::new(decoded)
        .lines()
        .map(|l| serde_json::from_str::<Value>(&l.unwrap()).unwrap())
        .find(|e| {
            e["phase"] == "balanced_readout_evaluation"
                && e["world_seed"] == 4_507_581u64
                && e["post_sleep"] == false
                && e["requested"] == "GetFood"
                && e["row"].get("replay_tick").is_some()
        })
        .unwrap();
    let row: CpuCollectedDecision = serde_json::from_value(event["row"].clone()).unwrap();
    assert_eq!(identity(&row).unwrap(), event["native_full_row_sha256"]);
    let food = event["scene"]["food"].as_u64().unwrap();
    let frame = row.patch.pre_action().perception();
    let selected = &frame.candidates()[row.action.representative as usize];
    assert_eq!(selected.kind, ActionKind::Vocalize);
    assert_eq!(event["owned_before"][0], Value::Null);
    assert!(event["emitted"].as_array().unwrap().is_empty());
    let desired = frame
        .candidates()
        .iter()
        .find(|c| {
            c.action_id == HeadlessActionIds::GRAB
                && c.target.entity.is_some_and(|e| e.raw() == food)
        })
        .unwrap();
    let q = readout::single(&actor, &row);
    let ev = actor.evaluate_replay(&q).unwrap();
    let (before, target, rival) = physical_margin(
        &ev.candidate_logits[0],
        row.action.representative_mask,
        1 << desired.candidate_index,
    )
    .unwrap();
    let mut adj = vec![0.; ev.candidate_logits[0].len()];
    if before > 0. {
        adj[target] = -1.;
        adj[rival] = 1.;
    }
    let g = actor.probe_replay_gradients(&q, &[adj]).unwrap();
    let mut shadow =
        CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset, actor.checkpoint())
            .unwrap();
    let frozen = shadow.checkpoint();
    let norm = shadow.apply_gradients(&g.gradients).unwrap();
    preserve_frozen(&frozen, &shadow.checkpoint()).unwrap();
    let after_eval = shadow.evaluate_replay(&q).unwrap();
    let after = physical_margin(
        &after_eval.candidate_logits[0],
        row.action.representative_mask,
        1 << desired.candidate_index,
    )
    .unwrap()
    .0;
    assert_eq!(identity(&actor.checkpoint()).unwrap(), original);
    let report = json!({"kind":"native-physical-priority-real-failure-row-probe","source_actor_sha256":original,"native_row_sha256":event["native_full_row_sha256"],"world_seed":4_507_581u64,"requested":"GetFood","actual_selected_kind":selected.kind,"actual_utterances":event["emitted"],"requested_food_owned_before":false,"legal_requested_physical_candidate":desired.candidate_index,"strongest_native_competitor":frame.candidates()[rival].kind,"target_native_logit":ev.candidate_logits[0][target],"competitor_native_logit":ev.candidate_logits[0][rival],"priority_margin_before":before,"priority_margin_after_one_disposable_native_adam_update":after,"native_gradient_norm":norm,"source_actor_unmodified":true,"frozen_weights_moments_ages_exact":true,"source_optimizer_step":actor.optimizer_step(),"disposable_shadow_optimizer_step":shadow.optimizer_step(),"no_world_action_or_retained_learning_claim":true});
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    serde_json::to_writer_pretty(&mut out, &report).unwrap();
}
