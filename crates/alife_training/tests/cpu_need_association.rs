#![cfg(feature = "cpu-backend")]
//! Reconstruction V2 of the frozen single native admission fixture. The first
//! environment's launch failed before process creation: no world exposure began.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
const SEED: u64 = 2026111301;
const STUDY: &str = "need-association-admission-v1";
const FROZEN_ASSET: &str = "1d2978cbd857a7ca99dc1b2cf65be9f6ff417a6f78199282e0431b94332c9377";
fn fixture() -> (CpuCanonicalHost, CpuFoundationTrainer, String) {
    let mut config = CanonicalNewGameConfig::phase3(SEED, 1).unwrap();
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
    let host = CpuCanonicalHost::new_cohort(game, &trainer, SEED as u32).unwrap();
    (host, trainer, hash)
}
fn plan() -> CpuNeedAssociationPlanV1 {
    let mut plan = CpuNeedAssociationPlanV1::identity_control(SelfActionV1::Rest);
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
        STUDY,SEED,first_hash,trainer.phenotype().phenotype_hash());
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
            matches!(
                c.kind,
                ActionKind::Rest | ActionKind::Idle | ActionKind::Inspect
            ) || c.action_id == HeadlessActionIds::NO_POSTURE
        })
        .count();
    let circuit =
        NativeNeedAssociationCircuitV1::compile_with_execution(plan().recipe, plan().execution)
            .unwrap();
    let measured = circuit
        .run(
            frame.homeostasis().drives.to_array(),
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
    println!("stationary_fixture_preflight candidates={} reader_rows={} extra_atp_q16={} ordinary_atp_q16={} available_atp_q16={}",
        frame.candidates().len(),reader_rows,extra_debit,ordinary.debit_atp_q16,ordinary.available_atp_q16);
    assert!(ordinary.available_atp_q16 > ordinary.debit_atp_q16 + extra_debit);
    host.enable_need_association_port(&trainer, plan()).unwrap();
    assert!(host.checkpoint(&trainer).is_err());
    assert!(host.step(&trainer, 1.).is_err());
    assert!(host.enable_need_association_port(&trainer, plan()).is_err());
    let restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&original).unwrap(), &trainer)
            .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
        original
    );
}
#[test]
#[ignore = "One-shot native fixture; requires frozen source, local output directory and no-retry marker"]
fn native_need_stationary_admission_once() {
    let directory = std::path::PathBuf::from(
        std::env::var("ALIFE_NEED_FIXTURE_DIR").expect("explicit evidence directory"),
    );
    assert!(directory.is_dir());
    let (mut host, trainer, asset_hash) = fixture();
    let founder_before = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let configuration = plan();
    host.enable_need_association_port(&trainer, configuration)
        .unwrap();
    let mut marker = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("exposure.started"))
        .expect("refuse duplicate or replacement exposure");
    writeln!(marker, "{STUDY} {SEED} {asset_hash}").unwrap();
    marker.sync_all().unwrap();
    let mut journal = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("native-summary-rows.jsonl"))
        .unwrap();
    let mut rows = 0;
    let mut stored = 0;
    let mut stored_own = 0;
    let mut stored_observed = 0;
    let mut paired_observed_support = 0;
    let mut changed_logits = 0;
    let mut paired_support = 0;
    let mut nonzero = 0;
    let mut stop = "decision_budget";
    for _ in 0..12 {
        let record = host.world().organism_registry().iter().next().unwrap();
        if record.biochemistry().body.health < 0.98 || record.biochemistry().body.energy < 0.65 {
            stop = "health_or_reserve_safeguard";
            break;
        }
        let step = match host.step_need_association_port(&trainer, 1.) {
            Ok(step) => step,
            Err(error) => {
                writeln!(journal, "{}", json!({"host_error":error.to_string()})).unwrap();
                stop = "host_error";
                break;
            }
        };
        if step.decisions.is_empty() {
            stop = "sleep_or_no_decision";
            break;
        }
        for row in step.decisions {
            row.patch.validate_contract().unwrap();
            rows += 1;
            stored +=
                usize::from(row.port.source_observation == CpuNeedObservationStatusV1::Stored);
            let selected = row.patch.decision().neural_evidence().unwrap();
            if row.port.source_observation == CpuNeedObservationStatusV1::Stored {
                if selected.action_family == CandidateActionFamily::Inspect {
                    stored_observed += 1;
                } else {
                    stored_own += 1;
                }
            }
            for candidate in &row.port.candidates {
                if let (
                    CandidateActionFamily::Inspect,
                    Some(action_sequence),
                    Some(reference_sequence),
                ) = (
                    candidate.family,
                    candidate.support.action_sequence,
                    candidate.support.reference_sequence,
                ) {
                    paired_observed_support += 1;
                    assert!(action_sequence < row.patch.header().sequence_id.0);
                    assert!(reference_sequence < row.patch.header().sequence_id.0);
                }
                changed_logits +=
                    usize::from(candidate.final_logit.to_bits() != candidate.base_logit.to_bits());
            }
            let pairs = row
                .port
                .candidates
                .iter()
                .filter(|c| {
                    c.support.action_sequence.is_some() && c.support.reference_sequence.is_some()
                })
                .count();
            paired_support += pairs;
            nonzero += row
                .port
                .candidates
                .iter()
                .filter(|c| c.native.confidence_product_gated_readout != 0.)
                .count();
            let measured = row.patch.outcome().measured_physiology.as_ref().unwrap();
            let value = json!({"tick":row.patch.pre_action().tick.raw(),"sequence":row.patch.header().sequence_id.0,
                "selected_action":selected.action_id.raw(),"selected_family":selected.action_family,"selected_logit":selected.logit,
                "motor_candidates":row.action.motor_candidates,"source_observation":row.port.source_observation,"pair_count":pairs,
                "raw_measured_delta":std::array::from_fn::<_,11,_>(|i|measured.after.homeostasis.drives.to_array()[i]-measured.before.homeostasis.drives.to_array()[i]),
                "final_host_work":row.patch.outcome().cognitive_work,"reader_rows":row.port.candidates});
            writeln!(journal, "{value}").unwrap();
            journal.flush().unwrap();
            journal.sync_all().unwrap();
            println!(
                "native_admission tick={} sequence={} action={} source={:?} pairs={}",
                row.patch.pre_action().tick.raw(),
                row.patch.header().sequence_id.0,
                selected.action_id.raw(),
                row.port.source_observation,
                pairs
            );
        }
    }
    let founder_unchanged = serde_json::to_vec(&trainer.checkpoint()).unwrap() == founder_before;
    let summary = json!({"study":STUDY,"source_revision":"reconstruction-v2","prior_launch_status":"launch_failed_before_process",
        "seed":SEED,"asset_sha256":asset_hash,"plan":configuration,"native_decisions":rows,"stored_intervals":stored,
        "stored_own_intervals":stored_own,"stored_observed_intervals":stored_observed,"paired_observed_support_rows":paired_observed_support,
        "changed_candidate_logits":changed_logits,"paired_candidate_support_rows":paired_support,"nonzero_native_readouts":nonzero,
        "stop":stop,"founder_optimizer_exact":founder_unchanged,"learned_policy_competence_claim":false,"checkpoint_retention_claim":false});
    std::fs::write(
        directory.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("{summary}");
    assert!(founder_unchanged);
    assert!(rows <= 12);
    assert!(
        paired_observed_support > 0 && stored_own > 0 && stored_observed > 0,
        "preserved null: no paired observed recall; never retry"
    );
}
