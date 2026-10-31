use super::*;
use alife_core::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity};

#[test]
fn cached_population_proof_rejects_an_unrelated_corrupt_peer_before_staged_action_mutation() {
    for fault in 0..4 {
        let mut world = HeadlessScenarioBuilder::new(81_050)
            .agent("actor", OrganismId(1), Vec3f::ZERO)
            .agent("peer", OrganismId(2), Vec3f::new(4.0, 0.0, 0.0))
            .food("food", Vec3f::new(2.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        for (id, label) in [(1, "actor"), (2, "peer")] {
            let genome = CreatureGenome::early_mammal_founder(
                81_050 + id,
                FoundationGeneticIdentity::new(100 + id, 1, 7, BrainCapacityClass::N512_ID)
                    .unwrap(),
            )
            .unwrap();
            world
                .register_organism_record(
                    WorldOrganismRecord::newborn(
                        OrganismId(id),
                        world.entity_id(label).unwrap(),
                        genome.clone(),
                        genome.express().unwrap(),
                        Tick::ZERO,
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        // Populate every proof before corrupting the other organism.
        world.validate_organism_bindings().unwrap();
        let actor = world.entity_id("actor").unwrap();
        let peer = world.entity_id("peer").unwrap();
        let food = world.entity_id("food").unwrap();
        let bundle = MotorCommandBundle::new(
            OrganismId(1),
            alife_core::ExperienceSequenceId::new(1).unwrap(),
            Tick::ZERO,
            vec![ChannelCommand::new(
                MotorChannel::Locomotion,
                HeadlessActionIds::APPROACH,
                Some(alife_core::ActionTarget::new(Some(food), None)),
                Vec3f::new(1.0, 0.0, 0.0),
                Intensity::new(1.0).unwrap(),
                alife_core::DurationTicks::new(1),
                0.0,
                Confidence::new(0.9).unwrap(),
                0,
            )
            .unwrap()],
        )
        .unwrap();
        match fault {
            0 => {
                world
                    .organism_registry
                    .edit_biology_unchecked_for_test(OrganismId(2), |biology| {
                        biology.body.energy = f32::NAN;
                    })
            }
            1 => world.objects.get_mut(&peer.raw()).unwrap().kind = WorldObjectKind::Food,
            2 => world.objects.get_mut(&peer.raw()).unwrap().organism_id = Some(OrganismId(3)),
            _ => {
                world.objects.remove(&peer.raw());
            }
        }
        let expected_error = world.validate_organism_bindings().unwrap_err();
        let before_objects = world.object_snapshots();
        let before_actor =
            serde_json::to_vec(world.organism_registry.get(OrganismId(1)).unwrap()).unwrap();
        let before_touched = world.last_touched_entities.clone();
        let neural = NeuralEmissionFrame::new(Tick::ZERO, 1, Vec::new()).unwrap();
        // This entry deliberately has no local rollback. Rejection must precede
        // movement, biology, outcome publication and other world mutation.
        assert_eq!(
            world.apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
                &bundle,
                actor,
                &neural,
                0.0,
                &[],
            ),
            Err(HeadlessMotorTransactionError::Contract(expected_error)),
            "fault {fault}",
        );
        assert_eq!(world.object_snapshots(), before_objects);
        assert_eq!(world.tick(), Tick::ZERO);
        assert_eq!(world.last_touched_entities, before_touched);
        assert!(world.last_action_result.is_none());
        assert_eq!(
            serde_json::to_vec(world.organism_registry.get(OrganismId(1)).unwrap()).unwrap(),
            before_actor
        );
    }
}
