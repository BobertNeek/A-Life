use alife_core::{
    ensure_current_version, ActionCommand, ActionKind, Confidence, ContractDiagnostic,
    DiagnosticCode, DurationTicks, ExperiencePatchHeader, ExperienceSequenceId,
    LineageExportManifest, LineageId, OrganismId, ScaffoldContractError, SchemaKind,
    SchemaVersions, Tick, Validate, Validated, WorldEntityId,
};

#[test]
fn central_versions_are_shared_by_placeholder_abis() {
    assert_eq!(
        ActionCommand::ABI_VERSION,
        SchemaVersions::CURRENT.action_abi.raw()
    );
    assert_eq!(
        ExperiencePatchHeader::ABI_VERSION,
        SchemaVersions::CURRENT.experience.raw()
    );
    assert_eq!(
        LineageExportManifest::ABI_VERSION,
        SchemaVersions::CURRENT.lineage_export.raw()
    );
    assert!(ensure_current_version(SchemaKind::ActionAbi, ActionCommand::ABI_VERSION).is_ok());
}

#[test]
fn incompatible_abi_returns_typed_error_and_diagnostic() {
    let error = ensure_current_version(SchemaKind::ActionAbi, 99).unwrap_err();
    assert_eq!(
        error,
        ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::ActionAbi,
            expected: 2,
            actual: 99,
        }
    );
    assert!(format!("{error}").contains("incompatible ActionAbi version"));

    let diagnostic = ContractDiagnostic::from(&error);
    assert_eq!(diagnostic.code, DiagnosticCode::IncompatibleAbi);
    assert_eq!(diagnostic.schema, Some(SchemaKind::ActionAbi));
    assert_eq!(diagnostic.expected, Some(2));
    assert_eq!(diagnostic.actual, Some(99));
}

#[test]
fn validated_wrapper_accepts_good_contracts_and_rejects_bad_versions() {
    let command = ActionCommand::new(
        OrganismId(1),
        ActionKind::Interact,
        Some(WorldEntityId(2)),
        Confidence::new(0.8).unwrap(),
        DurationTicks::new(4),
    )
    .unwrap();
    let validated = Validated::try_new(command).unwrap();
    assert_eq!(validated.get().kind, ActionKind::Interact);

    let mut bad = *validated.get();
    bad.abi_version = 999;
    assert!(matches!(
        bad.validate_contract(),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::ActionAbi,
            expected: 2,
            actual: 999
        })
    ));
}

#[test]
fn existing_headers_and_manifests_validate_versions_and_ids() {
    let header =
        ExperiencePatchHeader::new(OrganismId(1), ExperienceSequenceId(2), Tick(3)).unwrap();
    assert!(header.validate_contract().is_ok());

    let mut bad_header = header;
    bad_header.abi_version = 2;
    assert!(bad_header.validate_contract().is_err());

    let manifest = LineageExportManifest {
        abi_version: LineageExportManifest::ABI_VERSION,
        lineage_id: LineageId(1),
        founder_genome_id: alife_core::GenomeId(2),
        source_brain_class_id: alife_core::BrainClassId(3),
        target_brain_class_id: None,
        exported_at_tick: 4,
    };
    assert!(manifest.validate_contract().is_ok());
}
