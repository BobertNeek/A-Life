#![cfg(feature = "cpu-backend")]
//! V4 detached qualification. Closed V2/V3 lives remain in their own checkouts.
//! Ordinary tests perform no neural decisions. Fixed preparation and discarded
//! physical-only clones establish mechanism. The separately ignored, explicitly
//! frozen entrypoint runs only the bounded matched protocol described below.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const CONSTRUCTION_SEED: u64 = 2026111301;
const SAMPLING_SEED: u64 = 1; // No native decisions are executed by these tests.
const STUDY: &str = "need-association-v4-detached-feasibility";
const FROZEN_ASSET: &str = "1d2978cbd857a7ca99dc1b2cf65be9f6ff417a6f78199282e0431b94332c9377";
fn fixture() -> (CpuCanonicalHost, CpuFoundationTrainer, String) {
    fixture_with_preparation(0)
}
fn fixture_with_preparation(ticks: u64) -> (CpuCanonicalHost, CpuFoundationTrainer, String) {
    fixture_with_sampling(ticks, SAMPLING_SEED as u32)
}
fn fixture_with_sampling(
    ticks: u64,
    sampling_seed: u32,
) -> (CpuCanonicalHost, CpuFoundationTrainer, String) {
    let mut config = CanonicalNewGameConfig::phase3(CONSTRUCTION_SEED, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile).unwrap();
    let initial = create_canonical_new_game_with_n2048_candidate(&config, &builtin).unwrap();
    let record = initial.world.organism_registry().iter().next().unwrap();
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let base = PhenotypeCompiler::compile(&genome, &capacity, &development, config.sensor_profile)
        .unwrap();
    let mut counts = BTreeMap::new();
    for synapse in base.synapses() {
        if let CompiledSynapseKind::Decoder(c) = synapse.kind() {
            if c.head() == DecoderHeadKind::ActionCandidate {
                *counts
                    .entry((c.family().raw(), c.input_lane()))
                    .or_insert(0usize) += 1;
            }
        }
    }
    let weights = base
        .synapses()
        .iter()
        .map(|s| {
            let CompiledSynapseKind::Decoder(c) = s.kind() else {
                return s.genetic_weight();
            };
            if c.head() != DecoderHeadKind::ActionCandidate {
                return s.genetic_weight();
            }
            let coefficient = match (c.family(), c.input_lane()) {
                (CandidateActionFamily::Rest, 23) | (CandidateActionFamily::Inspect, 18) => 8.,
                (CandidateActionFamily::Approach, 23) | (CandidateActionFamily::Contact, 23) => 6.,
                _ => 0.,
            };
            coefficient / counts[&(c.family().raw(), c.input_lane())] as f32
        })
        .collect();
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        weights,
        TrainingStageManifest::bootstrap(),
    )
    .unwrap();
    let hash = format!("{:x}", Sha256::digest(asset.encode_canonical().unwrap()));
    assert_eq!(
        hash, FROZEN_ASSET,
        "frozen inherited asset bytes changed before exposure"
    );
    let (p, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    // Admission requires a nonempty optimizer mask. No optimizer update occurs.
    let mask = StageTrainableMask::from_synapse_indices(&p, &[0]).unwrap();
    let trainer =
        CpuFoundationTrainer::new(p, asset.clone(), mask, AdamWConfig::default()).unwrap();
    let mut game = create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
    // Construct the complete fresh scene before roster admission.
    let world = &mut game.world;
    let (_, body) = world.organism_entity_ids()[0];
    world.configure_ecology(EcologyState::default()).unwrap();
    for object in world.object_snapshots() {
        if object.id != body {
            world.editor_remove_object(object.id).unwrap();
        }
    }
    let position = world.entity(body).unwrap().position;
    world
        .spawn_social_agent(
            "need-association-static-peer",
            OrganismId(9000001),
            Vec3f::new(position.x + 0.2, position.y, position.z),
            0.,
        )
        .unwrap();
    let (host, ()) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, sampling_seed, |world| {
            for _ in 0..ticks {
                world.try_advance_tick()?;
                let record = world.organism_registry().iter().next().unwrap();
                if !record.lifecycle().is_alive()
                    || record.biochemistry().body.health < 0.95
                    || record.biochemistry().body.energy < 0.35
                    || record.biochemistry().body.sleeping
                {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
            Ok::<_, ScaffoldContractError>(())
        })
        .unwrap();
    (host, trainer, hash)
}
fn plan() -> CpuNeedAssociationPlanV4 {
    let mut plan = CpuNeedAssociationPlanV4::identity_control(SelfActionV1::Noop);
    plan.execution = NativeNeedExecutionV1::ActiveCoordinates;
    plan.recipe.normalization = NativeNeedNormalizationV1::PressureQuantumTanh { quantum: 0.001 };
    plan
}
#[test]
fn native_need_port_identity_and_legacy_exports_fail_closed_without_steps() {
    let (mut host, trainer, first_hash) = fixture();
    let original = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let (_, _, second_hash) = fixture();
    assert_eq!(first_hash, second_hash);
    println!("stationary_fixture_identity study={} brain=N2048 seed={} asset_sha256={} phenotype={:?} coefficients=Rest:23:8,Inspect:18:8,Approach:23:6,Contact:23:6;other_action_coordinates:0;other_genes:unchanged",
        STUDY,SAMPLING_SEED,first_hash,trainer.phenotype().phenotype_hash());
    let mut world = host.world().clone();
    let record = world.organism_registry().iter().next().unwrap().clone();
    let frame = world
        .perception_frame(
            record.organism_id(),
            world.tick(),
            SensorProfile::GroundedObjectSlotsV1,
            record.biochemistry().homeostasis,
        )
        .unwrap();
    let reader_rows = frame
        .candidates()
        .iter()
        .filter(|c| {
            matches!(c.kind, ActionKind::Rest | ActionKind::Inspect)
                || c.action_id == HeadlessActionIds::NO_POSTURE
                || c.action_id == HeadlessActionIds::NO_LOCOMOTION
        })
        .count();
    let circuit = NativeNeedAssociationCircuitV2::compile_bound(
        record.phenotype(),
        plan().recipe,
        plan().execution,
    )
    .unwrap();
    let measured = circuit
        .run(
            &circuit
                .bind_current(record.phenotype(), record.biochemistry())
                .unwrap(),
            NativeNeedTransitionV1::Unknown,
            NativeNeedReferenceV1 {
                kind: NativeNeedReferenceKindV1::Rest,
                transition: NativeNeedTransitionV1::Unknown,
            },
        )
        .unwrap();
    let policy = BrainActivityPolicyV1::production_v1();
    let mut extra = measured.work.brain_work_counters(0);
    extra.neuron_updates *= reader_rows as u64;
    extra.synapse_ops *= reader_rows as u64;
    extra.tile_visits *= reader_rows as u64;
    extra.decoder_candidate_ops = reader_rows as u64;
    let extra_debit = policy
        .cost
        .q24_to_atp_q16_round_half_up(policy.cost.neural_cost_q24(&extra).unwrap())
        .unwrap();
    let ordinary = cpu_activity_budget(
        trainer.phenotype(),
        CpuActivityAnchor {
            organism_id: record.organism_id(),
            tick: Tick::ZERO,
            sequence_cursor: 1,
            previous_dispatch_generation: 0,
            source_frame_digest: [0; 4],
            frame_digest: frame.frame_digest().0,
        },
        frame.homeostasis().drives.brain_atp,
        frame.candidates().len() as u32,
        0,
    )
    .unwrap();
    println!("stationary_fixture_neural_rows_only_excludes_binding_cost candidates={} reader_rows={} extra_atp_q16={} ordinary_atp_q16={} available_atp_q16={}",
        frame.candidates().len(),reader_rows,extra_debit,ordinary.debit_atp_q16,ordinary.available_atp_q16);
    assert!(ordinary.available_atp_q16 > ordinary.debit_atp_q16 + extra_debit);
    host.enable_need_association_port_v4(&trainer, plan())
        .unwrap();
    assert!(host.checkpoint(&trainer).is_err());
    assert!(host.step(&trainer, 1.).is_err());
    assert!(host
        .enable_need_association_port_v4(&trainer, plan())
        .is_err());
    let restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&original).unwrap(), &trainer)
            .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
        original
    );
}

#[test]
fn native_need_v4_detached_candidate_and_motor_probabilities_preserve_host() {
    let (host, trainer, asset_hash) = fixture();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let preview = host.preview_need_feasibility(&trainer, 1.).unwrap();
    assert_eq!(
        serde_json::to_vec(&preview).unwrap(),
        serde_json::to_vec(&host.preview_need_feasibility(&trainer, 1.).unwrap()).unwrap()
    );
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
    for p in &preview {
        // Preserve the already-measured unchanged-asset operating point while
        // the new source/port is qualified. This is a same-platform diagnostic,
        // not a GPU or cross-platform parity claim.
        for (id, expected) in [
            (ActionKind::Rest.canonical_id(), 4.5360937f32),
            (HeadlessActionIds::NO_LOCOMOTION, 3.390938),
            (HeadlessActionIds::APPROACH, 2.4467943),
        ] {
            let row = p.candidates.iter().find(|c| c.action_id == id).unwrap();
            assert!((row.logit - expected).abs() < 1.0e-6);
        }
        assert_eq!(p.world_steps, 0);
        assert_eq!(p.live_dispatches, 0);
        assert!(p.counterfactual_only);
        assert!(
            (p.candidates
                .iter()
                .map(|c| c.representative_probability)
                .sum::<f64>()
                - 1.)
                .abs()
                < 1.0e-12
        );
        for c in &p.candidates {
            assert!(
                (c.logit
                    - (c.inherited_bias
                        + c.action_readout
                        + c.memory_readout
                        + c.cognitive_readout))
                    .abs()
                    < 1.0e-4
            );
        }
    }
    println!(
        "{}",
        serde_json::json!({"study":STUDY,"asset_sha256":asset_hash,"host_exact":true,"founder_exact":true,"world_exposures":0,"preview":preview})
    );
}

fn detached_physical_mechanism(host: &CpuCanonicalHost) -> serde_json::Value {
    // These physical branches are explicitly chosen mechanism diagnostics,
    // not network decisions, acquired records, or alternative policy lives.
    let mut perception_world = host.world().clone();
    let record = perception_world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let frame = perception_world
        .perception_frame(
            record.organism_id(),
            perception_world.tick(),
            SensorProfile::GroundedObjectSlotsV1,
            record.biochemistry().homeostasis,
        )
        .unwrap();
    let command = |id, channel| {
        let candidate = frame
            .candidates()
            .iter()
            .find(|c| c.action_id == id)
            .unwrap();
        channel_command_for_action(
            channel,
            &candidate
                .to_command(record.organism_id(), Confidence::new(1.).unwrap())
                .unwrap(),
        )
        .unwrap()
    };
    let no_locomotion = command(HeadlessActionIds::NO_LOCOMOTION, MotorChannel::Locomotion);
    let mut rows = Vec::new();
    let mut states = Vec::new();
    for (name, posture) in [
        ("rest", Some(ActionKind::Rest.canonical_id())),
        ("explicit_no_posture", Some(HeadlessActionIds::NO_POSTURE)),
        ("no_locomotion_only", None),
    ] {
        let mut world = host.world().clone();
        let mut commands = vec![no_locomotion.clone()];
        if let Some(posture) = posture {
            commands.push(command(posture, MotorChannel::Posture));
        }
        let bundle = MotorCommandBundle::new(
            record.organism_id(),
            ExperienceSequenceId::new(1).unwrap(),
            world.tick(),
            commands,
        )
        .unwrap();
        let receipt = world
            .apply_registered_motor_bundle(&bundle, record.world_entity_id())
            .unwrap();
        assert_eq!(world.tick(), host.world().tick()); // The API advances biology, not the world clock.
        assert_eq!(receipt.outcome_tick.raw(), host.world().tick().raw() + 1);
        assert_eq!(receipt.biology_before, *record.biochemistry());
        assert_eq!(
            receipt
                .biology_after
                .biochemical_work()
                .neural_emitter_evaluations,
            0
        );
        let before = receipt.biology_before.homeostasis.drives.to_array();
        let after = receipt.biology_after.homeostasis.drives.to_array();
        let delta: [f32; 11] = std::array::from_fn(|i| after[i] - before[i]);
        rows.push(serde_json::json!({"branch":name,"body_event":receipt.body_event,
            "requested":bundle.channels,"executed":receipt.channel_receipts.iter().map(|r|serde_json::json!({"command":r.command,"phase":format!("{:?}",r.phase),"physical":r.physical,"observation":r.observation})).collect::<Vec<_>>(),
            "before":CpuNeedBiologyCostV4::from_biology(&receipt.biology_before),
            "world_after":CpuNeedBiologyCostV4::from_biology(&receipt.biology_after),
            "raw_before":before,"raw_after":after,"raw_delta":delta,
            "biochemical_work":receipt.biology_after.biochemical_work(),"host_cognitive_charge_applied":false}));
        states.push(receipt.biology_after);
    }
    assert_eq!(
        states[1], states[2],
        "adding an exact no-posture must not change biology"
    );
    serde_json::json!({"kind":"detached physical mechanism, not selected learning evidence","physical_intervals":3,"native_decisions":0,"source_writes":0,"rows":rows,
        "rest_minus_noop_fatigue":states[0].homeostasis.drives.fatigue-states[1].homeostasis.drives.fatigue,
        "rest_has_differential_fatigue":states[0].homeostasis.drives.fatigue < states[1].homeostasis.drives.fatigue})
}

#[test]
fn native_need_v4_prepared_600_detached_comparator_and_actual_binding_cost() {
    // Fixed, prospective juvenile preparation. Ordinary body ticks, no selected
    // command, no drive overwrite, no synthetic source, and no native decision.
    let (mut host, trainer, asset_hash) = fixture_with_preparation(600);
    let ordinary = host.preview_need_feasibility(&trainer, 1.).unwrap();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let mechanism = detached_physical_mechanism(&host);
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    host.enable_need_association_port_v4(&trainer, plan())
        .unwrap();
    let bound = host.preview_need_feasibility(&trainer, 1.).unwrap();
    assert_eq!(host.world().tick(), Tick::new(600));
    assert_eq!(ordinary[0].candidates.len(), bound[0].candidates.len());
    for (a, b) in ordinary[0].candidates.iter().zip(&bound[0].candidates) {
        assert_eq!(
            a.logit, b.logit,
            "unknown history must have zero policy effect"
        );
    }
    let record = host.world().organism_registry().iter().next().unwrap();
    println!(
        "{}",
        serde_json::json!({
            "study":"need-association-v4-prepared-detached-feasibility", "asset_sha256":asset_hash,
            "ordinary_preparation_ticks":600,"native_decisions":0,"acquired_sources":0,
            "maturation":record.phenotype().development_state_at(Tick::new(600)).unwrap().maturation,
            "biology":CpuNeedBiologyCostV4::from_biology(record.biochemistry()),
            "mechanism":mechanism,
            "ordinary":ordinary,"bound_reader":bound,
            "pre_port_checkpoint_sha256":format!("{:x}",Sha256::digest(before))
        })
    );
    assert!(
        mechanism["rest_has_differential_fatigue"]
            .as_bool()
            .unwrap(),
        "physical Rest had no measured differential fatigue"
    );
    assert!(bound[0].planned_atp_q16 >= ordinary[0].planned_atp_q16);
    assert!(bound[0].available_atp_q16 >= bound[0].planned_atp_q16);
    assert!(
        ordinary[0].input_drives[1] > 0.,
        "fixed preparation did not establish fatigue"
    );
    assert!(bound[0].pure_rest_selection_probability > 0.);
    assert!(bound[0].pure_supported_noop_selection_probability > 0.);
}

const MATCHED_PROTOCOL: &str =
    include_str!("../../../docs/experiments/native-need-association-v4/PROTOCOL.json");

#[test]
fn native_need_v4_protocol_pins_the_predeclared_bounded_configuration() {
    let p: serde_json::Value = serde_json::from_str(MATCHED_PROTOCOL).unwrap();
    assert_eq!(p["construction_seed"], CONSTRUCTION_SEED);
    assert_eq!(p["sampling_seed"], 2026111303u64);
    assert_eq!(p["max_host_intervals_per_arm"], 12);
    assert_eq!(p["gains"], serde_json::json!([0., 0.35]));
    assert_eq!(p["asset_sha256"], FROZEN_ASSET);
}

fn write_json_new(path: &std::path::Path, value: &serde_json::Value) {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    serde_json::to_writer_pretty(&mut f, value).unwrap();
    f.write_all(b"\n").unwrap();
    f.sync_all().unwrap();
}

/// Explicit opt-in only. A consumed marker prevents silent reruns, including
/// after an infrastructure failure. Ordinary cargo test never launches lives.
#[test]
#[ignore = "frozen disposable matched study; requires verified freeze manifest and no-retry output directory"]
fn native_need_v4_frozen_matched_association_protocol() {
    use std::io::Write;
    let dir = std::path::PathBuf::from(
        std::env::var("ALIFE_NEED_V4_RUN_DIR").expect("explicit frozen output directory required"),
    );
    let freeze: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("freeze.json")).unwrap()).unwrap();
    let executable = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let executable_hash = format!("{:x}", Sha256::digest(executable));
    let protocol_hash = format!("{:x}", Sha256::digest(MATCHED_PROTOCOL.as_bytes()));
    assert_eq!(freeze["executable_sha256"], executable_hash);
    assert_eq!(freeze["protocol_sha256"], protocol_hash);
    assert_eq!(freeze["asset_sha256"], FROZEN_ASSET);
    assert_eq!(freeze["sampling_seed"], 2026111303u64);
    write_json_new(
        &dir.join("NO_RETRY.started.json"),
        &serde_json::json!({"study":"native-need-association-v4-matched-own-history","protocol_sha256":protocol_hash,"executable_sha256":executable_hash,"sampling_seed":2026111303u64,"maximum_lives":2,"maximum_intervals_per_life":12}),
    );
    let mut summaries = Vec::new();
    for (name, gain) in [("gain-zero", 0.), ("gain-035", 0.35)] {
        write_json_new(
            &dir.join(format!("{name}.started.json")),
            &serde_json::json!({"arm":name,"gain":gain,"sampling_seed":2026111303u64,"status":"started, outcome not yet known"}),
        );
        let (mut host, trainer, asset) = fixture_with_sampling(600, 2026111303);
        assert_eq!(asset, FROZEN_ASSET);
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        let mut recipe = plan();
        recipe.recipe.gain = gain;
        host.enable_need_association_port_v4(&trainer, recipe)
            .unwrap();
        let mut journal = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(format!("{name}.fresh-local-diagnostics.jsonl")))
            .unwrap();
        let mut intervals = 0u64;
        let mut decisions = 0u64;
        let mut stored_rest = 0u64;
        let mut stored_noop = 0u64;
        let mut stored_observed = 0u64;
        let mut compatible_rest = 0u64;
        let mut nonzero_rest = 0u64;
        let mut max_abs_rest = 0f32;
        let mut ablation_changed = 0u64;
        let mut stop = "budget_exhausted";
        let mut failures = BTreeMap::<String, u64>::new();
        let start = CpuNeedBiologyCostV4::from_biology(
            host.world()
                .organism_registry()
                .iter()
                .next()
                .unwrap()
                .biochemistry(),
        );
        for counter in 0..12u32 {
            let before = host.world().organism_registry().iter().next().unwrap();
            if !before.lifecycle().is_alive() {
                stop = "death";
                break;
            }
            if before.biochemistry().body.health < 0.95
                || before.biochemistry().body.injury > 0.
                || before.biochemistry().body.energy < 0.35
            {
                stop = "biology_safeguard";
                break;
            }
            let step = match host.step_need_association_port_v4(&trainer, 1.) {
                Ok(s) => s,
                Err(e) => {
                    stop = "host_error";
                    writeln!(journal,"{}",serde_json::json!({"kind":"host_error","counter":counter,"error":e.to_string()})).unwrap();
                    journal.sync_all().unwrap();
                    break;
                }
            };
            intervals += 1;
            assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
            for decision in step.decisions {
                decisions += 1;
                let frame = decision.patch.pre_action().perception();
                let selected = &frame.candidates()[decision.action.representative as usize];
                match decision.port.source_observation {
                    CpuNeedObservationStatusV4::Stored => {
                        if selected.kind == ActionKind::Rest {
                            stored_rest += 1;
                        } else if matches!(
                            selected.action_id,
                            HeadlessActionIds::NO_LOCOMOTION | HeadlessActionIds::NO_POSTURE
                        ) {
                            stored_noop += 1;
                        } else {
                            stored_observed += 1;
                        }
                    }
                    CpuNeedObservationStatusV4::Unsupported(reason) => {
                        *failures.entry(reason.to_owned()).or_default() += 1
                    }
                }
                for row in &decision.port.candidates {
                    if row.family == CandidateActionFamily::Rest
                        && row.support.action_sequence.is_some()
                        && row.support.reference_sequence.is_some()
                    {
                        compatible_rest += 1;
                        let value = row.native.confidence_product_gated_readout;
                        if value != 0. {
                            nonzero_rest += 1;
                        }
                        max_abs_rest = max_abs_rest.max(value.abs());
                    }
                }
                let audit = decision.port.sampling_audit.as_ref().unwrap();
                let exact = sample_cpu_joint_action(
                    trainer.phenotype(),
                    frame,
                    &audit.candidate_logits,
                    2026111303,
                    counter,
                    1.,
                )
                .unwrap();
                assert_eq!(
                    exact, decision.action,
                    "actual row draw provenance mismatch"
                );
                let mut ablated_logits = audit.candidate_logits.clone();
                for row in &decision.port.candidates {
                    ablated_logits[row.candidate_index as usize] = row.base_logit;
                }
                let ablated = sample_cpu_joint_action(
                    trainer.phenotype(),
                    frame,
                    &ablated_logits,
                    2026111303,
                    counter,
                    1.,
                )
                .unwrap();
                let changed = ablated.representative != exact.representative
                    || ablated.motor_candidates != exact.motor_candidates;
                if changed {
                    ablation_changed += 1;
                }
                let after = host
                    .world()
                    .organism_registry()
                    .get(decision.organism_id)
                    .unwrap();
                let value = serde_json::json!({"arm":name,"counter":counter,"tick_before":step.tick_before,"tick_after":step.tick_after,
                    "sequence":decision.patch.header().sequence_id,"frame_digest":frame.frame_digest(),"selected_command":decision.patch.decision().selected_action,
                    "action":decision.action,"candidates":frame.candidates().iter().enumerate().map(|(index, c)|serde_json::json!({"index":index,"action_id":c.action_id,"kind":c.kind,"target":c.target})).collect::<Vec<_>>(),"port":decision.port,"brain_work":decision.brain_work,"biological_reward":decision.biological_reward,
                    "final_biology":CpuNeedBiologyCostV4::from_biology(after.biochemistry()),"actual_cognitive_energy_debit":after.cognitive_energy_debit(),
                    "founder_optimizer_exact":true,"sleep_controller_phase":format!("{:?}",after.sleep_phase()),
                    "same_row_reader_ablation":{"counterfactual_only":true,"action":ablated,"logits":ablated_logits,"joint_changed":changed,"not_an_alternate_world_trajectory":true}});
                writeln!(journal, "{value}").unwrap();
                journal.sync_all().unwrap();
            }
            let after_interval = host.world().organism_registry().iter().next().unwrap();
            if !after_interval.lifecycle().is_alive() {
                stop = "death";
                break;
            }
            if after_interval.biochemistry().body.health < 0.95
                || after_interval.biochemistry().body.injury > 0.
                || after_interval.biochemistry().body.energy < 0.35
            {
                stop = "biology_safeguard";
                break;
            }
            if step.sleeping_organisms > 0 || step.sleep_consolidations > 0 {
                stop = "autonomous_sleep";
                break;
            }
        }
        let after = host.world().organism_registry().iter().next().unwrap();
        assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
        let summary = serde_json::json!({"arm":name,"gain":gain,"sampling_seed":2026111303u64,"construction_seed":CONSTRUCTION_SEED,"asset_sha256":asset,
            "intervals":intervals,"decisions":decisions,"stop_reason":stop,"stored_rest":stored_rest,"stored_noop":stored_noop,"stored_observed":stored_observed,
            "compatible_rest_opportunities":compatible_rest,"nonzero_rest_readout_opportunities":nonzero_rest,"max_abs_rest_readout":max_abs_rest,
            "same_row_ablation_changed_joint_choices":ablation_changed,"source_rejections":failures,"initial_biology":start,
            "final_biology":CpuNeedBiologyCostV4::from_biology(after.biochemistry()),"founder_optimizer_exact":true,"legacy_checkpoint_unsupported":true,
            "claim":"bounded native association/reader test; source counts and self-reference alone are not learned competence"});
        write_json_new(&dir.join(format!("{name}.summary.json")), &summary);
        summaries.push(summary);
    }
    write_json_new(
        &dir.join("summary.json"),
        &serde_json::json!({"study":"native-need-association-v4-matched-own-history","protocol_sha256":protocol_hash,"executable_sha256":executable_hash,"arms":summaries}),
    );
}
