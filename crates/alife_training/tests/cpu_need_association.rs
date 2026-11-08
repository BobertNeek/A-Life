#![cfg(feature = "cpu-backend")]
//! V4 detached qualification. Closed V2/V3 lives remain in their own checkouts.
//! These tests perform no neural decisions or world actions.
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
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, SAMPLING_SEED as u32, |world| {
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
            matches!(
                c.kind,
                ActionKind::Rest | ActionKind::Idle | ActionKind::Inspect
            ) || c.action_id == HeadlessActionIds::NO_POSTURE
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

#[test]
fn native_need_v4_prepared_600_detached_comparator_and_actual_binding_cost() {
    // Fixed, prospective juvenile preparation. Ordinary body ticks, no selected
    // command, no drive overwrite, no synthetic source, and no native decision.
    let (mut host, trainer, asset_hash) = fixture_with_preparation(600);
    let ordinary = host.preview_need_feasibility(&trainer, 1.).unwrap();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
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
    assert!(bound[0].planned_atp_q16 >= ordinary[0].planned_atp_q16);
    assert!(bound[0].available_atp_q16 >= bound[0].planned_atp_q16);
    let record = host.world().organism_registry().iter().next().unwrap();
    println!(
        "{}",
        serde_json::json!({
            "study":"need-association-v4-prepared-detached-feasibility", "asset_sha256":asset_hash,
            "ordinary_preparation_ticks":600,"native_decisions":0,"acquired_sources":0,
            "maturation":record.phenotype().development_state_at(Tick::new(600)).unwrap().maturation,
            "biology":CpuNeedBiologyCostV4::from_biology(record.biochemistry()),
            "ordinary":ordinary,"bound_reader":bound,
            "pre_port_checkpoint_sha256":format!("{:x}",Sha256::digest(before))
        })
    );
}
