use alife_core::{CandidateActionFamily, PhysicalContactKind, SensorProfile, Validate, Vec3f};
use alife_training::record_founder_demonstration;
use alife_world::HeadlessActionIds;

fn assert_grab_then_eat(demo: &alife_training::FounderDemonstration) {
    let grab_steps: Vec<_> = demo
        .steps
        .iter()
        .enumerate()
        .filter(|(_, step)| step.command.action_id == HeadlessActionIds::GRAB)
        .collect();
    assert_eq!(
        grab_steps.len(),
        1,
        "acquire once without releasing the food"
    );
    let (grab_index, grab) = grab_steps[0];
    let eat = &demo.steps[grab_index + 1];
    assert_eq!(grab_index + 2, demo.steps.len());
    assert!(grab.execution.succeeded);
    assert_eq!(grab.execution.physical.contact, PhysicalContactKind::Touch);
    assert_eq!(eat.command.action_id, HeadlessActionIds::EAT);
    assert_eq!(grab.command.target_entity, eat.command.target_entity);
    assert_eq!(grab.position_after, eat.position_before);
    assert_eq!(grab.body_after, eat.body_before);
    assert!(eat.execution.succeeded);
    assert_eq!(
        eat.execution.physical.contact,
        PhysicalContactKind::Consumed
    );
    assert!(eat.body_after.body.energy > eat.body_before.body.energy);
}

#[test]
fn sampling_teacher_uses_observed_reach_without_an_inspection_phase() {
    for (x, family) in [
        (4.0, CandidateActionFamily::Approach),
        (-7.9, CandidateActionFamily::Approach),
        (0.5, CandidateActionFamily::Contact),
    ] {
        let demo =
            alife_training::record_founder_sampling_demonstration(92001, Vec3f::new(x, 0.0, 0.0))
                .unwrap();
        assert!(demo.steps.len() <= alife_training::MAX_TRAINING_SEQUENCE_TICKS);
        assert_grab_then_eat(&demo);
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
fn demonstration_records_real_approach_inspection_grab_ingestion_and_bodily_consequence() {
    for (seed, x) in [(91001, 2.0), (91002, -4.0), (91003, 0.5), (91004, 7.9)] {
        let demo = record_founder_demonstration(seed, Vec3f::new(x, 0.0, 0.0)).unwrap();
        assert!(!demo.steps.is_empty());
        assert!(demo.steps.len() <= alife_training::MAX_TRAINING_SEQUENCE_TICKS);
        assert_grab_then_eat(&demo);
        if x.abs() >= 4.0 {
            assert!(demo.steps.len() > 32, "retain the entire physical approach");
        }
        for pair in demo.steps.windows(2) {
            assert_eq!(pair[0].position_after, pair[1].position_before);
            assert_eq!(pair[0].body_after, pair[1].body_before);
        }
        let mut inspected = false;
        let mut approached = false;
        let mut grabbed = false;
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
                CandidateActionFamily::Contact => {
                    assert!(inspected, "inspect before acquiring the food");
                    assert_eq!(step.command.action_id, HeadlessActionIds::GRAB);
                    assert_eq!(step.execution.physical.contact, PhysicalContactKind::Touch);
                    grabbed = true;
                }
                CandidateActionFamily::Ingest => {
                    assert!(
                        inspected,
                        "the demonstration must include an actual inspection"
                    );
                    assert!(grabbed, "the demonstration must establish possession");
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
