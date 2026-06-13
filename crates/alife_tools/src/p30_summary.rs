//! Replay summary views and exports for offline packed-log bundles.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use alife_core::{PackedExperienceFrame, PackedSideBufferKind};
use serde::{Deserialize, Serialize};

use crate::p30_markers::{BenchmarkMarker, ScenarioMarker};
use crate::{
    p30_bundle::{PackedLogBundle, PackedLogBundleValidationError},
    p30_cluster::{deterministic_clusters_for_records_with_config, ClusterConfig},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTrajectoryPoint {
    pub index: usize,
    pub outcome_tick: u64,
    pub decision_tick: u64,
    pub pre_action_tick: u64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub selected_action_kind: u16,
    pub selected_action_id: u32,
    pub reward_valence: f32,
    pub pain_delta: f32,
    pub frustration_delta: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTrend {
    pub min: f32,
    pub max: f32,
    pub first: f32,
    pub last: f32,
    pub average: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayClusters {
    pub k: usize,
    pub iterations: usize,
    pub counts: Vec<usize>,
    pub centroids: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySummary {
    pub frames: usize,
    pub organism_ids: Vec<u64>,
    pub scenario_markers: Vec<ScenarioMarker>,
    pub benchmark_markers: Vec<BenchmarkMarker>,
    pub action_distribution: Vec<(String, usize)>,
    pub reward_total: f32,
    pub pain_total: f32,
    pub reward_mean: f32,
    pub drive_summary: Vec<ChannelTrend>,
    pub hormone_summary: Vec<ChannelTrend>,
    pub memory_summary_ids: Vec<u64>,
    pub topology_summary_ids: Vec<u64>,
    pub trajectory: Vec<ReplayTrajectoryPoint>,
    pub clusters: Option<ReplayClusters>,
}

#[derive(Debug, Clone, Copy)]
pub struct SummaryConfig {
    pub cluster_k: Option<usize>,
    pub cluster_iterations: usize,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            cluster_k: None,
            cluster_iterations: 8,
        }
    }
}

impl ReplaySummary {
    pub fn from_bundle(
        bundle: &PackedLogBundle,
        config: SummaryConfig,
    ) -> Result<Self, ReplaySummaryError> {
        bundle.validate()?;
        if bundle.records.is_empty() {
            return Err(ReplaySummaryError::NoRecords(NoRecordsError));
        }

        let mut action_counts = BTreeMap::<String, usize>::new();
        let mut memory_ids = BTreeSet::<u64>::new();
        let mut topology_ids = BTreeSet::<u64>::new();
        let mut organisms = BTreeSet::<u64>::new();
        let mut trajectory = Vec::new();
        let mut drives = vec![Vec::new(); 10];
        let mut hormones = vec![Vec::new(); 11];
        let mut reward_total = 0.0f32;
        let mut pain_total = 0.0f32;

        for (index, record) in bundle.records.iter().enumerate() {
            let frame = &record.frame;
            reward_total += frame.reward_valence;
            pain_total += frame.pain_delta;
            organisms.insert(frame.organism_id);

            action_counts
                .entry(action_kind_label(frame.selected_action_kind_code))
                .and_modify(|count| *count += 1)
                .or_insert(1);
            trajectory.push(point_from_frame(index, frame));

            for (channel, values) in drives.iter_mut().enumerate() {
                if channel >= frame.drive_summary.len() {
                    break;
                }
                values.push(frame.drive_summary[channel]);
            }
            for (channel, values) in hormones.iter_mut().enumerate() {
                if channel >= frame.hormone_summary.len() {
                    break;
                }
                values.push(frame.hormone_summary[channel]);
            }
            for side in &record.side_buffers {
                match side.kind {
                    PackedSideBufferKind::MemoryLink => {
                        memory_ids.insert(side.primary_id);
                    }
                    PackedSideBufferKind::ConceptLink => {
                        topology_ids.insert(side.primary_id);
                    }
                    _ => {}
                }
            }
        }

        let drive_summary = drives
            .into_iter()
            .map(|values| summarize_series(&values))
            .collect::<Vec<_>>();
        let hormone_summary = hormones
            .into_iter()
            .map(|values| summarize_series(&values))
            .collect::<Vec<_>>();
        let reward_mean = reward_total / bundle.records.len() as f32;

        let clusters = config.cluster_k.map(|k| {
            let result = deterministic_clusters_for_records_with_config(
                &bundle.records,
                ClusterConfig {
                    k: k.max(1),
                    iterations: config.cluster_iterations,
                },
            );
            ReplayClusters {
                k: k.max(1),
                iterations: config.cluster_iterations,
                counts: result.counts,
                centroids: result.centroids,
            }
        });

        Ok(Self {
            frames: bundle.records.len(),
            organism_ids: organisms.into_iter().collect(),
            scenario_markers: bundle.scenario_markers.clone(),
            benchmark_markers: bundle.benchmark_markers.clone(),
            action_distribution: action_counts.into_iter().collect(),
            reward_total,
            pain_total,
            reward_mean,
            drive_summary,
            hormone_summary,
            memory_summary_ids: memory_ids.into_iter().collect(),
            topology_summary_ids: topology_ids.into_iter().collect(),
            trajectory,
            clusters,
        })
    }

    pub fn write_markdown<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        fs::write(path, self.to_markdown())
    }

    pub fn write_json<P: AsRef<Path>>(&self, path: P) -> Result<(), serde_json::Error> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text).map_err(serde_json::Error::io)
    }

    pub fn write_trajectory_csv<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mut output = String::from(
            "index,outcome_tick,decision_tick,pre_action_tick,x,y,z,selected_action_kind,selected_action_id,reward,pain,frustration\n",
        );
        for point in &self.trajectory {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6}\n",
                point.index,
                point.outcome_tick,
                point.decision_tick,
                point.pre_action_tick,
                point.x,
                point.y,
                point.z,
                point.selected_action_kind,
                point.selected_action_id,
                point.reward_valence,
                point.pain_delta,
                point.frustration_delta,
            ));
        }
        std::fs::write(path, output)
    }

    pub fn write_action_distribution_csv<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), std::io::Error> {
        let mut output = String::from("action,count\n");
        for (action, count) in &self.action_distribution {
            output.push_str(&format!("{action},{count}\n"));
        }
        std::fs::write(path, output)
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Packed log replay summary\n\n");
        out.push_str(&format!(
            "- Frames: {}\n- Organisms: {:?}\n- Total reward: {:.4}\n- Total pain: {:.4}\n- Mean reward: {:.4}\n",
            self.frames,
            self.organism_ids,
            self.reward_total,
            self.pain_total,
            self.reward_mean
        ));

        if !self.scenario_markers.is_empty() {
            out.push_str("\n## Scenario markers\n\n");
            for marker in &self.scenario_markers {
                out.push_str(&format!(
                    "- {} (seed {}) | patches: {} | memory: {} | topology concepts: {} | gaps: {} | source: `{}`\n",
                    marker.scenario_key,
                    marker.scenario_seed,
                    marker.patch_count,
                    marker.memory_record_count,
                    marker.topology_concept_count,
                    marker.topology_gap_count,
                    marker.source_path.display()
                ));
            }
        }
        if !self.benchmark_markers.is_empty() {
            out.push_str("\n## Benchmark markers\n\n");
            out.push_str("| Population | Manual slow | Tick ms | Patches/s | Success |\n|---:|---:|---:|---:|---:|\n");
            for marker in &self.benchmark_markers {
                out.push_str(&format!(
                    "| {} | {} | {:.3} | {:.3} | {:.3} |\n",
                    marker.population,
                    marker.manual_expected_slow,
                    marker.tick_time_ms,
                    marker.patches_per_second,
                    marker.success_rate
                ));
            }
        }

        out.push_str("\n## Action distribution\n\n| Action | Count |\n|---|---:|\n");
        for (action, count) in &self.action_distribution {
            out.push_str(&format!("| {action} | {count} |\n"));
        }

        out.push_str("\n## Drive trend\n\n| Index | Min | Max | First | Last | Mean |\n|---:|---:|---:|---:|---:|---:|\n");
        for (index, trend) in self.drive_summary.iter().enumerate() {
            out.push_str(&format!(
                "| {index} | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} |\n",
                trend.min, trend.max, trend.first, trend.last, trend.average
            ));
        }

        out.push_str("\n## Hormone trend\n\n| Index | Min | Max | First | Last | Mean |\n|---:|---:|---:|---:|---:|---:|\n");
        for (index, trend) in self.hormone_summary.iter().enumerate() {
            out.push_str(&format!(
                "| {index} | {:.6} | {:.6} | {:.6} | {:.6} | {:.6} |\n",
                trend.min, trend.max, trend.first, trend.last, trend.average
            ));
        }

        out.push_str(&format!(
            "\n## Memory summary IDs\n\n{}\n\n",
            if self.memory_summary_ids.is_empty() {
                "No memory links".to_string()
            } else {
                self.memory_summary_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));

        out.push_str("\n## Topology summary IDs\n\n");
        let topology_text = if self.topology_summary_ids.is_empty() {
            "No topology links\n".to_string()
        } else {
            format!(
                "{}\n",
                self.topology_summary_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        out.push_str(&topology_text);

        if !self.trajectory.is_empty() {
            out.push_str("\n## Trajectory (head)\n\n| Step | Outcome tick | x | y | z | Action |\n|---:|---:|---:|---:|---:|---|\n");
            for point in self.trajectory.iter() {
                out.push_str(&format!(
                    "| {} | {} | {:.4} | {:.4} | {:.4} | {} {} |\n",
                    point.index,
                    point.outcome_tick,
                    point.x,
                    point.y,
                    point.z,
                    point.selected_action_kind,
                    point.selected_action_id
                ));
            }
        }

        if let Some(clusters) = &self.clusters {
            out.push_str("\n## Cluster assignments\n\n");
            out.push_str(&format!(
                "- k = {}, iterations = {}\n\n| Cluster | Count |\n|---:|---:|\n",
                clusters.k, clusters.iterations
            ));
            for (index, count) in clusters.counts.iter().enumerate() {
                out.push_str(&format!("| {index} | {count} |\n"));
            }
        }

        out
    }
}

fn summarize_series(values: &[f32]) -> ChannelTrend {
    if values.is_empty() {
        return ChannelTrend {
            min: 0.0,
            max: 0.0,
            first: 0.0,
            last: 0.0,
            average: 0.0,
        };
    }
    let first = values[0];
    let last = values[values.len() - 1];
    let mut min = values[0];
    let mut max = values[0];
    let mut total = 0.0f32;
    for value in values {
        min = min.min(*value);
        max = max.max(*value);
        total += *value;
    }
    ChannelTrend {
        min,
        max,
        first,
        last,
        average: total / values.len() as f32,
    }
}

fn point_from_frame(index: usize, frame: &PackedExperienceFrame) -> ReplayTrajectoryPoint {
    ReplayTrajectoryPoint {
        index,
        outcome_tick: frame.outcome_tick,
        decision_tick: frame.decision_tick,
        pre_action_tick: frame.pre_action_tick,
        x: frame.position[0],
        y: frame.position[1],
        z: frame.position[2],
        selected_action_kind: frame.selected_action_kind_code,
        selected_action_id: frame.selected_action_id,
        reward_valence: frame.reward_valence,
        pain_delta: frame.pain_delta,
        frustration_delta: frame.frustration_delta,
    }
}

fn action_kind_label(code: u16) -> String {
    match code {
        1 => "Idle".to_string(),
        2 => "Hold".to_string(),
        3 => "Rest".to_string(),
        4 => "Inspect".to_string(),
        100 => "Move".to_string(),
        200 => "Interact".to_string(),
        300 => "Gesture".to_string(),
        400 => "Vocalize".to_string(),
        500 => "Write".to_string(),
        other => format!("Unknown({other})"),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("no records in bundle")]
pub struct NoRecordsError;

#[derive(Debug, thiserror::Error)]
pub enum ReplaySummaryError {
    #[error("{0}")]
    Contract(#[from] alife_core::ScaffoldContractError),
    #[error("{0}")]
    Bundle(#[from] PackedLogBundleValidationError),
    #[error("{0}")]
    NoRecords(#[from] NoRecordsError),
}

#[cfg(test)]
mod tests {
    use super::{ReplaySummary, SummaryConfig};
    use crate::p30_bundle::{BundleConfig, PackedLogBundle};
    use crate::p30_markers::{BenchmarkMarker, ScenarioMarker};
    use alife_core::ExperiencePacker;
    use alife_world::{ScenarioFixture, ScenarioName};
    use std::path::PathBuf;

    fn build_bundle_from_food_seed() -> PackedLogBundle {
        let fixture = ScenarioFixture::named(ScenarioName::FoodSeeking).unwrap();
        let run = fixture.run().unwrap();
        let packer = ExperiencePacker::default();
        let records: Vec<_> = run
            .patches
            .into_iter()
            .map(|patch| packer.pack(&patch).unwrap())
            .collect();
        let bundle = PackedLogBundle::from_records(
            records,
            BundleConfig {
                source: Some("test".to_string()),
                notes: vec!["replay test".to_string()],
                scenario_markers: vec![ScenarioMarker {
                    source_path: PathBuf::from("food-seeking.json"),
                    scenario_key: "food-seeking".to_string(),
                    scenario_seed: 18_001,
                    patch_count: 1,
                    memory_record_count: 1,
                    topology_concept_count: 1,
                    topology_gap_count: 0,
                    topology_edge_count: 0,
                    topology_simplex_count: 0,
                    world_signature: vec!["sig".to_string()],
                    notes: "food scenario".to_string(),
                }],
                benchmark_markers: vec![BenchmarkMarker {
                    source_path: PathBuf::from("benchmark.md"),
                    population: 1,
                    manual_expected_slow: false,
                    tick_time_ms: 4.2,
                    patches_per_second: 10.0,
                    success_rate: 1.0,
                    memory_bytes: 2048,
                }],
            },
        );
        assert!(!bundle.records.is_empty());
        bundle
    }

    #[test]
    fn summary_reports_action_distribution_and_markers() {
        let bundle = build_bundle_from_food_seed();
        let summary = ReplaySummary::from_bundle(
            &bundle,
            SummaryConfig {
                cluster_k: Some(1),
                cluster_iterations: 4,
            },
        )
        .unwrap();
        assert!(summary.frames > 0);
        assert!(!summary.action_distribution.is_empty());
        assert!(!summary.scenario_markers.is_empty());
        assert!(!summary.benchmark_markers.is_empty());
        assert!(summary.trajectory.len() == summary.frames);
        assert!(summary.clusters.is_some());
    }

    #[test]
    fn markdown_includes_trajectory_and_trends() {
        let bundle = build_bundle_from_food_seed();
        let summary = ReplaySummary::from_bundle(&bundle, SummaryConfig::default()).unwrap();
        let markdown = summary.to_markdown();
        assert!(markdown.contains("Packed log replay summary"));
        assert!(markdown.contains("Action distribution"));
        assert!(markdown.contains("Drive trend"));
        assert!(markdown.contains("Hormone trend"));
        assert!(markdown.contains("Trajectory"));
    }

    #[test]
    fn summary_outputs_are_stable_with_empty_cluster_request() {
        let bundle = build_bundle_from_food_seed();
        let summary = ReplaySummary::from_bundle(
            &bundle,
            SummaryConfig {
                cluster_k: Some(0),
                cluster_iterations: 4,
            },
        )
        .unwrap();
        assert!(summary.clusters.is_some());
        assert_eq!(summary.clusters.unwrap().k, 1);
    }
}
