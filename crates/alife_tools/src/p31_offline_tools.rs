//! Optional ETF and neural-collapse tooling for representation geometry analysis.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::p30_bundle;
use alife_core::sensory_abi::{AffordanceBits, SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const P31_ETF_PROTOTYPE_SCHEMA: &str = "alife.p31.etf_prototype_table.v1";
pub const P31_ETF_PROTOTYPE_SCHEMA_VERSION: u16 = 1;
pub const P31_NC_REPORT_SCHEMA: &str = "alife.p31.neural_collapse_report.v1";
pub const P31_NC_REPORT_SCHEMA_VERSION: u16 = 1;
pub const P31_LOBE_ASSET_SCHEMA: &str = "alife.p31.sensory_lobe_asset_bundle.v1";
pub const P31_LOBE_ASSET_SCHEMA_VERSION: u16 = 1;

const P31_SYNTHETIC_NOTICE: &str =
    "TRACE_ACTIVATION_EXPORTS_UNAVAILABLE: P35/P36 should add explicit activation exports";
const P31_SUPPORTED_TRACE_SCHEMA_PREFIXES: [&str; 2] = ["alife.p18", "alife.p19.golden_trace.v1"];
const P31_UNIT_NORM_EPSILON: f32 = 1e-5;
const P31_CENTER_EPSILON: f32 = 1e-5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedAffordanceClass {
    pub class_id: u16,
    pub class_key: &'static str,
    pub affordance_bit: u32,
}

pub const P31_FIXED_AFFORDANCE_CLASSES: [FixedAffordanceClass; 10] = [
    FixedAffordanceClass {
        class_id: 0,
        class_key: "food",
        affordance_bit: AffordanceBits::FOOD.0,
    },
    FixedAffordanceClass {
        class_id: 1,
        class_key: "water",
        affordance_bit: AffordanceBits::WATER.0,
    },
    FixedAffordanceClass {
        class_id: 2,
        class_key: "hazard",
        affordance_bit: AffordanceBits::HAZARD.0,
    },
    FixedAffordanceClass {
        class_id: 3,
        class_key: "mate",
        affordance_bit: AffordanceBits::MATE.0,
    },
    FixedAffordanceClass {
        class_id: 4,
        class_key: "social",
        affordance_bit: AffordanceBits::SOCIAL_AGENT.0,
    },
    FixedAffordanceClass {
        class_id: 5,
        class_key: "shelter",
        affordance_bit: AffordanceBits::SHELTER.0,
    },
    FixedAffordanceClass {
        class_id: 6,
        class_key: "tool",
        affordance_bit: AffordanceBits::TOOL.0,
    },
    FixedAffordanceClass {
        class_id: 7,
        class_key: "glyph",
        affordance_bit: AffordanceBits::GLYPH_OR_WRITING.0,
    },
    FixedAffordanceClass {
        class_id: 8,
        class_key: "teacher",
        affordance_bit: AffordanceBits::TEACHER_OBJECT.0,
    },
    FixedAffordanceClass {
        class_id: 9,
        class_key: "resource",
        affordance_bit: AffordanceBits::RESOURCE.0,
    },
];

#[derive(Debug, Clone, Copy)]
pub struct EtfGeneratorConfig {
    pub class_count: usize,
    pub embedding_dimension: usize,
    pub source: &'static str,
}

impl Default for EtfGeneratorConfig {
    fn default() -> Self {
        Self {
            class_count: P31_FIXED_AFFORDANCE_CLASSES.len(),
            embedding_dimension: SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
            source: "alife_core::sensory_abi::AffordanceBits",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EtfClassPrototype {
    pub class_id: u16,
    pub class_key: String,
    pub class_affordance_bit: u32,
    pub mean_embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EtfPrototypeTable {
    pub schema: String,
    pub schema_version: u16,
    pub source: String,
    pub embedding_dimension: usize,
    pub classes: Vec<EtfClassPrototype>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceActivationRecord {
    pub step: u64,
    pub class_id: u16,
    pub activation: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassMeanAlignment {
    pub class_id: u16,
    pub class_key: String,
    pub sample_count: usize,
    pub mean_alignment: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassVariance {
    pub class_id: u16,
    pub class_key: String,
    pub sample_count: usize,
    pub within_class_variance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BetweenClassSimplexMetric {
    pub class_pairs: usize,
    pub target_offdiag_dot: Option<f32>,
    pub observed_mean_dot: f32,
    pub mean_angle_deg: f32,
    pub min_angle_deg: Option<f32>,
    pub max_angle_deg: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriftPoint {
    pub from_step: u64,
    pub to_step: u64,
    pub class_id: u16,
    pub class_key: String,
    pub drift_l2: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriftSeries {
    pub class_count: usize,
    pub point_count: usize,
    pub mean_drift_l2: f32,
    pub min_drift_l2: f32,
    pub max_drift_l2: f32,
    pub points: Vec<DriftPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeuralCollapseSummary {
    pub schema: String,
    pub schema_version: u16,
    pub sample_count: usize,
    pub class_count: usize,
    pub class_mean_alignment: Vec<ClassMeanAlignment>,
    pub class_variance: Vec<ClassVariance>,
    pub between_class_simplex: Option<BetweenClassSimplexMetric>,
    pub drift: Option<DriftSeries>,
    pub synthetic_notice: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SensoryLobePrototypeAssetBundle {
    pub schema: String,
    pub schema_version: u16,
    pub generated_for: String,
    pub prototype_table: EtfPrototypeTable,
}

#[derive(Debug)]
pub struct TraceLoadSummary {
    pub synthetic: bool,
}

#[derive(Debug, Error)]
pub enum P31Error {
    #[error("schema mismatch: expected '{expected}', got '{actual}'")]
    SchemaMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid activation data: {0}")]
    InvalidActivation(String),
    #[error("invalid trace format: {0}")]
    InvalidTrace(String),
    #[error("packed log read failed: {0}")]
    PackedLogBundle(#[from] crate::p30_bundle::PackedLogBundleError),
    #[error("I/O failed at '{0}': {1}")]
    Io(PathBuf, #[source] io::Error),
    #[error("JSON parse/serialize failed at '{0}': {1}")]
    Json(PathBuf, #[source] serde_json::Error),
}

pub fn fixed_affordance_classes() -> &'static [FixedAffordanceClass] {
    &P31_FIXED_AFFORDANCE_CLASSES
}

pub fn fixed_class_key(class_id: u16) -> Option<&'static str> {
    P31_FIXED_AFFORDANCE_CLASSES
        .iter()
        .find(|item| item.class_id == class_id)
        .map(|item| item.class_key)
}

fn fixed_class_id_from_key(class_key: &str) -> Option<u16> {
    let class_key = class_key.trim().to_ascii_lowercase();
    P31_FIXED_AFFORDANCE_CLASSES
        .iter()
        .find(|item| item.class_key == class_key)
        .map(|item| item.class_id)
}

pub fn generate_simplex_etf_prototypes(
    config: EtfGeneratorConfig,
) -> Result<EtfPrototypeTable, P31Error> {
    if config.class_count == 0 {
        return Err(P31Error::InvalidConfig(
            "class_count must be greater than zero".to_string(),
        ));
    }
    if config.class_count > P31_FIXED_AFFORDANCE_CLASSES.len() {
        return Err(P31Error::InvalidConfig(format!(
            "class_count {} exceeds fixed class count {}",
            config.class_count,
            P31_FIXED_AFFORDANCE_CLASSES.len()
        )));
    }
    if config.embedding_dimension == 0 {
        return Err(P31Error::InvalidConfig(
            "embedding_dimension must be greater than zero".to_string(),
        ));
    }
    if config.class_count > 1 && config.embedding_dimension < config.class_count {
        return Err(P31Error::InvalidConfig(
            "embedding_dimension must be at least class_count for exact simplex geometry"
                .to_string(),
        ));
    }

    let class_count_f = config.class_count as f32;
    let scale = (class_count_f / (class_count_f - 1.0)).sqrt();
    let mut classes = Vec::with_capacity(config.class_count);
    for (class_idx, class) in P31_FIXED_AFFORDANCE_CLASSES
        .iter()
        .enumerate()
        .take(config.class_count)
    {
        let mut mean_embedding = vec![0.0f32; config.embedding_dimension];
        for (channel, value) in mean_embedding
            .iter_mut()
            .enumerate()
            .take(config.class_count)
        {
            *value = if channel == class_idx {
                (1.0 - 1.0 / class_count_f) * scale
            } else {
                (-1.0 / class_count_f) * scale
            };
        }
        if config.class_count == 1 {
            mean_embedding[0] = 1.0;
        }
        classes.push(EtfClassPrototype {
            class_id: class.class_id,
            class_key: class.class_key.to_string(),
            class_affordance_bit: class.affordance_bit,
            mean_embedding,
        });
    }
    Ok(EtfPrototypeTable {
        schema: P31_ETF_PROTOTYPE_SCHEMA.to_string(),
        schema_version: P31_ETF_PROTOTYPE_SCHEMA_VERSION,
        source: config.source.to_string(),
        embedding_dimension: config.embedding_dimension,
        classes,
    })
}

pub fn validate_prototype_table(table: &EtfPrototypeTable) -> Result<(), P31Error> {
    if table.schema != P31_ETF_PROTOTYPE_SCHEMA {
        return Err(P31Error::SchemaMismatch {
            expected: P31_ETF_PROTOTYPE_SCHEMA,
            actual: table.schema.clone(),
        });
    }
    if table.schema_version != P31_ETF_PROTOTYPE_SCHEMA_VERSION {
        return Err(P31Error::InvalidConfig(format!(
            "unsupported schema_version {}; expected {}",
            table.schema_version, P31_ETF_PROTOTYPE_SCHEMA_VERSION
        )));
    }
    if table.embedding_dimension == 0 {
        return Err(P31Error::InvalidConfig(
            "embedding_dimension must be greater than zero".to_string(),
        ));
    }
    if table.classes.is_empty() {
        return Err(P31Error::InvalidConfig(
            "prototype table must contain at least one class".to_string(),
        ));
    }

    let allowed_class_ids: HashSet<_> = fixed_affordance_classes()
        .iter()
        .map(|entry| entry.class_id)
        .collect();
    let mut class_ids = HashSet::new();
    let mut class_keys = HashSet::new();
    let mut class_count_map = HashSet::new();
    let mut centroid = vec![0.0f32; table.embedding_dimension];
    for class in &table.classes {
        if !allowed_class_ids.contains(&class.class_id) {
            return Err(P31Error::InvalidConfig(format!(
                "unexpected fixed class_id {}",
                class.class_id
            )));
        }
        if !class.class_key.is_empty() && !class_keys.insert(class.class_key.clone()) {
            return Err(P31Error::InvalidConfig(format!(
                "duplicate class_key {}",
                class.class_key
            )));
        }
        if !class_ids.insert(class.class_id) {
            return Err(P31Error::InvalidConfig(format!(
                "duplicate class_id {}",
                class.class_id
            )));
        }
        if class.mean_embedding.len() != table.embedding_dimension {
            return Err(P31Error::InvalidConfig(format!(
                "class {} embedding length {} does not match embedding_dimension {}",
                class.class_id,
                class.mean_embedding.len(),
                table.embedding_dimension
            )));
        }
        if !class.mean_embedding.iter().all(|value| value.is_finite()) {
            return Err(P31Error::InvalidConfig(format!(
                "class {} has non-finite embedding values",
                class.class_id
            )));
        }
        if (l2_norm(&class.mean_embedding) - 1.0).abs() > P31_UNIT_NORM_EPSILON {
            return Err(P31Error::InvalidConfig(format!(
                "class {} is not unit-norm",
                class.class_id
            )));
        }
        for (centroid_value, mean_value) in centroid.iter_mut().zip(class.mean_embedding.iter()) {
            *centroid_value += mean_value;
        }
        class_count_map.insert(class.class_id);
    }

    if table.classes.len() > 1 {
        if l2_norm(&centroid) > P31_CENTER_EPSILON {
            return Err(P31Error::InvalidConfig(format!(
                "prototypes are not centered (norm={})",
                l2_norm(&centroid)
            )));
        }
        let fixed_count = fixed_affordance_classes().len();
        for class_id in class_count_map {
            if class_id as usize >= fixed_count {
                return Err(P31Error::InvalidConfig(format!(
                    "fixed class_id {} out of fixed class range {}",
                    class_id, fixed_count
                )));
            }
        }
    }
    Ok(())
}

pub fn write_etf_prototype_table(table: &EtfPrototypeTable, path: &Path) -> Result<(), P31Error> {
    validate_prototype_table(table)?;
    let text = to_json(path, table)?;
    write_json_file(path, &text)?;
    Ok(())
}

pub fn write_lobe_asset_bundle(
    path: &Path,
    bundle: &SensoryLobePrototypeAssetBundle,
) -> Result<(), P31Error> {
    let text = to_json(path, bundle)?;
    write_json_file(path, &text)?;
    Ok(())
}

pub fn default_lobe_asset_path(output_dir: &Path) -> PathBuf {
    output_dir.join(format!(
        "sensory_lobe_etf_prototypes_v{}.json",
        P31_LOBE_ASSET_SCHEMA_VERSION
    ))
}

pub fn write_default_lobe_asset(
    output_dir: &Path,
    table: &EtfPrototypeTable,
) -> Result<PathBuf, P31Error> {
    let bundle = SensoryLobePrototypeAssetBundle {
        schema: P31_LOBE_ASSET_SCHEMA.to_string(),
        schema_version: P31_LOBE_ASSET_SCHEMA_VERSION,
        generated_for: "p08+p14 sensory projection prototypes".to_string(),
        prototype_table: table.clone(),
    };
    let path = default_lobe_asset_path(output_dir);
    write_lobe_asset_bundle(&path, &bundle)?;
    Ok(path)
}

pub fn read_etf_prototype_table(path: &Path) -> Result<EtfPrototypeTable, P31Error> {
    let text = read_text(path)?;
    let table: EtfPrototypeTable = parse_json(path, &text)?;
    validate_prototype_table(&table)?;
    Ok(table)
}

pub fn default_etf_prototype_table() -> Result<EtfPrototypeTable, P31Error> {
    generate_simplex_etf_prototypes(EtfGeneratorConfig::default())
}

pub fn analyze_activation_records(
    records: &[TraceActivationRecord],
    prototypes: &EtfPrototypeTable,
) -> Result<NeuralCollapseSummary, P31Error> {
    validate_prototype_table(prototypes)?;
    if records.is_empty() {
        return Err(P31Error::InvalidActivation(
            "no records provided".to_string(),
        ));
    }
    let dim = prototypes.embedding_dimension;
    let known_class_ids = fixed_class_ids(prototypes);
    let mut class_samples: HashMap<u16, Vec<Vec<f32>>> = HashMap::new();
    let mut warnings = Vec::new();

    for record in records {
        if record.activation.len() != dim {
            return Err(P31Error::InvalidActivation(format!(
                "class {} activation at step {} has {} dims, expected {}",
                record.class_id,
                record.step,
                record.activation.len(),
                dim
            )));
        }
        if !record.activation.iter().all(|value| value.is_finite()) {
            return Err(P31Error::InvalidActivation(format!(
                "non-finite activation for class {} at step {}",
                record.class_id, record.step
            )));
        }
        if known_class_ids.contains(&record.class_id) {
            class_samples
                .entry(record.class_id)
                .or_default()
                .push(record.activation.clone());
        } else {
            warnings.push(format!(
                "skipping unrecognized class_id {} at step {}",
                record.class_id, record.step
            ));
        }
    }

    if class_samples.is_empty() {
        return Err(P31Error::InvalidActivation(
            "no records with recognized class ids".to_string(),
        ));
    }

    let mut class_means = HashMap::new();
    let mut class_mean_alignment = Vec::with_capacity(prototypes.classes.len());
    let mut class_variance = Vec::with_capacity(prototypes.classes.len());
    for class in &prototypes.classes {
        let samples = class_samples
            .get(&class.class_id)
            .cloned()
            .unwrap_or_default();
        if samples.is_empty() {
            warnings.push(format!(
                "no samples for class {} ({})",
                class.class_id, class.class_key
            ));
            class_mean_alignment.push(ClassMeanAlignment {
                class_id: class.class_id,
                class_key: class.class_key.clone(),
                sample_count: 0,
                mean_alignment: 0.0,
            });
            class_variance.push(ClassVariance {
                class_id: class.class_id,
                class_key: class.class_key.clone(),
                sample_count: 0,
                within_class_variance: 0.0,
            });
            continue;
        }

        let samples_refs: Vec<&[f32]> = samples.iter().map(|value| value.as_slice()).collect();
        let mean = mean_vector(&samples_refs);
        let mean_norm = l2_norm(&mean);
        let mean_alignment = if mean_norm > 0.0 {
            dot(&mean, &class.mean_embedding) / mean_norm
        } else {
            0.0
        };
        let variance = samples
            .iter()
            .map(|sample| {
                let delta = euclidean_distance(sample, &mean);
                delta * delta
            })
            .sum::<f32>()
            / samples.len() as f32;

        class_means.insert(class.class_id, mean);
        class_mean_alignment.push(ClassMeanAlignment {
            class_id: class.class_id,
            class_key: class.class_key.clone(),
            sample_count: samples.len(),
            mean_alignment,
        });
        class_variance.push(ClassVariance {
            class_id: class.class_id,
            class_key: class.class_key.clone(),
            sample_count: samples.len(),
            within_class_variance: variance,
        });
    }

    let between_class_simplex = if class_means.len() >= 2 {
        Some(between_class_simplex_metric(&class_means)?)
    } else {
        None
    };

    let drift = drift_over_time(records, &class_samples)?;
    Ok(NeuralCollapseSummary {
        schema: P31_NC_REPORT_SCHEMA.to_string(),
        schema_version: P31_NC_REPORT_SCHEMA_VERSION,
        sample_count: records.len(),
        class_count: prototypes.classes.len(),
        class_mean_alignment,
        class_variance,
        between_class_simplex,
        drift,
        synthetic_notice: None,
        warnings,
    })
}

fn between_class_simplex_metric(
    class_means: &HashMap<u16, Vec<f32>>,
) -> Result<BetweenClassSimplexMetric, P31Error> {
    let entries: Vec<(&u16, &Vec<f32>)> = class_means.iter().collect();
    let mut dot_sum = 0.0f32;
    let mut angle_sum = 0.0f32;
    let mut dots = Vec::new();
    let mut angles = Vec::new();
    let mut class_pairs = 0usize;

    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let left = entries[i].1;
            let right = entries[j].1;
            if left.is_empty() || right.is_empty() {
                continue;
            }
            let left_norm = l2_norm(left);
            let right_norm = l2_norm(right);
            if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
                continue;
            }
            let cos = dot(left, right) / (left_norm * right_norm);
            let angle = cos.clamp(-1.0, 1.0).acos().to_degrees();
            let cos = cos.clamp(-1.0, 1.0);
            dots.push(cos);
            angles.push(angle);
            dot_sum += cos;
            angle_sum += angle;
            class_pairs += 1;
        }
    }

    if class_pairs == 0 {
        return Err(P31Error::InvalidActivation(
            "insufficient non-zero class means to compute simplex metric".to_string(),
        ));
    }

    let mean_dot = dot_sum / class_pairs as f32;
    let mean_angle = angle_sum / class_pairs as f32;
    Ok(BetweenClassSimplexMetric {
        class_pairs,
        target_offdiag_dot: Some(-1.0 / ((class_means.len() as f32) - 1.0)),
        observed_mean_dot: mean_dot,
        mean_angle_deg: mean_angle,
        min_angle_deg: angles.iter().copied().reduce(f32::min),
        max_angle_deg: angles.iter().copied().reduce(f32::max),
    })
}

fn drift_over_time(
    records: &[TraceActivationRecord],
    _by_class: &HashMap<u16, Vec<Vec<f32>>>,
) -> Result<Option<DriftSeries>, P31Error> {
    if records.len() < 2 {
        return Ok(None);
    }
    let mut per_step: BTreeMap<u64, HashMap<u16, Vec<Vec<f32>>>> = BTreeMap::new();
    for record in records {
        per_step
            .entry(record.step)
            .or_default()
            .entry(record.class_id)
            .or_default()
            .push(record.activation.clone());
    }
    if per_step.len() < 2 {
        return Ok(None);
    }

    let mut previous: Option<(u64, HashMap<u16, Vec<f32>>)> = None;
    let mut points = Vec::new();
    let mut all_classes = HashSet::new();
    for (step, classes) in per_step {
        let means: HashMap<u16, Vec<f32>> = classes
            .into_iter()
            .filter_map(|(class_id, samples)| {
                if samples.is_empty() {
                    None
                } else {
                    all_classes.insert(class_id);
                    Some((
                        class_id,
                        mean_vector(
                            &samples
                                .iter()
                                .map(|value| value.as_slice())
                                .collect::<Vec<_>>(),
                        ),
                    ))
                }
            })
            .collect();

        if let Some((previous_step, previous_means)) = previous.as_ref() {
            for (class_id, mean) in &means {
                if let Some(previous_mean) = previous_means.get(class_id) {
                    points.push(DriftPoint {
                        from_step: *previous_step,
                        to_step: step,
                        class_id: *class_id,
                        class_key: fixed_class_key(*class_id).unwrap_or("unknown").to_string(),
                        drift_l2: euclidean_distance(previous_mean, mean),
                    });
                }
            }
        }
        previous = Some((step, means));
    }

    if points.is_empty() {
        return Ok(None);
    }
    let (sum_drift, min_drift, max_drift) =
        points
            .iter()
            .fold((0.0f32, f32::INFINITY, f32::NEG_INFINITY), |acc, point| {
                (
                    acc.0 + point.drift_l2,
                    acc.1.min(point.drift_l2),
                    acc.2.max(point.drift_l2),
                )
            });
    Ok(Some(DriftSeries {
        class_count: all_classes.len(),
        point_count: points.len(),
        mean_drift_l2: sum_drift / points.len() as f32,
        min_drift_l2: min_drift,
        max_drift_l2: max_drift,
        points,
    }))
}

#[derive(Debug, Deserialize)]
struct TraceEnvelope {
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    schema_version: Option<u16>,
    #[serde(default)]
    patches: Vec<TracePatch>,
    #[serde(default)]
    representations: Vec<TraceRepresentation>,
}

#[derive(Debug, Deserialize)]
struct TracePatch {
    #[serde(default)]
    class_id: Option<u16>,
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    activation: Option<Vec<f32>>,
    #[serde(default)]
    vector: Option<Vec<f32>>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    #[serde(default)]
    index: Option<u64>,
    #[serde(default)]
    step: Option<u64>,
    #[serde(default)]
    tick: Option<u64>,
    #[serde(default)]
    selected_action_id: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TraceRepresentation {
    #[serde(default)]
    class_id: Option<u16>,
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    #[serde(rename = "class_name")]
    class_name_alias: Option<String>,
    #[serde(default)]
    vector: Option<Vec<f32>>,
    #[serde(default)]
    values: Option<Vec<f32>>,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    #[serde(default)]
    activation: Option<Vec<f32>>,
    #[serde(default)]
    index: Option<u64>,
    #[serde(default)]
    step: Option<u64>,
    #[serde(default)]
    tick: Option<u64>,
    #[serde(default)]
    selected_action_id: Option<u32>,
}

#[derive(Debug)]
struct TraceSource {
    step: u64,
    class_id: Option<u16>,
    activation: Option<Vec<f32>>,
    selected_action_id: Option<u32>,
}

struct TraceSourceInput {
    patch_index: u64,
    class_id: Option<u16>,
    class_name: Option<String>,
    activation: Option<Vec<f32>>,
    step: Option<u64>,
    tick: Option<u64>,
    index: Option<u64>,
    selected_action_id: Option<u32>,
    class_count: usize,
}

fn fixed_class_ids(table: &EtfPrototypeTable) -> HashSet<u16> {
    table.classes.iter().map(|class| class.class_id).collect()
}

fn resolve_class_id(
    index: u64,
    class_id: Option<u16>,
    class_name: Option<&str>,
    selected_action_id: Option<u32>,
    class_count: usize,
) -> u16 {
    if let Some(class_id) = class_id {
        return class_id % class_count as u16;
    }
    if let Some(class_name) = class_name {
        if let Some(class_id) = fixed_class_id_from_key(class_name) {
            return class_id;
        }
    }
    if let Some(action_id) = selected_action_id {
        return (action_id as usize % class_count) as u16;
    }
    (index as usize % class_count) as u16
}

fn resolve_step(fallback: u64, step: Option<u64>, tick: Option<u64>, index: Option<u64>) -> u64 {
    if let Some(value) = step {
        value
    } else if let Some(value) = tick {
        value
    } else if let Some(value) = index {
        value
    } else {
        fallback
    }
}

fn first_present_vector2<'a>(
    source: &'a [Option<&'a Vec<f32>>],
    allow_empty: bool,
) -> Option<&'a [f32]> {
    source.iter().find_map(|value| match value {
        Some(values) if allow_empty || !values.is_empty() => Some(values.as_slice()),
        _ => None,
    })
}

fn to_trace_source(input: TraceSourceInput) -> TraceSource {
    let class_id = Some(resolve_class_id(
        input.patch_index,
        input.class_id,
        input.class_name.as_deref(),
        input.selected_action_id,
        input.class_count,
    ));
    let step = resolve_step(input.patch_index, input.step, input.tick, input.index);
    TraceSource {
        step,
        class_id,
        activation: input.activation,
        selected_action_id: input.selected_action_id,
    }
}

fn coerce_trace_patch_to_record(
    patch: TracePatch,
    patch_index: u64,
    dim: usize,
    class_count: usize,
    synthetic_counter: &mut u64,
) -> (TraceActivationRecord, bool) {
    let source = to_trace_source(TraceSourceInput {
        patch_index,
        class_id: patch.class_id,
        class_name: patch.class,
        activation: first_present_vector2(
            &[
                patch.activation.as_ref(),
                patch.vector.as_ref(),
                patch.embedding.as_ref(),
            ],
            false,
        )
        .map(|values| values.to_vec()),
        step: patch.step,
        tick: patch.tick,
        index: patch.index,
        selected_action_id: patch.selected_action_id,
        class_count,
    });
    let synthetic = source.activation.is_none();
    let seed = activation_seed(
        source.step,
        source.class_id.unwrap_or(0),
        source.selected_action_id,
        *synthetic_counter,
    );
    *synthetic_counter = synthetic_counter.saturating_add(1);
    (
        TraceActivationRecord {
            step: source.step,
            class_id: source.class_id.unwrap_or(0),
            activation: if let Some(activation) = source.activation {
                resize_and_normalize_activation(activation, dim)
            } else {
                deterministic_unit_vector(seed, dim)
            },
        },
        synthetic,
    )
}

fn coerce_trace_representation_to_record(
    representation: TraceRepresentation,
    patch_index: u64,
    dim: usize,
    class_count: usize,
    synthetic_counter: &mut u64,
) -> (TraceActivationRecord, bool) {
    let activation = first_present_vector2(
        &[
            representation.activation.as_ref(),
            representation.vector.as_ref(),
            representation.values.as_ref(),
            representation.embedding.as_ref(),
        ],
        false,
    )
    .map(|values| values.to_vec());

    let class_name = representation
        .class
        .or(representation.class_name_alias.clone());
    let source = to_trace_source(TraceSourceInput {
        patch_index,
        class_id: representation.class_id,
        class_name,
        activation,
        step: representation.step,
        tick: representation.tick,
        index: representation.index,
        selected_action_id: representation.selected_action_id,
        class_count,
    });
    let synthetic = source.activation.is_none();
    let seed = activation_seed(
        source.step,
        source.class_id.unwrap_or(0),
        source.selected_action_id,
        *synthetic_counter,
    );
    *synthetic_counter = synthetic_counter.saturating_add(1);
    (
        TraceActivationRecord {
            step: source.step,
            class_id: source.class_id.unwrap_or(0),
            activation: if let Some(activation) = source.activation {
                resize_and_normalize_activation(activation, dim)
            } else {
                deterministic_unit_vector(seed, dim)
            },
        },
        synthetic,
    )
}

pub fn load_activation_records_from_trace_json(
    path: &Path,
    embedding_dimension: usize,
) -> Result<(Vec<TraceActivationRecord>, TraceLoadSummary), P31Error> {
    let text = read_text(path)?;
    let envelope: TraceEnvelope = parse_json(path, &text)?;
    let schema = envelope
        .schema
        .as_deref()
        .ok_or_else(|| P31Error::SchemaMismatch {
            expected: "alife.p18.* or alife.p19.golden_trace.v1",
            actual: "missing".to_string(),
        })?;
    if !is_supported_trace_schema(schema) {
        return Err(P31Error::SchemaMismatch {
            expected: "alife.p18.* or alife.p19.golden_trace.v1",
            actual: schema.to_string(),
        });
    }
    let schema_version = envelope.schema_version.unwrap_or(0);
    if schema_version == 0 {
        return Err(P31Error::InvalidConfig(
            "invalid or missing schema_version for trace input".to_string(),
        ));
    }
    let class_count = fixed_affordance_classes().len();
    let mut synthetic = false;
    let mut counter = 0u64;
    let mut output = Vec::new();

    for (patch_index, patch) in envelope.patches.into_iter().enumerate() {
        let (record, patch_synthetic) = coerce_trace_patch_to_record(
            patch,
            patch_index as u64,
            embedding_dimension,
            class_count,
            &mut counter,
        );
        output.push(record);
        synthetic |= patch_synthetic;
    }
    for (patch_index, representation) in envelope.representations.into_iter().enumerate() {
        let (record, representation_synthetic) = coerce_trace_representation_to_record(
            representation,
            (patch_index as u64).saturating_add(1_000_000),
            embedding_dimension,
            class_count,
            &mut counter,
        );
        output.push(record);
        synthetic |= representation_synthetic;
    }

    if output.is_empty() {
        return Err(P31Error::InvalidTrace(
            "no activations found in trace payload".to_string(),
        ));
    }

    Ok((output, TraceLoadSummary { synthetic }))
}

pub fn load_activation_records_from_packed_json(
    path: &Path,
    embedding_dimension: usize,
) -> Result<(Vec<TraceActivationRecord>, TraceLoadSummary), P31Error> {
    let records = p30_bundle::read_packed_records_json_file(path)?;
    let mut output = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let frame = &record.frame;
        let class_count = fixed_affordance_classes().len();
        let class_id = (frame.selected_action_id as usize % class_count) as u16;
        let seed = activation_seed(
            frame.sequence_id,
            class_id,
            Some(frame.selected_action_id),
            index as u64,
        );
        output.push(TraceActivationRecord {
            step: frame.sequence_id,
            class_id,
            activation: deterministic_unit_vector(seed, embedding_dimension),
        });
    }
    if output.is_empty() {
        return Err(P31Error::InvalidTrace(
            "no packed log records found".to_string(),
        ));
    }
    Ok((output, TraceLoadSummary { synthetic: true }))
}

pub fn analyze_trace_file(
    path: &Path,
    prototypes: &EtfPrototypeTable,
) -> Result<(NeuralCollapseSummary, TraceLoadSummary), P31Error> {
    let text = read_text(path)?;
    let extension = path.extension().and_then(|value| value.to_str());
    let (records, trace_summary) =
        if extension.is_some_and(|value| value.eq_ignore_ascii_case("json")) {
            if text.trim_start().starts_with('[') {
                load_activation_records_from_packed_json(path, prototypes.embedding_dimension)?
            } else {
                load_activation_records_from_trace_json(path, prototypes.embedding_dimension)?
            }
        } else {
            return Err(P31Error::InvalidTrace(format!(
                "unsupported input extension for {path:?}"
            )));
        };

    let records = records
        .into_iter()
        .map(|record| TraceActivationRecord {
            step: record.step,
            class_id: record.class_id % fixed_affordance_classes().len() as u16,
            activation: resize_and_normalize_activation(
                record.activation,
                prototypes.embedding_dimension,
            ),
        })
        .collect::<Vec<_>>();
    if records.is_empty() {
        return Err(P31Error::InvalidActivation(
            "no activation records found".to_string(),
        ));
    }

    let mut summary = analyze_activation_records(&records, prototypes)?;
    if trace_summary.synthetic && summary.synthetic_notice.is_none() {
        summary.synthetic_notice = Some(P31_SYNTHETIC_NOTICE.to_string());
    }
    Ok((summary, trace_summary))
}

pub fn write_nc_summary_json(summary: &NeuralCollapseSummary, path: &Path) -> Result<(), P31Error> {
    let text = to_json(path, summary)?;
    write_json_file(path, &text)?;
    Ok(())
}

fn mean_vector(samples: &[&[f32]]) -> Vec<f32> {
    let dim = samples[0].len();
    let mut mean = vec![0.0f32; dim];
    for sample in samples {
        for (index, value) in sample.iter().enumerate() {
            mean[index] += value;
        }
    }
    for value in mean.iter_mut() {
        *value /= samples.len() as f32;
    }
    mean
}

fn resize_and_normalize_activation(mut activation: Vec<f32>, dim: usize) -> Vec<f32> {
    activation.resize(dim, 0.0);
    activation.truncate(dim);
    let norm = l2_norm(&activation);
    if norm <= f32::EPSILON {
        return deterministic_unit_vector(0, dim);
    }
    for value in &mut activation {
        *value /= norm;
    }
    activation
}

fn l2_norm(values: &[f32]) -> f32 {
    values.iter().map(|value| value * value).sum::<f32>().sqrt()
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right.iter()).map(|(l, r)| l * r).sum()
}

fn euclidean_distance(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn deterministic_unit_vector(seed: u64, dim: usize) -> Vec<f32> {
    if dim == 0 {
        return Vec::new();
    }
    let mut state = splitmix64(seed);
    let mut output = Vec::with_capacity(dim);
    for _ in 0..dim {
        state = splitmix64(state);
        let uniform = (state as f64) / (u64::MAX as f64);
        output.push((2.0 * uniform - 1.0) as f32);
    }
    let norm = l2_norm(&output);
    if norm <= f32::EPSILON {
        let mut fallback = vec![0.0f32; dim];
        fallback[0] = 1.0;
        return fallback;
    }
    for value in &mut output {
        *value /= norm;
    }
    output
}

fn activation_seed(
    step: u64,
    class_id: u16,
    selected_action_id: Option<u32>,
    fallback: u64,
) -> u64 {
    let selected = selected_action_id.unwrap_or(0) as u64;
    splitmix64(step ^ (selected << 8) ^ ((class_id as u64) << 16) ^ fallback)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn is_supported_trace_schema(schema: &str) -> bool {
    P31_SUPPORTED_TRACE_SCHEMA_PREFIXES
        .iter()
        .any(|prefix| schema.starts_with(prefix))
}

fn read_text(path: &Path) -> Result<String, P31Error> {
    fs::read_to_string(path).map_err(|error| P31Error::Io(path.to_path_buf(), error))
}

fn parse_json<T: DeserializeOwned>(path: &Path, text: &str) -> Result<T, P31Error> {
    serde_json::from_str(text).map_err(|error| P31Error::Json(path.to_path_buf(), error))
}

fn to_json<T: Serialize>(path: &Path, value: &T) -> Result<String, P31Error> {
    serde_json::to_string_pretty(value).map_err(|error| P31Error::Json(path.to_path_buf(), error))
}

fn write_json_file(path: &Path, text: &str) -> Result<(), P31Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| P31Error::Io(parent.to_path_buf(), error))?;
    }
    fs::write(path, text).map_err(|error| P31Error::Io(path.to_path_buf(), error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn generate_simplex_has_unit_norm_embeddings() {
        let table = generate_simplex_etf_prototypes(EtfGeneratorConfig {
            class_count: 8,
            embedding_dimension: 16,
            source: "tests",
        })
        .unwrap();
        assert_eq!(table.classes.len(), 8);
        assert_eq!(table.embedding_dimension, 16);
        for class in &table.classes {
            assert_eq!(class.mean_embedding.len(), 16);
            assert!((l2_norm(&class.mean_embedding) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn generate_simplex_is_centered_for_multiple_classes() {
        let table = generate_simplex_etf_prototypes(EtfGeneratorConfig {
            class_count: 10,
            embedding_dimension: 12,
            source: "tests",
        })
        .unwrap();
        let mut centroid = vec![0.0f32; table.embedding_dimension];
        for class in &table.classes {
            for (centroid_value, class_value) in
                centroid.iter_mut().zip(class.mean_embedding.iter())
            {
                *centroid_value += class_value;
            }
        }
        for value in centroid {
            assert!(value.abs() < 1e-5);
        }
    }

    #[test]
    fn simplex_pairs_match_theory_within_tolerance() {
        let table = generate_simplex_etf_prototypes(EtfGeneratorConfig {
            class_count: 6,
            embedding_dimension: 12,
            source: "tests",
        })
        .unwrap();
        let expected = -1.0 / ((table.classes.len() - 1) as f32);
        for i in 0..table.classes.len() {
            for j in (i + 1)..table.classes.len() {
                let value = dot(
                    &table.classes[i].mean_embedding,
                    &table.classes[j].mean_embedding,
                );
                assert!((value - expected).abs() < 1e-5, "{value} != {expected}");
            }
        }
    }

    #[test]
    fn deterministic_generation_reuses_same_inputs() {
        let config = EtfGeneratorConfig {
            class_count: 5,
            embedding_dimension: 11,
            source: "tests",
        };
        let first = generate_simplex_etf_prototypes(config).unwrap();
        let second = generate_simplex_etf_prototypes(config).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn schema_version_roundtrips_for_etf_tables() {
        let table = generate_simplex_etf_prototypes(EtfGeneratorConfig {
            class_count: 4,
            embedding_dimension: 10,
            source: "tests",
        })
        .unwrap();
        let path = env::temp_dir().join("alife_p31_etf_schema.json");
        write_etf_prototype_table(&table, &path).unwrap();
        let loaded = read_etf_prototype_table(&path).unwrap();
        assert_eq!(loaded.schema_version, P31_ETF_PROTOTYPE_SCHEMA_VERSION);
        std::fs::remove_file(&path).unwrap();

        let mut bad = table.clone();
        bad.schema_version = 7;
        let bad_path = env::temp_dir().join("alife_p31_etf_schema_bad.json");
        let text = serde_json::to_string_pretty(&bad).unwrap();
        std::fs::write(&bad_path, text).unwrap();
        assert!(read_etf_prototype_table(&bad_path).is_err());
        std::fs::remove_file(&bad_path).unwrap();
    }

    #[test]
    fn analyze_synthetic_records_emits_statistics() {
        let table = generate_simplex_etf_prototypes(EtfGeneratorConfig {
            class_count: 3,
            embedding_dimension: 6,
            source: "tests",
        })
        .unwrap();
        let records = vec![
            TraceActivationRecord {
                step: 0,
                class_id: 0,
                activation: deterministic_unit_vector(1, 6),
            },
            TraceActivationRecord {
                step: 0,
                class_id: 1,
                activation: deterministic_unit_vector(2, 6),
            },
            TraceActivationRecord {
                step: 1,
                class_id: 0,
                activation: deterministic_unit_vector(3, 6),
            },
            TraceActivationRecord {
                step: 1,
                class_id: 1,
                activation: deterministic_unit_vector(4, 6),
            },
        ];
        let summary = analyze_activation_records(&records, &table).unwrap();
        assert_eq!(summary.sample_count, 4);
        assert_eq!(summary.class_variance.len(), 3);
        assert!(summary
            .class_mean_alignment
            .iter()
            .any(|entry| entry.sample_count > 0));
        assert!(summary.between_class_simplex.is_some());
        assert!(summary.drift.is_some());
    }

    #[test]
    fn analyze_p19_trace_without_activations_uses_synthetic_records() {
        let table = default_etf_prototype_table().unwrap();
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("crates")
            .join("alife_world")
            .join("tests")
            .join("fixtures")
            .join("golden_traces")
            .join("food-seeking.json");
        let (summary, _load) = analyze_trace_file(&fixture_path, &table).unwrap();
        assert_eq!(summary.class_count, table.classes.len());
        assert!(summary.synthetic_notice.is_some());
        assert_eq!(summary.sample_count, 1);
        assert!(!summary.class_mean_alignment.is_empty());
        assert!(summary.between_class_simplex.is_none());
        assert!(summary.drift.is_none());
    }
}
