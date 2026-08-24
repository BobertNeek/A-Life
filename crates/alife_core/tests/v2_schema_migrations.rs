use alife_core::{
    migrate_body_v2_to_v3, migrate_founder_region_ranges_v1_to_v2, migrate_neuromodulator_v2_to_v3,
    require_complete_v3_organism_state, ArchitectureMigrationError, ArchitectureMigrationKind,
    LegacyBodyStateV2, LegacyLobeRangeV1, LegacyNeuromodulatorV2, LobeKind, Validate,
};

#[test]
fn legacy_region_migration_is_deterministic_and_preserves_runtime_ranges() {
    let ranges = [
        LegacyLobeRangeV1 {
            legacy_kind_raw: 1,
            start: 0,
            len: 64,
        },
        LegacyLobeRangeV1 {
            legacy_kind_raw: 2,
            start: 64,
            len: 32,
        },
        LegacyLobeRangeV1 {
            legacy_kind_raw: 3,
            start: 96,
            len: 32,
        },
    ];
    let first = migrate_founder_region_ranges_v1_to_v2(1, 2, &ranges).unwrap();
    let second = migrate_founder_region_ranges_v1_to_v2(1, 2, &ranges).unwrap();
    assert_eq!(first, second);
    assert!(first.1.preserved_runtime_indices);
    assert_eq!(
        first.0[0].founder_homologue,
        LobeKind::PerceptualIntegration
    );
    assert_eq!(first.0[2].founder_homologue, LobeKind::SocialCommunication);
    assert_eq!(first.0[2].start, ranges[2].start);
}

#[test]
fn scalar_modulator_migrates_only_when_legacy_derivation_is_exact() {
    let mut legacy = LegacyNeuromodulatorV2 {
        prediction_residual: 0.2,
        pain: 0.1,
        homeostatic_improvement: 0.4,
        frustration: 0.2,
        novelty: 0.5,
        derived_value: 0.0,
    };
    legacy.derived_value =
        (0.75 * legacy.homeostatic_improvement - legacy.pain - 0.5 * legacy.frustration
            + 0.2 * legacy.novelty * legacy.prediction_residual)
            .clamp(-1.0, 1.0);
    let first = migrate_neuromodulator_v2_to_v3(2, 3, legacy).unwrap();
    let second = migrate_neuromodulator_v2_to_v3(2, 3, legacy).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.0.frame().lanes()[0], legacy.prediction_residual);

    legacy.derived_value += 0.01;
    assert_eq!(
        migrate_neuromodulator_v2_to_v3(2, 3, legacy).unwrap_err(),
        ArchitectureMigrationError::DerivedScalarMismatch
    );
}

#[test]
fn legacy_body_migration_builds_valid_typed_organs_deterministically() {
    let legacy = LegacyBodyStateV2 {
        energy: 0.7,
        health: 0.8,
        injury: 0.2,
        temperature_stress: 0.1,
        sleeping: false,
    };
    let first = migrate_body_v2_to_v3(2, 3, legacy).unwrap();
    let second = migrate_body_v2_to_v3(2, 3, legacy).unwrap();
    assert_eq!(first, second);
    first.0.validate_contract().unwrap();
    assert_eq!(first.0.organs().len(), 6);
}

#[test]
fn missing_new_authority_is_an_explicit_failure_not_a_default() {
    let error = require_complete_v3_organism_state(2, 3, true, false, true).unwrap_err();
    assert_eq!(
        error,
        ArchitectureMigrationError::MissingAuthoritativeState {
            field: "embodiment_state"
        }
    );
    assert!(matches!(
        require_complete_v3_organism_state(1, 3, true, true, true),
        Err(ArchitectureMigrationError::UnsupportedTransition {
            kind: ArchitectureMigrationKind::OrganismStateGraph,
            ..
        })
    ));
}
