use alife_core::{CandidateActionFamily, PhysicalContactKind, SensorProfile, Validate, Vec3f};
use alife_training::record_founder_demonstration;

#[test]
fn sampling_teacher_uses_observed_reach_without_an_inspection_phase() {
    for (x, family) in [
        (4.0, CandidateActionFamily::Approach),
        (-7.9, CandidateActionFamily::Approach),
        (0.5, CandidateActionFamily::Ingest),
    ] {
        let demo =
            alife_training::record_founder_sampling_demonstration(92001, Vec3f::new(x, 0.0, 0.0))
                .unwrap();
        let first = &demo.steps[0];
        assert_eq!(
            first.observation.candidates()[usize::from(first.teacher_candidate_index)].family,
            family
        );
        assert!(demo.steps.iter().all(|s| s.observation.candidates()
            [usize::from(s.teacher_candidate_index)]
        .family
            != CandidateActionFamily::Inspect));
        assert_eq!(
            demo.steps.last().unwrap().execution.physical.contact,
            PhysicalContactKind::Consumed
        );
    }
}

#[test]
fn demonstration_records_real_approach_inspection_ingestion_and_bodily_consequence() {
    for (seed, x) in [(91001, 2.0), (91002, -4.0), (91003, 0.5), (91004, 7.9)] {
        let demo = record_founder_demonstration(seed, Vec3f::new(x, 0.0, 0.0)).unwrap();
        assert!(!demo.steps.is_empty());
        assert!(demo.steps.len() <= alife_training::MAX_TRAINING_SEQUENCE_TICKS);
        if x.abs() >= 4.0 {
            assert!(demo.steps.len() > 32, "retain the entire physical approach");
        }
        for pair in demo.steps.windows(2) {
            assert_eq!(pair[0].position_after, pair[1].position_before);
            assert_eq!(pair[0].body_after, pair[1].body_before);
        }
        let mut inspected = false;
        let mut approached = false;
        for step in &demo.steps {
            step.observation.validate_contract().unwrap();
            assert_eq!(
                step.observation.sensor_profile(),
                SensorProfile::GroundedObjectSlotsV1
            );
            assert_eq!(
                step.observation.homeostasis(),
                &step.body_before.homeostasis
            );
            let target = step.observation.candidates()[usize::from(step.teacher_candidate_index)];
            assert_eq!(target.action_id, step.command.action_id);
            assert!(step.execution.succeeded);
            assert_ne!(step.world_before_digest, step.world_after_digest);
            match target.family {
                CandidateActionFamily::Approach => {
                    approached = true;
                    assert_ne!(step.position_before, step.position_after);
                }
                CandidateActionFamily::Inspect => {
                    assert_eq!(step.execution.physical.contact, PhysicalContactKind::Touch);
                    inspected = true;
                }
                CandidateActionFamily::Ingest => {
                    assert!(
                        inspected,
                        "the demonstration must include an actual inspection"
                    );
                    assert_eq!(
                        step.execution.physical.contact,
                        PhysicalContactKind::Consumed
                    );
                    assert!(step.body_after.body.energy > step.body_before.body.energy);
                }
                family => panic!("unexpected demonstrated family {family:?}"),
            }
        }
        if x.abs() > 1.0 {
            assert!(approached);
        }
        assert_eq!(
            demo.steps.last().unwrap().execution.physical.contact,
            PhysicalContactKind::Consumed
        );
    }
}

#[test]
fn demonstration_requires_an_observed_target_before_executing_teacher_commands() {
    assert!(record_founder_demonstration(91005, Vec3f::new(9.0, 0.0, 0.0)).is_err());
}
