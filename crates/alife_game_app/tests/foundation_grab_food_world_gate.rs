//! CPU scaffold/world + curriculum-gate regression, not on-policy GPU admission.
#![cfg(feature = "foundation-training")]

use alife_core::{
    ActionKind, ActionProposal, ActionTarget, BrainScaleTier, BrainTickInput, BrainTickStatus,
    Confidence, CreatureMind, EvidenceKind, ExperiencePatchPhase, NormalizedScalar, OrganismId,
    PhysicalContactKind, ScaffoldContractError, Tick, Validate, Vec3f,
};
use alife_game_app::FoundationTrainingStep;
use alife_world::{HeadlessActionIds, HeadlessBrainHarness, HeadlessScenarioBuilder};

// Exercise the unchanged private production gate without exporting test APIs or
// changing source-bound training/provider files. Its GPU-only observer is unused.
#[allow(dead_code)]
#[path = "../src/foundation_grab_food.rs"]
mod foundation_grab_food;

use foundation_grab_food::{GrabRewardGate, GRAB_FOOD_ACQUISITION_REWARD};

#[test]
fn sealed_cpu_world_acquire_release_reacquire_receipts_get_one_gate_credit() {
    let organism = OrganismId(17);
    let world = HeadlessScenarioBuilder::new(202_610_10)
        .agent("agent", organism, Vec3f::ZERO)
        .food("berry", Vec3f::new(0.5, 0.0, 0.0), 0.6)
        .build()
        .unwrap();
    let target = world.entity_id("berry").unwrap();
    let mut harness = HeadlessBrainHarness::new(world);
    let mut mind =
        CreatureMind::scaffold(organism, BrainScaleTier::Nano512, 99, Tick::ZERO).unwrap();
    let mut gate = GrabRewardGate::default();
    let mut acquisitions = Vec::new();

    for (row, expected_owner) in [Some(organism), None, Some(organism)]
        .into_iter()
        .enumerate()
    {
        let owner_before = harness.world().entity(target).unwrap().carried_by;
        // The real world Grab primitive releases an already-held target; there
        // is no separate Drop action ID. No editor mutation simulates release.
        let proposal = ActionProposal::new(
            HeadlessActionIds::GRAB,
            ActionKind::Hold,
            0.9,
            Confidence::new(0.9).unwrap(),
            None,
            0b11,
            ActionTarget::new(Some(target), None),
            NormalizedScalar::new(0.8).unwrap(),
        )
        .unwrap();
        let result = harness.tick_mind(
            &mut mind,
            BrainTickInput::new(Tick::new(row as u64), vec![proposal]).with_pack_experience(true),
        );
        assert_eq!(result.brain.status, BrainTickStatus::Normal);
        let action = result.action_result.unwrap();
        assert!(action.execution.succeeded);
        assert_eq!(action.command.organism_id, organism);
        assert_eq!(action.command.action_id, HeadlessActionIds::GRAB);
        assert_eq!(action.command.target_entity, Some(target));
        assert_eq!(
            action.execution.physical.contact,
            PhysicalContactKind::Touch
        );
        assert_eq!(action.execution.physical.target_entity, Some(target));
        assert_eq!(action.body_event.nutrition, 0.0);
        assert!(action.body_event.energy < 0.0);

        let patch = result.brain.experience_patch.unwrap();
        patch.validate_contract().unwrap();
        assert_eq!(patch.header().phase, ExperiencePatchPhase::Sealed);
        assert_eq!(patch.header().organism_id, organism);
        assert_eq!(patch.header().world_tick, Tick::new(row as u64));
        assert_eq!(patch.decision().selected_action, action.command);
        assert_eq!(patch.outcome().physical, action.execution.physical);
        assert_eq!(patch.outcome().homeostatic_delta.drives.hunger, 0.0);
        assert!(patch.outcome().energy_delta.raw() < 0.0);
        // An honestly sealed CPU scaffold patch cannot satisfy the production
        // acquisition observer's neural-evidence requirement. No GPU receipt or
        // on-policy training step is fabricated for this narrower gate test.
        assert_eq!(
            patch.decision().evidence_kind(),
            EvidenceKind::HeuristicBaseline
        );
        assert_eq!(
            patch.decision().neural_evidence().unwrap_err(),
            ScaffoldContractError::EvidenceKindMismatch
        );

        let world = harness.world();
        let food = world.entity(target).unwrap();
        assert_eq!(food.carried_by, expected_owner);
        assert!(!food.consumed);
        assert_eq!(food.nutrition, 0.6);
        if owner_before.is_none() && food.carried_by == Some(organism) {
            acquisitions.push(row);
            gate.observe(row);
        }
    }

    assert_eq!(acquisitions, vec![0, 2]);
    assert_eq!(harness.telemetry().sealed_patches.len(), 3);
    assert_eq!(harness.telemetry().packed_records.len(), 3);
    assert_eq!(harness.world().tick(), Tick::new(3));
    assert_eq!(gate.reward(0, 3), GRAB_FOOD_ACQUISITION_REWARD);
    assert_eq!(gate.reward(1, 3), 0.0);
    assert_eq!(gate.reward(2, 3), 0.0);
    assert_eq!((0..3).map(|row| gate.reward(row, 3)).sum::<f32>(), 0.25);
}
