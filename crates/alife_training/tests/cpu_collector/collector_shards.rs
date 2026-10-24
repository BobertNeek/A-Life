//! Synthetic, untrained native hosts only. No accepted checkpoints or learning
//! claims: this proves transport, admission and one shared optimizer history.
use super::*;
use alife_training::{FoundationTrainingBackendV1, PpoBoundary, PpoConfig, PpoValueHeadCheckpoint};

const VERSION: u64 = 7400;
fn config() -> PpoConfig {
    PpoConfig {
        temperature: 0.9,
        epochs: 1,
        ..Default::default()
    }
}
fn fixture() -> (CpuFoundationTrainer, CpuValueHead, PhenotypeCompilerInputs) {
    let (_, mut trainer, inputs) = new_game_with_inputs_fixture(None);
    let mut actor = trainer.checkpoint();
    actor.optimizer_step = 16_684;
    actor.update_ages.fill(16_684);
    actor.first_moment.fill(0.001);
    actor.second_moment.fill(0.002);
    trainer.restore_checkpoint(actor).unwrap();
    let n = trainer.phenotype().neuron_count() as usize + 1;
    let mut parameters = vec![0.; n * 3];
    parameters[..n].fill(0.001);
    parameters[n..2 * n].fill(0.002);
    parameters[2 * n..].fill(0.003);
    let value = CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
        feature_count: trainer.phenotype().neuron_count(),
        optimizer_step: 19_800,
        parameters,
        last_updated_policy_version: Some(VERSION - 1),
    })
    .unwrap();
    (trainer, value, inputs)
}
fn shard(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    inputs: &PhenotypeCompilerInputs,
    run: &str,
    seed: u32,
    terminal: bool,
) -> CpuCollectorShardV1 {
    let (game, _, _) = new_game_with_inputs_fixture(None);
    let mut host = CpuCanonicalHost::new_cohort(game, trainer, seed).unwrap();
    let mut decisions = Vec::new();
    for _ in 0..4 {
        decisions.extend(host.step(trainer, config().temperature).unwrap().decisions);
    }
    assert_eq!(decisions.len(), 4);
    let mut record = host
        .world()
        .organism_registry()
        .get(decisions[0].organism_id)
        .unwrap()
        .clone();
    if terminal {
        record.mark_dead(record.biochemistry().tick).unwrap();
    }
    CpuCollectorShardV1::seal(
        CpuCollectionSnapshotV1::capture(trainer, value, inputs, config(), VERSION).unwrap(),
        CpuCollectorAssignmentV1 {
            collector_run_id: run.into(),
            backend: FoundationTrainingBackendV1::NativeCpu,
            producer_revision: "synthetic-fixture-v1".into(),
            world_seed: 0x7A1123,
            sampling_seed: seed,
            sampling_counter_start: 0,
            sampling_counter_end: decisions.len() as u32,
            lesson: format!("synthetic-{run}"),
            episode_config_sha256: [1; 32],
        },
        vec![CpuCollectorTrajectoryV1 {
            final_record: record,
            decisions,
        }],
    )
    .unwrap()
}
fn admit(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    inputs: &PhenotypeCompilerInputs,
    shards: Vec<CpuCollectorShardV1>,
) -> Result<AdmittedCpuCollectorCohort, alife_training::TrainingError> {
    let assignments = shards
        .iter()
        .map(|s| s.assignment.clone())
        .collect::<Vec<_>>();
    admit_cpu_collector_shards(
        trainer,
        value,
        inputs,
        config(),
        VERSION,
        &assignments,
        shards,
        2,
        1,
    )
}
fn reseal(shard: CpuCollectorShardV1) -> CpuCollectorShardV1 {
    CpuCollectorShardV1::seal(shard.snapshot, shard.assignment, shard.trajectories).unwrap()
}

#[test]
fn two_collectors_roundtrip_and_update_one_canonical_actor_and_critic() {
    let (mut trainer, mut value, inputs) = fixture();
    let a = shard(&trainer, &value, &inputs, "a", 42, false);
    let b = shard(&trainer, &value, &inputs, "integration", 43, true);
    assert_eq!(
        a.trajectories[0].final_record.organism_id(),
        b.trajectories[0].final_record.organism_id()
    );
    let before_actor = trainer.checkpoint();
    let before_value = value.checkpoint();
    let bytes = serde_json::to_vec(&a).unwrap();
    let restored: CpuCollectorShardV1 = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    assert_eq!(
        restored.trajectories[0].decisions[2].action,
        a.trajectories[0].decisions[2].action
    );
    let expected_a = cpu_training_windows(
        &a.trajectories[0].decisions,
        None,
        &trainer,
        &value,
        a.trajectories[0].final_record.phenotype(),
        VERSION,
        config(),
        2,
        1,
    )
    .unwrap();
    let cohort = admit(&trainer, &value, &inputs, vec![restored, b]).unwrap();
    assert_eq!(cohort.windows().len(), 4);
    assert_eq!(cohort.windows()[0].actor.batch, expected_a[0].actor.batch);
    assert_eq!(cohort.windows()[1].actor.batch, expected_a[1].actor.batch);
    assert_eq!(cohort.windows()[1].actor.sequence.burn_in_ticks, 1);
    assert_eq!(cohort.window_trajectories()[0].collector_run_id, "a");
    assert_eq!(
        cohort.window_trajectories()[2].collector_run_id,
        "integration"
    );
    let truncated = cohort.windows()[1]
        .actor
        .batch
        .transitions()
        .last()
        .unwrap();
    assert_eq!(truncated.boundary, PpoBoundary::Truncated);
    assert_ne!(truncated.next_value, 0.);
    let terminated = cohort.windows()[3]
        .actor
        .batch
        .transitions()
        .last()
        .unwrap();
    assert_eq!(terminated.boundary, PpoBoundary::Terminated);
    assert_eq!(terminated.next_value, 0.);
    assert_eq!(trainer.checkpoint(), before_actor);
    assert_eq!(value.checkpoint(), before_value);
    let receipt = cohort
        .train(&mut trainer, &mut value, &inputs, config(), 4)
        .unwrap();
    assert_eq!(receipt.completed_epochs, 1);
    assert_eq!(
        receipt.actor_optimizer_step,
        before_actor.optimizer_step + 1
    );
    assert_eq!(
        receipt.value_optimizer_step,
        before_value.optimizer_step + 1
    );
    assert!(trainer
        .checkpoint()
        .update_ages
        .iter()
        .zip(&before_actor.update_ages)
        .all(|(a, b)| *a == b + 1));
    assert_ne!(trainer.checkpoint().first_moment, before_actor.first_moment);
    assert_ne!(value.checkpoint().parameters, before_value.parameters);
}

#[test]
fn corrupted_stale_duplicate_and_resealed_invalid_evidence_never_mutates_optimizers() {
    let (trainer, value, inputs) = fixture();
    let source = shard(&trainer, &value, &inputs, "a", 42, false);
    let before_actor = trainer.checkpoint();
    let before_value = value.checkpoint();
    let mut cases = Vec::new();
    let mut wrong = source.clone();
    wrong.trajectories[0].decisions[0].biological_reward += 0.1;
    cases.push(vec![wrong.clone()]); // Broken seal.
    cases.push(vec![reseal(wrong)]); // Recomputed seal cannot authorize wrong credit.
    let mut wrong = source.clone();
    wrong.trajectories[0].decisions[0]
        .action
        .old_joint_log_probability -= 0.1;
    cases.push(vec![reseal(wrong)]);
    let mut wrong = source.clone();
    wrong.trajectories[0]
        .decisions
        .last_mut()
        .unwrap()
        .final_features[0] += 0.1;
    cases.push(vec![reseal(wrong)]); // Bootstrap is checked too.
    let mut wrong = source.clone();
    wrong.trajectories[0].decisions[1].sampling_counter = 0;
    cases.push(vec![reseal(wrong)]);
    let mut wrong = source.clone();
    wrong.trajectories[0].decisions.swap(0, 1);
    cases.push(vec![reseal(wrong)]);
    let mut wrong = source.clone();
    wrong.assignment.sampling_seed = 987_654;
    cases.push(vec![reseal(wrong)]);
    for field in 0..7 {
        let mut wrong = source.clone();
        match field {
            0 => wrong.snapshot.policy_version += 1,
            1 => wrong.snapshot.biological_objective_version += 1,
            2 => wrong.snapshot.critic_objective_version += 1,
            3 => wrong.snapshot.actor_sha256[0] ^= 1,
            4 => wrong.snapshot.critic_sha256[0] ^= 1,
            5 => wrong.snapshot.compiler_sha256[0] ^= 1,
            _ => wrong.snapshot.ppo_config_sha256[0] ^= 1,
        }
        cases.push(vec![reseal(wrong)]);
    }
    let mut wrong = source.clone();
    wrong.schema_version = 2;
    cases.push(vec![wrong]);
    let mut wrong = source.clone();
    let record = &mut wrong.trajectories[0].final_record;
    record
        .mark_dead(Tick(record.biochemistry().tick.raw() + 1))
        .unwrap();
    cases.push(vec![reseal(wrong)]); // Unobserved terminal gap.
    let mut wrong = source.clone();
    wrong.trajectories.push(wrong.trajectories[0].clone());
    cases.push(vec![reseal(wrong)]);
    cases.push(vec![source.clone(), source.clone()]);
    let mut relabelled = source.clone();
    relabelled.assignment.collector_run_id = "duplicate-disguised-as-b".into();
    cases.push(vec![source.clone(), reseal(relabelled)]);
    for (index, shards) in cases.into_iter().enumerate() {
        assert!(
            admit(&trainer, &value, &inputs, shards).is_err(),
            "case {index}"
        );
        assert_eq!(trainer.checkpoint(), before_actor, "case {index}");
        assert_eq!(value.checkpoint(), before_value, "case {index}");
    }
    let mut expected = source.assignment.clone();
    expected.lesson = "different-lesson".into();
    assert!(admit_cpu_collector_shards(
        &trainer,
        &value,
        &inputs,
        config(),
        VERSION,
        &[expected],
        vec![source],
        2,
        1,
    )
    .is_err());
}

#[test]
fn invalid_window_bounds_and_incomplete_inventory_are_rejected() {
    let (trainer, value, inputs) = fixture();
    let source = shard(&trainer, &value, &inputs, "a", 42, false);
    for (window, burn) in [(0, 1), (usize::MAX, 1), (2, usize::MAX)] {
        assert!(admit_cpu_collector_shards(
            &trainer,
            &value,
            &inputs,
            config(),
            VERSION,
            &[source.assignment.clone()],
            vec![source.clone()],
            window,
            burn,
        )
        .is_err());
    }
    assert!(admit_cpu_collector_shards(
        &trainer,
        &value,
        &inputs,
        config(),
        VERSION,
        &[],
        vec![source.clone()],
        2,
        1,
    )
    .is_err());
    let mut incomplete = source;
    incomplete.assignment.sampling_counter_end += 1;
    assert!(admit(&trainer, &value, &inputs, vec![reseal(incomplete)]).is_err());
}

#[test]
fn admission_does_not_authorize_a_changed_central_state_or_config() {
    let (mut trainer, mut value, inputs) = fixture();
    let source = shard(&trainer, &value, &inputs, "a", 42, false);
    let before_value = value.checkpoint();
    let original = trainer.checkpoint();
    let mut changed = original.clone();
    changed.first_moment[0] += 0.001;
    trainer.restore_checkpoint(changed.clone()).unwrap();
    assert!(admit(&trainer, &value, &inputs, vec![source.clone()]).is_err());
    trainer.restore_checkpoint(original).unwrap();
    let cohort = admit(&trainer, &value, &inputs, vec![source.clone()]).unwrap();
    trainer.restore_checkpoint(changed.clone()).unwrap();
    assert!(cohort
        .train(&mut trainer, &mut value, &inputs, config(), 2)
        .is_err());
    assert_eq!(trainer.checkpoint(), changed);
    assert_eq!(value.checkpoint(), before_value);

    let (mut trainer, mut value, inputs) = fixture();
    let cohort = admit(&trainer, &value, &inputs, vec![source.clone()]).unwrap();
    let before = trainer.checkpoint();
    let mut changed_config = config();
    changed_config.temperature = 1.1;
    assert!(cohort
        .train(&mut trainer, &mut value, &inputs, changed_config, 2)
        .is_err());
    assert_eq!(trainer.checkpoint(), before);

    let cohort = admit(&trainer, &value, &inputs, vec![source.clone()]).unwrap();
    let mut changed_value = value.checkpoint();
    let first_moment = changed_value.feature_count as usize + 1;
    changed_value.parameters[first_moment] += 0.001;
    value = CpuValueHead::from_checkpoint(changed_value.clone()).unwrap();
    assert!(admit(&trainer, &value, &inputs, vec![source]).is_err());
    assert!(cohort
        .train(&mut trainer, &mut value, &inputs, config(), 2)
        .is_err());
    assert_eq!(value.checkpoint(), changed_value);
    assert_eq!(trainer.checkpoint(), before);
}
