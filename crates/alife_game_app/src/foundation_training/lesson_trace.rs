//! Passive lesson diagnostics from already-resident CPU receipts.
//! No world queries, neural readbacks, or changes to policy/outcome contracts.

use alife_core::{MotorCommandBundle, PerceptionFrame, PostActionOutcome};
use alife_gpu_backend::GpuTrainingRolloutReceipt;

/// The representative is retained for older readers; actual concurrent motor
/// commands come from the sealed patch's bounded (at most six-channel) bundle.
/// Global grip and joint success do not identify a candidate's owner or success.
pub(super) fn serialize_step(
    frame: &PerceptionFrame,
    behavior: &GpuTrainingRolloutReceipt,
    selected_bundle: Option<&MotorCommandBundle>,
    outcome: &PostActionOutcome,
    lesson_target_held_before_decision: Option<bool>,
) -> serde_json::Value {
    let chosen = frame
        .candidates()
        .get(behavior.representative_index as usize);
    serde_json::json!({
        "tick": frame.tick().raw(),
        "lesson_target_held_before_decision": lesson_target_held_before_decision,
        "global_grip_before_decision": frame.sensory().channels.tactile_contact[2],
        "position": frame.body().pose.translation,
        "gaze_rotation": frame.body().pose.rotation,
        "terrain_ranges": frame.sensory().channels.visual_affordance,
        "heard_tokens": frame.sensory().language_context.heard_tokens,
        "vocalized_token": frame.sensory().language_context.vocalized_token,
        "semantic_context": frame.sensory().semantic_context,
        "chosen_action": chosen.map(|candidate| candidate.action_id.raw()),
        "chosen_target": chosen.and_then(|candidate| candidate.target.entity),
        "chosen_family": chosen.map(|candidate| candidate.family),
        "object_slots": frame.grounded_object_slots(),
        "selected_commands": selected_bundle.map(|bundle| bundle.channels.iter().map(|command| serde_json::json!({
            "channel": command.channel,
            "primitive": command.primitive.raw(),
            "target": command.target,
        })).collect::<Vec<_>>()),
        "representative_mask": behavior.representative_mask,
        "motor_masks": behavior.motor_masks,
        "motor_indices": behavior.motor_indices,
        "candidates": frame.candidates().iter().enumerate().map(|(index, candidate)| serde_json::json!({
            "index": candidate.candidate_index,
            "action": candidate.action_id.raw(),
            "family": candidate.family,
            "target": candidate.target.entity,
            "observation": candidate.observation,
            "target_position": candidate.target.position,
            "logit": behavior.logits[index],
        })).collect::<Vec<_>>(),
        "success": outcome.success,
        "frustration_delta": outcome.frustration_delta.raw(),
        "prediction_error": outcome.prediction_error.raw(),
        "physical": outcome.physical,
        "channel_outcomes": outcome.joint.as_ref().map(|joint| &joint.channel_outcomes),
        "experienced_valence": outcome.experienced_valence(),
        "physiology": outcome.measured_physiology.map(|transition| serde_json::json!({
            "before_energy": transition.before.body.energy,
            "after_energy": transition.after.body.energy,
            "before_hunger": transition.before.homeostasis.drives.hunger,
            "after_hunger": transition.after.homeostasis.drives.hunger,
            "before_health": transition.before.body.health,
            "after_health": transition.after.body.health,
            "before_injury": transition.before.body.injury,
            "after_injury": transition.after.body.injury,
            "before_pain": transition.before.homeostasis.drives.pain,
            "after_pain": transition.after.homeostasis.drives.pain,
            "before_brain_atp": transition.before.homeostasis.drives.brain_atp,
            "after_brain_atp": transition.after.homeostasis.drives.brain_atp,
            "before_fatigue": transition.before.homeostasis.drives.fatigue,
            "after_fatigue": transition.after.homeostasis.drives.fatigue,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, CandidateActionFamily,
        CandidateFeatureVector, CandidateObservationRef, ChannelCommand, Confidence, DurationTicks,
        ExperienceSequenceId, GroundedObjectSlotV1, HomeostaticDelta, HomeostaticSnapshot,
        Intensity, MotorChannel, NormalizedScalar, OrganismId, PerceptionContextBlock,
        PerceptionFrameDraft, PhysicalActionOutcome, PhysicalContactKind, Pose, SensorProfile,
        SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot,
        SignedValence, Tick, TrackedObjectId, Vec3f, Velocity, WorldEntityId,
    };
    use alife_gpu_backend::{GpuTrainingBehaviorKind, GpuTrainingSamplingConfig};
    use serde_json::json;

    const TARGET: WorldEntityId = WorldEntityId(3);
    const TARGET_POSITION: Vec3f = Vec3f::new(-1.25, 0.125, 3.5);

    // Synthetic CPU data tests serialization only. They are not a replay of a
    // failed assessment, neural behavior evidence, or target ownership evidence.
    fn fixture(
        grip: f32,
    ) -> (
        PerceptionFrame,
        GpuTrainingRolloutReceipt,
        PostActionOutcome,
    ) {
        let organism = OrganismId(1);
        let tick = Tick(7);
        let slot = GroundedObjectSlotV1 {
            slot_index: 0,
            tracked_object_id: TrackedObjectId(9),
            bearing: [-0.25, 0.5],
            distance: 0.5,
            relative_velocity: [0.0; 3],
            color: [0.2; 3],
            material: [0.0; 3],
            shape: [0.0; 3],
            chemical: [0.0; 3],
            contact: 1.0,
            proprioception: [0.0; 2],
            temperature: 0.0,
            terrain: [0.0; 2],
            confidence: Confidence::new(0.75).unwrap(),
        };
        let candidates = [
            (101, ActionKind::Move, CandidateActionFamily::Approach),
            (211, ActionKind::Hold, CandidateActionFamily::Other),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (action, kind, family))| {
            ActionCandidate::new(
                index as u16,
                ActionId(action),
                kind,
                family,
                CandidateObservationRef::ObjectSlot(0),
                ActionTarget::new(Some(TARGET), Some(TARGET_POSITION)),
                slot.candidate_features().unwrap(),
                Confidence::new(0.75).unwrap(),
                NormalizedScalar::new(0.25).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap()
        })
        .chain(std::iter::once(
            ActionCandidate::new(
                2,
                ActionId(1),
                ActionKind::Idle,
                CandidateActionFamily::Idle,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector::zero(),
                Confidence::new(0.75).unwrap(),
                NormalizedScalar::ZERO,
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap(),
        ))
        .collect();
        let mut channels = SensoryChannels::ZERO;
        channels.tactile_contact[2] = grip;
        let frame = PerceptionFrameDraft::new(
            organism,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            SensorySnapshot::new(organism, tick, Vec3f::ZERO, channels, Default::default())
                .unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::baseline(tick),
            candidates,
            SensorProfileProvenance::new(
                SensorProfile::GroundedObjectSlotsV1,
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            vec![slot],
        )
        .unwrap()
        .finalize(PerceptionContextBlock::empty())
        .unwrap();
        let behavior = GpuTrainingRolloutReceipt {
            organism_id: organism.raw(),
            tick: tick.raw(),
            dispatch_generation: 1,
            frame_digest: frame.frame_digest(),
            active_weight_generation: 0,
            sampling: GpuTrainingSamplingConfig {
                seed: 1,
                counter: 0,
                temperature: 1.0,
                demonstrator: None,
            },
            logits: vec![-1.25, 0.5, 0.0],
            decoder_inputs: Vec::new(),
            decoder_input_stride: 24,
            representative_mask: 7,
            motor_masks: [1, 0, 2, 0, 0, 0],
            forced_motor_slots: vec![0, 2, 4],
            representative_index: 0,
            motor_indices: [0, u16::MAX, 1, u16::MAX, u16::MAX, u16::MAX],
            factor_log_probabilities: [0.0; 7],
            behavior: GpuTrainingBehaviorKind::OnPolicy,
            joint_log_probability: Some(0.0),
            policy_joint_log_probability: 0.0,
        };
        let outcome = PostActionOutcome::new(
            organism,
            ExperienceSequenceId(1),
            Tick(8),
            false,
            PhysicalActionOutcome {
                contact: PhysicalContactKind::Touch,
                target_entity: Some(TARGET),
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar::ZERO,
            },
            HomeostaticDelta::zero(),
            SignedValence::new(-0.625).unwrap(),
            NormalizedScalar::new(0.25).unwrap(),
            NormalizedScalar::ZERO,
            SignedValence::ZERO,
            NormalizedScalar::new(0.875).unwrap(),
        )
        .unwrap();
        (frame, behavior, outcome)
    }

    fn command(channel: MotorChannel, target: Option<ActionTarget>) -> ChannelCommand {
        ChannelCommand::new(
            channel,
            ActionId(211),
            target,
            Vec3f::ZERO,
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            0.0,
            Confidence::new(0.75).unwrap(),
            0,
        )
        .unwrap()
    }

    fn bundle(commands: Vec<ChannelCommand>) -> MotorCommandBundle {
        MotorCommandBundle::new(OrganismId(1), ExperienceSequenceId(1), Tick(7), commands).unwrap()
    }

    #[test]
    fn contact_does_not_replace_global_grip_and_raw_value_is_preserved() {
        for grip in [0.0, 0.375, 1.0] {
            let (frame, behavior, outcome) = fixture(grip);
            let trace = serialize_step(&frame, &behavior, None, &outcome, None);
            assert_eq!(trace["global_grip_before_decision"], json!(grip));
            assert_eq!(trace["object_slots"][0]["contact"], json!(1.0));
            assert_eq!(trace["physical"]["contact"], json!("Touch"));
            assert!(trace["lesson_target_held_before_decision"].is_null());
            assert!(trace.get("owner_before").is_none());
            assert!(trace["candidates"][1].get("owner").is_none());
        }
    }

    #[test]
    fn selected_manipulation_is_independent_of_representative_and_joint_success() {
        let (frame, behavior, mut outcome) = fixture(0.375);
        outcome.joint = Some(
            alife_core::JointPhysicalOutcome::new(outcome.physical, Vec::new())
                .unwrap()
                .with_channel_outcomes(vec![alife_core::experience::ChannelPhysicalOutcome {
                    channel: MotorChannel::Manipulation,
                    physical: outcome.physical,
                }])
                .unwrap(),
        );
        let selected = bundle(vec![command(
            MotorChannel::Manipulation,
            Some(ActionTarget::new(Some(TARGET), Some(TARGET_POSITION))),
        )]);
        let trace = serialize_step(&frame, &behavior, Some(&selected), &outcome, Some(true));
        assert_eq!(trace["chosen_action"], json!(101));
        assert_eq!(trace["chosen_family"], json!("Approach"));
        assert_eq!(
            trace["selected_commands"],
            json!([{
                "channel": "Manipulation", "primitive": 211,
                "target": {"entity": 3, "position": TARGET_POSITION},
            }])
        );
        assert_eq!(
            trace["candidates"][1]["observation"],
            json!({"ObjectSlot": 0})
        );
        assert_eq!(
            trace["candidates"][1]["target_position"],
            json!(TARGET_POSITION)
        );
        assert_eq!(trace["candidates"][1]["target"], json!(3));
        assert_eq!(
            trace["candidates"][1]["observation"]["ObjectSlot"],
            trace["object_slots"][0]["slot_index"]
        );
        assert_eq!(trace["object_slots"][0]["tracked_object_id"], json!(9));
        assert_eq!(
            trace["channel_outcomes"],
            json!(outcome.joint.as_ref().unwrap().channel_outcomes)
        );

        assert_eq!(trace["frustration_delta"], json!(0.25));
        assert_eq!(trace["prediction_error"], json!(0.875));
        assert_eq!(trace["experienced_valence"], json!(-0.625));
        // Touch does not imply joint success or per-channel success.
        assert_eq!(trace["success"], json!(false));
        assert!(trace["selected_commands"][0].get("success").is_none());
        outcome.success = true;
        let trace = serialize_step(&frame, &behavior, Some(&selected), &outcome, Some(true));
        assert_eq!(trace["success"], json!(true));
    }

    #[test]
    fn missing_bundle_and_missing_target_fields_stay_null() {
        let (frame, behavior, outcome) = fixture(0.0);
        let trace = serialize_step(&frame, &behavior, None, &outcome, None);
        assert!(trace["selected_commands"].is_null());
        assert!(trace["candidates"][2]["target"].is_null());
        assert!(trace["candidates"][2]["target_position"].is_null());
        assert_eq!(trace["candidates"][2]["observation"], json!("None"));
        let selected = bundle(vec![
            command(MotorChannel::Manipulation, None),
            command(
                MotorChannel::Orientation,
                Some(ActionTarget::new(Some(TARGET), None)),
            ),
        ]);
        let trace = serialize_step(&frame, &behavior, Some(&selected), &outcome, None);
        for command in trace["selected_commands"].as_array().unwrap() {
            assert!(command["target"]["position"].is_null());
        }
        assert!(trace["selected_commands"][0]["target"].is_null());
        assert_eq!(trace["selected_commands"][1]["target"]["entity"], json!(3));
        let empty = bundle(Vec::new());
        assert_eq!(
            serialize_step(&frame, &behavior, Some(&empty), &outcome, None)["selected_commands"],
            json!([])
        );
    }

    #[test]
    fn bounded_bundle_keeps_six_commands_without_expanding_payloads() {
        let (frame, behavior, outcome) = fixture(0.0);
        let selected = bundle(
            [
                MotorChannel::Locomotion,
                MotorChannel::Orientation,
                MotorChannel::Manipulation,
                MotorChannel::Vocal,
                MotorChannel::Posture,
                MotorChannel::SpeciesSpecific(0),
            ]
            .into_iter()
            .map(|channel| command(channel, None))
            .collect(),
        );
        let trace = serialize_step(&frame, &behavior, Some(&selected), &outcome, None);
        let commands = trace["selected_commands"].as_array().unwrap();
        assert_eq!(commands.len(), alife_core::MAX_MOTOR_CHANNELS);
        assert!(commands
            .iter()
            .all(|command| command.as_object().unwrap().len() == 3));
    }

    #[test]
    fn additive_fields_preserve_the_previous_lesson_step_shape() {
        let (frame, behavior, outcome) = fixture(0.0);
        let mut trace = serialize_step(&frame, &behavior, None, &outcome, Some(false));
        let object = trace.as_object_mut().unwrap();
        for added in [
            "global_grip_before_decision",
            "success",
            "frustration_delta",
            "prediction_error",
            "selected_commands",
        ] {
            object.remove(added);
        }
        for candidate in object["candidates"].as_array_mut().unwrap() {
            candidate.as_object_mut().unwrap().remove("observation");
            candidate.as_object_mut().unwrap().remove("target_position");
        }
        assert_eq!(
            trace,
            json!({
                "tick": 7, "lesson_target_held_before_decision": false,
                "position": frame.body().pose.translation,
                "gaze_rotation": frame.body().pose.rotation,
                "terrain_ranges": frame.sensory().channels.visual_affordance,
                "heard_tokens": frame.sensory().language_context.heard_tokens,
                "vocalized_token": frame.sensory().language_context.vocalized_token,
                "semantic_context": frame.sensory().semantic_context,
                "chosen_action": 101, "chosen_target": 3, "chosen_family": "Approach",
                "object_slots": frame.grounded_object_slots(),
                "representative_mask": behavior.representative_mask,
                "motor_masks": behavior.motor_masks, "motor_indices": behavior.motor_indices,
                "candidates": [
                    {"index": 0, "action": 101, "family": "Approach", "target": 3, "logit": -1.25},
                    {"index": 1, "action": 211, "family": "Other", "target": 3, "logit": 0.5},
                    {"index": 2, "action": 1, "family": "Idle", "target": null, "logit": 0.0},
                ],
                "physical": outcome.physical, "channel_outcomes": null,
                "experienced_valence": outcome.experienced_valence(), "physiology": null,
            })
        );
    }
}
