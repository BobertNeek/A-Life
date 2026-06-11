use alife_core::{
    ensure_current_version, ChemistryModulation, Confidence, DriveDelta, DriveSnapshot,
    EndocrineDelta, EndocrineSnapshot, HomeostaticCadence, HomeostaticCadenceBand,
    HomeostaticDelta, HomeostaticParameters, HomeostaticSnapshot, RecoveryTrigger,
    ScaffoldContractError, SchemaKind, SchemaVersions, Tick, Validate, Validated,
};

#[test]
fn homeostatic_snapshots_are_versioned_through_the_central_schema_registry() {
    let snapshot = HomeostaticSnapshot::baseline(Tick::new(1));

    assert_eq!(
        HomeostaticSnapshot::SCHEMA_VERSION,
        SchemaVersions::CURRENT.chemistry.raw()
    );
    assert_eq!(snapshot.schema_version, HomeostaticSnapshot::SCHEMA_VERSION);
    assert!(ensure_current_version(SchemaKind::Chemistry, snapshot.schema_version).is_ok());

    let mut stale = snapshot;
    stale.schema_version = 999;
    assert!(matches!(
        stale.validate_contract(),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::Chemistry,
            expected: 1,
            actual: 999
        })
    ));
}

#[test]
fn snapshots_reject_nan_and_out_of_range_learning_values() {
    let drives = DriveSnapshot::baseline();
    let hormones = EndocrineSnapshot::baseline();

    assert!(drives.validate_contract().is_ok());
    assert!(hormones.validate_contract().is_ok());
    assert!(Validated::try_new(drives).is_ok());
    assert!(Validated::try_new(hormones).is_ok());

    let mut nan_drive = drives;
    nan_drive.hunger = f32::NAN;
    assert_eq!(
        nan_drive.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    );

    let mut high_atp = drives;
    high_atp.brain_atp = 1.01;
    assert_eq!(
        high_atp.validate_contract(),
        Err(ScaffoldContractError::OutOfRangeDriveHormone)
    );

    let mut low_atp = drives;
    low_atp.brain_atp = -0.01;
    assert_eq!(
        low_atp.validate_contract(),
        Err(ScaffoldContractError::OutOfRangeDriveHormone)
    );

    let mut invalid_hormone = hormones;
    invalid_hormone.dopamine = f32::INFINITY;
    assert_eq!(
        invalid_hormone.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    );

    let mut high_hormone = hormones;
    high_hormone.learning_modulator = 1.2;
    assert_eq!(
        high_hormone.validate_contract(),
        Err(ScaffoldContractError::OutOfRangeDriveHormone)
    );
}

#[test]
fn endocrine_cadence_bands_match_the_performance_contract_without_engine_time() {
    assert_eq!(
        HomeostaticCadence::for_band(HomeostaticCadenceBand::Hot),
        HomeostaticCadence {
            min_hz: 10,
            max_hz: 30
        }
    );
    assert_eq!(
        HomeostaticCadence::for_band(HomeostaticCadenceBand::Warm),
        HomeostaticCadence {
            min_hz: 2,
            max_hz: 10
        }
    );
    assert_eq!(
        HomeostaticCadence::for_band(HomeostaticCadenceBand::Cold),
        HomeostaticCadence {
            min_hz: 0,
            max_hz: 1
        }
    );
}

#[test]
fn deltas_are_distinct_signed_changes_and_update_clamps_results() {
    let state = HomeostaticSnapshot::new(
        Tick::new(10),
        DriveSnapshot {
            brain_atp: 0.05,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot::baseline(),
    )
    .unwrap();

    let delta = HomeostaticDelta {
        drives: DriveDelta {
            hunger: 0.85,
            brain_atp: -0.25,
            ..DriveDelta::zero()
        },
        hormones: EndocrineDelta {
            dopamine: 0.75,
            ..EndocrineDelta::zero()
        },
    };

    let next = state
        .advance(Tick::new(11), delta, HomeostaticParameters::reference())
        .unwrap();

    assert_eq!(next.drives.brain_atp, 0.0);
    assert!(next.drives.hunger <= 1.0);
    assert!(next.drives.hunger > state.drives.hunger);
    assert!(next.hormones.dopamine <= 1.0);
    assert!(next.hormones.dopamine > state.hormones.dopamine);

    let invalid_delta = HomeostaticDelta {
        drives: DriveDelta {
            pain: f32::NAN,
            ..DriveDelta::zero()
        },
        hormones: EndocrineDelta::zero(),
    };
    assert_eq!(
        invalid_delta.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    );
}

#[test]
fn baseline_drift_decay_and_pain_frustration_spikes_are_deterministic() {
    let state = HomeostaticSnapshot::new(
        Tick::new(20),
        DriveSnapshot {
            pain: 0.8,
            fatigue: 0.2,
            hunger: 0.2,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot {
            adrenaline: 0.9,
            cortisol: 0.8,
            dopamine: 0.9,
            ..EndocrineSnapshot::baseline()
        },
    )
    .unwrap();

    let params = HomeostaticParameters::reference();
    let drifted = state
        .advance(Tick::new(21), HomeostaticDelta::zero(), params)
        .unwrap();

    assert!(drifted.drives.hunger > state.drives.hunger);
    assert!(drifted.drives.fatigue > state.drives.fatigue);
    assert!(drifted.drives.pain < state.drives.pain);
    assert!(drifted.hormones.adrenaline < state.hormones.adrenaline);
    assert!(drifted.hormones.cortisol < state.hormones.cortisol);
    assert!(drifted.hormones.dopamine < state.hormones.dopamine);

    let spiked = drifted
        .advance(
            Tick::new(22),
            HomeostaticDelta::pain_frustration_spike(0.35, 0.25, 0.2).unwrap(),
            params,
        )
        .unwrap();

    assert!(spiked.drives.pain > drifted.drives.pain);
    assert!(spiked.hormones.cortisol > drifted.hormones.cortisol);
    assert!(spiked.hormones.adrenaline > drifted.hormones.adrenaline);
}

#[test]
fn recovery_triggers_cover_hyperactivity_catatonia_sleep_pain_and_safe_idle() {
    let params = HomeostaticParameters::reference();

    let hyper = HomeostaticSnapshot::new(
        Tick::new(1),
        DriveSnapshot {
            fear: 0.95,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot {
            adrenaline: 0.98,
            cortisol: 0.96,
            ..EndocrineSnapshot::baseline()
        },
    )
    .unwrap();
    let hyper_recovery = ChemistryModulation::recovery_triggers(&hyper, params).unwrap();
    assert!(hyper_recovery.contains(RecoveryTrigger::SeizureHyperactivity));

    let depleted = HomeostaticSnapshot::new(
        Tick::new(2),
        DriveSnapshot {
            brain_atp: 0.03,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot::baseline(),
    )
    .unwrap();
    let depleted_recovery = ChemistryModulation::recovery_triggers(&depleted, params).unwrap();
    assert!(depleted_recovery.contains(RecoveryTrigger::CatatoniaEnergyHypoplasia));
    assert!(depleted_recovery.contains(RecoveryTrigger::SafeIdleFallback));

    let exhausted = HomeostaticSnapshot::new(
        Tick::new(3),
        DriveSnapshot {
            fatigue: 0.94,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot {
            sleep_pressure: 0.9,
            ..EndocrineSnapshot::baseline()
        },
    )
    .unwrap();
    let sleep_recovery = ChemistryModulation::recovery_triggers(&exhausted, params).unwrap();
    assert!(sleep_recovery.contains(RecoveryTrigger::FatigueSleepEntry));
    assert!(ChemistryModulation::should_enter_sleep(&exhausted, params).unwrap());

    let injured = HomeostaticSnapshot::new(
        Tick::new(4),
        DriveSnapshot {
            pain: 0.91,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot {
            cortisol: 0.88,
            ..EndocrineSnapshot::baseline()
        },
    )
    .unwrap();
    let pain_recovery = ChemistryModulation::recovery_triggers(&injured, params).unwrap();
    assert!(pain_recovery.contains(RecoveryTrigger::PainFrustrationSpike));
}

#[test]
fn modulation_helpers_bound_learning_salience_thresholds_and_motor_confidence() {
    let params = HomeostaticParameters::reference();
    let baseline = HomeostaticSnapshot::baseline(Tick::new(1));
    let stressed = HomeostaticSnapshot::new(
        Tick::new(2),
        DriveSnapshot {
            fear: 0.7,
            pain: 0.6,
            curiosity: 0.9,
            fatigue: 0.5,
            brain_atp: 0.35,
            ..DriveSnapshot::baseline()
        },
        EndocrineSnapshot {
            adrenaline: 0.8,
            cortisol: 0.75,
            dopamine: 0.65,
            learning_modulator: 0.7,
            ..EndocrineSnapshot::baseline()
        },
    )
    .unwrap();

    let baseline_threshold = ChemistryModulation::threshold_scale(&baseline, params).unwrap();
    let stressed_threshold = ChemistryModulation::threshold_scale(&stressed, params).unwrap();
    assert!(stressed_threshold > baseline_threshold);

    let learning = ChemistryModulation::learning_rate_scale(&stressed, params).unwrap();
    let salience = ChemistryModulation::salience_weight(&stressed, params).unwrap();
    assert!((0.0..=1.0).contains(&learning));
    assert!((0.0..=1.0).contains(&salience));
    assert!(salience > ChemistryModulation::salience_weight(&baseline, params).unwrap());

    let base_confidence = Confidence::new(0.8).unwrap();
    let adjusted =
        ChemistryModulation::motor_confidence(base_confidence, &stressed, params).unwrap();
    assert!((0.0..=1.0).contains(&adjusted.raw()));
    assert!(adjusted.raw() < base_confidence.raw());
}

#[test]
fn deterministic_finite_inputs_never_produce_nan_or_out_of_range_outputs() {
    let params = HomeostaticParameters::reference();
    let mut seed = 0xA17E_5EED_u64;

    for step in 1..=128 {
        let drives = DriveSnapshot {
            hunger: unit_value(&mut seed),
            fatigue: unit_value(&mut seed),
            fear: unit_value(&mut seed),
            pain: unit_value(&mut seed),
            loneliness: unit_value(&mut seed),
            curiosity: unit_value(&mut seed),
            brain_atp: unit_value(&mut seed),
            temperature_stress: unit_value(&mut seed),
            reproductive_drive: unit_value(&mut seed),
            extension: [unit_value(&mut seed), unit_value(&mut seed)],
        };
        let hormones = EndocrineSnapshot {
            adrenaline: unit_value(&mut seed),
            cortisol: unit_value(&mut seed),
            dopamine: unit_value(&mut seed),
            oxytocin: unit_value(&mut seed),
            serotonin: unit_value(&mut seed),
            acetylcholine: unit_value(&mut seed),
            learning_modulator: unit_value(&mut seed),
            developmental_hormone: unit_value(&mut seed),
            sleep_pressure: unit_value(&mut seed),
            extension: [unit_value(&mut seed), unit_value(&mut seed)],
        };
        let delta = HomeostaticDelta {
            drives: DriveDelta {
                hunger: signed_small_value(&mut seed),
                fatigue: signed_small_value(&mut seed),
                fear: signed_small_value(&mut seed),
                pain: signed_small_value(&mut seed),
                loneliness: signed_small_value(&mut seed),
                curiosity: signed_small_value(&mut seed),
                brain_atp: signed_small_value(&mut seed),
                temperature_stress: signed_small_value(&mut seed),
                reproductive_drive: signed_small_value(&mut seed),
                extension: [signed_small_value(&mut seed), signed_small_value(&mut seed)],
            },
            hormones: EndocrineDelta {
                adrenaline: signed_small_value(&mut seed),
                cortisol: signed_small_value(&mut seed),
                dopamine: signed_small_value(&mut seed),
                oxytocin: signed_small_value(&mut seed),
                serotonin: signed_small_value(&mut seed),
                acetylcholine: signed_small_value(&mut seed),
                learning_modulator: signed_small_value(&mut seed),
                developmental_hormone: signed_small_value(&mut seed),
                sleep_pressure: signed_small_value(&mut seed),
                extension: [signed_small_value(&mut seed), signed_small_value(&mut seed)],
            },
        };

        let state = HomeostaticSnapshot::new(Tick::new(step), drives, hormones).unwrap();
        let next = state
            .advance(Tick::new(step + 1), delta, params)
            .expect("finite bounded input should update");

        next.validate_contract().unwrap();
        assert!((0.0..=1.0).contains(&ChemistryModulation::threshold_scale(&next, params).unwrap()));
        assert!(
            (0.0..=1.0).contains(&ChemistryModulation::learning_rate_scale(&next, params).unwrap())
        );
        assert!((0.0..=1.0).contains(&ChemistryModulation::salience_weight(&next, params).unwrap()));
        ChemistryModulation::motor_confidence(Confidence::new(0.5).unwrap(), &next, params)
            .unwrap();
    }
}

#[test]
fn recovery_and_sleep_helpers_reject_invalid_learning_state() {
    let params = HomeostaticParameters::reference();
    let mut invalid = HomeostaticSnapshot::baseline(Tick::new(99));
    invalid.drives.fear = f32::NAN;

    assert_eq!(
        ChemistryModulation::recovery_triggers(&invalid, params),
        Err(ScaffoldContractError::NonFiniteFloat)
    );
    assert_eq!(
        ChemistryModulation::should_enter_sleep(&invalid, params),
        Err(ScaffoldContractError::NonFiniteFloat)
    );
}

fn unit_value(seed: &mut u64) -> f32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*seed >> 40) as f32) / ((1_u64 << 24) as f32)
}

fn signed_small_value(seed: &mut u64) -> f32 {
    unit_value(seed) * 0.4 - 0.2
}
