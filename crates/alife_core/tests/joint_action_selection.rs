use alife_core::{
    channel_command_for_action, ActionCommand, ActionId, ActionKind, Confidence, DurationTicks,
    ExperienceSequenceId, Intensity, JointActionSelectionV1, MotorChannel, MotorCommandBundle,
    OrganismId, Tick, Vec3f,
};

#[test]
fn joint_receipt_binds_command_parameters_and_roundtrips() {
    let mut command = ActionCommand::new(
        OrganismId(1),
        ActionKind::Move,
        None,
        Confidence::new(0.8).unwrap(),
        DurationTicks::new(2),
    )
    .unwrap();
    command.action_id = ActionId(102);
    command.target_position = Some(Vec3f::new(2.0, 0.0, 3.0));
    command.intensity = Intensity::new(0.7).unwrap();
    let bundle = MotorCommandBundle::new(
        OrganismId(1),
        ExperienceSequenceId(4),
        Tick::new(9),
        vec![channel_command_for_action(MotorChannel::Locomotion, &command).unwrap()],
    )
    .unwrap();
    let proof = JointActionSelectionV1::new([2, 0, 0, 0, 0, 0], &bundle).unwrap();
    let decoded: JointActionSelectionV1 =
        serde_json::from_slice(&serde_json::to_vec(&proof).unwrap()).unwrap();
    assert_eq!(decoded, proof);
    decoded.validate_bundle(&bundle).unwrap();
    for mutation in 0..6 {
        let mut changed = bundle.clone();
        match mutation {
            0 => changed.channels[0].intensity = Intensity::new(0.2).unwrap(),
            1 => changed.channels[0].direction.x += 1.0,
            2 => changed.channels[0].duration_ticks = DurationTicks::new(3),
            3 => changed.channels[0].confidence = Confidence::new(0.3).unwrap(),
            4 => changed.channels[0].primitive = ActionId(103),
            _ => changed.tick = Tick::new(10),
        }
        assert!(
            proof.validate_bundle(&changed).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(JointActionSelectionV1::new([2, 0, 2, 0, 0, 0], &bundle).is_err());
    assert!(JointActionSelectionV1::new([0; 6], &bundle).is_err());
    assert!(JointActionSelectionV1::new([33, 0, 0, 0, 0, 0], &bundle).is_err());
    let mut other_target = bundle.clone();
    other_target.channels[0].target = None;
    assert!(proof.validate_bundle(&other_target).is_err());
}
