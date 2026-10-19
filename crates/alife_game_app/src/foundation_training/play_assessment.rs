//! Read-only supporting evidence for later need-choice assessments.
//! These observations are neither a curriculum gate nor causal attribution.
use alife_core::{
    BiochemicalDriveChannel, BiochemicalSourceLocus, BiochemicalTargetLocus, ChemicalSpeciesId,
    CreaturePhenotype, ExperiencePatch, MotorChannel, OrganismId, Tick, Validate, WorldEntityId,
};

use crate::gpu_live_runtime::FoundationActivityEvidence;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FoundationPlayAssessment {
    pub organism_id: OrganismId,
    pub target: WorldEntityId,
    pub source_tick: Tick,
    pub outcome_tick: Tick,
    pub sequence_id: u64,
    pub play_stimulation: f32,
    /// Investigation remains evidence even when stimulation is zero.
    pub investigation: f32,
    /// Unknown if physiology or the supported inherited boredom circuit is absent.
    pub boredom: Option<FoundationPlayBoredomObservation>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FoundationPlayBoredomObservation {
    pub before: f32,
    pub after: f32,
    pub inherited_nominal: f32,
    pub inherited_value_weight: f32,
    /// Existing need and measured decrease during this stimulated activation.
    /// Concurrent chemistry/channels prevent attributing the decrease to Play alone.
    pub relief_observed: bool,
}

/// Assess one exact-target completed Play, independently of vocabulary completion
/// and praise. Missing/mismatched activation returns None; novelty-only activation
/// remains observable but cannot establish boredom relief. No episode-wide delta
/// can substitute for this activation's measured before/after transition.
pub fn assess_foundation_play(
    phenotype: &CreaturePhenotype,
    patch: &ExperiencePatch,
    activity: Option<&FoundationActivityEvidence>,
    target: WorldEntityId,
) -> Option<FoundationPlayAssessment> {
    patch.validate_contract().ok()?;
    let activity = activity?;
    let receipt = &activity.receipt;
    if !activity.completed_play(patch, Some(target))
        || ![receipt.play_stimulation, receipt.investigation]
            .iter()
            .all(|dose| dose.is_finite() && (0.0..=1.0).contains(dose))
        || !patch
            .outcome()
            .joint
            .as_ref()?
            .channel_outcomes
            .iter()
            .any(|channel| {
                channel.channel == MotorChannel::Manipulation
                    && channel.physical == receipt.physical
            })
    {
        return None;
    }
    Some(FoundationPlayAssessment {
        organism_id: activity.organism_id,
        target,
        source_tick: activity.source_tick,
        outcome_tick: activity.outcome_tick,
        sequence_id: activity.sequence_id,
        play_stimulation: receipt.play_stimulation,
        investigation: receipt.investigation,
        boredom: boredom_observation(phenotype, patch, receipt.play_stimulation),
    })
}

fn boredom_observation(
    phenotype: &CreaturePhenotype,
    patch: &ExperiencePatch,
    stimulation: f32,
) -> Option<FoundationPlayBoredomObservation> {
    let physiology = patch.outcome().measured_physiology?;
    physiology.validate_contract().ok()?;
    physiology.before.validate_against(phenotype).ok()?;
    physiology.after.validate_against(phenotype).ok()?;
    if physiology.before.tick != patch.pre_action().perception().tick()
        || physiology.after.tick != patch.outcome().outcome_tick
        || physiology.before.source_genome_id != patch.pre_action().genome_id
        || physiology.before.homeostasis != *patch.pre_action().homeostasis()
    {
        return None;
    }
    // Extension0 is replaceable, not a universal boredom ABI. Recognize the
    // inherited reference circuit only: species 22, one positive analogue
    // receptor, relief-oriented value, and a PlayStimulation release that lowers
    // that species. Other inherited circuits need an explicit interpretation.
    let graph = &phenotype.chemistry.biochemical;
    let mut receptors = graph.receptors().iter().filter(|receptor| {
        receptor.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Extension0)
    });
    let receptor = receptors.next()?;
    let weight = physiology.before.value_profile().drives[9];
    if receptors.next().is_some()
        || receptor.source != ChemicalSpeciesId(22)
        || receptor.gain <= 0.0
        || receptor.digital
        || weight >= 0.0
        || !graph.emitters().iter().any(|emitter| {
            emitter.source == BiochemicalSourceLocus::PlayStimulation
                && emitter.target == receptor.source
                && emitter.gain < 0.0
        })
    {
        return None;
    }
    let before = physiology.before.homeostasis.drives.extension[0];
    let after = physiology.after.homeostasis.drives.extension[0];
    Some(FoundationPlayBoredomObservation {
        before,
        after,
        inherited_nominal: receptor.nominal,
        inherited_value_weight: weight,
        relief_observed: stimulation > 0.0 && before > receptor.nominal && after < before,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation_training::held_food_lesson_tests::{action_patch, frame, unconfigured};
    use alife_core::{
        Confidence, ExperienceSequenceId, HomeostaticDelta, MeasuredPhysiologyTransition,
        MotorCommandBundle, NormalizedScalar, PhysicalContactKind, PostActionOutcome,
        SignedValence, Vec3f,
    };
    use alife_world::{ActionPhase, HeadlessActionIds as Ids, HeadlessWorld};

    fn prepared(carryable: bool) -> (HeadlessWorld, WorldEntityId) {
        let mut world = unconfigured(42);
        let scenario = crate::foundation_training::configure_foundation_scenario(
            &mut world,
            42,
            Some(crate::FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        let (_, entity) = world.organism_entity_ids()[0];
        let body = world.entity(entity).unwrap().clone();
        crate::foundation_training::move_scenario_object(
            &mut world,
            scenario.food,
            Vec3f::new(
                body.position.x - 30.0,
                body.position.y,
                body.position.z + 20.0,
            ),
        )
        .unwrap();
        let toy = world
            .spawn_toy(
                "assessment-toy",
                Vec3f::new(
                    body.position.x + body.body_yaw.cos() * 0.7,
                    body.position.y,
                    body.position.z + body.body_yaw.sin() * 0.7,
                ),
                carryable,
            )
            .unwrap();
        (world, toy)
    }

    // Ordinary CPU world execution/biology, synthetic selection scaffolding.
    // The novelty input exercises the same world modulation as the live runtime;
    // this is no claim of policy competence or learned familiarity.
    fn execute(
        world: &mut HeadlessWorld,
        target: WorldEntityId,
        action: alife_core::ActionId,
        novelty: f32,
    ) -> (ExperiencePatch, FoundationActivityEvidence) {
        let before = frame(world);
        let (subject, entity) = world.organism_entity_ids()[0];
        let command = before
            .candidates()
            .iter()
            .find(|c| c.action_id == action && c.target.entity == Some(target))
            .unwrap()
            .to_command(subject, Confidence::new(1.0).unwrap())
            .unwrap();
        let bundle = MotorCommandBundle::new(
            subject,
            ExperienceSequenceId::new(world.tick().raw() + 1).unwrap(),
            before.tick(),
            vec![
                alife_core::channel_command_for_action(MotorChannel::Manipulation, &command)
                    .unwrap(),
            ],
        )
        .unwrap();
        let neural = alife_core::NeuralEmissionFrame::new(before.tick(), 1, vec![]).unwrap();
        let measured = world
            .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
                &bundle,
                entity,
                &neural,
                novelty,
                &[(target, novelty)],
            )
            .unwrap();
        world.try_advance_tick().unwrap();
        let outcome = PostActionOutcome::new(
            subject,
            bundle.sequence_id,
            world.tick(),
            measured.succeeded,
            measured.joint.execution,
            HomeostaticDelta::zero(),
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
            NormalizedScalar::ZERO,
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
        )
        .unwrap()
        .with_measured_physiology(
            MeasuredPhysiologyTransition::new(measured.biology_before, measured.biology_after)
                .unwrap(),
        )
        .unwrap();
        let evidence = FoundationActivityEvidence {
            organism_id: subject,
            source_tick: before.tick(),
            outcome_tick: world.tick(),
            sequence_id: bundle.sequence_id.raw(),
            receipt: measured.channel_receipts[0].clone(),
        };
        (
            action_patch(
                before,
                Some(target),
                outcome,
                action,
                Some((bundle, measured.joint)),
            ),
            evidence,
        )
    }

    fn assess(
        world: &HeadlessWorld,
        target: WorldEntityId,
        patch: &ExperiencePatch,
        evidence: Option<&FoundationActivityEvidence>,
    ) -> Option<FoundationPlayAssessment> {
        assess_foundation_play(
            world
                .organism_registry()
                .get(patch.header().organism_id)
                .unwrap()
                .phenotype(),
            patch,
            evidence,
            target,
        )
    }

    #[test]
    fn ordinary_preparation_novelty_and_stimulated_relief_are_distinct() {
        let (mut world, toy) = prepared(true);
        let (preparation, evidence) = execute(&mut world, toy, Ids::PLAY, 0.0);
        assert_eq!(evidence.receipt.phase, ActionPhase::Preparation);
        assert!(assess(&world, toy, &preparation, Some(&evidence)).is_none());

        let (novel, evidence) = execute(&mut world, toy, Ids::PLAY, 1.0);
        assert!(
            evidence.completed_play(&novel, Some(toy)),
            "novel Play remains narratable"
        );
        let observed = assess(&world, toy, &novel, Some(&evidence)).unwrap();
        assert_eq!(observed.play_stimulation, 0.0);
        assert!(observed.investigation > 0.0);
        assert!(!observed.boredom.unwrap().relief_observed);

        let mut partial_world = world.clone();
        let (familiar, evidence) = execute(&mut world, toy, Ids::PLAY, 0.0);
        let observed = assess(&world, toy, &familiar, Some(&evidence)).unwrap();
        assert!(observed.play_stimulation > 0.0);
        assert_eq!(observed.investigation, 0.0);
        let boredom = observed.boredom.as_ref().unwrap();
        assert!(boredom.before > boredom.inherited_nominal);
        assert!(boredom.after < boredom.before);
        assert!(boredom.relief_observed);
        assert_eq!(observed.organism_id, familiar.header().organism_id);
        assert_eq!(observed.sequence_id, familiar.header().sequence_id.raw());
        assert_eq!(
            serde_json::from_value::<FoundationPlayAssessment>(
                serde_json::to_value(&observed).unwrap()
            )
            .unwrap(),
            observed
        );
        let (partial, evidence) = execute(&mut partial_world, toy, Ids::PLAY, 0.5);
        let observed = assess(&partial_world, toy, &partial, Some(&evidence)).unwrap();
        assert!(observed.play_stimulation > 0.0 && observed.investigation > 0.0);
        assert!(observed.boredom.unwrap().relief_observed);
    }

    #[test]
    fn activation_identity_selection_and_physical_provenance_are_required() {
        let (mut world, toy) = prepared(false);
        let (patch, evidence) = execute(&mut world, toy, Ids::PLAY, 0.0);
        assert!(
            assess(&world, toy, &patch, Some(&evidence))
                .unwrap()
                .boredom
                .unwrap()
                .relief_observed
        );
        assert!(assess(&world, toy, &patch, None).is_none(), "selected only");
        assert!(assess(
            &world,
            WorldEntityId(toy.raw() + 1000),
            &patch,
            Some(&evidence)
        )
        .is_none());
        for field in 0..10 {
            let mut changed = evidence.clone();
            match field {
                0 => changed.sequence_id += 1,
                1 => changed.source_tick = Tick(changed.source_tick.raw() + 1),
                2 => changed.outcome_tick = Tick(changed.outcome_tick.raw() + 1),
                3 => changed.organism_id = OrganismId(changed.organism_id.raw() + 1),
                4 => changed.receipt.phase = ActionPhase::Preparation,
                5 => changed.receipt.physical.contact = PhysicalContactKind::None,
                6 => changed.receipt.physical.target_entity = Some(WorldEntityId(toy.raw() + 1)),
                7 => changed.receipt.physical.energy_cost = NormalizedScalar::new(0.5).unwrap(),
                8 => changed.receipt.command.channel = MotorChannel::Orientation,
                _ => {
                    changed.receipt.command.target.as_mut().unwrap().entity =
                        Some(WorldEntityId(toy.raw() + 1))
                }
            }
            assert!(
                assess(&world, toy, &patch, Some(&changed)).is_none(),
                "field {field}"
            );
        }
        let mut changed = evidence.clone();
        changed.receipt.command.primitive = Ids::GRAB;
        assert!(assess(&world, toy, &patch, Some(&changed)).is_none());
        let mut changed = evidence.clone();
        changed.receipt.play_stimulation = f32::NAN;
        assert!(assess(&world, toy, &patch, Some(&changed)).is_none());

        let mut outcome = patch.outcome().clone();
        outcome.success = false;
        let joint_failure = action_patch(
            patch.pre_action().perception().clone(),
            Some(toy),
            outcome.clone(),
            Ids::PLAY,
            patch.selected_bundle().cloned().zip(outcome.joint),
        );
        assert!(
            assess(&world, toy, &joint_failure, Some(&evidence))
                .unwrap()
                .boredom
                .unwrap()
                .relief_observed,
            "another channel's failure does not invalidate measured manipulation"
        );
        let legacy = action_patch(
            patch.pre_action().perception().clone(),
            Some(toy),
            PostActionOutcome::new(
                patch.header().organism_id,
                patch.header().sequence_id,
                patch.outcome().outcome_tick,
                true,
                patch.outcome().physical,
                HomeostaticDelta::zero(),
                SignedValence::ZERO,
                NormalizedScalar::ZERO,
                NormalizedScalar::ZERO,
                SignedValence::ZERO,
                NormalizedScalar::ZERO,
            )
            .unwrap(),
            Ids::PLAY,
            None,
        );
        assert!(
            assess(&world, toy, &legacy, Some(&evidence)).is_none(),
            "no executed bundle/measurement"
        );
        let mut outcome = patch.outcome().clone();
        outcome.joint.as_mut().unwrap().channel_outcomes.clear();
        let unsealed_channel = action_patch(
            patch.pre_action().perception().clone(),
            Some(toy),
            outcome.clone(),
            Ids::PLAY,
            patch.selected_bundle().cloned().zip(outcome.joint),
        );
        assert!(assess(&world, toy, &unsealed_channel, Some(&evidence)).is_none());
    }

    // Counterfactual numerical observations exercise boundaries only. They are
    // re-sealed with matching frame/transition values, not biological evidence.
    fn observed_boredom(patch: &ExperiencePatch, before: f32, after: f32) -> ExperiencePatch {
        let outcome = patch.outcome().clone();
        let mut t = outcome.measured_physiology.unwrap();
        t.before.homeostasis.drives.extension[0] = before;
        t.after.homeostasis.drives.extension[0] = after;
        observed_transition(
            patch,
            MeasuredPhysiologyTransition::new(t.before, t.after).unwrap(),
        )
    }

    fn observed_transition(
        patch: &ExperiencePatch,
        t: MeasuredPhysiologyTransition,
    ) -> ExperiencePatch {
        let outcome = patch.outcome().clone().with_measured_physiology(t).unwrap();
        let f = patch.pre_action().perception();
        let frame = alife_core::PerceptionFrameDraft::new(
            f.organism_id(),
            f.tick(),
            f.sensor_profile(),
            f.sensory().clone(),
            f.body(),
            t.before.homeostasis,
            f.candidates().to_vec(),
            f.profile_provenance(),
            f.grounded_object_slots().to_vec(),
        )
        .unwrap()
        .finalize(f.context().clone())
        .unwrap();
        action_patch(
            frame,
            patch.decision().selected_action.target_entity,
            outcome.clone(),
            patch.decision().selected_action.action_id,
            patch.selected_bundle().cloned().zip(outcome.joint),
        )
    }

    #[test]
    fn same_transition_need_and_decrease_are_required_without_a_curriculum_threshold() {
        let (mut world, toy) = prepared(false);
        let (patch, evidence) = execute(&mut world, toy, Ids::PLAY, 0.0);
        for (before, after, relief) in [
            (0.0, 0.0, false),
            (0.1, 0.1, false),
            (0.1, 0.11, false),
            (0.001, 0.0005, true),
        ] {
            let patch = observed_boredom(&patch, before, after);
            assert_eq!(
                assess(&world, toy, &patch, Some(&evidence))
                    .unwrap()
                    .boredom
                    .unwrap()
                    .relief_observed,
                relief
            );
        }
        let reduction = observed_boredom(&patch, 0.1, 0.05);
        let mut novelty = evidence.clone();
        novelty.receipt.play_stimulation = 0.0;
        novelty.receipt.investigation = 1.0;
        assert!(novelty.completed_play(&reduction, Some(toy)));
        assert!(
            !assess(&world, toy, &reduction, Some(&novelty))
                .unwrap()
                .boredom
                .unwrap()
                .relief_observed
        );

        let (grab, grab_evidence) = execute(&mut world, toy, Ids::GRAB, 0.0);
        let unrelated = observed_boredom(&grab, 0.1, 0.05);
        assert!(assess(&world, toy, &unrelated, Some(&grab_evidence)).is_none());
        assert!(
            assess(&world, toy, &unrelated, Some(&evidence)).is_none(),
            "earlier Play cannot borrow a later decrease"
        );

        let mut phenotype = world
            .organism_registry()
            .get(patch.header().organism_id)
            .unwrap()
            .phenotype()
            .clone();
        phenotype.source_genome_id = alife_core::GenomeId(phenotype.source_genome_id.raw() + 1);
        assert!(
            assess_foundation_play(&phenotype, &patch, Some(&evidence), toy)
                .unwrap()
                .boredom
                .is_none(),
            "unbound inherited circuitry is unknown"
        );
    }

    #[test]
    fn starting_physiology_must_match_the_selected_frame() {
        let (mut world, toy) = prepared(false);
        let (earlier, _) = execute(&mut world, toy, Ids::PLAY, 0.0);
        let (patch, evidence) = execute(&mut world, toy, Ids::PLAY, 0.0);
        for stale_tick in [false, true] {
            let t = patch.outcome().measured_physiology.unwrap();
            let mut before = t.before;
            if stale_tick {
                before = earlier.outcome().measured_physiology.unwrap().before;
            } else {
                before.homeostasis.drives.extension[0] += 0.01;
            }
            let outcome = patch
                .outcome()
                .clone()
                .with_measured_physiology(
                    MeasuredPhysiologyTransition::new(before, t.after).unwrap(),
                )
                .unwrap();
            let mismatched = action_patch(
                patch.pre_action().perception().clone(),
                Some(toy),
                outcome.clone(),
                Ids::PLAY,
                patch.selected_bundle().cloned().zip(outcome.joint),
            );
            assert!(assess(&world, toy, &mismatched, Some(&evidence))
                .unwrap()
                .boredom
                .is_none());
        }
    }

    #[test]
    fn inherited_nominal_and_value_orientation_define_the_supported_need() {
        let (mut world, toy) = prepared(false);
        let (patch, evidence) = execute(&mut world, toy, Ids::PLAY, 0.0);
        let record = world
            .organism_registry()
            .get(patch.header().organism_id)
            .unwrap();
        let index = record
            .phenotype()
            .chemistry
            .biochemical
            .receptors()
            .iter()
            .position(|r| {
                r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Extension0)
            })
            .unwrap();
        for weight in [-0.08, 0.0, 0.08] {
            // Disposable inherited variants and counterfactual observations only.
            let mut genome = record.genome().clone();
            let mut profile = record.phenotype().chemistry.biochemical.value_profile();
            profile.drives[9] = weight;
            for side in [
                alife_core::AlleleSide::Maternal,
                alife_core::AlleleSide::Paternal,
            ] {
                genome.chemistry.graph = genome
                    .chemistry
                    .graph
                    .with_receptor_nominal(side, index, 0.2)
                    .unwrap()
                    .with_value_profile(side, profile)
                    .unwrap();
            }
            let phenotype = genome.express().unwrap();
            for (before, after, relief) in [(0.2, 0.19, false), (0.21, 0.2, true)] {
                let mut starting =
                    alife_core::BiochemistryState::new(&phenotype, evidence.source_tick).unwrap();
                let mut ending = starting
                    .advance(
                        evidence.outcome_tick,
                        alife_core::BodyEventDelta::zero(),
                        &phenotype,
                    )
                    .unwrap();
                starting.homeostasis.drives.extension[0] = before;
                ending.homeostasis.drives.extension[0] = after;
                let observed = observed_transition(
                    &patch,
                    MeasuredPhysiologyTransition::new(starting, ending).unwrap(),
                );
                let assessment =
                    assess_foundation_play(&phenotype, &observed, Some(&evidence), toy).unwrap();
                if weight < 0.0 {
                    let need = assessment.boredom.unwrap();
                    assert_eq!(need.inherited_nominal, 0.2);
                    assert_eq!(need.relief_observed, relief);
                } else {
                    assert!(
                        assessment.boredom.is_none(),
                        "no inherited relief-oriented value"
                    );
                }
            }
        }
    }
}
