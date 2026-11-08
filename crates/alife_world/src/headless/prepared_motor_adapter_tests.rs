//! Pure adapter qualification. These fixtures never construct/step a world,
//! execute an action, advance biology, or expose a native learning life.
use super::*;

const ORGANISM: OrganismId = OrganismId(7);
const ENTITY: WorldEntityId = WorldEntityId(41);

fn record(energy: f32, health: f32, calibration: f32, body_schema: f32) -> WorldOrganismRecord {
    let genome = alife_core::CreatureGenome::early_mammal_founder(
        0xE10_3601,
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
    let mut biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    biology.body.set_energy(energy).unwrap();
    biology.body.set_health(health).unwrap();
    let mut record =
        WorldOrganismRecord::new(ORGANISM, ENTITY, genome, phenotype, biology, Tick::ZERO).unwrap();
    let mut embodiment = record.embodiment().clone();
    embodiment
        .replace_calibration(
            Tick::ZERO,
            vec![calibration; embodiment.sensor_calibration().len()],
            vec![calibration; embodiment.effector_controllability().len()],
            vec![body_schema; embodiment.body_schema().len()],
        )
        .unwrap();
    record.replace_embodiment_state(embodiment).unwrap();
    record
}

fn command(channel: MotorChannel, primitive: ActionId, intensity: f32) -> ChannelCommand {
    ChannelCommand::new(
        channel,
        primitive,
        Some(alife_core::ActionTarget::new(
            Some(WorldEntityId(71)),
            Some(Vec3f::new(0.2, -0.0, -0.4)),
        )),
        Vec3f::new(0.3, -0.0, 0.4),
        Intensity::new(intensity).unwrap(),
        alife_core::DurationTicks::new(3),
        0.125,
        Confidence::new(0.625).unwrap(),
        2,
    )
    .unwrap()
    .with_payload(alife_core::BoundedMotorPayload::new(vec![3, 5, 8]).unwrap())
    .unwrap()
}

// Frozen pre-extraction arithmetic, only in tests. Do not use the new helper or
// the production capability selector to calculate the expected command.
fn old_adapter(
    command: &ChannelCommand,
    embodiment: &EmbodimentState,
    locomotor_capacity: Option<f32>,
) -> Result<ChannelCommand, HeadlessMotorTransactionError> {
    let capability = match command.channel {
        MotorChannel::Locomotion => EffectorCapability::Translation,
        MotorChannel::Orientation => EffectorCapability::Rotation,
        MotorChannel::Manipulation if command.primitive == HeadlessActionIds::EAT => {
            EffectorCapability::Ingestion
        }
        MotorChannel::Manipulation | MotorChannel::SpeciesSpecific(_) => {
            EffectorCapability::Manipulation
        }
        MotorChannel::Vocal => EffectorCapability::Vocalization,
        MotorChannel::Posture => EffectorCapability::Rest,
    };
    let mut gain = embodiment.effector_gain(capability);
    if gain == 0.0 {
        return Err(HeadlessMotorTransactionError::UnsupportedChannel(
            command.channel,
        ));
    }
    if matches!(
        capability,
        EffectorCapability::Translation | EffectorCapability::Rotation
    ) {
        gain *= embodiment.proprioceptive_gain();
    }
    let mut adapted = command.clone();
    let intensity = (adapted.intensity.raw() * gain).clamp(0.0, 1.0);
    adapted.intensity = Intensity::new(if capability == EffectorCapability::Translation {
        intensity * locomotor_capacity.unwrap_or(1.0)
    } else {
        intensity
    })?;
    adapted.validate_contract()?;
    Ok(adapted)
}

fn old_bundle_path(
    command: &ChannelCommand,
    record: &WorldOrganismRecord,
) -> Result<ChannelCommand, HeadlessMotorTransactionError> {
    let embodiment = record.embodiment().clone();
    let biology_before = *record.biochemistry();
    let phenotype = record.phenotype();
    let locomotor_capacity = biology_before
        .graph_state()
        .locomotor_capacity(&phenotype.chemistry.biochemical)?
        .map(|chemical| {
            let organ = biology_before.body.organ(alife_core::OrganKind::Locomotor);
            let reserve =
                (organ.energy / (0.5 + 0.5 * phenotype.body.metabolic_efficiency)).clamp(0.0, 1.0);
            chemical * reserve * organ.integrity
        });
    old_adapter(command, &embodiment, locomotor_capacity)
}

fn assert_identical(
    actual: Result<ChannelCommand, HeadlessMotorTransactionError>,
    expected: Result<ChannelCommand, HeadlessMotorTransactionError>,
) {
    match (actual, expected) {
        (Ok(actual), Ok(expected)) => {
            assert_eq!(actual, expected);
            assert_eq!(
                actual.intensity.raw().to_bits(),
                expected.intensity.raw().to_bits()
            );
            assert_eq!(
                serde_json::to_vec(&actual).unwrap(),
                serde_json::to_vec(&expected).unwrap()
            );
        }
        (Err(actual), Err(expected)) => assert_eq!(actual, expected),
        pair => panic!("adapter result differs: {pair:?}"),
    }
}

#[test]
fn prepared_motor_adapter_matches_old_bytes_all_channels_and_record_states() {
    let channels = [
        (MotorChannel::Posture, ActionKind::Rest.canonical_id()),
        (MotorChannel::Posture, ActionKind::Inspect.canonical_id()),
        (MotorChannel::Locomotion, HeadlessActionIds::NO_LOCOMOTION),
        (MotorChannel::Locomotion, HeadlessActionIds::STEP_FORWARD),
        (MotorChannel::Locomotion, HeadlessActionIds::APPROACH),
        (MotorChannel::Orientation, HeadlessActionIds::TURN_LEFT),
        (MotorChannel::Manipulation, HeadlessActionIds::EAT),
        (MotorChannel::Vocal, ActionKind::Vocalize.canonical_id()),
        (MotorChannel::SpeciesSpecific(3), HeadlessActionIds::GRAB),
    ];
    for (energy, health, calibration, schema) in [
        (0.0, 1.0, -1.0, -1.0),
        (0.2, 0.7, 0.0, 1.0),
        (1.0, 1.0, 1.0, 1.0),
        (1.0, 0.0, 1.0, -1.0),
        (0.6, 0.4, -1.0, 0.0),
    ] {
        let record = record(energy, health, calibration, schema);
        let before = serde_json::to_vec(&record).unwrap();
        let adapter = PreparedMotorChannelAdapter::from_record(&record).unwrap();
        for (channel, primitive) in channels {
            for intensity in [0.0, -0.0, f32::from_bits(1), 0.25, 0.9, 1.0] {
                let command = command(channel, primitive, intensity);
                let request_before = serde_json::to_vec(&command).unwrap();
                assert_identical(adapter.adapt(&command), old_bundle_path(&command, &record));
                let adapted = adapter.adapt(&command).unwrap();
                // Every field except intensity must remain byte-identical.
                let mut preserved = command.clone();
                preserved.intensity = adapted.intensity;
                assert_eq!(
                    serde_json::to_vec(&adapted).unwrap(),
                    serde_json::to_vec(&preserved).unwrap()
                );
                assert_eq!(serde_json::to_vec(&command).unwrap(), request_before);
            }
        }
        assert_eq!(serde_json::to_vec(&record).unwrap(), before);
        assert_eq!(record.biochemistry().tick, Tick::ZERO);
    }
}

#[test]
fn prepared_motor_adapter_preserves_clamp_then_capacity_and_legacy_none() {
    let record = record(1.0, 1.0, 1.0, 1.0);
    let movement = command(
        MotorChannel::Locomotion,
        HeadlessActionIds::STEP_FORWARD,
        1.0,
    );
    let rest = command(MotorChannel::Posture, ActionKind::Rest.canonical_id(), 1.0);
    // Explicit immutable capacities isolate the old multiplication/clamp order;
    // record-based capacity calculation is separately covered above.
    for capacity in [None, Some(0.0), Some(0.25), Some(1.0)] {
        let adapter = PreparedMotorChannelAdapter {
            embodiment: record.embodiment().clone(),
            locomotor_capacity: capacity,
        };
        assert_identical(
            adapter.adapt(&movement),
            old_adapter(&movement, record.embodiment(), capacity),
        );
        assert_eq!(
            adapter.adapt(&movement).unwrap().intensity.raw().to_bits(),
            capacity.unwrap_or(1.0).to_bits()
        );
        assert_eq!(adapter.adapt(&rest).unwrap().intensity.raw(), 1.0);
    }
    let weak_record = record_with_zero_energy();
    let chemical = weak_record
        .biochemistry()
        .graph_state()
        .locomotor_capacity(&weak_record.phenotype().chemistry.biochemical)
        .unwrap();
    let adapted = PreparedMotorChannelAdapter::from_record(&weak_record)
        .unwrap()
        .adapt(&movement)
        .unwrap();
    if chemical.is_some() {
        assert_eq!(adapted.intensity.raw(), 0.0);
    } else {
        assert_identical(Ok(adapted), old_bundle_path(&movement, &weak_record));
    }
}

fn record_with_zero_energy() -> WorldOrganismRecord {
    record(0.0, 1.0, 1.0, 1.0)
}

#[test]
fn prepared_motor_adapter_preserves_missing_capability_errors_and_input_bounds() {
    let mut record = record(1.0, 1.0, 0.0, 0.0);
    // Reference embodiment lacks Rest and Manipulation effectors. Give it the
    // next valid revision without a world step, then bind it through record API.
    let mut limited = EmbodimentState::reference(ENTITY, Tick::ZERO).unwrap();
    limited
        .replace_calibration(
            Tick::ZERO,
            limited.sensor_calibration().to_vec(),
            limited.effector_controllability().to_vec(),
            limited.body_schema().to_vec(),
        )
        .unwrap();
    // The fixture's first calibration already incremented record revision once.
    limited
        .replace_calibration(
            Tick::ZERO,
            limited.sensor_calibration().to_vec(),
            limited.effector_controllability().to_vec(),
            limited.body_schema().to_vec(),
        )
        .unwrap();
    record.replace_embodiment_state(limited).unwrap();
    let adapter = PreparedMotorChannelAdapter::from_record(&record).unwrap();
    for (channel, primitive) in [
        (MotorChannel::Posture, ActionKind::Rest.canonical_id()),
        (
            MotorChannel::Manipulation,
            ActionKind::Inspect.canonical_id(),
        ),
    ] {
        let command = command(channel, primitive, 0.0);
        assert_eq!(
            adapter.adapt(&command),
            Err(HeadlessMotorTransactionError::UnsupportedChannel(channel))
        );
        assert_identical(adapter.adapt(&command), old_bundle_path(&command, &record));
    }
    let valid = command(
        MotorChannel::Locomotion,
        HeadlessActionIds::STEP_FORWARD,
        0.5,
    );
    let mut invalids = Vec::new();
    let mut invalid = valid.clone();
    invalid.primitive = ActionId(0);
    invalids.push(invalid);
    let mut invalid = valid.clone();
    invalid.stand_off_distance = -0.1;
    invalids.push(invalid);
    let mut invalid = valid.clone();
    invalid.direction = Vec3f::new(f32::NAN, 0.0, 0.0);
    invalids.push(invalid);
    let mut invalid = valid;
    invalid.payload.values = vec![0; alife_core::MAX_MOTOR_PAYLOAD_VALUES + 1];
    invalids.push(invalid);
    for invalid in invalids {
        assert!(adapter.adapt(&invalid).is_err());
        assert_identical(adapter.adapt(&invalid), old_bundle_path(&invalid, &record));
    }
}

#[test]
fn prepared_motor_adapter_keeps_noop_identity_and_prepared_snapshot() {
    let mut record = record(1.0, 1.0, -1.0, -1.0);
    let adapter = PreparedMotorChannelAdapter::from_record(&record).unwrap();
    let noop = command(
        MotorChannel::Locomotion,
        HeadlessActionIds::NO_LOCOMOTION,
        0.5,
    );
    let movement = command(
        MotorChannel::Locomotion,
        HeadlessActionIds::STEP_FORWARD,
        0.5,
    );
    let adapted_noop = adapter.adapt(&noop).unwrap();
    let adapted_movement = adapter.adapt(&movement).unwrap();
    let rest = command(MotorChannel::Posture, ActionKind::Rest.canonical_id(), 0.5);
    let adapted_rest = adapter.adapt(&rest).unwrap();
    assert_eq!(
        adapted_noop.intensity.raw().to_bits(),
        adapted_movement.intensity.raw().to_bits()
    );
    assert_ne!(adapted_noop.primitive, adapted_movement.primitive);
    assert_ne!(
        serde_json::to_vec(&adapted_noop).unwrap(),
        serde_json::to_vec(&adapted_movement).unwrap()
    );
    // A later calibration cannot retroactively change the prepared adapter.
    let mut embodiment = record.embodiment().clone();
    embodiment
        .replace_calibration(
            Tick::ZERO,
            embodiment.sensor_calibration().to_vec(),
            vec![1.0; embodiment.effector_controllability().len()],
            vec![1.0; embodiment.body_schema().len()],
        )
        .unwrap();
    record.replace_embodiment_state(embodiment).unwrap();
    assert_eq!(adapter.adapt(&movement).unwrap(), adapted_movement);
    let fresh = PreparedMotorChannelAdapter::from_record(&record).unwrap();
    assert_identical(fresh.adapt(&movement), old_bundle_path(&movement, &record));
    assert_eq!(adapter.adapt(&rest).unwrap(), adapted_rest);
    assert!(fresh.adapt(&rest).unwrap().intensity.raw() > adapted_rest.intensity.raw());
    assert_eq!(record.biochemistry().tick, Tick::ZERO);
}
