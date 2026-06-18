//! v0 scaffold: optional Gaussian cluster observations and egocentric hashing.

use alife_core::{
    Confidence, ContextFeatureFlags, GaussianClusterId, GaussianContextRef, GaussianSalienceEntry,
    NormalizedScalar, ScaffoldContractError, Vec3f,
};

/// Maximum number of Gaussian clusters kept in a single optional Gaussian context.
pub const MAX_GAUSSIAN_CONTEXT_CLUSTERS: usize = 8;

/// Adapter-side observation of a Gaussian-rendered semantic cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianClusterObservation {
    pub cluster_id: GaussianClusterId,
    pub salience: f32,
    pub distance_meters: f32,
    pub egocentric_offset: Vec3f,
}

impl GaussianClusterObservation {
    fn to_entry(self) -> Result<GaussianSalienceEntry, ScaffoldContractError> {
        self.cluster_id.validate()?;
        self.egocentric_offset.validate()?;
        let salience = NormalizedScalar::new(self.salience)?;
        if self.distance_meters < 0.0 || !self.distance_meters.is_finite() {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }

        Ok(GaussianSalienceEntry {
            cluster_id: self.cluster_id,
            salience,
            distance_meters: self.distance_meters,
        })
    }
}

/// Bin config for producing a deterministic egocentric hash from spatial offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EgocentricBinGrid {
    pub radial_bins: u16,
    pub azimuth_bins: u16,
    pub elevation_bins: u16,
    pub max_radius_meters: f32,
}

impl Default for EgocentricBinGrid {
    fn default() -> Self {
        Self {
            radial_bins: 8,
            azimuth_bins: 12,
            elevation_bins: 8,
            max_radius_meters: 64.0,
        }
    }
}

/// Deterministic hash for egocentric position bins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EgocentricBinHasher;

impl EgocentricBinHasher {
    pub const fn new() -> Self {
        Self
    }

    pub fn hash(&self, offset: Vec3f, grid: EgocentricBinGrid) -> u64 {
        if !offset.x.is_finite() || !offset.y.is_finite() || !offset.z.is_finite() {
            return 0;
        }
        if grid.radial_bins == 0 || grid.azimuth_bins == 0 || grid.elevation_bins == 0 {
            return 0;
        }
        if grid.max_radius_meters <= 0.0 {
            return 0;
        }

        let distance = (offset.x * offset.x + offset.y * offset.y + offset.z * offset.z).sqrt();
        if distance <= f32::EPSILON || !distance.is_finite() {
            return 0;
        }

        let radial =
            ((distance / grid.max_radius_meters).min(1.0) * grid.radial_bins as f32).floor();
        let radial_bin = (radial as u32).min(u32::from(grid.radial_bins).saturating_sub(1));

        let yaw = offset.z.atan2(offset.x); // [-pi, pi]
        let normalized_yaw = (yaw + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
        let azim = (normalized_yaw * grid.azimuth_bins as f32).floor();
        let azimuth_bin = (azim as u32).min(u32::from(grid.azimuth_bins).saturating_sub(1));

        let elevation = (offset.z / distance).clamp(-1.0, 1.0).asin();
        let normalized_elev = (elevation / std::f32::consts::FRAC_PI_2 + 1.0) * 0.5;
        let elev = (normalized_elev * grid.elevation_bins as f32).floor();
        let elevation_bin = (elev as u32).min(u32::from(grid.elevation_bins).saturating_sub(1));

        (u64::from(radial_bin) << 32) | (u64::from(elevation_bin) << 16) | u64::from(azimuth_bin)
    }
}

impl Default for EgocentricBinHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert adapter-side Gaussian observations into optional core Gaussian context.
pub fn build_gaussian_context(
    observations: &[GaussianClusterObservation],
    confidence: f32,
    egocentric_bin_hash: u64,
) -> Result<Option<GaussianContextRef>, ScaffoldContractError> {
    let mut entries = Vec::new();

    for obs in observations {
        if obs.salience <= 0.0 {
            continue;
        }
        entries.push(obs.to_entry()?);
    }

    if entries.is_empty() || confidence <= 0.0 {
        return Ok(None);
    }

    let confidence = Confidence::new(confidence)?;
    if confidence.raw() == 0.0 {
        return Ok(None);
    }

    entries.sort_by(|lhs, rhs| {
        rhs.salience
            .raw()
            .total_cmp(&lhs.salience.raw())
            .then(rhs.distance_meters.total_cmp(&lhs.distance_meters))
    });
    entries.truncate(MAX_GAUSSIAN_CONTEXT_CLUSTERS);

    Ok(Some(GaussianContextRef {
        egocentric_bin_hash,
        feature_flags: ContextFeatureFlags::GAUSSIAN_CLUSTERS
            | ContextFeatureFlags::EGOCENTRIC_BIN_HASH,
        confidence,
        clusters: entries,
    }))
}
