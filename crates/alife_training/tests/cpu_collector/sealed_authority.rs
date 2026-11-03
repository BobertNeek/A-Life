use super::*;

#[test]
fn changed_world_brain_or_memory_seal_rejects_before_preparation_and_keeps_state() {
    let (mut host, trainer) = fixture();
    for completed_ticks in 0..3 {
        if completed_ticks > 0 {
            host.step(&trainer, 0.9).unwrap();
        }
        for change_brain in [false, true] {
            let mut tampered = host.snapshot();
            let mut record = tampered
                .world()
                .organism_registry()
                .iter()
                .next()
                .unwrap()
                .clone();
            let mut brain = record.state_graph().brain.content_digest;
            let mut memory = record.state_graph().memory.content_digest;
            if change_brain {
                brain[0] ^= 1;
            } else {
                memory[0] ^= 1;
            }
            record
                .seal_cognitive_subsystems(record.biochemistry().tick, brain, memory)
                .unwrap();
            tampered
                .world_mut()
                .replace_organism_record_exact(record)
                .unwrap();
            let before = serde_json::to_vec(&tampered.checkpoint(&trainer).unwrap()).unwrap();
            let mut prepared = false;
            assert!(tampered
                .step_with_preparation(&trainer, 0.9, |draft, _| {
                    prepared = true;
                    Ok(draft)
                })
                .is_err());
            assert!(!prepared);
            assert_eq!(
                before,
                serde_json::to_vec(&tampered.checkpoint(&trainer).unwrap()).unwrap()
            );
            assert!(CpuCanonicalHost::from_checkpoint(
                tampered.checkpoint(&trainer).unwrap(),
                &trainer
            )
            .is_err());
        }
    }
}

#[test]
fn failed_preparation_and_snapshot_continue_like_cold_restore() {
    let (mut host, trainer) = fixture();
    host.step(&trainer, 0.9).unwrap();
    let checkpoint = host.checkpoint(&trainer).unwrap();
    let before = serde_json::to_vec(&checkpoint).unwrap();
    let snapshot = host.snapshot();
    let mut cold = CpuCanonicalHost::from_checkpoint(checkpoint, &trainer).unwrap();
    let actor = trainer.checkpoint();
    let mut prepared = false;
    assert!(host
        .step_with_preparation(&trainer, 0.9, |_, _| {
            prepared = true;
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        })
        .is_err());
    assert!(prepared);
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    let actual = host.step(&trainer, 0.9).unwrap();
    let expected = cold.step(&trainer, 0.9).unwrap();
    assert_eq!(
        serde_json::to_vec(&actual.decisions).unwrap(),
        serde_json::to_vec(&expected.decisions).unwrap()
    );
    host.restore_snapshot(snapshot, &trainer).unwrap();
    assert_eq!(
        serde_json::to_vec(&actual.decisions).unwrap(),
        serde_json::to_vec(&host.step(&trainer, 0.9).unwrap().decisions).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
    );
    assert_eq!(trainer.checkpoint(), actor);
}

#[test]
fn sleeping_resident_seal_follows_awake_peers_sampling_counter() {
    let (single, trainer) = new_game_fixture(None);
    let source = single.world.organism_registry().iter().next().unwrap();
    let asset = source.genome().n2048_foundation_candidate.as_ref().unwrap();
    let mut config = CanonicalNewGameConfig::phase3(0x7A1123, 2).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
    let mut game = create_canonical_new_game_with_n2048_candidate(&config, asset).unwrap();
    // A homogeneous training group shares its founder graph while each resident
    // keeps its own organism/entity identity and private lifetime state.
    let identities = game.world.organism_entity_ids();
    for (i, &(id, entity)) in identities.iter().enumerate() {
        let record = WorldOrganismRecord::newborn(
            id,
            entity,
            source.genome().clone(),
            source.phenotype().clone(),
            Tick::ZERO,
        )
        .unwrap();
        game.world.replace_organism_record_exact(record).unwrap();
        game.creatures[i] = single.creatures[0].clone();
        game.creatures[i].organism_id = id;
    }
    let mut host = CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap();
    assert_eq!(host.step(&trainer, 0.9).unwrap().decisions.len(), 2);
    let sleeping = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    host.request_recovery_sleep(sleeping).unwrap();
    let mut counter = host.checkpoint(&trainer).unwrap().sampling_counter;
    for _ in 0..4 {
        let mut cold =
            CpuCanonicalHost::from_checkpoint(host.checkpoint(&trainer).unwrap(), &trainer)
                .unwrap();
        let actual = host.step(&trainer, 0.9).unwrap();
        let expected = cold.step(&trainer, 0.9).unwrap();
        assert!(actual.sleeping_organisms > 0);
        assert!(actual.decisions.iter().any(|d| d.organism_id != sleeping));
        assert_eq!(
            serde_json::to_vec(&actual.decisions).unwrap(),
            serde_json::to_vec(&expected.decisions).unwrap()
        );
        let checkpoint = host.checkpoint(&trainer).unwrap();
        assert!(checkpoint.sampling_counter > counter);
        counter = checkpoint.sampling_counter;
        assert_eq!(
            serde_json::to_vec(&checkpoint).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
    }
}
