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
