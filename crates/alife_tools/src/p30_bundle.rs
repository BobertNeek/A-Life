//! Offline bundle format for packed logs and optional markers used by replay tooling.

use std::fs;
use std::path::Path;

use alife_core::{
    PackedExperienceRecord, PackedSideBufferRecord, PackedSideBuffers, ScaffoldContractError,
    Validate,
};
use thiserror::Error;

use crate::p30_markers::{BenchmarkMarker, ScenarioMarker};

pub const P30_OFFLINE_BUNDLE_SCHEMA: &str = "alife.p30.offline_log_bundle.v1";
pub const P30_OFFLINE_BUNDLE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Default)]
pub struct BundleConfig {
    pub source: Option<String>,
    pub notes: Vec<String>,
    pub scenario_markers: Vec<ScenarioMarker>,
    pub benchmark_markers: Vec<BenchmarkMarker>,
}

#[derive(Debug, Error)]
pub enum PackedLogBundleValidationError {
    #[error("bundle schema mismatch: expected '{expected}', got '{actual}'")]
    WrongSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("bundle schema version mismatch: expected {expected}, got {actual}")]
    WrongSchemaVersion { expected: u16, actual: u16 },
    #[error("bundle contract failed: {0}")]
    Contract(#[from] ScaffoldContractError),
}

#[derive(Debug, Error)]
pub enum PackedLogBundleError {
    #[error("failed to read bundle file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse bundle JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid bundle file: {0}")]
    Validation(#[from] PackedLogBundleValidationError),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackedLogBundle {
    pub schema: String,
    pub schema_version: u16,
    pub source: Option<String>,
    pub notes: Vec<String>,
    pub records: Vec<PackedLogBundleRecord>,
    pub scenario_markers: Vec<ScenarioMarker>,
    pub benchmark_markers: Vec<BenchmarkMarker>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackedLogBundleRecord {
    pub frame: alife_core::PackedExperienceFrame,
    pub side_buffers: Vec<PackedSideBufferRecord>,
}

impl From<PackedExperienceRecord> for PackedLogBundleRecord {
    fn from(value: PackedExperienceRecord) -> Self {
        Self {
            frame: value.frame,
            side_buffers: value.side_buffers.records().to_vec(),
        }
    }
}

impl TryFrom<PackedLogBundleRecord> for PackedExperienceRecord {
    type Error = ScaffoldContractError;

    fn try_from(value: PackedLogBundleRecord) -> Result<Self, Self::Error> {
        let frame = value.frame;
        frame.validate_contract()?;
        Ok(Self {
            frame,
            side_buffers: PackedSideBuffers::from_records(value.side_buffers)?,
        })
    }
}

impl PackedLogBundleRecord {
    pub fn validate(&self) -> Result<(), PackedLogBundleValidationError> {
        self.frame.validate_contract()?;
        let frame_len = self.side_buffers.len();
        let side_buffer_total = frame_len;
        self.frame
            .side_buffer_spans
            .validate_against_len(side_buffer_total)?;
        for side_buffer in &self.side_buffers {
            side_buffer.validate_contract()?;
        }
        Ok(())
    }
}

impl PackedLogBundle {
    pub fn from_records(records: Vec<PackedExperienceRecord>, config: BundleConfig) -> Self {
        Self {
            schema: P30_OFFLINE_BUNDLE_SCHEMA.to_string(),
            schema_version: P30_OFFLINE_BUNDLE_SCHEMA_VERSION,
            source: config.source,
            notes: config.notes,
            records: records
                .into_iter()
                .map(PackedLogBundleRecord::from)
                .collect(),
            scenario_markers: config.scenario_markers,
            benchmark_markers: config.benchmark_markers,
        }
    }

    pub fn validate(&self) -> Result<(), PackedLogBundleValidationError> {
        if self.schema != P30_OFFLINE_BUNDLE_SCHEMA {
            return Err(PackedLogBundleValidationError::WrongSchema {
                expected: P30_OFFLINE_BUNDLE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.schema_version != P30_OFFLINE_BUNDLE_SCHEMA_VERSION {
            return Err(PackedLogBundleValidationError::WrongSchemaVersion {
                expected: P30_OFFLINE_BUNDLE_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        for record in &self.records {
            record.validate()?;
        }
        Ok(())
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, PackedLogBundleError> {
        let text = fs::read_to_string(path)?;
        let bundle: Self = serde_json::from_str(&text)?;
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn to_json_file(&self, path: impl AsRef<Path>) -> Result<(), PackedLogBundleError> {
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text)?;
        Ok(())
    }

    pub fn to_records(
        &self,
    ) -> Result<Vec<PackedExperienceRecord>, PackedLogBundleValidationError> {
        self.validate()?;
        let mut records = Vec::with_capacity(self.records.len());
        for record in &self.records {
            records.push(
                PackedExperienceRecord::try_from(record.clone())
                    .map_err(PackedLogBundleValidationError::Contract)?,
            );
        }
        Ok(records)
    }
}

pub fn read_packed_records_json_file(
    path: impl AsRef<Path>,
) -> Result<Vec<PackedLogBundleRecord>, PackedLogBundleError> {
    let text = fs::read_to_string(path)?;
    let records: Vec<PackedExperienceRecord> = serde_json::from_str(&text)?;
    records
        .into_iter()
        .map(|record| {
            let record: PackedLogBundleRecord = record.into();
            record.validate()?;
            Ok(record)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        read_packed_records_json_file, BundleConfig, PackedLogBundle, PackedLogBundleError,
        PackedLogBundleRecord, PackedLogBundleValidationError,
    };
    use alife_core::{
        PackedExperienceFrame, PACKED_DRIVE_SUMMARY_CHANNELS, PACKED_HORMONE_SUMMARY_CHANNELS,
    };
    use std::path::PathBuf;

    fn valid_record() -> PackedLogBundleRecord {
        let mut frame = PackedExperienceFrame {
            schema_version: alife_core::PackedExperienceFrame::SCHEMA_VERSION,
            experience_schema_version: alife_core::SchemaVersions::CURRENT.experience.0,
            sensory_abi_version: 1,
            action_abi_version: 2,
            flags: 0,
            reserved_header: 0,
            organism_id: 1,
            sequence_id: 1,
            pre_action_tick: 1,
            decision_tick: 2,
            outcome_tick: 3,
            brain_class_id: 1,
            brain_scale_tier_code: 1,
            selected_action_kind_code: 1,
            reserved_kind: 0,
            selected_action_id: 2,
            action_duration_ticks: 1,
            action_source_mask: 0,
            target_entity_id: 0,
            position: [0.0; 3],
            heading_quat: [0.0, 0.0, 0.0, 1.0],
            target_position: [0.0; 3],
            drive_summary: [0.0; PACKED_DRIVE_SUMMARY_CHANNELS],
            hormone_summary: [0.0; PACKED_HORMONE_SUMMARY_CHANNELS],
            action_intensity: 0.0,
            action_confidence: 1.0,
            decision_confidence: 1.0,
            reward_valence: 0.1,
            frustration_delta: 0.0,
            pain_delta: 0.0,
            energy_delta: 0.0,
            prediction_error: 0.0,
            salience_summary: 0.0,
            memory_expected_valence: 0.0,
            memory_salience_hint: 0.0,
            side_buffer_spans: alife_core::PackedSideBufferSpans::EMPTY,
            reserved: [0; alife_core::PACKED_EXPERIENCE_FRAME_RESERVED_U32S],
        };
        let span = alife_core::PackedSideBufferSpans {
            visible_entities: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            touched_entities: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            heard_tokens: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            salience_clusters: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            memory_links: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            concept_links: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            ranked_action_proposals: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            arbitration_details: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            semantic_codes: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            gaussian_refs: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            teacher_school_refs: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
            diagnostic_extras: alife_core::SideBufferSpan {
                offset: 0,
                count: 0,
            },
        };
        frame.side_buffer_spans = span;
        PackedLogBundleRecord {
            frame,
            side_buffers: Vec::new(),
        }
    }

    fn packed_records_file_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("alife_p30_{name}.json"))
    }

    #[test]
    fn schema_version_mismatch_is_rejected() {
        let mut bundle = PackedLogBundle::from_records(vec![], BundleConfig::default());
        bundle.schema_version = 7;
        assert!(matches!(
            bundle.validate(),
            Err(PackedLogBundleValidationError::WrongSchemaVersion {
                expected: 1,
                actual: 7
            })
        ));
    }

    #[test]
    fn rejects_invalid_side_buffer_spans() {
        let mut record = valid_record();
        record.frame.side_buffer_spans.visible_entities.count = 1;
        let bundle = PackedLogBundle {
            schema: super::P30_OFFLINE_BUNDLE_SCHEMA.to_string(),
            schema_version: super::P30_OFFLINE_BUNDLE_SCHEMA_VERSION,
            source: None,
            notes: Vec::new(),
            records: vec![record],
            scenario_markers: Vec::new(),
            benchmark_markers: Vec::new(),
        };
        assert!(bundle.validate().is_err());
    }

    #[test]
    fn roundtrips_json_bundle() {
        let bundle = PackedLogBundle::from_records(vec![], BundleConfig::default());
        let path = std::env::temp_dir().join("alife_p30_bundle_roundtrip.json");
        bundle.to_json_file(&path).unwrap();
        let parsed = PackedLogBundle::from_json_file(&path).unwrap();
        assert_eq!(parsed.schema, bundle.schema);
        assert_eq!(parsed.records.len(), 0);
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn reads_and_validates_packed_records_json() {
        let record = valid_record();
        let path = packed_records_file_path("records_reader");
        std::fs::write(
            &path,
            serde_json::to_string(&vec![alife_core::PackedExperienceRecord {
                frame: record.frame,
                side_buffers: alife_core::PackedSideBuffers::from_records(record.side_buffers)
                    .unwrap(),
            }])
            .unwrap(),
        )
        .unwrap();
        let records = read_packed_records_json_file(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].validate().is_ok());
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn reads_records_json_rejects_invalid_log_schema() {
        let record = valid_record();
        let path = packed_records_file_path("records_schema_mismatch");
        let mut bad = record;
        bad.frame.schema_version = 0;
        std::fs::write(
            &path,
            serde_json::to_string(&vec![alife_core::PackedExperienceRecord {
                frame: bad.frame,
                side_buffers: alife_core::PackedSideBuffers::from_records(bad.side_buffers)
                    .unwrap(),
            }])
            .unwrap(),
        )
        .unwrap();
        let err = read_packed_records_json_file(&path).unwrap_err();
        assert!(matches!(err, PackedLogBundleError::Validation(_)));
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn invalid_schema_rejected_with_clear_error() {
        let text = r#"{
            "schema":"wrong.schema",
            "schema_version":1,
            "source":null,
            "notes":[],
            "records":[],
            "scenario_markers":[],
            "benchmark_markers":[]
        }"#;
        let path = std::env::temp_dir().join("alife_p30_invalid_bundle.json");
        std::fs::write(&path, text).unwrap();
        let err = PackedLogBundle::from_json_file(&path).unwrap_err();
        assert!(matches!(err, super::PackedLogBundleError::Validation(_)));
        std::fs::remove_file(path).expect("cleanup");
    }
}
