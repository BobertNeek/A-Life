//! CPU host-authority/retained-history contracts. These do not execute a GPU.
use super::*;
use alife_core::{ActionProposal, BrainTickInput, CreatureMind};
use alife_world::{HeadlessBrainHarness, HeadlessScenarioBuilder};

fn fixture(owners: &[u64]) -> (HeadlessWorld, BTreeMap<u64, ResidentCognition>) {
    let mut builder = HeadlessScenarioBuilder::new(83017);
    for &owner in owners {
        builder = builder.agent(&format!("owner-{owner}"), OrganismId(owner), Vec3f::ZERO);
    }
    let mut world = builder.build().unwrap();
    let mut residents = BTreeMap::new();
    for &owner in owners {
        let id = OrganismId(owner);
        super::tests::register_sealing_test_organism(&mut world, id);
        let record = world.organism_registry().get(id).unwrap();
        let plan = resident_authority_plan_from_record(
            record,
            id,
            record.world_entity_id(),
            world.tick(),
            BrainScaleTier::Nano512,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        residents.insert(owner, plan.into_fresh_resident().unwrap());
    }
    (world, residents)
}

fn patches(owner: u64) -> Vec<ExperiencePatch> {
    let id = OrganismId(owner);
    let world = HeadlessScenarioBuilder::new(83017)
        .agent("owner", id, Vec3f::ZERO)
        .build()
        .unwrap();
    let mut harness = HeadlessBrainHarness::new(world);
    let mut mind = CreatureMind::scaffold(id, BrainScaleTier::Nano512, 42, Tick::ZERO).unwrap();
    (0..3)
        .map(|_| {
            let rest = ActionProposal::new(
                alife_core::ActionKind::Rest.canonical_id(),
                alife_core::ActionKind::Rest,
                0.9,
                Confidence::new(1.0).unwrap(),
                None,
                0,
                alife_core::ActionTarget::NONE,
                NormalizedScalar::ZERO,
            )
            .unwrap();
            let tick = harness.world().tick();
            let result = harness.tick_mind(&mut mind, BrainTickInput::new(tick, vec![rest]));
            let patch = result.brain.experience_patch.unwrap();
            patch.validate_contract().unwrap();
            patch
        })
        .collect()
}

fn reference_retention(
    original: &[ExperiencePatch],
    additions: &[ExperiencePatch],
    residents: &BTreeMap<u64, ResidentCognition>,
) -> Vec<ExperiencePatch> {
    // Frozen pre-optimization policy: concatenate, retain the latest capacity
    // for each live organism, then preserve the original cross-organism order.
    let mut counts = BTreeMap::<u64, usize>::new();
    let mut rows: Vec<_> = original
        .iter()
        .chain(additions)
        .rev()
        .filter(|patch| {
            let raw = patch.header().organism_id.raw();
            let Some(resident) = residents.get(&raw) else {
                return false;
            };
            let count = counts.entry(raw).or_default();
            *count += 1;
            *count <= resident.phenotype.replay_capture_plan().event_capacity() as usize
        })
        .cloned()
        .collect();
    rows.reverse();
    rows
}

#[test]
fn replay_rollback_single_owner_append_eviction_and_retry_fill_restore_exact_history() {
    let (_, residents) = fixture(&[1]);
    let samples = patches(1);
    let capacity = residents[&1]
        .phenotype
        .replay_capture_plan()
        .event_capacity() as usize;
    assert!(capacity > 1);
    for original_len in [0, 1, capacity - 1, capacity] {
        for additions in [1, 2, capacity + 2] {
            let original: Vec<_> = (0..original_len).map(|i| samples[i % 3].clone()).collect();
            let mut actual = original.clone();
            let mut expected = original.clone();
            let mut rollback = StagedSealedPatchesRollback::Clean { original_len };
            for i in 0..additions {
                let next = std::slice::from_ref(&samples[(i + 1) % 3]);
                expected = reference_retention(&expected, next, &residents);
                retain_bounded_replay_patches(&residents, &mut actual, &mut rollback, next);
                assert_eq!(actual, expected);
            }
            restore_staged_sealed_patches(&mut rollback, &mut actual, original_len);
            assert_eq!(
                actual, original,
                "initial {original_len}, appended {additions}, capacity {capacity}"
            );
            assert!(matches!(rollback, StagedSealedPatchesRollback::Inactive));
        }
    }
}

#[test]
fn replay_rollback_multiple_owners_retirement_and_second_eviction_preserve_original_order() {
    let (_, mut residents) = fixture(&[1, 2]);
    let one = patches(1);
    let two = patches(2);
    let capacity = residents[&1]
        .phenotype
        .replay_capture_plan()
        .event_capacity() as usize;
    let original: Vec<_> = (0..capacity)
        .flat_map(|i| [one[i % 3].clone(), two[i % 3].clone()])
        .collect();
    let mut actual = original.clone();
    let mut rollback = StagedSealedPatchesRollback::Clean {
        original_len: original.len(),
    };
    let additions = [one[2].clone(), two[1].clone()];
    let expected = reference_retention(&actual, &additions, &residents);
    retain_bounded_replay_patches(&residents, &mut actual, &mut rollback, &additions);
    assert_eq!(actual, expected);
    assert!(matches!(
        rollback,
        StagedSealedPatchesRollback::FullSnapshot(_)
    ));
    residents.remove(&2);
    let expected = reference_retention(&actual, &[], &residents);
    retain_bounded_replay_patches(&residents, &mut actual, &mut rollback, &[]);
    assert_eq!(actual, expected);
    restore_staged_sealed_patches(&mut rollback, &mut actual, original.len());
    assert_eq!(actual, original);

    let original: Vec<_> = (0..capacity).map(|i| one[i % 3].clone()).collect();
    let mut actual = original.clone();
    let mut rollback = StagedSealedPatchesRollback::Clean {
        original_len: original.len(),
    };
    retain_bounded_replay_patches(&residents, &mut actual, &mut rollback, &one[..1]);
    assert!(matches!(
        rollback,
        StagedSealedPatchesRollback::EvictedFrontOne { .. }
    ));
    retain_bounded_replay_patches(&residents, &mut actual, &mut rollback, &one[1..2]);
    assert!(matches!(
        rollback,
        StagedSealedPatchesRollback::FullSnapshot(_)
    ));
    residents.clear();
    retain_bounded_replay_patches(&residents, &mut actual, &mut rollback, &[]);
    assert!(actual.is_empty());
    restore_staged_sealed_patches(&mut rollback, &mut actual, original.len());
    assert_eq!(actual, original);
}

#[test]
fn replay_rollback_arc_authority_clone_serde_and_failed_tick_preserve_values() {
    struct Owner {
        world: HeadlessWorld,
        residents: BTreeMap<u64, ResidentCognition>,
    }
    impl LiveAuthorityOwner for Owner {
        fn world_and_residents(
            &mut self,
        ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>) {
            (&mut self.world, &mut self.residents)
        }
    }
    let (world, residents) = fixture(&[1]);
    let mut owner = Owner { world, residents };
    let before = owner.residents[&1].clone();
    let wire = serde_json::to_value(&before).unwrap();
    assert_eq!(
        wire["phenotype"],
        serde_json::to_value(before.phenotype.as_ref()).unwrap()
    );
    assert_eq!(
        wire["compiler_inputs"],
        serde_json::to_value(before.compiler_inputs.as_ref()).unwrap()
    );
    let restored: PhenotypeCompilerInputs =
        serde_json::from_value(wire["compiler_inputs"].clone()).unwrap();
    assert_eq!(restored, *before.compiler_inputs);
    assert!(std::sync::Arc::ptr_eq(
        &before.phenotype,
        &owner.residents[&1].phenotype
    ));
    assert!(std::sync::Arc::ptr_eq(
        &before.compiler_inputs,
        &owner.residents[&1].compiler_inputs
    ));
    let world_before = owner.world.canonical_signature_digest().unwrap();
    let (result, _) = tick_with_sleep_progress_inner(&mut owner, false, |owner| {
        owner.world.try_advance_tick().unwrap();
        owner.residents.get_mut(&1).unwrap().next_sequence += 1;
        Err::<(), _>(ScaffoldContractError::InvalidId)
    });
    assert!(result.is_err());
    assert_eq!(
        owner.world.canonical_signature_digest().unwrap(),
        world_before
    );
    assert_eq!(serde_json::to_value(&owner.residents[&1]).unwrap(), wire);
    assert!(std::sync::Arc::ptr_eq(
        &before.phenotype,
        &owner.residents[&1].phenotype
    ));
    assert!(std::sync::Arc::ptr_eq(
        &before.compiler_inputs,
        &owner.residents[&1].compiler_inputs
    ));
    let (result, _) = tick_with_sleep_progress_inner(&mut owner, false, |owner| {
        owner.world.try_advance_tick().unwrap();
        owner.residents.get_mut(&1).unwrap().next_sequence += 1;
        Ok::<_, ScaffoldContractError>(())
    });
    assert!(result.is_ok());
    assert_eq!(owner.residents[&1].next_sequence, before.next_sequence + 1);
    assert_ne!(
        owner.world.canonical_signature_digest().unwrap(),
        world_before
    );
}

#[test]
fn staged_predictor_undo_restores_awake_observations_and_snapshots_across_rollback() {
    use alife_core::{
        ActionId, ChannelCommand, DurationTicks, Intensity, JointMotorCondition, MotorChannel,
        MotorCommandBundle, PredictionTargetReceipt, SemanticStateVector, Vec3f,
    };

    struct UndoOwner {
        world: HeadlessWorld,
        residents: BTreeMap<u64, ResidentCognition>,
        predictor_undos: Option<Vec<StagedPredictorUndo>>,
    }
    impl LiveAuthorityOwner for UndoOwner {
        fn world_and_residents(
            &mut self,
        ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>) {
            (&mut self.world, &mut self.residents)
        }

        fn begin_staged_predictor_undo(&mut self) -> bool {
            self.predictor_undos = Some(Vec::new());
            true
        }

        fn finish_staged_predictor_undo(&mut self, committed: bool) -> Vec<StagedPredictorUndo> {
            let undos = self.predictor_undos.take().unwrap_or_default();
            if committed {
                Vec::new()
            } else {
                undos
            }
        }
    }

    let (world, residents) = fixture(&[1]);
    let mut owner = UndoOwner {
        world,
        residents,
        predictor_undos: None,
    };
    let wire_before = serde_json::to_value(&owner.residents).unwrap();
    let world_before = owner.world.canonical_signature_digest().unwrap();
    let genome_arc_before = std::sync::Arc::clone(&owner.residents[&1].genome);

    let make_target = |seq: u64, tick: u64, delta_energy: f32| {
        let mut source_values = [0.0_f32; 13];
        source_values[0] = 0.5;
        let mut target_values = [0.0_f32; 13];
        target_values[0] = (0.5 + delta_energy).clamp(0.0, 1.0);
        let source = SemanticStateVector::new(source_values.to_vec()).unwrap();
        let target = SemanticStateVector::new(target_values.to_vec()).unwrap();
        let cmd = ChannelCommand::new(
            MotorChannel::Locomotion,
            ActionId(1),
            None,
            Vec3f::new(0.0, 0.0, 1.0),
            Intensity::new(0.5).unwrap(),
            DurationTicks(1),
            0.0,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap();
        let bundle = MotorCommandBundle::new(
            OrganismId(1),
            ExperienceSequenceId(seq),
            Tick(tick),
            vec![cmd],
        )
        .unwrap();
        let motor = JointMotorCondition::from_bundle(&bundle).unwrap();
        PredictionTargetReceipt::for_successor(
            OrganismId(1),
            ExperienceSequenceId(seq),
            ActionId(1),
            Tick(tick),
            [seq; 4],
            source,
            motor,
            target,
        )
        .unwrap()
    };

    let (failed, _) = tick_with_sleep_progress_inner(&mut owner, false, |owner| {
        owner.world.try_advance_tick().unwrap();
        let target = make_target(1, 0, 0.25);
        let (_, undo) = owner
            .residents
            .get_mut(&1)
            .unwrap()
            .predictor
            .observe_with_undo(&target)
            .unwrap();
        owner
            .predictor_undos
            .as_mut()
            .unwrap()
            .push(StagedPredictorUndo::Observe(1, undo));
        owner.residents.get_mut(&1).unwrap().next_sequence += 1;
        Err::<(), _>(ScaffoldContractError::InvalidId)
    });
    assert!(failed.is_err());
    assert!(owner.predictor_undos.is_none());
    assert_eq!(
        owner.world.canonical_signature_digest().unwrap(),
        world_before
    );
    assert_eq!(serde_json::to_value(&owner.residents).unwrap(), wire_before);
    assert!(std::sync::Arc::ptr_eq(
        &owner.residents[&1].genome,
        &genome_arc_before
    ));

    let (ok, _) = tick_with_sleep_progress_inner(&mut owner, false, |owner| {
        owner.world.try_advance_tick().unwrap();
        let target = make_target(1, 0, 0.25);
        let (_, undo) = owner
            .residents
            .get_mut(&1)
            .unwrap()
            .predictor
            .observe_with_undo(&target)
            .unwrap();
        owner
            .predictor_undos
            .as_mut()
            .unwrap()
            .push(StagedPredictorUndo::Observe(1, undo));
        Ok::<_, ScaffoldContractError>(())
    });
    ok.unwrap();
    assert!(owner.predictor_undos.is_none());
    assert_ne!(serde_json::to_value(&owner.residents).unwrap(), wire_before);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn pipelined_next_tick_preparation_matches_unpipelined_and_invalidates_on_mutation() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut pipelined =
        super::recovery_sleep_tests::fixture_with_population(&format!("pipelined-a-{nonce}"), 2);
    let mut unpipelined =
        super::recovery_sleep_tests::fixture_with_population(&format!("pipelined-b-{nonce}"), 2);
    assert!(pipelined.has_pipelined_next_prep_for_test());
    unpipelined.invalidate_pipelined_next_prep();
    assert!(!unpipelined.has_pipelined_next_prep_for_test());

    let assert_runtime_parity = |a: &GpuLiveBrainRuntime, b: &GpuLiveBrainRuntime, label: &str| {
        assert_eq!(a.world.tick(), b.world.tick(), "{label}: tick");
        assert_eq!(
            a.world.object_snapshots(),
            b.world.object_snapshots(),
            "{label}: objects"
        );
        assert_eq!(
            a.last_sealed_patches, b.last_sealed_patches,
            "{label}: sealed patches"
        );
        assert_eq!(a.memories, b.memories, "{label}: memories");
        assert_eq!(a.topologies, b.topologies, "{label}: topologies");
        assert_eq!(
            serde_json::to_value(&a.residents).unwrap(),
            serde_json::to_value(&b.residents).unwrap(),
            "{label}: residents"
        );
        for raw in [1_u64, 2_u64] {
            let org = OrganismId(raw);
            assert_eq!(
                a.world.tracked_objects().save_state(org).unwrap(),
                b.world.tracked_objects().save_state(org).unwrap(),
                "{label}: tracked_objects for {raw}"
            );
            let rec_a = a.world.organism_registry().get(org).unwrap();
            let rec_b = b.world.organism_registry().get(org).unwrap();
            assert_eq!(
                rec_a.biochemistry(),
                rec_b.biochemistry(),
                "{label}: bio {raw}"
            );
            assert_eq!(rec_a.embodiment(), rec_b.embodiment(), "{label}: emb {raw}");
            assert_eq!(
                rec_a.state_graph().body_biochemistry,
                rec_b.state_graph().body_biochemistry,
                "{label}: body state_graph {raw}"
            );
            assert_eq!(
                rec_a.state_graph().memory,
                rec_b.state_graph().memory,
                "{label}: memory state_graph {raw}"
            );
        }
    };

    let replay_pressures_from = |src: &GpuLiveBrainRuntime, dst: &mut GpuLiveBrainRuntime| {
        let pressures = [1_u64, 2_u64]
            .into_iter()
            .map(|raw| {
                src.backend
                    .snapshot_activity_state(src.handles[&raw])
                    .unwrap()
                    .pressure
                    .unwrap()
            })
            .collect::<Vec<_>>();
        dst.backend
            .install_recorded_pressure_replay(pressures)
            .unwrap();
    };

    for step in 0..4 {
        let sum_a = pipelined.tick().unwrap();
        replay_pressures_from(&pipelined, &mut unpipelined);
        let sum_b = unpipelined.tick().unwrap();
        assert_eq!(sum_a, sum_b, "mismatch at step {step}");
        assert_runtime_parity(&pipelined, &unpipelined, &format!("step {step}"));
        assert!(
            pipelined.has_pipelined_next_prep_for_test(),
            "expected pipelined prep after awake step {step}"
        );
        // Force unpipelined reference to recompute from scratch on the next tick.
        unpipelined.invalidate_pipelined_next_prep();
        assert!(!unpipelined.has_pipelined_next_prep_for_test());
    }

    // External mutations and rollback must bump world_mutation_epoch and drop pipelined prep.
    let epoch0 = pipelined.world_mutation_epoch_for_test();
    assert!(pipelined.has_pipelined_next_prep_for_test());
    pipelined
        .place_player_food(Vec3f::new(1.25, 0.0, 1.25))
        .unwrap();
    assert!(!pipelined.has_pipelined_next_prep_for_test());
    assert_eq!(pipelined.world_mutation_epoch_for_test(), epoch0 + 1);
    unpipelined
        .place_player_food(Vec3f::new(1.25, 0.0, 1.25))
        .unwrap();

    let sum_a = pipelined.tick().unwrap();
    replay_pressures_from(&pipelined, &mut unpipelined);
    let sum_b = unpipelined.tick().unwrap();
    assert_eq!(sum_a, sum_b);
    assert_runtime_parity(&pipelined, &unpipelined, "post-food");

    assert!(pipelined.has_pipelined_next_prep_for_test());
    let epoch1 = pipelined.world_mutation_epoch_for_test();
    let _ = pipelined.world_mut();
    assert!(!pipelined.has_pipelined_next_prep_for_test());
    assert_eq!(pipelined.world_mutation_epoch_for_test(), epoch1 + 1);

    assert!(unpipelined.has_pipelined_next_prep_for_test());
    let epoch_sleep = unpipelined.world_mutation_epoch_for_test();
    unpipelined.request_recovery_sleep(OrganismId(1)).unwrap();
    assert!(!unpipelined.has_pipelined_next_prep_for_test());
    assert_eq!(unpipelined.world_mutation_epoch_for_test(), epoch_sleep + 1);

    pipelined.tick().unwrap();
    assert!(pipelined.has_pipelined_next_prep_for_test());
    let epoch2 = pipelined.world_mutation_epoch_for_test();
    pipelined.force_late_advance_failure_for_test();
    assert!(pipelined.tick().is_err());
    assert!(!pipelined.has_pipelined_next_prep_for_test());
    assert_eq!(pipelined.world_mutation_epoch_for_test(), epoch2 + 1);
}
