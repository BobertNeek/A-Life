//! Scenario and benchmark marker parsing for offline replay and summary tooling.
//!
//! The module is intentionally small and dependency-light: it consumes P19 scenario
//! fixtures and benchmark markdown exports only when requested by tooling.

use std::path::{Path, PathBuf};

use alife_world::ScenarioName;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const P19_GOLDEN_TRACE_SCHEMA: &str = "alife.p19.golden_trace.v1";
pub const P19_GOLDEN_TRACE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum MarkerReadError {
    #[error("failed to read marker file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse marker JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("scenario marker schema mismatch at {path:?}: expected '{expected}', got '{actual}'")]
    ScenarioSchemaMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("scenario marker schema mismatch at {path:?}: expected {expected}, got {actual}")]
    ScenarioSchemaVersionMismatch {
        path: PathBuf,
        expected: u16,
        actual: u16,
    },
    #[error("failed to parse benchmark marker in {path:?}: {message}")]
    BenchmarkParse { path: PathBuf, message: String },
    #[error("unknown scenario key '{key}' at {path:?}")]
    UnknownScenarioKey { path: PathBuf, key: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioMarker {
    pub source_path: PathBuf,
    pub scenario_key: String,
    pub scenario_seed: u64,
    pub patch_count: usize,
    pub memory_record_count: usize,
    pub topology_concept_count: usize,
    pub topology_gap_count: usize,
    pub topology_edge_count: usize,
    pub topology_simplex_count: usize,
    pub world_signature: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkMarker {
    pub source_path: PathBuf,
    pub population: u16,
    pub manual_expected_slow: bool,
    pub tick_time_ms: f64,
    pub patches_per_second: f64,
    pub success_rate: f64,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoldenTraceFixture {
    schema: String,
    schema_version: u16,
    scenario: GoldenScenario,
    state: GoldenState,
    world_signature: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoldenScenario {
    key: String,
    label: Option<String>,
    seed: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GoldenState {
    patch_count: usize,
    memory_record_count: usize,
    topology_concept_count: usize,
    topology_gap_count: usize,
    topology_edge_count: usize,
    topology_simplex_count: usize,
}

impl ScenarioMarker {
    pub fn from_fixture_path(path: impl AsRef<Path>) -> Result<Self, MarkerReadError> {
        let path = path.as_ref().to_path_buf();
        let text = std::fs::read_to_string(&path)?;
        let fixture: GoldenTraceFixture = serde_json::from_str(&text)?;

        if fixture.schema != P19_GOLDEN_TRACE_SCHEMA {
            return Err(MarkerReadError::ScenarioSchemaMismatch {
                path: path.clone(),
                expected: P19_GOLDEN_TRACE_SCHEMA.to_string(),
                actual: fixture.schema,
            });
        }
        if fixture.schema_version != P19_GOLDEN_TRACE_SCHEMA_VERSION {
            return Err(MarkerReadError::ScenarioSchemaVersionMismatch {
                path: path.clone(),
                expected: P19_GOLDEN_TRACE_SCHEMA_VERSION,
                actual: fixture.schema_version,
            });
        }
        scenario_name_from_key(&fixture.scenario.key).ok_or_else(|| {
            MarkerReadError::UnknownScenarioKey {
                path: path.clone(),
                key: fixture.scenario.key.clone(),
            }
        })?;
        let notes = fixture
            .scenario
            .label
            .unwrap_or_else(|| fixture.scenario.key.clone());

        Ok(Self {
            source_path: path,
            scenario_key: fixture.scenario.key,
            scenario_seed: fixture.scenario.seed,
            patch_count: fixture.state.patch_count,
            memory_record_count: fixture.state.memory_record_count,
            topology_concept_count: fixture.state.topology_concept_count,
            topology_gap_count: fixture.state.topology_gap_count,
            topology_edge_count: fixture.state.topology_edge_count,
            topology_simplex_count: fixture.state.topology_simplex_count,
            world_signature: fixture.world_signature,
            notes,
        })
    }

    pub fn read_many<I, P>(paths: I) -> Result<Vec<Self>, MarkerReadError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        paths
            .into_iter()
            .map(ScenarioMarker::from_fixture_path)
            .collect()
    }

    pub fn scenario_name(&self) -> Result<ScenarioName, MarkerReadError> {
        scenario_name_from_key(&self.scenario_key).ok_or_else(|| {
            MarkerReadError::UnknownScenarioKey {
                path: self.source_path.clone(),
                key: self.scenario_key.clone(),
            }
        })
    }
}

impl BenchmarkMarker {
    pub fn read_many<I, P>(paths: I) -> Result<Vec<Self>, MarkerReadError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut markers = Vec::new();
        for path in paths {
            markers.extend(parse_benchmark_markers_from_file(path.as_ref())?);
        }
        Ok(markers)
    }
}

fn parse_benchmark_markers_from_file(path: &Path) -> Result<Vec<BenchmarkMarker>, MarkerReadError> {
    let text = std::fs::read_to_string(path)?;
    let mut markers = Vec::new();
    for line in text.lines() {
        if let Some(result) = parse_benchmark_table_row(line) {
            let mut marker = result.map_err(|message| MarkerReadError::BenchmarkParse {
                path: path.to_path_buf(),
                message,
            })?;
            marker.source_path = path.to_path_buf();
            markers.push(marker);
        }
    }
    Ok(markers)
}

/// Parses a single Markdown table row from benchmark report output.
///
/// Expected layout:
/// Population | Brain tier | Manual expected-slow | Tick time ms | Patches/sec |
/// Memory bytes | Success |
fn parse_benchmark_table_row(line: &str) -> Option<Result<BenchmarkMarker, String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }

    let columns = split_markdown_row(trimmed).collect::<Vec<_>>();
    if columns.is_empty() {
        return None;
    }
    let is_header_or_sep = columns
        .iter()
        .any(|column| column.eq_ignore_ascii_case("population"))
        || is_separator_row(&columns);
    if is_header_or_sep {
        return None;
    }
    if columns.len() < 7 {
        return Some(Err(format!(
            "expected at least 7 columns, got {}",
            columns.len()
        )));
    }

    let parse_u16 = |value: &str| -> Result<u16, String> {
        value
            .trim()
            .parse::<u16>()
            .map_err(|err| format!("invalid u16 '{value}': {err}"))
    };
    let parse_u64 = |value: &str| -> Result<u64, String> {
        value
            .trim()
            .parse::<u64>()
            .map_err(|err| format!("invalid u64 '{value}': {err}"))
    };
    let parse_f64 = |value: &str| -> Result<f64, String> {
        value
            .trim()
            .parse::<f64>()
            .map_err(|err| format!("invalid f64 '{value}': {err}"))
    };
    let parse_bool = |value: &str| -> Result<bool, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(format!("invalid bool '{value}'")),
        }
    };

    Some((|| -> Result<BenchmarkMarker, String> {
        Ok(BenchmarkMarker {
            source_path: PathBuf::from(""),
            population: parse_u16(columns[0])?,
            manual_expected_slow: parse_bool(columns[2])?,
            tick_time_ms: parse_f64(columns[3])?,
            patches_per_second: parse_f64(columns[4])?,
            memory_bytes: parse_u64(columns[5])?,
            success_rate: parse_f64(columns[6])?,
        })
    })())
}

fn is_separator_row(columns: &[&str]) -> bool {
    columns.iter().all(|column| {
        let trimmed = column.trim();
        !trimmed.is_empty() && trimmed.chars().all(|ch| ch == '-' || ch == ':')
    })
}

fn split_markdown_row(line: &str) -> impl Iterator<Item = &str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|value| value.trim())
}

/// Map scenario fixture scenario keys to `ScenarioName` values.
pub fn scenario_name_from_key(key: &str) -> Option<ScenarioName> {
    match key {
        "food-seeking" => Some(ScenarioName::FoodSeeking),
        "poison-pain-avoidance" => Some(ScenarioName::PoisonPainAvoidance),
        "obstacle-frustration" => Some(ScenarioName::ObstacleFrustration),
        "fatigue-sleep" => Some(ScenarioName::FatigueSleep),
        "curiosity-contradiction" => Some(ScenarioName::CuriosityContradiction),
        "word-token-grounding" => Some(ScenarioName::WordTokenGrounding),
        "simple-social-trust-fear" => Some(ScenarioName::SimpleSocialTrustFear),
        "teacher-perception-event" => Some(ScenarioName::TeacherPerceptionEvent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::scenario_name_from_key;
    use super::{parse_benchmark_table_row, MarkerReadError, ScenarioMarker};
    use alife_world::ScenarioName;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    fn fixture_path(key: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("crates")
            .join("alife_world")
            .join("tests")
            .join("fixtures")
            .join("golden_traces")
            .join(format!("{key}.json"))
    }

    #[test]
    fn parse_markdown_row_skips_headers_and_separators() {
        assert!(
            parse_benchmark_table_row(
                "| Population | Brain tier | Manual expected-slow | Tick time ms | Patches/sec | Memory bytes | Success |",
            )
            .is_none()
        );
        assert!(parse_benchmark_table_row("|---:|---|---:|---:|---:|---:|---:|").is_none());
    }

    #[test]
    fn parse_benchmark_row_fails_if_columns_missing() {
        let err = parse_benchmark_table_row("| 1 | Nano512 | false |")
            .expect("row recognized")
            .expect_err("invalid row should fail");
        assert!(err.contains("at least 7 columns"));
    }

    #[test]
    fn parse_scenario_fixture_marker_roundtrips_food_seeking_seed() {
        let marker =
            ScenarioMarker::from_fixture_path(fixture_path("food-seeking")).expect("fixture parse");
        assert_eq!(marker.scenario_key, "food-seeking");
        assert_eq!(marker.scenario_seed, 18_001);
        assert!(marker.patch_count > 0);
        assert_eq!(marker.scenario_name().ok(), Some(ScenarioName::FoodSeeking));
    }

    #[test]
    fn scenario_fixture_with_unknown_key_is_rejected() {
        let marker = ScenarioMarker::from_fixture_path(fixture_path("food-seeking")).unwrap();
        assert_eq!(
            marker.scenario_name().expect("known scenario maps"),
            ScenarioName::FoodSeeking
        );
        assert_eq!(scenario_name_from_key("missing-scenario-key"), None);
        let err = MarkerReadError::UnknownScenarioKey {
            path: PathBuf::from("x"),
            key: "missing-scenario-key".to_string(),
        };
        let message = format!("{err}");
        assert!(message.contains("missing-scenario-key"));
    }

    #[test]
    fn benchmark_rows_keep_source_path_and_population() {
        let path = Path::new("temp_benchmark_markers.md");
        fs::write(
            path,
            "| Population | Brain tier | Manual expected-slow | Tick time ms | Patches/sec | Memory bytes | Success |\n|---:|---|---:|---:|---:|---:|---:|\n| 1 | Nano512 | false | 4.2 | 10.0 | 2048 | 1.0 |\n",
        )
        .expect("fixture write");
        let markers = super::BenchmarkMarker::read_many([path]).expect("parse rows");
        fs::remove_file(path).expect("cleanup");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].population, 1);
        assert_eq!(
            markers[0].source_path,
            PathBuf::from("temp_benchmark_markers.md")
        );
        assert_eq!(markers[0].success_rate, 1.0);
        assert_eq!(markers[0].patches_per_second, 10.0);
    }
}
