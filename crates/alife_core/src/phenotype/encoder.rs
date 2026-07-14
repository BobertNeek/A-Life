//! Contract-only compiler-owned sensory/body/homeostasis encoder plan.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{CanonicalDigestBuilder, ScaffoldContractError, SensorProfile};

use super::{BrainPhenotype, PhenotypeCompilerInputs};

const ENCODER_SCHEMA_VERSION: u16 = 1;
const ENCODER_DOMAIN: &[u8] = b"alife.phenotype.sensor-encoder.v1";
const SENSORY_LANES: u16 = 42;
const BODY_LANES: u16 = 13;
const HOMEOSTASIS_LANES: u16 = 22;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorEncoderSourceGroup {
    SensoryChannel = 1,
    Body = 2,
    Homeostasis = 3,
}

impl SensorEncoderSourceGroup {
    pub const fn raw(self) -> u16 {
        self as u16
    }
    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::SensoryChannel),
            2 => Ok(Self::Body),
            3 => Ok(Self::Homeostasis),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SensorEncoderAssignment {
    source_group: SensorEncoderSourceGroup,
    source_index: u16,
    target_neuron: u32,
    scale: f32,
    bias: f32,
    clamp_min: f32,
    clamp_max: f32,
}

impl SensorEncoderAssignment {
    pub const fn source_group(&self) -> SensorEncoderSourceGroup {
        self.source_group
    }
    pub const fn source_index(&self) -> u16 {
        self.source_index
    }
    pub const fn target_neuron(&self) -> u32 {
        self.target_neuron
    }
    pub const fn scale(&self) -> f32 {
        self.scale
    }
    pub const fn bias(&self) -> f32 {
        self.bias
    }
    pub const fn clamp_range(&self) -> (f32, f32) {
        (self.clamp_min, self.clamp_max)
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) const fn new(
        source_group: SensorEncoderSourceGroup,
        source_index: u16,
        target_neuron: u32,
        scale: f32,
        bias: f32,
        clamp_min: f32,
        clamp_max: f32,
    ) -> Self {
        Self {
            source_group,
            source_index,
            target_neuron,
            scale,
            bias,
            clamp_min,
            clamp_max,
        }
    }
    fn validate_local(&self, widths: (u16, u16, u16)) -> Result<(), ScaffoldContractError> {
        SensorEncoderSourceGroup::try_from_raw(self.source_group.raw())?;
        let width = match self.source_group {
            SensorEncoderSourceGroup::SensoryChannel => widths.0,
            SensorEncoderSourceGroup::Body => widths.1,
            SensorEncoderSourceGroup::Homeostasis => widths.2,
        };
        if self.source_index >= width
            || ![self.scale, self.bias, self.clamp_min, self.clamp_max]
                .into_iter()
                .all(f32::is_finite)
            || self.clamp_min > self.clamp_max
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SensorEncoderPlan {
    schema_version: u16,
    sensor_profile: SensorProfile,
    sensory_lane_count: u16,
    body_lane_count: u16,
    homeostasis_lane_count: u16,
    assignments: Vec<SensorEncoderAssignment>,
    canonical_digest: [u64; 4],
}

impl SensorEncoderPlan {
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn sensor_profile(&self) -> SensorProfile {
        self.sensor_profile
    }
    pub const fn sensory_lane_count(&self) -> u16 {
        self.sensory_lane_count
    }
    pub const fn body_lane_count(&self) -> u16 {
        self.body_lane_count
    }
    pub const fn homeostasis_lane_count(&self) -> u16 {
        self.homeostasis_lane_count
    }
    pub fn assignments(&self) -> &[SensorEncoderAssignment] {
        &self.assignments
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub(super) fn try_new(
        sensor_profile: SensorProfile,
        assignments: Vec<SensorEncoderAssignment>,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: ENCODER_SCHEMA_VERSION,
            sensor_profile,
            sensory_lane_count: SENSORY_LANES,
            body_lane_count: BODY_LANES,
            homeostasis_lane_count: HOMEOSTASIS_LANES,
            assignments,
            canonical_digest: [0; 4],
        };
        value.validate_shape()?;
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_local()?;
        if self.sensor_profile != phenotype.sensor_profile() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        for assignment in &self.assignments {
            let region = phenotype
                .lobe_layout()
                .lobe_by_neuron_index(assignment.target_neuron)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            if !region.enabled {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }

    pub(super) fn validate_against_inputs(
        &self,
        phenotype: &BrainPhenotype,
        inputs: &PhenotypeCompilerInputs,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_against(phenotype)?;
        let genome = inputs.genome();
        let development = inputs.development();
        let active_genes = genome
            .sensor_layout
            .channels
            .iter()
            .filter(|gene| {
                f32::from(gene.enabled_at_maturation) <= development.maturation.raw() * 100.0
                    && (development.active_sensor_channels.is_empty()
                        || development.active_sensor_channels.contains(&gene.kind))
                    && (development.enabled_lobes.is_empty()
                        || development.enabled_lobes.contains(&gene.target_lobe))
            })
            .collect::<Vec<_>>();

        let mut gene_keys = Vec::with_capacity(active_genes.len());
        let mut groups: Vec<(u16, SensorEncoderSourceGroup, u16, u16, usize)> = Vec::new();
        for gene in &active_genes {
            let key = (gene.kind.raw(), gene.target_lobe.raw());
            if gene_keys.contains(&key) {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            gene_keys.push(key);
            let _target = phenotype
                .lobe_layout()
                .region(gene.target_lobe)
                .filter(|region| region.enabled)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let (group, start, end) = source_lane_range(gene.kind);
            if let Some(row) = groups.iter_mut().find(|row| {
                (row.0, row.1, row.2, row.3) == (gene.target_lobe.raw(), group, start, end)
            }) {
                row.4 = row
                    .4
                    .checked_add(usize::from(gene.receptor_count))
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            } else {
                groups.push((
                    gene.target_lobe.raw(),
                    group,
                    start,
                    end,
                    usize::from(gene.receptor_count),
                ));
            }
        }

        for &(target_raw, group, start, end, expected) in &groups {
            let target_lobe = crate::LobeKind::try_from_raw(target_raw)?;
            let target = phenotype
                .lobe_layout()
                .region(target_lobe)
                .filter(|region| region.enabled)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let count = self
                .assignments
                .iter()
                .filter(|assignment| {
                    assignment.source_group == group
                        && (start..end).contains(&assignment.source_index)
                        && target.contains_neuron(assignment.target_neuron)
                })
                .count();
            if count != expected {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }

        for assignment in &self.assignments {
            let matches = groups
                .iter()
                .filter(|(target_raw, group, start, end, _)| {
                    let Ok(target_lobe) = crate::LobeKind::try_from_raw(*target_raw) else {
                        return false;
                    };
                    phenotype
                        .lobe_layout()
                        .region(target_lobe)
                        .is_some_and(|target| {
                            assignment.source_group == *group
                                && (*start..*end).contains(&assignment.source_index)
                                && target.contains_neuron(assignment.target_neuron)
                        })
                })
                .count();
            if matches != 1 {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != ENCODER_SCHEMA_VERSION
            || SensorProfile::try_from_raw(self.sensor_profile.raw()).is_err()
            || (
                self.sensory_lane_count,
                self.body_lane_count,
                self.homeostasis_lane_count,
            ) != (SENSORY_LANES, BODY_LANES, HOMEOSTASIS_LANES)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let widths = (
            self.sensory_lane_count,
            self.body_lane_count,
            self.homeostasis_lane_count,
        );
        let mut previous = None;
        for assignment in &self.assignments {
            assignment.validate_local(widths)?;
            let key = (
                assignment.target_neuron,
                assignment.source_group.raw(),
                assignment.source_index,
            );
            if previous.is_some_and(|prior| prior >= key) {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            previous = Some(key);
        }
        Ok(())
    }

    pub(super) fn validate_local(&self) -> Result<(), ScaffoldContractError> {
        self.validate_shape()?;
        if self.recompute_digest()? != self.canonical_digest {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(ENCODER_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.sensor_profile.raw());
        digest.write_u16(self.sensory_lane_count);
        digest.write_u16(self.body_lane_count);
        digest.write_u16(self.homeostasis_lane_count);
        digest.write_sequence_len(self.assignments.len());
        for assignment in &self.assignments {
            digest.write_u16(assignment.source_group.raw());
            digest.write_u16(assignment.source_index);
            digest.write_u32(assignment.target_neuron);
            digest.write_f32(assignment.scale)?;
            digest.write_f32(assignment.bias)?;
            digest.write_f32(assignment.clamp_min)?;
            digest.write_f32(assignment.clamp_max)?;
        }
        Ok(digest.finish256())
    }
}

fn source_lane_range(kind: crate::SensorChannelKind) -> (SensorEncoderSourceGroup, u16, u16) {
    use crate::SensorChannelKind;
    match kind {
        SensorChannelKind::Vision | SensorChannelKind::GlyphVision => {
            (SensorEncoderSourceGroup::SensoryChannel, 0, 16)
        }
        SensorChannelKind::Hearing => (SensorEncoderSourceGroup::SensoryChannel, 16, 24),
        SensorChannelKind::Smell | SensorChannelKind::Taste => {
            (SensorEncoderSourceGroup::SensoryChannel, 24, 32)
        }
        SensorChannelKind::Touch => (SensorEncoderSourceGroup::SensoryChannel, 32, 40),
        SensorChannelKind::Proprioception => (SensorEncoderSourceGroup::Body, 0, 13),
        SensorChannelKind::Interoception => (SensorEncoderSourceGroup::Homeostasis, 0, 22),
    }
}

impl<'de> Deserialize<'de> for SensorEncoderPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AssignmentWire {
            source_group: SensorEncoderSourceGroup,
            source_index: u16,
            target_neuron: u32,
            scale: f32,
            bias: f32,
            clamp_min: f32,
            clamp_max: f32,
        }
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            sensor_profile: SensorProfile,
            sensory_lane_count: u16,
            body_lane_count: u16,
            homeostasis_lane_count: u16,
            assignments: Vec<AssignmentWire>,
            canonical_digest: [u64; 4],
        }
        let w = Wire::deserialize(deserializer)?;
        let assignments = w
            .assignments
            .into_iter()
            .map(|a| {
                SensorEncoderAssignment::new(
                    a.source_group,
                    a.source_index,
                    a.target_neuron,
                    a.scale,
                    a.bias,
                    a.clamp_min,
                    a.clamp_max,
                )
            })
            .collect();
        let value = Self {
            schema_version: w.schema_version,
            sensor_profile: w.sensor_profile,
            sensory_lane_count: w.sensory_lane_count,
            body_lane_count: w.body_lane_count,
            homeostasis_lane_count: w.homeostasis_lane_count,
            assignments,
            canonical_digest: w.canonical_digest,
        };
        value.validate_local().map_err(D::Error::custom)?;
        Ok(value)
    }
}
