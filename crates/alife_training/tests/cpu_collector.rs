#![cfg(feature = "cpu-backend")]
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
#[path = "cpu_collector/collector_shards.rs"]
mod collector_shards;
#[path = "cpu_collector/need_switching.rs"]
mod need_switching;
fn fixture() -> (CpuCanonicalHost, CpuFoundationTrainer) {
    fixture_atp(None)
}
fn fixture_atp(atp: Option<f32>) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let (game, trainer) = new_game_fixture(atp);
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap(),
        trainer,
    )
}
fn new_game_fixture(atp: Option<f32>) -> (CanonicalNewGame, CpuFoundationTrainer) {
    let (game, trainer, _) = new_game_with_inputs_fixture(atp);
    (game, trainer)
}
fn new_game_with_inputs_fixture(
    atp: Option<f32>,
) -> (
    CanonicalNewGame,
    CpuFoundationTrainer,
    PhenotypeCompilerInputs,
) {
    new_game_with_profile_fixture(atp, SensorProfile::GroundedObjectSlotsV1)
}
fn new_game_with_profile_fixture(
    atp: Option<f32>,
    profile: SensorProfile,
) -> (
    CanonicalNewGame,
    CpuFoundationTrainer,
    PhenotypeCompilerInputs,
) {
    let builtin = if profile == SensorProfile::GroundedTerrainVisionV1 {
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x7A1123, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let phenotype = PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            profile,
        )
        .unwrap();
        FoundationWeightAsset::from_phenotype_for_genetic_birth(&phenotype).unwrap()
    } else {
        FoundationWeightAsset::builtin_n2048_v1(profile).unwrap()
    };
    let mut config = CanonicalNewGameConfig::phase3(0x7A1123, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = profile;
    let game = create_canonical_new_game_with_n2048_candidate(&config, &builtin).unwrap();
    let record = game.world.organism_registry().iter().next().unwrap();
    let genome = &record.phenotype().brain_genome;
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let base =
        PhenotypeCompiler::compile(genome, &capacity, &development, config.sensor_profile).unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        base.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (p, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    let mut game = create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
    if let Some(atp) = atp {
        let record = game.world.organism_registry().iter().next().unwrap();
        let mut chemistry = *record.biochemistry();
        chemistry.homeostasis.drives.brain_atp = atp;
        let replacement = WorldOrganismRecord::new(
            record.organism_id(),
            record.world_entity_id(),
            record.genome().clone(),
            record.phenotype().clone(),
            chemistry,
            record.birth_tick(),
        )
        .unwrap();
        game.world
            .replace_organism_record_exact(replacement)
            .unwrap();
    }
    let mask = StageTrainableMask::from_synapse_indices(
        &p,
        &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
    )
    .unwrap();
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
    (game, trainer, inputs)
}
#[test]
fn canonical_restore_retains_observed_objects_and_their_identity_cursor() {
    let (mut host, trainer) = fixture();
    let id = host.world().organism_entity_ids()[0].0;
    let homeostasis = host
        .world()
        .organism_registry()
        .get(id)
        .unwrap()
        .biochemistry()
        .homeostasis;
    host.world_mut()
        .perception_frame(
            id,
            Tick::ZERO,
            trainer.phenotype().sensor_profile(),
            homeostasis,
        )
        .unwrap();
    let tracked = host.world().tracked_objects().save_state(id).unwrap();
    assert!(!tracked.records.is_empty());
    let saved = host.checkpoint(&trainer).unwrap();
    let saved = serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let mut restored = CpuCanonicalHost::from_checkpoint(saved, &trainer).unwrap();
    assert_eq!(
        restored.world().tracked_objects().save_state(id).unwrap(),
        tracked
    );
    let a = host.step(&trainer, 1.0).unwrap();
    let b = restored.step(&trainer, 1.0).unwrap();
    assert!(serde_json::to_vec(&a.decisions).unwrap() == serde_json::to_vec(&b.decisions).unwrap());
}

#[test]
fn canonical_restore_rejects_missing_or_inconsistent_perception_identity() {
    let (mut host, trainer) = fixture();
    host.step(&trainer, 1.0).unwrap();
    let good = host.checkpoint(&trainer).unwrap();
    assert!(
        !good.tracked_objects[0].records.is_empty(),
        "native perception must commit its tracked identities"
    );
    let mut invalid = Vec::new();
    let mut legacy = serde_json::to_value(&good).unwrap();
    legacy["schema_version"] = serde_json::json!(1);
    legacy.as_object_mut().unwrap().remove("tracked_objects");
    // Old bytes remain decodable/inspectable, but lack evidence for exact resume.
    invalid.push(serde_json::from_value(legacy).unwrap());
    let mut missing = good.clone();
    missing.tracked_objects.clear();
    invalid.push(missing);
    let mut duplicate = good.clone();
    duplicate
        .tracked_objects
        .push(duplicate.tracked_objects[0].clone());
    invalid.push(duplicate);
    let mut foreign = good.clone();
    foreign.tracked_objects[0].organism_id = OrganismId(2);
    invalid.push(foreign);
    let mut future = good;
    future.tracked_objects[0].records[0].last_seen_tick = Tick(host.world().tick().raw() + 1);
    invalid.push(future);
    for checkpoint in invalid {
        assert!(CpuCanonicalHost::from_checkpoint(checkpoint, &trainer).is_err());
    }
}

#[test]
fn canonical_native_perception_commits_the_identity_used_by_each_frame() {
    let (mut host, trainer) = fixture();
    let mut observed = 0;
    for _ in 0..3 {
        for row in host.step(&trainer, 1.0).unwrap().decisions {
            for slot in row.patch.pre_action().perception().grounded_object_slots() {
                let tracked = host
                    .world()
                    .tracked_objects()
                    .record(row.organism_id, slot.tracked_object_id)
                    .expect("sealed perception identity missing from world");
                assert_eq!(tracked.last_seen_tick, row.patch.pre_action().tick);
                observed += 1;
            }
        }
    }
    assert!(observed > 0);
}

#[test]
fn canonical_cpu_seals_biology_memory_and_exact_restore_without_an_icd() {
    let (mut host, trainer) = fixture();
    let started = std::time::Instant::now();
    let mut decisions = 0;
    for _ in 0..5 {
        let step = host.step(&trainer, 1.).unwrap();
        assert_eq!(step.tick_after.raw(), step.tick_before.raw() + 1);
        for decision in step.decisions {
            decision.patch.validate_contract().unwrap();
            let packet = OutcomeCreditPacket::from_sealed_patch(&decision.patch).unwrap();
            assert_eq!(packet.policy_backend(), PolicyBackend::NeuralClosedLoopCpu);
            assert!(alife_gpu_backend::GpuOutcomeCreditRecord::try_from(&packet).is_err());
            assert_eq!(
                decision.patch.pre_action().evidence_kind(),
                EvidenceKind::NeuralClosedLoopCpu
            );
            assert_eq!(
                decision.patch.decision().evidence_kind(),
                EvidenceKind::NeuralClosedLoopCpu
            );
            assert!(decision.patch.outcome().measured_physiology.is_some());
            assert_eq!(
                decision.patch.outcome().outcome_tick.raw(),
                decision.patch.pre_action().tick.raw() + 1
            );
            decisions += 1;
        }
    }
    assert!(decisions > 0);
    let checkpoint = host.checkpoint(&trainer).unwrap();
    let json = serde_json::to_vec(&checkpoint).unwrap();
    let checkpoint = serde_json::from_slice(&json).unwrap();
    let mut restored = CpuCanonicalHost::from_checkpoint(checkpoint, &trainer).unwrap();
    let a = host.step(&trainer, 1.).unwrap();
    let b = restored.step(&trainer, 1.).unwrap();
    assert_eq!(a.decisions.len(), b.decisions.len());
    for (a, b) in a.decisions.iter().zip(&b.decisions) {
        assert_eq!(a.patch, b.patch);
        assert_eq!(a.action, b.action);
    }
    assert_eq!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap()
    );
    eprintln!(
        "native CPU canonical fixture: valid_decisions={decisions}, wall_seconds={:.6}",
        started.elapsed().as_secs_f64()
    );
}
#[test]
fn snapshot_branch_and_failed_step_preserve_all_private_state() {
    let (mut host, trainer) = fixture();
    host.step(&trainer, 1.).unwrap();
    let snapshot = host.snapshot();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    assert!(host.step(&trainer, 0.).is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    let a = host.step(&trainer, 1.).unwrap();
    host.restore_snapshot(snapshot, &trainer).unwrap();
    let b = host.step(&trainer, 1.).unwrap();
    assert_eq!(a.decisions[0].patch, b.decisions[0].patch);
}
#[test]
fn canonical_rollout_trains_full_residual_with_native_optimizer_ages() {
    let (mut host, mut trainer) = fixture();
    let mut rows = Vec::new();
    for _ in 0..7 {
        rows.extend(host.step(&trainer, 0.9).unwrap().decisions);
    }
    let phenotype = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .phenotype()
        .clone();
    let mut actor = trainer.checkpoint();
    actor.optimizer_step = 16_684;
    actor.update_ages.fill(16_684);
    trainer.restore_checkpoint(actor).unwrap();
    let mut value = CpuValueHead::from_checkpoint(alife_training::PpoValueHeadCheckpoint {
        feature_count: trainer.phenotype().neuron_count(),
        optimizer_step: 19_800,
        parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
        last_updated_policy_version: Some(7399),
    })
    .unwrap();
    let config = alife_training::PpoConfig {
        temperature: 0.9,
        epochs: 1,
        ..Default::default()
    };
    let windows = cpu_training_windows(
        &rows, None, &trainer, &value, &phenotype, 7400, config, 3, 2,
    )
    .unwrap();
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[1].actor.sequence.burn_in_ticks, 2);
    let before = trainer.checkpoint();
    let started = std::time::Instant::now();
    let receipt = train_cpu_residual_cohort(
        &mut trainer,
        &mut value,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        config,
        7400,
    )
    .unwrap();
    assert_eq!(receipt.actor_optimizer_step, 16_685);
    assert_eq!(receipt.value_optimizer_step, 19_801);
    assert!(trainer
        .weights()
        .iter()
        .zip(before.weights)
        .any(|(a, b)| *a != b));
    let checkpoint = value.checkpoint();
    let json = serde_json::to_vec(&checkpoint).unwrap();
    assert_eq!(
        CpuValueHead::from_checkpoint(serde_json::from_slice(&json).unwrap())
            .unwrap()
            .checkpoint(),
        checkpoint
    );
    assert!(host.step(&trainer, 0.9).is_err());
    eprintln!("native CPU canonical PPO/residual: valid_decisions={}, train_rows={}, actor_age={}, critic_age={}, update_wall_seconds={:.6}",rows.len(),rows.len()-1,receipt.actor_optimizer_step,receipt.value_optimizer_step,started.elapsed().as_secs_f64());
}
#[test]
fn recovery_sleep_replays_structural_state_and_roundtrips_authority() {
    let (mut host, trainer) = fixture();
    for _ in 0..3 {
        host.step(&trainer, 1.).unwrap();
    }
    let id = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let before = host.checkpoint(&trainer).unwrap();
    assert!(before.residents[0]
        .neural
        .eligibility
        .iter()
        .any(|v| *v != 0.));
    host.request_recovery_sleep(id).unwrap();
    let mut committed = false;
    for _ in 0..14 {
        let step = host.step(&trainer, 1.).unwrap();
        if step.sleep_consolidations > 0 {
            committed = true;
            let checkpoint = host.checkpoint(&trainer).unwrap();
            assert!(checkpoint.residents[0]
                .neural
                .eligibility
                .iter()
                .all(|v| *v == 0.));
            assert!(
                checkpoint.residents[0]
                    .neural_topology
                    .structural
                    .connection_count()
                    > 0
            );
            assert!(
                host.world()
                    .organism_registry()
                    .get(id)
                    .unwrap()
                    .sleep_work_units()
                    > 0
            );
            let mut restored = CpuCanonicalHost::from_checkpoint(
                serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap(),
                &trainer,
            )
            .unwrap();
            let a = host.step(&trainer, 1.).unwrap();
            let b = restored.step(&trainer, 1.).unwrap();
            assert_eq!(a.decisions.len(), b.decisions.len());
            assert_eq!(
                serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
                serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap()
            );
            break;
        }
    }
    assert!(committed);
}
#[test]
fn restore_rejects_cross_owner_or_modified_private_state() {
    let (mut host, trainer) = fixture();
    host.step(&trainer, 1.).unwrap();
    let good = host.checkpoint(&trainer).unwrap();
    let mut wrong = good.clone();
    wrong.sampling_counter += 1;
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good.clone();
    wrong.sampling_seed ^= 1;
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good.clone();
    wrong.residents[0].language_grounding = LanguageGroundingLedger::default();
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good.clone();
    wrong.residents[0].neural.fast[0] += 0.1;
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good.clone();
    let topology = &good.residents[0].topology;
    wrong.residents[0].topology = TopologySidecar::new_profiled(
        OrganismId(topology.organism_id_raw),
        topology.profile,
        TopologicalMapConfig::default(),
    )
    .unwrap()
    .export_portable()
    .unwrap();
    TopologySidecar::restore_portable(wrong.residents[0].topology.clone()).unwrap();
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good.clone();
    wrong.residents[0].memory.max_match_count = 3;
    wrong.residents[0].memory.canonical_digest = wrong.residents[0]
        .memory
        .recompute_canonical_digest()
        .unwrap();
    MemorySidecarState::restore_portable(
        wrong.residents[0].memory.profile,
        wrong.residents[0].memory_compaction,
        wrong.residents[0].memory.clone(),
        wrong.residents[0].staged_memory.clone(),
    )
    .unwrap();
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good.clone();
    wrong.residents[0].memory_compaction.next_cycle_id += 1;
    wrong.residents[0]
        .memory_compaction
        .validate_contract()
        .unwrap();
    MemorySidecarState::restore_portable(
        wrong.residents[0].memory.profile,
        wrong.residents[0].memory_compaction,
        wrong.residents[0].memory.clone(),
        wrong.residents[0].staged_memory.clone(),
    )
    .unwrap();
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
    let mut wrong = good;
    wrong.residents[0].neural.organism_id = OrganismId(2);
    assert!(CpuCanonicalHost::from_checkpoint(wrong, &trainer).is_err());
}

#[test]
fn forced_sleep_completes_wake_and_resumes_real_decisions() {
    let (mut host, trainer) = fixture();
    host.step(&trainer, 1.).unwrap();
    let id = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    host.request_recovery_sleep(id).unwrap();
    let mut saw_commit = false;
    let mut resumed = false;
    for _ in 0..80 {
        let step = host.step(&trainer, 1.).unwrap();
        saw_commit |= step.sleep_consolidations > 0;
        if !step.decisions.is_empty() {
            resumed = true;
            assert!(saw_commit);
            break;
        }
    }
    assert!(
        resumed,
        "sleep must not deadlock at the first waking boundary"
    );
    CpuCanonicalHost::from_checkpoint(host.checkpoint(&trainer).unwrap(), &trainer).unwrap();
}
#[test]
fn atp_policy_reduces_native_work_and_refuses_unaffordable_dispatch() {
    let (_, trainer) = fixture();
    let p = trainer.phenotype();
    let anchor = CpuActivityAnchor {
        organism_id: OrganismId(1),
        tick: Tick::ZERO,
        sequence_cursor: 1,
        previous_dispatch_generation: 0,
        source_frame_digest: [0; 4],
        frame_digest: p.phenotype_hash().0,
    };
    let full = cpu_activity_budget(p, anchor, 1., 32, 16).unwrap();
    let low = cpu_activity_budget(p, anchor, 0.1, 32, 16).unwrap();
    let empty = cpu_activity_budget(p, anchor, 0., 32, 16).unwrap();
    assert_eq!(full.microsteps, p.microstep_count());
    assert_eq!(
        low.microsteps,
        BrainCapacityClass::n2048().execution().microstep_range().0
    );
    assert_eq!(low.microsteps, 2);
    assert!(low.affordable);
    assert!(low.enabled_routes.iter().filter(|x| **x).count() < full.enabled_routes.len());
    assert!(low.work.synapse_ops < full.work.synapse_ops);
    assert!(full.affordable);
    assert!(!empty.affordable);
}
#[test]
fn sleep_gap_reward_uses_only_gap_duration_and_preserves_full_gae_elapsed() {
    let (mut host, trainer) = fixture();
    let mut rows = host.step(&trainer, 1.).unwrap().decisions;
    let id = rows[0].organism_id;
    host.request_recovery_sleep(id).unwrap();
    for _ in 0..80 {
        rows.extend(host.step(&trainer, 1.).unwrap().decisions);
        if rows.len() >= 3 {
            break;
        }
    }
    assert_eq!(rows.len(), 3);
    let phenotype = host
        .world()
        .organism_registry()
        .get(id)
        .unwrap()
        .phenotype();
    let value = CpuValueHead::from_checkpoint(alife_training::PpoValueHeadCheckpoint {
        feature_count: trainer.phenotype().neuron_count(),
        optimizer_step: 0,
        parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
        last_updated_policy_version: None,
    })
    .unwrap();
    let config = alife_training::PpoConfig::default();
    let windows =
        cpu_training_windows(&rows, None, &trainer, &value, phenotype, 1, config, 4, 0).unwrap();
    let after = rows[0]
        .patch
        .outcome()
        .measured_physiology
        .as_ref()
        .unwrap()
        .after;
    let before = rows[1]
        .patch
        .outcome()
        .measured_physiology
        .as_ref()
        .unwrap()
        .before;
    let gap = MeasuredPhysiologyTransition::new(after, before).unwrap();
    let receptors = after.neural_receptor_frame(phenotype).unwrap();
    let sample = NeuromodulatorSample::from_components(
        0.,
        gap.aversive_value(),
        gap.homeostatic_improvement(),
        0.,
        0.,
    )
    .unwrap()
    .with_biochemical_receptors(&receptors)
    .unwrap();
    let parameters = phenotype.brain_genome.plasticity_parameters();
    let profile = parameters
        .action_candidate_credit_profile()
        .map_or_else(|| parameters.receptor_profile(), |p| p.receptor_profile());
    let dt = (before.tick.raw() - after.tick.raw()) as f32 / 20.;
    let expected = (rows[0].biological_reward
        + profile.project(&sample.frame()).unwrap()
            * (-std::f32::consts::LN_2 * dt / config.discount_half_life_seconds).exp())
    .clamp(-1., 1.);
    let transition = &windows[0].actor.batch.transitions()[0];
    assert!((transition.reward - expected).abs() < 1e-6);
    assert_eq!(
        transition.elapsed_seconds,
        (rows[1].patch.pre_action().tick.raw() - rows[0].patch.pre_action().tick.raw()) as f32
            / 20.
    );
}

#[test]
fn canonical_low_atp_executes_minimum_budget_and_empty_atp_sleeps() {
    let (mut host, trainer) = fixture_atp(Some(0.1));
    let step = host.step(&trainer, 1.).unwrap();
    assert_eq!(step.decisions.len(), 1);
    let decision = &step.decisions[0];
    assert_eq!(decision.replay_tick.microstep_count, 2);
    assert!(decision
        .replay_tick
        .enabled_routes
        .iter()
        .any(|enabled| !enabled));
    assert_eq!(decision.brain_work.microsteps, 2);
    assert_eq!(
        decision.brain_work.neuron_updates,
        2 * u64::from(trainer.phenotype().neuron_count())
    );
    let budget = cpu_activity_budget(
        trainer.phenotype(),
        CpuActivityAnchor {
            organism_id: decision.organism_id,
            tick: Tick::ZERO,
            sequence_cursor: 1,
            previous_dispatch_generation: 0,
            source_frame_digest: [0; 4],
            frame_digest: decision.patch.pre_action().frame_digest().unwrap().0,
        },
        0.1,
        decision.replay_tick.candidates.len() as u32,
        decision.replay_tick.candidates.len() as u32,
    )
    .unwrap();
    assert_eq!(decision.brain_work, budget.work);
    let work = decision.patch.cognitive_work().unwrap();
    assert_eq!(work.synapses_evaluated, decision.brain_work.synapse_ops);
    assert_eq!(
        work.neural_updates,
        2 * u64::from(trainer.phenotype().neuron_count())
    );
    let (mut host, trainer) = fixture_atp(Some(0.));
    let step = host.step(&trainer, 1.).unwrap();
    assert!(step.decisions.is_empty());
    assert_eq!(step.sleeping_organisms, 1);
    CpuCanonicalHost::from_checkpoint(host.checkpoint(&trainer).unwrap(), &trainer).unwrap();
}

#[test]
fn prepared_episode_admits_newborn_before_ordinary_aging_and_keeps_founder_state() {
    let (game, trainer) = new_game_fixture(None);
    let founder = trainer.checkpoint();
    let (mut host, prepared_at) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, 123, |world| {
            for _ in 0..12 {
                world.try_advance_tick()?;
            }
            Ok::<_, OrganismRegistryError>(world.tick())
        })
        .unwrap();
    assert_eq!(prepared_at.raw(), 12);
    assert_eq!(trainer.checkpoint(), founder);
    let inputs = host.semantic_prior_inputs(&trainer).unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].1.raw(), 1);
    assert_eq!(host.world().tick(), prepared_at);
    let step = host.step(&trainer, 1.).unwrap();
    assert_eq!(step.decisions.len(), 1);
    let row = &step.decisions[0];
    assert_eq!(row.patch.pre_action().tick, prepared_at);
    assert_eq!(row.patch.header().sequence_id.raw(), 1);
    let bytes = serde_json::to_vec(row).unwrap();
    let restored: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
    restored.action.validate(1.).unwrap();
    assert_eq!(restored.patch, row.patch);
    assert_eq!(restored.action, row.action);
    assert_eq!(trainer.checkpoint(), founder);

    let (mut aged, trainer) = new_game_fixture(None);
    aged.world.try_advance_tick().unwrap();
    let called = std::cell::Cell::new(false);
    let result = CpuCanonicalHost::new_prepared_cohort(aged, &trainer, 123, |_| {
        called.set(true);
        Ok::<_, CpuHostError>(())
    });
    assert!(result.is_err());
    assert!(
        !called.get(),
        "preparation must never bypass existing-individual admission"
    );
}

#[test]
fn semantic_preparation_cannot_edit_ordinary_senses_or_advance_a_failed_host() {
    let (mut host, trainer) = fixture();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let inputs = host.semantic_prior_inputs(&trainer).unwrap();
    assert_eq!(inputs[0].1.raw(), 1);
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    let failure = host.step_with_preparation(&trainer, 1., |draft, _| {
        draft.with_remembered_novelty(0.736)
    });
    assert!(failure.is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    let success = host
        .step_with_preparation(&trainer, 1., |draft, _| Ok(draft))
        .unwrap();
    assert_eq!(success.decisions[0].patch.header().sequence_id.raw(), 1);
}
