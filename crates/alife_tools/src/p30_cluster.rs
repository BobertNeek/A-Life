//! Deterministic baseline clustering for packed log summaries.
//!
//! The first P30 implementation intentionally uses a small deterministic K-Means
//! baseline so P31/P32 can build on a stable record format before adding heavier
//! ML dependencies.

use crate::p30_bundle::PackedLogBundleRecord;

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterConfig {
    pub k: usize,
    pub iterations: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            k: 4,
            iterations: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterSummary {
    pub centroids: Vec<[f32; 4]>,
    pub counts: Vec<usize>,
    pub assignments: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
struct CentroidAccumulator {
    sum: [f32; 4],
    count: usize,
}

impl CentroidAccumulator {
    fn new() -> Self {
        Self {
            sum: [0.0; 4],
            count: 0,
        }
    }

    fn add(&mut self, values: &[f32; 4]) {
        for (axis, value) in values.iter().enumerate() {
            self.sum[axis] += *value;
        }
        self.count += 1;
    }

    fn average(&self) -> [f32; 4] {
        if self.count == 0 {
            return [0.0; 4];
        }
        [
            self.sum[0] / self.count as f32,
            self.sum[1] / self.count as f32,
            self.sum[2] / self.count as f32,
            self.sum[3] / self.count as f32,
        ]
    }
}

fn feature_vector(record: &PackedLogBundleRecord) -> [f32; 4] {
    [
        record.frame.drive_summary[0],
        record.frame.drive_summary[1],
        record.frame.hormone_summary[0],
        record.frame.hormone_summary[1],
    ]
}

pub fn deterministic_kmeans(
    records: &[PackedLogBundleRecord],
    config: ClusterConfig,
) -> ClusterSummary {
    if records.is_empty() || config.k == 0 {
        return ClusterSummary {
            centroids: Vec::new(),
            counts: Vec::new(),
            assignments: Vec::new(),
        };
    }

    let cluster_count = config.k.min(records.len());
    let mut centroids = records
        .iter()
        .take(cluster_count)
        .map(feature_vector)
        .collect::<Vec<_>>();
    let mut assignments = vec![0usize; records.len()];

    for _ in 0..config.iterations.max(1) {
        let mut changed = false;
        let mut sums = vec![CentroidAccumulator::new(); cluster_count];

        for (index, record) in records.iter().enumerate() {
            let values = feature_vector(record);
            let mut best_cluster = 0usize;
            let mut best_distance = distance_sq(&values, &centroids[0]);

            for (cluster, centroid) in centroids.iter().enumerate().skip(1) {
                let candidate = distance_sq(&values, centroid);
                if candidate < best_distance {
                    best_distance = candidate;
                    best_cluster = cluster;
                }
            }

            if assignments[index] != best_cluster {
                changed = true;
                assignments[index] = best_cluster;
            }
            sums[best_cluster].add(&values);
        }

        for cluster in 0..cluster_count {
            let next = sums[cluster].average();
            if next != centroids[cluster] {
                centroids[cluster] = next;
            }
        }

        if !changed {
            break;
        }
    }

    let mut counts = vec![0usize; cluster_count];
    for assignment in assignments.iter() {
        counts[*assignment] += 1;
    }

    ClusterSummary {
        centroids,
        counts,
        assignments,
    }
}

pub fn deterministic_clusters_for_records(
    records: &[PackedLogBundleRecord],
    k: usize,
) -> ClusterSummary {
    deterministic_kmeans(records, ClusterConfig { k, iterations: 8 })
}

pub fn deterministic_clusters_for_records_with_config(
    records: &[PackedLogBundleRecord],
    config: ClusterConfig,
) -> ClusterSummary {
    deterministic_kmeans(records, config)
}

fn distance_sq(left: &[f32; 4], right: &[f32; 4]) -> f32 {
    let mut total = 0.0f32;
    for axis in 0..4 {
        let delta = left[axis] - right[axis];
        total += delta * delta;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::{deterministic_kmeans, ClusterConfig};
    use crate::p30_bundle::PackedLogBundleRecord;
    use alife_core::{
        PackedExperienceFrame, PACKED_DRIVE_SUMMARY_CHANNELS, PACKED_HORMONE_SUMMARY_CHANNELS,
    };

    fn make_record(d0: f32, d1: f32, h0: f32, h1: f32) -> PackedLogBundleRecord {
        let frame = PackedExperienceFrame {
            schema_version: alife_core::PackedExperienceFrame::SCHEMA_VERSION,
            experience_schema_version: alife_core::SchemaVersions::CURRENT.experience.0,
            sensory_abi_version: 1,
            action_abi_version: 2,
            flags: 0,
            sensor_profile_id: alife_core::SensorProfile::PrivilegedAffordanceV1.raw(),
            sensor_profile_schema_version: alife_core::SchemaVersions::current_for(
                alife_core::SchemaKind::SensorProfile,
            )
            .raw(),
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
            drive_summary: {
                let mut values = [0.0f32; PACKED_DRIVE_SUMMARY_CHANNELS];
                values[0] = d0;
                values[1] = d1;
                values
            },
            hormone_summary: {
                let mut values = [0.0f32; PACKED_HORMONE_SUMMARY_CHANNELS];
                values[0] = h0;
                values[1] = h1;
                values
            },
            action_intensity: 0.0,
            action_confidence: 0.0,
            decision_confidence: 0.0,
            reward_valence: 0.0,
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
        PackedLogBundleRecord {
            frame,
            side_buffers: Vec::new(),
        }
    }

    #[test]
    fn deterministic_results_are_reproducible_for_fixture_like_data() {
        let records = vec![
            make_record(0.1, 0.1, 0.2, 0.2),
            make_record(0.11, 0.12, 0.21, 0.19),
            make_record(0.9, 0.85, 0.8, 0.81),
            make_record(0.88, 0.82, 0.81, 0.79),
        ];
        let first = deterministic_kmeans(
            &records,
            ClusterConfig {
                k: 2,
                iterations: 12,
            },
        );
        let second = deterministic_kmeans(
            &records,
            ClusterConfig {
                k: 2,
                iterations: 12,
            },
        );
        assert_eq!(first, second);
        assert_eq!(first.centroids.len(), 2);
        assert_eq!(first.counts.iter().sum::<usize>(), records.len());
    }

    #[test]
    fn empty_input_is_empty_summary() {
        let summary = deterministic_kmeans(&[], ClusterConfig::default());
        assert_eq!(summary.assignments.len(), 0);
        assert_eq!(summary.centroids.len(), 0);
        assert_eq!(summary.counts.len(), 0);
    }
}
