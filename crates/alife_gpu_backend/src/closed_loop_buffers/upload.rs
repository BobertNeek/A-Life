use alife_core::{
    BrainCapacityClass, BrainPhenotype, CandidateActionFamily, CompiledSynapseKind, PhenotypeHash,
};

use super::{
    GpuClosedLoopError, GpuDecoderFamilyRecord, GpuDecoderPlanRecord, GpuDecoderWeightIndexRecord,
    GpuEncoderAssignmentRecord, GpuEncoderPlanRecord, GpuNeuronDynamicsRecord,
    GpuPhenotypeIdentityRecord, GpuProjectionRecord, GpuRouteMetadataRecord,
    GPU_CLOSED_LOOP_LAYOUT_VERSION, GPU_NO_EXTENSION_SENTINEL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProjectionSpanDomain {
    A3ProvenanceOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPhenotypeCountPlan {
    pub neurons: usize,
    pub synapses: usize,
    pub recurrent_synapses: usize,
    pub decoder_synapses: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPhenotypeBytePlan {
    pub immutable_weight_bytes: usize,
    pub mutable_weight_bytes: usize,
    pub activation_bytes: usize,
    pub homeostasis_bytes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuPhenotypeUpload {
    pub class_id: u32,
    pub neuron_count: u32,
    pub microstep_count: u32,
    pub gpu_layout_version: u16,
    pub identity: GpuPhenotypeIdentityRecord,
    pub encoder_plans: Vec<GpuEncoderPlanRecord>,
    pub encoder_assignments: Vec<GpuEncoderAssignmentRecord>,
    pub encoder_target_offsets: Vec<u32>,
    pub neuron_dynamics: Vec<GpuNeuronDynamicsRecord>,
    pub projections: Vec<GpuProjectionRecord>,
    pub route_metadata: Vec<GpuRouteMetadataRecord>,
    pub target_offsets: Vec<u32>,
    pub source_indices: Vec<u32>,
    pub route_indices: Vec<u32>,
    pub decoder_plans: Vec<GpuDecoderPlanRecord>,
    pub decoder_families: Vec<GpuDecoderFamilyRecord>,
    pub decoder_weight_indices: Vec<GpuDecoderWeightIndexRecord>,
    pub genetic_weights: Vec<f32>,
    pub alpha: Vec<f32>,
    pub decoder_weight_index_word_base: u32,
    pub extension_record_offset: u32,
}

impl GpuPhenotypeUpload {
    pub const fn projection_span_domain(&self) -> GpuProjectionSpanDomain {
        GpuProjectionSpanDomain::A3ProvenanceOnly
    }
    pub const fn immutable_genetic_owner_count(&self) -> usize {
        1
    }
    pub const fn has_mutable_projection_copy(&self) -> bool {
        false
    }
    pub fn recurrent_global_ids(&self) -> Vec<u32> {
        (0..self.source_indices.len() as u32).collect()
    }
    pub fn exact_count_plan(&self) -> GpuPhenotypeCountPlan {
        GpuPhenotypeCountPlan {
            neurons: self.neuron_count as usize,
            synapses: self.genetic_weights.len(),
            recurrent_synapses: self.source_indices.len(),
            decoder_synapses: self.decoder_weight_indices.len(),
        }
    }
    pub fn exact_byte_plan(&self) -> GpuPhenotypeBytePlan {
        let c = self.exact_count_plan();
        GpuPhenotypeBytePlan {
            immutable_weight_bytes: c.synapses * 8,
            mutable_weight_bytes: c.synapses * 12,
            activation_bytes: c.neurons * 3 * 4,
            homeostasis_bytes: c.neurons * 2 * 4,
        }
    }
    pub fn validate_against(&self, phenotype: &BrainPhenotype) -> Result<(), GpuClosedLoopError> {
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        if self.gpu_layout_version != capacity.execution().gpu_layout_version() {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
        let expected = Self::build(phenotype)?;
        if self != &expected {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(())
    }

    fn build(phenotype: &BrainPhenotype) -> Result<Self, GpuClosedLoopError> {
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        if u32::from(capacity.execution().gpu_layout_version()) != GPU_CLOSED_LOOP_LAYOUT_VERSION {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
        phenotype
            .validate_against(&capacity)
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
        let neuron_count = phenotype.neuron_count();
        let mut encoder_target_offsets = vec![0_u32; neuron_count as usize + 1];
        let mut encoder_assignments =
            Vec::with_capacity(phenotype.sensor_encoder().assignments().len());
        for row in phenotype.sensor_encoder().assignments() {
            let (min, max) = row.clamp_range();
            for value in [row.scale(), row.bias(), min, max] {
                if !value.is_finite() {
                    return Err(GpuClosedLoopError::NonFinitePayload);
                }
            }
            encoder_target_offsets[row.target_neuron() as usize + 1] += 1;
            encoder_assignments.push(GpuEncoderAssignmentRecord {
                source_group_raw: row.source_group().raw() as u32,
                source_index: row.source_index() as u32,
                target_neuron: row.target_neuron(),
                reserved0: 0,
                scale_bits: row.scale().to_bits(),
                bias_bits: row.bias().to_bits(),
                clamp_min_bits: min.to_bits(),
                clamp_max_bits: max.to_bits(),
            });
        }
        prefix_sum(&mut encoder_target_offsets)?;

        let neuron_dynamics = phenotype
            .neuron_dynamics()
            .iter()
            .map(|row| {
                let values = [
                    row.bias(),
                    row.leak(),
                    row.homeostatic_gain(),
                    row.activity_ema_decay(),
                    row.metabolic_decay(),
                ];
                if values.iter().any(|v| !v.is_finite()) {
                    return Err(GpuClosedLoopError::NonFinitePayload);
                }
                Ok(GpuNeuronDynamicsRecord {
                    bias_bits: row.bias().to_bits(),
                    leak_bits: row.leak().to_bits(),
                    activation_raw: row.activation().raw() as u32,
                    homeostatic_gain_bits: row.homeostatic_gain().to_bits(),
                    activity_ema_decay_bits: row.activity_ema_decay().to_bits(),
                    metabolic_decay_bits: row.metabolic_decay().to_bits(),
                    reserved0: 0,
                    reserved1: 0,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut projections = Vec::new();
        let mut route_metadata = Vec::new();
        for row in phenotype.projections() {
            let source = phenotype
                .lobe_layout()
                .region(row.source_lobe())
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            let target = phenotype
                .lobe_layout()
                .region(row.target_lobe())
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            let (start, count) = row.synapse_range();
            projections.push(GpuProjectionRecord {
                route_index: row.route_index() as u32,
                source_lobe_raw: row.source_lobe().raw() as u32,
                target_lobe_raw: row.target_lobe().raw() as u32,
                synapse_start: start,
                synapse_count: count,
                active_tile_count: row.active_tile_count(),
                reserved0: 0,
                reserved1: 0,
            });
            route_metadata.push(GpuRouteMetadataRecord {
                route_index: row.route_index() as u32,
                projection_type_raw: row.projection_type().raw() as u32,
                active_tile_policy_raw: row.active_tile_policy().raw() as u32,
                update_cadence_raw: row.update_cadence().raw() as u32,
                biological_priority_raw: row.priority().raw() as u32,
                delay_microsteps: row.delay_microsteps() as u32,
                source_start: source.start,
                source_count: source.len,
                target_start: target.start,
                target_count: target.len,
                reserved0: 0,
                reserved1: 0,
            });
        }

        let mut recurrent = phenotype
            .synapses()
            .iter()
            .filter(|row| matches!(row.kind(), CompiledSynapseKind::Recurrent))
            .collect::<Vec<_>>();
        recurrent.sort_by_key(|row| (row.target(), row.source(), row.route_index()));
        let mut target_offsets = vec![0_u32; neuron_count as usize + 1];
        for row in &recurrent {
            target_offsets[row.target() as usize + 1] += 1;
        }
        prefix_sum(&mut target_offsets)?;
        let source_indices = recurrent.iter().map(|row| row.source()).collect::<Vec<_>>();
        let route_indices = recurrent
            .iter()
            .map(|row| row.route_index() as u32)
            .collect::<Vec<_>>();

        let mut decoder = phenotype
            .synapses()
            .iter()
            .filter_map(|row| match row.kind() {
                CompiledSynapseKind::Decoder(coord) => Some((row, coord)),
                CompiledSynapseKind::Recurrent => None,
            })
            .collect::<Vec<_>>();
        decoder.sort_by_key(|(row, c)| {
            (
                c.head().raw(),
                c.family().raw(),
                c.input_lane(),
                c.motor_index(),
                row.source(),
                row.target(),
            )
        });
        let recurrent_count = recurrent.len() as u32;
        let decoder_weight_indices = decoder
            .iter()
            .enumerate()
            .map(|(index, (_, c))| GpuDecoderWeightIndexRecord {
                global_synapse_id: recurrent_count + index as u32,
                input_lane: c.input_lane() as u32,
                motor_index: c.motor_index() as u32,
                reserved0: 0,
            })
            .collect::<Vec<_>>();
        let ordered = recurrent
            .iter()
            .copied()
            .chain(decoder.iter().map(|(row, _)| *row));
        let (genetic_weights, alpha): (Vec<_>, Vec<_>) = ordered
            .map(|row| (row.genetic_weight(), row.alpha()))
            .unzip();
        if genetic_weights.iter().chain(&alpha).any(|v| !v.is_finite()) {
            return Err(GpuClosedLoopError::NonFinitePayload);
        }

        let encoder_plan_words = 8_u32;
        let encoder_assignment_words = (encoder_assignments.len() as u32)
            .checked_mul(8)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let encoder_target_words = encoder_target_offsets.len() as u32;
        let dynamics_words = (neuron_dynamics.len() as u32)
            .checked_mul(8)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let projection_words = (projections.len() as u32)
            .checked_mul(8)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let route_words = (route_metadata.len() as u32)
            .checked_mul(12)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let target_words = target_offsets.len() as u32;
        let source_words = source_indices.len() as u32;
        let route_index_words = route_indices.len() as u32;
        let decoder_plan_base = checked_sum(&[
            encoder_plan_words,
            encoder_assignment_words,
            encoder_target_words,
            dynamics_words,
            projection_words,
            route_words,
            target_words,
            source_words,
            route_index_words,
        ])?;
        let decoder_family_base = decoder_plan_base
            .checked_add(8)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let decoder_weight_index_word_base = decoder_family_base
            .checked_add(
                (phenotype.candidate_decoder().families().len() as u32)
                    .checked_mul(8)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let encoder_plan = GpuEncoderPlanRecord {
            schema_version: phenotype.sensor_encoder().schema_version() as u32,
            sensor_profile_raw: phenotype.sensor_profile().raw() as u32,
            assignment_offset: encoder_plan_words,
            assignment_count: encoder_assignments.len() as u32,
            target_offsets_offset: encoder_plan_words + encoder_assignment_words,
            sensory_lane_count: phenotype.sensor_encoder().sensory_lane_count() as u32,
            body_lane_count: phenotype.sensor_encoder().body_lane_count() as u32,
            homeostasis_lane_count: phenotype.sensor_encoder().homeostasis_lane_count() as u32,
        };
        let decoder_plan = GpuDecoderPlanRecord {
            schema_version: phenotype.candidate_decoder().schema_version() as u32,
            motor_start: phenotype.candidate_decoder().motor_start(),
            motor_width: phenotype.candidate_decoder().motor_width() as u32,
            feature_count: phenotype.candidate_decoder().feature_count() as u32,
            flattened_input_lane_count: phenotype.candidate_decoder().flattened_input_lane_count()
                as u32,
            family_offset: decoder_family_base,
            family_count: phenotype.candidate_decoder().families().len() as u32,
            decoder_synapse_count: phenotype.candidate_decoder().decoder_synapse_count(),
        };
        let mut local = 0_u32;
        let mut decoder_families = Vec::new();
        for raw in 0_u8..8 {
            let family = CandidateActionFamily::try_from_raw(raw)
                .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
            let row = phenotype
                .candidate_decoder()
                .families()
                .get(raw as usize)
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            if row.family() != family || !row.bias().is_finite() {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            decoder_families.push(GpuDecoderFamilyRecord {
                family_raw: raw as u32,
                bias_bits: row.bias().to_bits(),
                decoder_synapse_start: recurrent_count + local,
                decoder_synapse_count: row.decoder_synapse_count(),
                weight_index_start: decoder_weight_index_word_base + local * 4,
                weight_index_count: row.decoder_synapse_count(),
                reserved0: 0,
                reserved1: 0,
            });
            local = local
                .checked_add(row.decoder_synapse_count())
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        }

        let PhenotypeHash(hash64) = phenotype.phenotype_hash();
        let mut hash = [0_u32; 8];
        for (index, value) in hash64.into_iter().enumerate() {
            hash[index * 2] = value as u32;
            hash[index * 2 + 1] = (value >> 32) as u32;
        }
        Ok(Self {
            class_id: phenotype.brain_class_id().raw() as u32,
            neuron_count,
            microstep_count: phenotype.microstep_count() as u32,
            gpu_layout_version: capacity.execution().gpu_layout_version(),
            identity: GpuPhenotypeIdentityRecord {
                phenotype_hash: hash,
            },
            encoder_plans: vec![encoder_plan],
            encoder_assignments,
            encoder_target_offsets,
            neuron_dynamics,
            projections,
            route_metadata,
            target_offsets,
            source_indices,
            route_indices,
            decoder_plans: vec![decoder_plan],
            decoder_families,
            decoder_weight_indices,
            genetic_weights,
            alpha,
            decoder_weight_index_word_base,
            extension_record_offset: GPU_NO_EXTENSION_SENTINEL,
        })
    }
}

impl TryFrom<&BrainPhenotype> for GpuPhenotypeUpload {
    type Error = GpuClosedLoopError;
    fn try_from(value: &BrainPhenotype) -> Result<Self, Self::Error> {
        Self::build(value)
    }
}

fn prefix_sum(values: &mut [u32]) -> Result<(), GpuClosedLoopError> {
    for index in 1..values.len() {
        values[index] = values[index]
            .checked_add(values[index - 1])
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    }
    Ok(())
}
fn checked_sum(values: &[u32]) -> Result<u32, GpuClosedLoopError> {
    values.iter().try_fold(0_u32, |sum, value| {
        sum.checked_add(*value)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)
    })
}
