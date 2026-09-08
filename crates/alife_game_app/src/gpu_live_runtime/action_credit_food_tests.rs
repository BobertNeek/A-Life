//! One bounded V2 choice probe using the unchanged V1 trained genetic weights.
use super::nociception_food_tests::{assert_choices, run_food_life};
use super::*;
use alife_core::{ActionCandidateCreditProfileV1, Nano512ActionCreditCandidateV2};

#[path = "readout_calibration_tests.rs"]
mod readout_calibration_tests;

thread_local! {
    static SELECTOR_CAPTURE_ROOT: std::cell::RefCell<Option<PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
    static CHOICE_TRIAL_SELECTOR_CAPTURE: std::cell::Cell<bool> = const {
        std::cell::Cell::new(false)
    };
}

pub(super) fn tick_with_selector_capture(
    backend: &mut GpuAuthoritativeSession,
    memory_batch: &GpuClosedLoopMemoryBatchInput<'_>,
    prepared: &[PreparedGpuBrainFrame],
) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
    let Some(root) = SELECTOR_CAPTURE_ROOT.with(|root| root.borrow().clone()) else {
        return backend.tick_memory_batch(memory_batch);
    };
    assert_eq!(prepared.len(), 1, "diagnostic has exactly one organism");
    let row = &prepared[0];
    let choice_trial = CHOICE_TRIAL_SELECTOR_CAPTURE.with(|capture| capture.get());
    let tick_limit = if choice_trial { 32 } else { 4 };
    assert!(
        row.frame.tick().raw() < tick_limit,
        "finite diagnostic tick bound"
    );
    if choice_trial && (4..16).contains(&row.frame.tick().raw()) {
        return backend.tick_memory_batch(memory_batch);
    }
    let requested = row
        .frame
        .candidates()
        .iter()
        .filter(|candidate| {
            candidate.family == alife_core::CandidateActionFamily::Ingest
                || (choice_trial
                    && matches!(
                        candidate.family,
                        alife_core::CandidateActionFamily::Approach
                            | alife_core::CandidateActionFamily::Avoid
                            | alife_core::CandidateActionFamily::Contact
                    ))
        })
        .map(|candidate| candidate.candidate_index)
        .collect::<Vec<_>>();
    // Detailed rows are bounded by the existing API. Every candidate still
    // has its final score in the receipt, including families not requested.
    assert!(
        requested.len() <= 8,
        "existing GPU diagnostic request bound"
    );
    if !choice_trial {
        assert_eq!(
            requested.len(),
            2,
            "both food candidates must remain available"
        );
    }
    let ticks = backend
        .tick_memory_batch_with_selector_diagnostics(memory_batch, &requested)
        .unwrap_or_else(|error| panic!("authoritative selector capture failed: {error:?}"));
    let tick = row.frame.tick().raw() + 1;
    let memory = row
        .memory_upload
        .records
        .iter()
        .map(|record| {
            serde_json::json!({
                "candidate_index": record.candidate_index,
                "target_latent": record.target_latent,
                "family_value": record.family_value,
                "target_confidence": record.target_confidence,
                "family_confidence": record.family_confidence,
                "source_counts_packed": record.source_counts_packed,
            })
        })
        .collect::<Vec<_>>();
    write_capture(
        &root.join(format!("tick-{tick:02}-selector.json")),
        &serde_json::json!({
            "selector": ticks[0].selector_diagnostic,
            "frame": row.frame,
            "memory_upload_inputs": memory,
            "memory_recall_receipt": row.memory_recall.receipt(),
            "neural_receptors": row.neural_receptors,
            "receptor_effects": row.receptor_effects,
            "memory_input_provenance": "host records bound to the exact GPU perception dispatch",
        }),
    );
    capture_gpu_banks(
        backend,
        row.handle,
        row.frame.tick(),
        &root.join(format!("tick-{tick:02}-pending-gpu.json")),
    );
    Ok(ticks)
}

fn write_capture(path: &Path, value: &serde_json::Value) {
    assert!(!path.exists(), "preserve existing diagnostic evidence");
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn capture_gpu_banks(
    backend: &mut GpuAuthoritativeSession,
    handle: GpuBrainHandle,
    tick: Tick,
    path: &Path,
) {
    let snapshot = backend.snapshot_brain(handle, tick).unwrap();
    let digest = snapshot.canonical_digest();
    let parts = snapshot.into_parts();
    write_capture(
        path,
        &serde_json::json!({
            "provenance": "exact GPU checkpoint readback; no neural recomputation",
            "canonical_digest": digest,
            "active_activation_side": parts.active_activation_side,
            "activation_a_bits": parts.activation_a_bits,
            "activation_b_bits": parts.activation_b_bits,
            "active_weight_bank": parts.active_weight_bank,
            "active_weight_generation": parts.active_weight_generation,
            "fast_bank_0_bits": parts.fast_bank_0_bits,
            "fast_bank_1_bits": parts.fast_bank_1_bits,
            "lifetime_bank_0_bits": parts.lifetime_bank_0_bits,
            "lifetime_bank_1_bits": parts.lifetime_bank_1_bits,
            "active_eligibility_bank": parts.active_eligibility_bank,
            "active_eligibility_generation": parts.active_eligibility_generation,
            "inactive_eligibility_generation": parts.inactive_eligibility_generation,
            "recurrent_eligibility_bank_0_bits": parts.recurrent_eligibility_bank_0_bits,
            "recurrent_eligibility_bank_1_bits": parts.recurrent_eligibility_bank_1_bits,
            "decoder_eligibility_bank_0_bits": parts.decoder_eligibility_bank_0_bits,
            "decoder_eligibility_bank_1_bits": parts.decoder_eligibility_bank_1_bits,
            "pending_present": parts.pending_eligibility.is_some(),
            "joint_selection": parts.pending_eligibility.and_then(|pending| pending.joint_selection()),
        }),
    );
}

#[test]
#[ignore = "one manual diagnostic, at most four ticks; requires retained ALIFE_FOUNDER_CANDIDATE"]
fn first_harmful_meal_selector_diagnostic() {
    let bytes = std::fs::read(std::env::var("ALIFE_FOUNDER_CANDIDATE").unwrap()).unwrap();
    let source = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    let configured = Nano512ActionCreditCandidateV2::new(
        &source,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    assert_eq!(
        configured.asset().unwrap().encode_canonical().unwrap(),
        bytes
    );
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    let evidence =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/founder-training-evidence");
    let prior = evidence.join("action-credit-food-12744");
    let saved: Nano512ActionCreditCandidateV2 =
        serde_json::from_slice(&std::fs::read(prior.join("configured-candidate.json")).unwrap())
            .unwrap();
    assert_eq!(
        configured, saved,
        "retain the exact candidate and V2 parameters"
    );
    let root = evidence.join(format!("first-meal-selector-{}", std::process::id()));
    assert!(!root.exists(), "preserve prior evidence");
    std::fs::create_dir_all(&root).unwrap();
    write_capture(
        &root.join("neural-phenotype.json"),
        &serde_json::to_value(&phenotype).unwrap(),
    );
    write_capture(
        &root.join("compiler-inputs.json"),
        &serde_json::to_value(&inputs).unwrap(),
    );
    let references = (1..=4)
        .map(|tick| {
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(prior.join(format!("cyan-nutritious/tick-{tick:02}.json"))).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let (mut runtime, _) = super::founder_consequence_tests::paired_food_runtime_with_action_credit(
        &source,
        true,
        &root.join("world"),
        true,
        Some(&configured),
    );
    let saved_genome: serde_json::Value =
        serde_json::from_slice(&std::fs::read(prior.join("cyan-nutritious/genome.json")).unwrap())
            .unwrap();
    let initial_genome: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("world/genome.json")).unwrap()).unwrap();
    assert_eq!(
        initial_genome, saved_genome,
        "identical organism starting genome"
    );
    let saved_world: serde_json::Value =
        serde_json::from_slice(&std::fs::read(prior.join("cyan-nutritious/world.json")).unwrap())
            .unwrap();
    let initial_world: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("world/world.json")).unwrap()).unwrap();
    assert_eq!(
        initial_world["world"], saved_world["world"],
        "identical initial world"
    );
    let organism = OrganismId(1);
    let initial_bits = runtime
        .active_fast_weights_for_test(organism)
        .unwrap()
        .into_iter()
        .map(f32::to_bits)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(initial_bits).unwrap(),
        references[0]["fast_before"]
    );
    SELECTOR_CAPTURE_ROOT.with(|capture| *capture.borrow_mut() = Some(root.clone()));
    let started = Instant::now();
    let mut first_meal = None;
    let mut meal_memory_generation = None;
    let mut next_decision_memory_delta = None;
    let mut completed = 0;
    let mut polls = 0;
    while completed < 4 && started.elapsed().as_secs() < 75 {
        match runtime.tick_outcome().unwrap() {
            GpuLiveTickOutcome::NoProgress(_) => {
                polls += 1;
                assert!(polls <= 400, "bounded no-progress polls");
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            GpuLiveTickOutcome::Progressed(summaries) => assert!(summaries[0].patch_sealed),
        }
        completed += 1;
        let patch = runtime.sealed_patches().last().unwrap().clone();
        let selector: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("tick-{completed:02}-selector.json"))).unwrap(),
        )
        .unwrap();
        let receptors: alife_core::NeuralReceptorFrame =
            serde_json::from_value(selector["neural_receptors"].clone()).unwrap();
        let credit = alife_core::OutcomeCreditPacket::from_sealed_patch(&patch)
            .unwrap()
            .with_biochemical_receptors(&receptors)
            .unwrap();
        let learning = *runtime.last_learning_receipts().first().unwrap();
        let activity = runtime.evidence_activity_snapshot(organism).unwrap();
        write_capture(
            &root.join(format!("tick-{completed:02}-outcome.json")),
            &serde_json::json!({
                "patch": patch,
                "bound_credit_lanes": credit.modulator().frame().lanes(),
                "credit_provenance": "host projection from actual sealed outcome and dispatch-bound receptor frame",
                "activity": {"pressure": activity.pressure, "throttle": activity.throttle,
                    "work": activity.work, "brain_atp_q16": activity.brain_atp_q16},
                "memory_updates": runtime.last_memory_update_receipts(),
                "memory_observation_errors": format!("{:?}", runtime.last_memory_observation_errors()),
                "learning": {"changed": learning.fast_weights_changed, "max_abs_delta": learning.max_abs_delta,
                    "input_fast": learning.input_fast_generation, "output_fast": learning.output_fast_generation},
            }),
        );
        let handle = *runtime.handles.get(&organism.raw()).unwrap();
        let tick = runtime.world_snapshot().tick();
        capture_gpu_banks(
            &mut runtime.backend,
            handle,
            tick,
            &root.join(format!("tick-{completed:02}-after-gpu.json")),
        );
        let actual_patch = serde_json::to_value(&patch).unwrap();
        if completed == 1 {
            assert_eq!(
                actual_patch["pre_action"]["perception"],
                references[0]["patch"]["pre_action"]["perception"],
                "preserve initial empty-memory perception; later recall may change attention and inputs"
            );
        }
        assert!(runtime.last_memory_observation_errors().is_empty());
        println!(
            "selector_diagnostic_tick={completed}; contact={:?}; evidence={}",
            patch.outcome().physical.contact,
            root.display()
        );
        if patch.outcome().physical.contact == PhysicalContactKind::Consumed && first_meal.is_none()
        {
            assert_eq!(
                patch.outcome().physical.target_entity,
                Some(WorldEntityId(3))
            );
            assert!(patch.outcome().pain_delta.raw() > 0.0);
            let update = runtime
                .last_memory_update_receipts()
                .iter()
                .find(|receipt| receipt.sealed_sequence_id == patch.header().sequence_id)
                .unwrap();
            assert!(update.output_generation > update.input_generation);
            meal_memory_generation = Some(update.output_generation);
            first_meal = Some(completed);
        }
        if first_meal.is_some_and(|tick| completed > tick) {
            assert_eq!(
                selector["memory_recall_receipt"]["input_generation"].as_u64(),
                meal_memory_generation
            );
            let food = selector["selector"]["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .find(|candidate| {
                    candidate["family"] == "Ingest" && candidate["target"]["entity"] == 3
                })
                .unwrap();
            let memory = selector["memory_upload_inputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|memory| memory["candidate_index"] == food["candidate_index"])
                .unwrap();
            let sources = memory["source_counts_packed"].as_u64().unwrap();
            assert!(sources & 0xffff > 0 && sources >> 16 > 0);
            assert!(memory["target_confidence"].as_f64().unwrap() > 0.0);
            assert!(memory["family_confidence"].as_f64().unwrap() > 0.0);
            assert!(memory["target_latent"][2].as_f64().unwrap() > 0.0);
            assert!(memory["family_value"][2].as_f64().unwrap() > 0.0);
            next_decision_memory_delta = food["memory_context_delta"].as_f64();
            assert!(next_decision_memory_delta.is_some());
            break;
        }
    }
    SELECTOR_CAPTURE_ROOT.with(|capture| *capture.borrow_mut() = None);
    assert!(
        first_meal.is_some_and(|tick| completed > tick),
        "first harmful meal and next decision must fit the bound"
    );
    write_capture(
        &root.join("receipt.json"),
        &serde_json::json!({
            "max_world_ticks": 4, "completed_world_ticks": completed, "first_harmful_meal_tick": first_meal,
            "pressure_source": "actual production dispatch pressure; no replay", "no_progress_polls": polls,
            "post_initial_perception_divergence": "expected: repaired recall may change neural input and attention",
            "meal_memory_generation": meal_memory_generation,
            "next_decision_memory_delta": next_decision_memory_delta,
            "recall_repaired": true,
            "gpu_memory_influence_nonzero": next_decision_memory_delta.is_some_and(|delta| delta != 0.0),
            "separate_hebbian_oja_terms_measured": false,
            "term_limit": "GPU eligibility, activations, weights, logits are measured; separate update terms require labeled reconstruction",
        }),
    );
}

#[test]
#[ignore = "bounded manual V2 probe; requires retained ALIFE_FOUNDER_CANDIDATE"]
fn inherited_action_credit_food_choice_probe() {
    let bytes = std::fs::read(std::env::var("ALIFE_FOUNDER_CANDIDATE").unwrap()).unwrap();
    let source = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        source.digest().bytes(),
        &[
            14, 148, 152, 50, 97, 32, 184, 207, 71, 123, 153, 74, 88, 1, 122, 247, 18, 136, 157,
            226, 80, 112, 127, 203, 223, 222, 92, 5, 5, 204, 232, 246
        ]
    );
    let configured = Nano512ActionCreditCandidateV2::new(
        &source,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    assert_eq!(
        configured.asset().unwrap().encode_canonical().unwrap(),
        bytes
    );
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    for (synapse, weight) in phenotype.synapses().iter().zip(source.weights()) {
        assert_eq!(synapse.genetic_weight().to_bits(), weight.to_bits());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/action-credit-food-{}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve prior evidence");
    std::fs::create_dir_all(&root).unwrap();
    for (name, value) in [
        (
            "configured-candidate.json",
            serde_json::to_value(&configured).unwrap(),
        ),
        (
            "neural-phenotype.json",
            serde_json::to_value(&phenotype).unwrap(),
        ),
        (
            "compiler-inputs.json",
            serde_json::to_value(&inputs).unwrap(),
        ),
    ] {
        std::fs::write(root.join(name), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    let harmful = run_food_life(
        &source,
        &phenotype,
        true,
        &root.join("cyan-nutritious"),
        Some(&configured),
    );
    let nutritious = run_food_life(
        &source,
        &phenotype,
        false,
        &root.join("amber-nutritious"),
        Some(&configured),
    );
    std::fs::write(
        root.join("outcomes.json"),
        serde_json::to_vec_pretty(&[&harmful, &nutritious]).unwrap(),
    )
    .unwrap();
    println!(
        "action_credit_choices: {harmful:?}; {nutritious:?}; evidence={}",
        root.display()
    );
    assert_choices(&harmful, &nutritious);
}
