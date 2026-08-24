//! Deterministic, fail-closed transitions for the v2 architecture repair.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    BodyState, CanonicalDigestBuilder, LegacyLobeKindV1, LobeKind, NeuromodulatorSample,
    ScaffoldContractError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchitectureMigrationKind {
    FounderRegionLayout,
    NeuromodulatoryFrame,
    TypedOrganPhysiology,
    OrganismStateGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureMigrationReceipt {
    pub kind: ArchitectureMigrationKind,
    pub source_schema_version: u16,
    pub target_schema_version: u16,
    pub input_digest: [u64; 4],
    pub output_digest: [u64; 4],
    pub preserved_runtime_indices: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArchitectureMigrationError {
    #[error("unsupported {kind:?} migration from schema {from} to {to}")]
    UnsupportedTransition {
        kind: ArchitectureMigrationKind,
        from: u16,
        to: u16,
    },
    #[error("legacy state is missing authoritative v3 subsystem data: {field}")]
    MissingAuthoritativeState { field: &'static str },
    #[error("legacy derived scalar does not match its causal components")]
    DerivedScalarMismatch,
    #[error(
        "legacy learned/runtime neural topology requires an explicit index-preserving remap asset"
    )]
    LearnedTopologyRemapRequired,
    #[error(transparent)]
    Contract(#[from] ScaffoldContractError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyLobeRangeV1 {
    pub legacy_kind_raw: u16,
    pub start: u32,
    pub len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedFounderRangeV2 {
    pub legacy_kind: LegacyLobeKindV1,
    pub founder_homologue: LobeKind,
    pub start: u32,
    pub len: u32,
}

pub fn migrate_founder_region_ranges_v1_to_v2(
    source_schema_version: u16,
    target_schema_version: u16,
    ranges: &[LegacyLobeRangeV1],
) -> Result<(Vec<MigratedFounderRangeV2>, ArchitectureMigrationReceipt), ArchitectureMigrationError>
{
    require_transition(
        ArchitectureMigrationKind::FounderRegionLayout,
        source_schema_version,
        target_schema_version,
        1,
        2,
    )?;
    if ranges.is_empty() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let mut previous_end = 0_u32;
    let mut input = CanonicalDigestBuilder::new(b"alife.migration.regions.v1");
    input.write_sequence_len(ranges.len());
    let mut migrated = Vec::with_capacity(ranges.len());
    for range in ranges {
        if range.len == 0 || range.start % 16 != 0 || range.len % 16 != 0 {
            return Err(ScaffoldContractError::LobeAlignment.into());
        }
        if !migrated.is_empty() && range.start != previous_end {
            return Err(ScaffoldContractError::LobeRangeCoverage.into());
        }
        let legacy_kind = LegacyLobeKindV1::try_from_raw(range.legacy_kind_raw)?;
        input.write_u16(range.legacy_kind_raw);
        input.write_u32(range.start);
        input.write_u32(range.len);
        migrated.push(MigratedFounderRangeV2 {
            legacy_kind,
            founder_homologue: legacy_kind.migrate_to_founder(),
            start: range.start,
            len: range.len,
        });
        previous_end = range
            .start
            .checked_add(range.len)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
    }
    let mut output = CanonicalDigestBuilder::new(b"alife.migration.regions.v2");
    output.write_sequence_len(migrated.len());
    for range in &migrated {
        output.write_u16(range.legacy_kind.raw());
        output.write_u16(range.founder_homologue.raw());
        output.write_u32(range.start);
        output.write_u32(range.len);
    }
    Ok((
        migrated,
        ArchitectureMigrationReceipt {
            kind: ArchitectureMigrationKind::FounderRegionLayout,
            source_schema_version,
            target_schema_version,
            input_digest: input.finish256(),
            output_digest: output.finish256(),
            preserved_runtime_indices: true,
        },
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LegacyNeuromodulatorV2 {
    pub prediction_residual: f32,
    pub pain: f32,
    pub homeostatic_improvement: f32,
    pub frustration: f32,
    pub novelty: f32,
    pub derived_value: f32,
}

pub fn migrate_neuromodulator_v2_to_v3(
    source_schema_version: u16,
    target_schema_version: u16,
    legacy: LegacyNeuromodulatorV2,
) -> Result<(NeuromodulatorSample, ArchitectureMigrationReceipt), ArchitectureMigrationError> {
    require_transition(
        ArchitectureMigrationKind::NeuromodulatoryFrame,
        source_schema_version,
        target_schema_version,
        2,
        3,
    )?;
    let migrated = NeuromodulatorSample::from_components(
        legacy.prediction_residual,
        legacy.pain,
        legacy.homeostatic_improvement,
        legacy.frustration,
        legacy.novelty,
    )?;
    let expected = (0.75 * legacy.homeostatic_improvement - legacy.pain - 0.5 * legacy.frustration
        + 0.2 * legacy.novelty * legacy.prediction_residual)
        .clamp(-1.0, 1.0);
    if !legacy.derived_value.is_finite() || expected.to_bits() != legacy.derived_value.to_bits() {
        return Err(ArchitectureMigrationError::DerivedScalarMismatch);
    }
    let mut input = CanonicalDigestBuilder::new(b"alife.migration.learning.v2");
    for value in [
        legacy.prediction_residual,
        legacy.pain,
        legacy.homeostatic_improvement,
        legacy.frustration,
        legacy.novelty,
        legacy.derived_value,
    ] {
        input.write_f32(value)?;
    }
    let mut output = CanonicalDigestBuilder::new(b"alife.migration.learning.v3");
    for lane in migrated.frame().lanes() {
        output.write_f32(*lane)?;
    }
    Ok((
        migrated,
        ArchitectureMigrationReceipt {
            kind: ArchitectureMigrationKind::NeuromodulatoryFrame,
            source_schema_version,
            target_schema_version,
            input_digest: input.finish256(),
            output_digest: output.finish256(),
            preserved_runtime_indices: false,
        },
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LegacyBodyStateV2 {
    pub energy: f32,
    pub health: f32,
    pub injury: f32,
    pub temperature_stress: f32,
    pub sleeping: bool,
}

pub fn migrate_body_v2_to_v3(
    source_schema_version: u16,
    target_schema_version: u16,
    legacy: LegacyBodyStateV2,
) -> Result<(BodyState, ArchitectureMigrationReceipt), ArchitectureMigrationError> {
    require_transition(
        ArchitectureMigrationKind::TypedOrganPhysiology,
        source_schema_version,
        target_schema_version,
        2,
        3,
    )?;
    let migrated = BodyState::migrate_legacy_v1(
        legacy.energy,
        legacy.health,
        legacy.injury,
        legacy.temperature_stress,
        legacy.sleeping,
    )?;
    let mut input = CanonicalDigestBuilder::new(b"alife.migration.body.v2");
    for value in [
        legacy.energy,
        legacy.health,
        legacy.injury,
        legacy.temperature_stress,
    ] {
        input.write_f32(value)?;
    }
    input.write_bool(legacy.sleeping);
    let mut output = CanonicalDigestBuilder::new(b"alife.migration.body.v3");
    for organ in migrated.organs() {
        output.write_u8(organ.kind as u8);
        output.write_f32(organ.energy)?;
        output.write_f32(organ.integrity)?;
        output.write_f32(organ.damage)?;
        output.write_f32(organ.temperature_stress)?;
        output.write_f32(organ.repair_capacity)?;
        output.write_u32(organ.cadence_ticks);
        output.write_f32(organ.energetic_cost)?;
        output.write_u16(organ.exposed_locus);
    }
    Ok((
        migrated,
        ArchitectureMigrationReceipt {
            kind: ArchitectureMigrationKind::TypedOrganPhysiology,
            source_schema_version,
            target_schema_version,
            input_digest: input.finish256(),
            output_digest: output.finish256(),
            preserved_runtime_indices: false,
        },
    ))
}

/// Whole-organism v2 saves did not contain the new authoritative graph fields.
/// Callers must provide those fields through a format-specific decoder; this
/// check refuses to invent them.
pub fn require_complete_v3_organism_state(
    source_schema_version: u16,
    target_schema_version: u16,
    has_genetic_biochemical_graph: bool,
    has_embodiment_state: bool,
    has_subsystem_state_graph: bool,
) -> Result<(), ArchitectureMigrationError> {
    require_transition(
        ArchitectureMigrationKind::OrganismStateGraph,
        source_schema_version,
        target_schema_version,
        2,
        3,
    )?;
    for (present, field) in [
        (has_genetic_biochemical_graph, "genetic_biochemical_graph"),
        (has_embodiment_state, "embodiment_state"),
        (has_subsystem_state_graph, "organism_subsystem_state_graph"),
    ] {
        if !present {
            return Err(ArchitectureMigrationError::MissingAuthoritativeState { field });
        }
    }
    Ok(())
}

fn require_transition(
    kind: ArchitectureMigrationKind,
    source: u16,
    target: u16,
    expected_source: u16,
    expected_target: u16,
) -> Result<(), ArchitectureMigrationError> {
    if source == expected_source && target == expected_target {
        Ok(())
    } else {
        Err(ArchitectureMigrationError::UnsupportedTransition {
            kind,
            from: source,
            to: target,
        })
    }
}
