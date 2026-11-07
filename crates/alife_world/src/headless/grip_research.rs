//! Opt-in physical grip operations. No scoring, memory or desirability lives here.
//!
//! The new profile alone enumerates these operations; all command dispatch paths
//! independently enforce their physical semantics. Observations are captured
//! from already grounded V1 slots; their 24 lanes and all persisted world
//! formats remain unchanged.
use super::*;
use alife_core::grip_research::{
    GripObservationV1, GripOperationV1, GripOwnershipV1, GroundedGripSlotV1,
};
use alife_core::CandidateObservationRef;

impl HeadlessActionIds {
    pub const RESEARCH_CLOSE_GRIP: ActionId = GripOperationV1::Close.action_id();
    pub const RESEARCH_RELEASE_GRIP: ActionId = GripOperationV1::Release.action_id();
}

fn action_id(operation: GripOperationV1) -> ActionId {
    match operation {
        GripOperationV1::Close => HeadlessActionIds::RESEARCH_CLOSE_GRIP,
        GripOperationV1::Release => HeadlessActionIds::RESEARCH_RELEASE_GRIP,
    }
}

impl HeadlessWorld {
    pub(super) fn has_grip_reader_abi(&self, organism: OrganismId) -> bool {
        self.organism_registry
            .get(organism)
            .and_then(|record| record.genome().n2048_foundation_candidate.as_ref())
            .is_some_and(|asset| {
                asset.is_n2048_grip_reader_candidate()
                    && asset.manifest().sensor_profile() == SensorProfile::GroundedGripObjectSlotsV1
            })
    }

    fn research_grip_ownership(
        &self,
        organism: OrganismId,
        target: WorldEntityId,
    ) -> GripOwnershipV1 {
        // Proprioception identifies only this body's own constraint. It does
        // not reveal another organism's or the player's hidden ownership.
        if self.player_held_object() != Some(target)
            && self.objects[&target.raw()].carried_by == Some(organism)
        {
            GripOwnershipV1::SelfHeld
        } else {
            GripOwnershipV1::NotSelfHeld
        }
    }

    pub(super) fn research_grip_slots(
        &self,
        organism: OrganismId,
        grounded: &crate::GroundedSensingFrame,
    ) -> Result<Vec<GroundedGripSlotV1>, ScaffoldContractError> {
        let gain = self
            .organism_registry
            .get(organism)
            .map_or(1.0, |record| record.embodiment().proprioceptive_gain());
        grounded
            .transports()
            .iter()
            .map(|transport| {
                Ok(GroundedGripSlotV1 {
                    slot_index: transport.slot_index,
                    ownership: self.research_grip_ownership(organism, transport.transport_entity),
                    confidence: Confidence::new(gain.clamp(0.0, 1.0))?,
                })
            })
            .collect()
    }

    /// Read self-constraint presence for an object in an exact, current grounded
    /// draft. Recapture rejects stale geometry, tracking, transport and frames
    /// assembled outside this world. It deliberately admits no unobserved target.
    /// Later attention/context mutation must create a newly bound research frame.
    pub fn observe_research_grip(
        &mut self,
        draft: &PerceptionFrameDraft,
        target: WorldEntityId,
    ) -> Result<GripObservationV1, ScaffoldContractError> {
        draft.validate_contract()?;
        if draft.tick() != self.tick
            || draft.sensor_profile() != SensorProfile::GroundedGripObjectSlotsV1
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let current = self.perception_frame_draft(
            draft.organism_id(),
            self.tick,
            draft.sensor_profile(),
            *draft.homeostasis(),
        )?;
        if current.base_digest() != draft.base_digest() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let slot_index = draft
            .candidates()
            .iter()
            .find_map(|candidate| {
                if candidate.target.entity != Some(target) {
                    return None;
                }
                match candidate.observation {
                    CandidateObservationRef::ObjectSlot(slot) => Some(slot),
                    CandidateObservationRef::None => None,
                }
            })
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        let slot = draft
            .grounded_object_slots()
            .iter()
            .find(|slot| slot.slot_index == slot_index)
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        self.objects
            .get(&target.raw())
            .filter(|object| !object.consumed)
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        let ownership = self.research_grip_ownership(draft.organism_id(), target);
        let confidence = draft
            .grip_observations()
            .iter()
            .find(|observation| observation.slot_index == slot_index)
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?
            .confidence;
        GripObservationV1::new(
            draft.organism_id(),
            self.tick,
            target,
            slot.tracked_object_id,
            slot_index,
            draft.base_digest(),
            ownership,
            confidence,
        )
    }

    /// Execute one explicitly selected operation after exact-frame validation.
    /// Close never opens a grip; release never closes one, including on retry.
    /// This exact-frame fixture boundary complements ordinary motor dispatch;
    /// registered motor execution supplies normal outcome sealing and credit.
    pub fn execute_research_grip(
        &mut self,
        draft: &PerceptionFrameDraft,
        observation: &GripObservationV1,
        operation: GripOperationV1,
        command: ActionCommand,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        command.validate_contract()?;
        if draft.sensor_profile() != SensorProfile::GroundedGripObjectSlotsV1
            || command.action_id != action_id(operation)
            || command.kind != ActionKind::Interact
            || command.organism_id != observation.organism_id
            || command.target_entity != Some(observation.target)
            || command.target_position.is_some()
            || observation.organism_id != draft.organism_id()
            || observation.tick != draft.tick()
            || observation.base_frame_digest != draft.base_digest()
        {
            return Err(ScaffoldContractError::InvalidActionCandidate);
        }
        let current = self.observe_research_grip(draft, observation.target)?;
        if &current != observation {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        self.execute_explicit_grip(operation, command)
    }

    pub(super) fn execute_explicit_grip(
        &mut self,
        operation: GripOperationV1,
        command: ActionCommand,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        command.validate_contract()?;
        if command.action_id != operation.action_id() || command.kind != ActionKind::Interact {
            return Err(ScaffoldContractError::InvalidActionCandidate);
        }
        let target = match self.require_target(&command) {
            Ok(target) => target,
            Err(_) => return self.invalid_target(command, command.target_entity),
        };
        let organism = command.organism_id;
        let agent = self.agent_entity_id(organism)?;
        let ownership = self.research_grip_ownership(organism, target);
        let owned = ownership == GripOwnershipV1::SelfHeld;
        let legal = match operation {
            GripOperationV1::Close => {
                self.objects[&target.raw()].carried_by.is_none()
                    && self.player_held_object() != Some(target)
                    && self.grab_target_is_legal(organism, agent, target)
            }
            // Detaching an actual constraint is not a placement search. In
            // particular blocked ground placement must not make release acquire,
            // relocate to a preferred destination, or keep a harmful object.
            GripOperationV1::Release => {
                owned && self.research_release_effector_is_live(organism, agent, target)
            }
        };
        if !legal {
            return self.finish_action(
                command,
                false,
                Some(ReferenceActionFailure::MissingAffordance),
                physical(PhysicalContactKind::None, Some(target), Vec3f::ZERO, 0.06)?,
                OutcomeProfile::missing_affordance(),
                Vec::new(),
            );
        }
        let positions = self.carried_interval_positions(organism, Some(target));
        let mut result = match operation {
            // Ownership was checked immediately above; execute_grab cannot take
            // its toggle-release branch in this exclusive world transaction.
            GripOperationV1::Close => self.execute_grab(command, agent)?,
            GripOperationV1::Release => {
                let previous_position = self.objects[&target.raw()].position;
                // Construct the fallible physical receipt before detaching.
                let mut result = self.finish_action(
                    command,
                    true,
                    None,
                    physical(PhysicalContactKind::Touch, Some(target), Vec3f::ZERO, 0.06)?,
                    OutcomeProfile::grab(),
                    vec![target],
                )?;
                let object = self
                    .objects
                    .get_mut(&target.raw())
                    .expect("validated constrained object exists");
                object.carried_by = None;
                object.activity_hold = false;
                result.grip_transfers.push(GripTransfer {
                    object: target,
                    previous_owner: Some(organism),
                    new_owner: None,
                    previous_position,
                    new_position: object.position,
                    kind: GripTransferKind::Released,
                });
                result
            }
        };
        self.commit_carried_interval_velocity(organism, &positions);
        // Mirror apply_command's ordinary physical diagnostic bookkeeping.
        result.command = command;
        self.last_touched_entities = result.touched_entities.clone();
        self.last_action_result = Some(result.clone());
        Ok(result)
    }

    fn research_release_effector_is_live(
        &self,
        organism: OrganismId,
        agent: WorldEntityId,
        target: WorldEntityId,
    ) -> bool {
        let actor = &self.objects[&agent.raw()];
        let object = &self.objects[&target.raw()];
        !actor.consumed
            && !object.consumed
            && target != agent
            && object.organism_id != Some(organism)
            && object.carried_by == Some(organism)
            && self.player_held_object() != Some(target)
            && matches!(
                object.kind,
                WorldObjectKind::Food
                    | WorldObjectKind::Hazard
                    | WorldObjectKind::Token
                    | WorldObjectKind::Ball
            )
            && self.organism_registry.get(organism).is_none_or(|record| {
                record.lifecycle().is_alive()
                    && record
                        .embodiment()
                        .effector_gain(EffectorCapability::Manipulation)
                        > 0.0
            })
            && object.organism_id.is_none_or(|owner| {
                self.organism_registry
                    .get(owner)
                    .is_none_or(|record| record.lifecycle().is_alive())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ORGANISM: OrganismId = OrganismId(7);

    fn world() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
        let world = HeadlessScenarioBuilder::new(2201)
            .agent("observer", ORGANISM, Vec3f::ZERO)
            .food("a", Vec3f::new(0.5, 0.0, 0.0), 0.4)
            .food("b", Vec3f::new(0.4, 0.0, 0.3), 0.4)
            .build()
            .unwrap();
        let a = world.entity_id("a").unwrap();
        let b = world.entity_id("b").unwrap();
        (world, a, b)
    }

    fn draft(world: &mut HeadlessWorld) -> PerceptionFrameDraft {
        world
            .perception_frame_draft(
                ORGANISM,
                world.tick(),
                SensorProfile::GroundedGripObjectSlotsV1,
                HomeostaticSnapshot::baseline(world.tick()),
            )
            .unwrap()
    }

    fn legacy_draft(world: &mut HeadlessWorld) -> PerceptionFrameDraft {
        world
            .perception_frame_draft(
                ORGANISM,
                world.tick(),
                SensorProfile::GroundedObjectSlotsV1,
                HomeostaticSnapshot::baseline(world.tick()),
            )
            .unwrap()
    }

    fn command(target: WorldEntityId, operation: GripOperationV1) -> ActionCommand {
        HeadlessWorldCommand::structured(
            ORGANISM,
            action_id(operation),
            ActionKind::Interact,
            Some(target),
            None,
        )
        .unwrap()
    }

    fn execute(
        world: &mut HeadlessWorld,
        target: WorldEntityId,
        operation: GripOperationV1,
    ) -> HeadlessActionResult {
        let frame = draft(world);
        let observation = world.observe_research_grip(&frame, target).unwrap();
        world
            .execute_research_grip(&frame, &observation, operation, command(target, operation))
            .unwrap()
    }

    #[test]
    fn close_and_release_are_not_toggle_aliases() {
        let (mut world, a, _) = world();
        assert!(
            execute(&mut world, a, GripOperationV1::Close)
                .execution
                .succeeded
        );
        assert!(
            !execute(&mut world, a, GripOperationV1::Close)
                .execution
                .succeeded
        );
        assert_eq!(world.entity(a).unwrap().carried_by, Some(ORGANISM));
        let released = execute(&mut world, a, GripOperationV1::Release);
        assert!(released.execution.succeeded);
        assert_eq!(released.grip_transfers[0].kind, GripTransferKind::Released);
        assert!(
            !execute(&mut world, a, GripOperationV1::Release)
                .execution
                .succeeded
        );
        assert_eq!(world.entity(a).unwrap().carried_by, None);
    }

    #[test]
    fn held_a_does_not_mark_touching_b_as_held_or_release_it() {
        let (mut world, a, b) = world();
        execute(&mut world, a, GripOperationV1::Close);
        let frame = draft(&mut world);
        let a_observation = world.observe_research_grip(&frame, a).unwrap();
        let b_observation = world.observe_research_grip(&frame, b).unwrap();
        assert_eq!(a_observation.ownership, GripOwnershipV1::SelfHeld);
        assert_eq!(b_observation.ownership, GripOwnershipV1::NotSelfHeld);
        assert!(world
            .execute_research_grip(
                &frame,
                &a_observation,
                GripOperationV1::Release,
                command(b, GripOperationV1::Release)
            )
            .is_err());
        assert_eq!(world.entity(a).unwrap().carried_by, Some(ORGANISM));
        assert_eq!(world.entity(b).unwrap().carried_by, None);
    }

    #[test]
    fn research_stale_release_fails_and_legacy_unknown_id_keeps_original_behavior() {
        let (mut world, a, _) = world();
        execute(&mut world, a, GripOperationV1::Close);
        let frame = draft(&mut world);
        let observation = world.observe_research_grip(&frame, a).unwrap();
        let release = command(a, GripOperationV1::Release);
        world
            .execute_research_grip(&frame, &observation, GripOperationV1::Release, release)
            .unwrap();
        assert!(world
            .execute_research_grip(&frame, &observation, GripOperationV1::Release, release)
            .is_err());
        // No inherited grip ABI: legacy unknown-Interact remains its old toggle.
        assert!(world.apply_command(&release).unwrap().execution.succeeded);
        assert_eq!(world.entity(a).unwrap().carried_by, Some(ORGANISM));
    }

    #[test]
    fn ownership_change_consumption_and_tick_changes_reject_old_binding() {
        for case in 0..3 {
            let (mut world, a, _) = world();
            execute(&mut world, a, GripOperationV1::Close);
            let frame = draft(&mut world);
            let observation = world.observe_research_grip(&frame, a).unwrap();
            match case {
                0 => world.objects.get_mut(&a.raw()).unwrap().carried_by = Some(OrganismId(8)),
                1 => world.objects.get_mut(&a.raw()).unwrap().consumed = true,
                _ => world.tick = Tick::new(world.tick.raw() + 1),
            }
            let before = world.entity(a).unwrap().clone();
            assert!(world
                .execute_research_grip(
                    &frame,
                    &observation,
                    GripOperationV1::Release,
                    command(a, GripOperationV1::Release)
                )
                .is_err());
            assert_eq!(world.entity(a).unwrap(), &before);
        }
    }

    #[test]
    fn player_owned_target_cannot_be_acquired_or_released() {
        let (mut world, a, _) = world();
        world.begin_player_hold(a).unwrap();
        let frame = draft(&mut world);
        let observation = world.observe_research_grip(&frame, a).unwrap();
        assert_eq!(observation.ownership, GripOwnershipV1::NotSelfHeld);
        for operation in [GripOperationV1::Close, GripOperationV1::Release] {
            assert!(
                !world
                    .execute_research_grip(&frame, &observation, operation, command(a, operation))
                    .unwrap()
                    .execution
                    .succeeded
            );
        }
        assert_eq!(world.player_held_object(), Some(a));
    }

    #[test]
    fn release_detaches_in_place_without_searching_for_ground_placement() {
        let (mut world, a, _) = world();
        execute(&mut world, a, GripOperationV1::Close);
        // Deliberate holds never authorize automatic activity placement. Release
        // nevertheless opens this exact physical grip and preserves its pose.
        assert!(!world.entity(a).unwrap().activity_hold);
        let before = world.entity(a).unwrap().position;
        assert!(
            execute(&mut world, a, GripOperationV1::Release)
                .execution
                .succeeded
        );
        assert_eq!(world.entity(a).unwrap().position, before);
        assert_eq!(world.entity(a).unwrap().carried_by, None);
    }

    fn grip_draft(world: &mut HeadlessWorld) -> PerceptionFrameDraft {
        world
            .perception_frame_draft(
                ORGANISM,
                world.tick(),
                SensorProfile::GroundedGripObjectSlotsV1,
                HomeostaticSnapshot::baseline(world.tick()),
            )
            .unwrap()
    }

    #[test]
    fn new_profile_preserves_object_lanes_and_separates_actual_grip_operations() {
        let (mut world, a, b) = world();
        execute(&mut world, a, GripOperationV1::Close);
        let legacy = legacy_draft(&mut world);
        let framed = grip_draft(&mut world);
        assert_eq!(
            legacy.grounded_object_slots(),
            framed.grounded_object_slots()
        );
        assert_eq!(legacy.sensory(), framed.sensory());
        assert_eq!(legacy.body(), framed.body());
        assert!(framed
            .candidates()
            .iter()
            .any(|c| c.target.entity == Some(a)
                && c.action_id == HeadlessActionIds::RESEARCH_RELEASE_GRIP
                && c.family == alife_core::CandidateActionFamily::Other));
        assert!(framed
            .candidates()
            .iter()
            .any(|c| c.target.entity == Some(b)
                && c.action_id == HeadlessActionIds::RESEARCH_CLOSE_GRIP
                && c.family == alife_core::CandidateActionFamily::Contact));
        assert!(!framed
            .candidates()
            .iter()
            .any(|c| c.action_id == HeadlessActionIds::GRAB));
        for candidate in framed.candidates() {
            if let CandidateObservationRef::ObjectSlot(index) = candidate.observation {
                assert_eq!(
                    candidate.features,
                    framed.grounded_object_slots()[usize::from(index)]
                        .candidate_features()
                        .unwrap()
                );
            }
        }
    }

    #[test]
    fn frame_binding_retains_ownership_when_focus_changes() {
        let (mut world, a, b) = world();
        execute(&mut world, a, GripOperationV1::Close);
        let (frame, binding) = world
            .perception_frame_draft_with_bindings(
                ORGANISM,
                world.tick(),
                SensorProfile::GroundedGripObjectSlotsV1,
                HomeostaticSnapshot::baseline(world.tick()),
            )
            .unwrap();
        let b_slot = frame
            .candidates()
            .iter()
            .find_map(|c| {
                if c.target.entity != Some(b) {
                    return None;
                }
                if let CandidateObservationRef::ObjectSlot(i) = c.observation {
                    Some(frame.grounded_object_slots()[usize::from(i)].tracked_object_id)
                } else {
                    None
                }
            })
            .unwrap();
        let focused = binding
            .unwrap()
            .candidates_for_focus(&frame, Some(b_slot))
            .unwrap();
        assert!(focused.iter().any(|c| c.target.entity == Some(a)
            && c.action_id == HeadlessActionIds::RESEARCH_RELEASE_GRIP));
        assert!(!focused.iter().any(|c| c.target.entity == Some(b)
            && c.action_id == HeadlessActionIds::RESEARCH_RELEASE_GRIP));
    }

    #[test]
    fn factorized_adapter_preserves_explicit_operation_ids() {
        let (_, a, _) = world();
        for operation in [GripOperationV1::Close, GripOperationV1::Release] {
            let channel = ChannelCommand::new(
                MotorChannel::Manipulation,
                operation.action_id(),
                Some(alife_core::ActionTarget::new(Some(a), None)),
                Vec3f::ZERO,
                Intensity::new(1.0).unwrap(),
                alife_core::DurationTicks::new(1),
                0.0,
                Confidence::new(1.0).unwrap(),
                0,
            )
            .unwrap();
            let adapted = action_for_motor_channel_with_grip(ORGANISM, &channel, true).unwrap();
            let legacy = legacy_action_for_motor_channel(ORGANISM, &channel).unwrap();
            assert_eq!(legacy.action_id, ActionKind::Interact.canonical_id());
            assert_eq!(adapted.action_id, operation.action_id());
            assert_eq!(adapted.kind, ActionKind::Interact);
        }
    }

    #[test]
    fn held_target_survives_gaze_and_blocked_placement_without_new_appearance() {
        let (mut world, a, _) = world();
        execute(&mut world, a, GripOperationV1::Close);
        let agent = world.agent_entity_id(ORGANISM).unwrap();
        world.objects.get_mut(&agent.raw()).unwrap().head_yaw = std::f32::consts::PI;
        // An enclosing terrain obstacle prevents every placement path. It is a
        // controlled fixture edit, not a life or a new world rule.
        world.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 3,
                    depth: 3,
                    origin_x: -1.0,
                    origin_z: -1.0,
                    spacing: 1.0,
                    heights: vec![0.0; 9],
                    obstacles: vec![[-2.0, -2.0, 2.0, 2.0, -1.0, 2.0]],
                    water_level: None,
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
        );
        let legacy = legacy_draft(&mut world);
        let framed = grip_draft(&mut world);
        assert_eq!(
            legacy.grounded_object_slots(),
            framed.grounded_object_slots()
        );
        let release = framed
            .candidates()
            .iter()
            .find(|c| {
                c.target.entity == Some(a)
                    && c.action_id == HeadlessActionIds::RESEARCH_RELEASE_GRIP
            })
            .unwrap();
        assert_eq!(release.family, alife_core::CandidateActionFamily::Other);
        let before = world.entity(a).unwrap().position;
        assert!(
            execute(&mut world, a, GripOperationV1::Release)
                .execution
                .succeeded
        );
        assert_eq!(world.entity(a).unwrap().position, before);
        assert_eq!(world.entity(a).unwrap().carried_by, None);
    }

    #[test]
    fn missing_manipulation_effector_cannot_close_or_release() {
        for held in [false, true] {
            let (mut world, a, _) = world();
            if held {
                execute(&mut world, a, GripOperationV1::Close);
            }
            let agent = world.agent_entity_id(ORGANISM).unwrap();
            let genome = alife_core::CreatureGenome::early_mammal_founder(
                2201,
                alife_core::FoundationGeneticIdentity::new(
                    10,
                    1,
                    7,
                    alife_core::BrainCapacityClass::N512_ID,
                )
                .unwrap(),
            )
            .unwrap();
            let phenotype = genome.express().unwrap();
            let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
            let mut record =
                WorldOrganismRecord::new(ORGANISM, agent, genome, phenotype, biology, Tick::ZERO)
                    .unwrap();
            // Zero calibration leaves an existing port at neutral gain 1.0.
            // Model an absent effector by removing its port and aligned lanes,
            // while preserving every other control in this fresh body fixture.
            let mut value = serde_json::to_value(record.embodiment()).unwrap();
            let ports = value["effectors"].as_array_mut().unwrap();
            let index = ports
                .iter()
                .position(|port| port["capability"] == "Manipulation")
                .unwrap();
            let start: usize = ports[..index]
                .iter()
                .map(|port| port["command_lanes"].as_u64().unwrap() as usize)
                .sum();
            let width = ports[index]["command_lanes"].as_u64().unwrap() as usize;
            ports.remove(index);
            value["effector_controllability"]
                .as_array_mut()
                .unwrap()
                .drain(start..start + width);
            value["revision"] = serde_json::json!(record.embodiment().revision() + 1);
            record
                .replace_embodiment_state(serde_json::from_value(value).unwrap())
                .unwrap();
            assert_eq!(
                record
                    .embodiment()
                    .effector_gain(EffectorCapability::Manipulation),
                0.0
            );
            world.register_organism_record(record).unwrap();
            let operation = if held {
                GripOperationV1::Release
            } else {
                GripOperationV1::Close
            };
            // This fixture exercises the exact-frame physical research API;
            // registered ABI admission is covered by the joint-dispatch tests.
            let result = execute(&mut world, a, operation);
            assert!(!result.execution.succeeded);
            assert!(result.grip_transfers.is_empty());
            assert_eq!(
                world.entity(a).unwrap().carried_by,
                held.then_some(ORGANISM)
            );
        }
    }

    #[test]
    fn factorized_release_receipt_and_repeated_release_never_acquire() {
        let (mut world, a, _) = world();
        execute(&mut world, a, GripOperationV1::Close);
        let agent = world.agent_entity_id(ORGANISM).unwrap();
        let brain =
            alife_core::BrainGenome::scaffold(2202, alife_core::BrainCapacityClass::N2048_ID);
        let development = alife_core::DevelopmentState::new(
            brain.id,
            Tick::ZERO,
            NormalizedScalar::new(1.0).unwrap(),
        );
        let compiled = alife_core::PhenotypeCompiler::compile_n2048_grip_reader_graph(
            brain,
            development,
            SensorProfile::GroundedGripObjectSlotsV1,
        )
        .unwrap()
        .0;
        let asset =
            alife_core::FoundationWeightAsset::from_phenotype_for_genetic_birth(&compiled).unwrap();
        let manifest = asset.manifest();
        let identity = alife_core::FoundationGeneticIdentity::new(
            manifest.foundation_id().raw(),
            manifest.foundation_version().raw() as u16,
            manifest.compatibility_family_id().raw(),
            alife_core::BrainCapacityClass::N2048_ID,
        )
        .unwrap();
        let genome = alife_core::CreatureGenome::early_mammal_founder(2202, identity)
            .unwrap()
            .with_n2048_foundation_candidate(asset)
            .unwrap();
        let phenotype = genome.express().unwrap();
        let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(ORGANISM, agent, genome, phenotype, biology, Tick::ZERO)
                    .unwrap(),
            )
            .unwrap();
        let channel = ChannelCommand::new(
            MotorChannel::Manipulation,
            GripOperationV1::Release.action_id(),
            Some(alife_core::ActionTarget::new(Some(a), None)),
            Vec3f::ZERO,
            Intensity::new(1.0).unwrap(),
            alife_core::DurationTicks::new(1),
            0.0,
            Confidence::new(1.0).unwrap(),
            0,
        )
        .unwrap();
        for sequence in 1..=2 {
            let bundle = MotorCommandBundle::new(
                ORGANISM,
                alife_core::ExperienceSequenceId::new(sequence).unwrap(),
                world.tick(),
                vec![channel.clone()],
            )
            .unwrap();
            let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();
            assert_eq!(receipt.channel_receipts.len(), 1);
            let physical = &receipt.channel_receipts[0];
            assert_eq!(physical.command.channel, MotorChannel::Manipulation);
            assert_eq!(
                physical.command.primitive,
                GripOperationV1::Release.action_id()
            );
            assert_eq!(physical.command.target.unwrap().entity, Some(a));
            assert_eq!(receipt.succeeded, sequence == 1);
            if sequence == 1 {
                assert_eq!(physical.grip_transfers.len(), 1);
                assert_eq!(physical.grip_transfers[0].object, a);
                assert_eq!(physical.grip_transfers[0].kind, GripTransferKind::Released);
            } else {
                assert!(physical.grip_transfers.is_empty());
                assert_eq!(physical.physical.contact, PhysicalContactKind::None);
            }
            assert_eq!(world.entity(a).unwrap().carried_by, None);
            world.try_advance_tick().unwrap();
        }
    }

    #[test]
    fn crowding_keeps_actual_held_target_in_slot_and_candidate_budgets() {
        let (mut world, a, _) = world();
        execute(&mut world, a, GripOperationV1::Close);
        for i in 0..20 {
            world
                .insert_object(SpawnSpec {
                    label: &format!("crowd-{i}"),
                    kind: WorldObjectKind::Token,
                    organism_id: None,
                    position: Vec3f::new(0.01 + i as f32 * 0.01, 0.0, 0.0),
                    nutrition: 0.0,
                    hazard_pain: 0.0,
                    token_id: None,
                    social_affinity: 0.0,
                    teacher_channel: None,
                })
                .unwrap();
        }
        let frame = grip_draft(&mut world);
        assert!(frame.grounded_object_slots().len() <= alife_core::MAX_GROUNDED_OBJECT_SLOTS);
        assert!(frame.candidates().len() <= alife_core::MAX_ACTION_CANDIDATES);
        assert!(frame
            .candidates()
            .iter()
            .any(|c| c.target.entity == Some(a)
                && c.action_id == GripOperationV1::Release.action_id()));
    }

    #[test]
    fn hidden_harm_does_not_change_grip_opportunities_or_legality() {
        for pain in [0.0, 0.8] {
            let (mut world, a, _) = world();
            world.objects.get_mut(&a.raw()).unwrap().hazard_pain = pain;
            for operation in [GripOperationV1::Close, GripOperationV1::Release] {
                let frame = grip_draft(&mut world);
                assert!(frame
                    .candidates()
                    .iter()
                    .any(|c| c.target.entity == Some(a) && c.action_id == operation.action_id()));
                assert!(execute(&mut world, a, operation).execution.succeeded);
            }
        }
    }

    #[test]
    fn nonself_ownership_is_not_exposed_by_grip_observation_or_menu() {
        for player in [false, true] {
            let (mut world, a, _) = world();
            let free_frame = grip_draft(&mut world);
            let free_observation = world.observe_research_grip(&free_frame, a).unwrap();
            let free_menu: Vec<_> = free_frame
                .candidates()
                .iter()
                .filter(|c| c.target.entity == Some(a))
                .map(|c| (c.action_id, c.family))
                .collect();
            if player {
                world.begin_player_hold(a).unwrap();
            } else {
                world.objects.get_mut(&a.raw()).unwrap().carried_by = Some(OrganismId(8));
            }
            let frame = grip_draft(&mut world);
            let observation = world.observe_research_grip(&frame, a).unwrap();
            assert_eq!(observation.ownership, GripOwnershipV1::NotSelfHeld);
            assert_eq!(observation.ownership, free_observation.ownership);
            assert_eq!(observation.confidence, free_observation.confidence);
            let menu: Vec<_> = frame
                .candidates()
                .iter()
                .filter(|c| c.target.entity == Some(a))
                .map(|c| (c.action_id, c.family))
                .collect();
            assert_eq!(menu, free_menu);
            assert!(menu.contains(&(
                GripOperationV1::Close.action_id(),
                GripOperationV1::Close.family()
            )));
            let before = world.entity(a).unwrap().clone();
            for operation in [GripOperationV1::Close, GripOperationV1::Release] {
                let result = execute(&mut world, a, operation);
                assert!(!result.execution.succeeded);
                assert!(result.grip_transfers.is_empty());
                assert_eq!(world.entity(a).unwrap(), &before);
            }
        }
    }

    #[test]
    fn legacy_direct_raw_ids_keep_unknown_action_semantics_for_every_kind() {
        for id in [ActionId(2201), ActionId(2202)] {
            for kind in [
                ActionKind::Idle,
                ActionKind::Rest,
                ActionKind::Inspect,
                ActionKind::Look,
                ActionKind::Move,
                ActionKind::Hold,
                ActionKind::Interact,
                ActionKind::Vocalize,
                ActionKind::Write,
                ActionKind::Gesture,
            ] {
                let (world, a, _) = world();
                let mut old_id_world = world.clone();
                let mut generic_world = world;
                let explicit =
                    HeadlessWorldCommand::structured(ORGANISM, id, kind, Some(a), None).unwrap();
                let generic =
                    HeadlessWorldCommand::structured(ORGANISM, ActionId(9900), kind, Some(a), None)
                        .unwrap();
                let actual = old_id_world.apply_command(&explicit);
                let expected = generic_world.apply_command(&generic);
                assert_eq!(actual.is_ok(), expected.is_ok());
                if let (Ok(actual), Ok(expected)) = (actual, expected) {
                    assert_eq!(actual.execution, expected.execution);
                    assert_eq!(actual.grip_transfers, expected.grip_transfers);
                    assert_eq!(actual.body_event, expected.body_event);
                }
                assert_eq!(old_id_world.objects, generic_world.objects);
            }
        }
    }

    #[test]
    fn legacy_registered_raw_ids_keep_generic_manipulation_toggle() {
        for id in [ActionId(2201), ActionId(2202)] {
            let (mut world, a, _) = world();
            let agent = world.agent_entity_id(ORGANISM).unwrap();
            let genome = alife_core::CreatureGenome::early_mammal_founder(
                9900,
                alife_core::FoundationGeneticIdentity::new(
                    10,
                    1,
                    7,
                    alife_core::BrainCapacityClass::N512_ID,
                )
                .unwrap(),
            )
            .unwrap();
            let phenotype = genome.express().unwrap();
            let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
            world
                .register_organism_record(
                    WorldOrganismRecord::new(
                        ORGANISM,
                        agent,
                        genome,
                        phenotype,
                        biology,
                        Tick::ZERO,
                    )
                    .unwrap(),
                )
                .unwrap();
            assert!(!world.has_grip_reader_abi(ORGANISM));
            let channel = ChannelCommand::new(
                MotorChannel::Manipulation,
                id,
                Some(alife_core::ActionTarget::new(Some(a), None)),
                Vec3f::ZERO,
                Intensity::new(1.0).unwrap(),
                alife_core::DurationTicks::new(1),
                0.0,
                Confidence::new(1.0).unwrap(),
                0,
            )
            .unwrap();
            for sequence in 1..=2 {
                let bundle = MotorCommandBundle::new(
                    ORGANISM,
                    alife_core::ExperienceSequenceId::new(sequence).unwrap(),
                    world.tick(),
                    vec![channel.clone()],
                )
                .unwrap();
                let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();
                assert!(receipt.succeeded);
                assert_eq!(receipt.grip_transfers.len(), 1);
                assert_eq!(
                    receipt.grip_transfers[0].kind,
                    if sequence == 1 {
                        GripTransferKind::Acquired
                    } else {
                        GripTransferKind::Released
                    }
                );
                assert_eq!(
                    world.entity(a).unwrap().carried_by,
                    (sequence == 1).then_some(ORGANISM)
                );
                world.try_advance_tick().unwrap();
            }
        }
    }

    #[test]
    fn legacy_frame_cannot_open_the_explicit_research_interface() {
        let (mut world, a, _) = world();
        let frame = legacy_draft(&mut world);
        assert!(world.observe_research_grip(&frame, a).is_err());
    }
}
