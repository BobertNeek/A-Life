use crate::closed_loop_memory::GpuMemoryChannelPlan;
use alife_core::{
    BrainCapacityClass, BrainPhenotype, CandidateActionFamily, CompiledSynapseKind, PhenotypeHash,
};

use super::{
    GpuClosedLoopError, GpuDecoderEligibilityMetadata, GpuDecoderFamilyRecord,
    GpuDecoderPlanRecord, GpuDecoderWeightIndexRecord, GpuEncoderAssignmentRecord,
    GpuEncoderPlanRecord, GpuNeuronDynamicsRecord, GpuPhenotypeIdentityRecord,
    GpuPlasticityReceptorRecord, GpuProjectionRecord, GpuReplayCaptureIdentityRecord,
    GpuRouteMetadataRecord, GpuSleepParameterRecord, GpuSynapseLearningMetadata,
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
    pub candidate_decoder_synapses: usize,
    pub decoder_synapses: usize,
    pub plasticity_receptors: usize,
    pub replay_capture_synapses: usize,
    pub sleep_parameters: usize,
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
    pub memory_channel_plans: Vec<GpuMemoryChannelPlan>,
    pub memory_weight_indices: Vec<u32>,
    pub plasticity_receptors: Vec<GpuPlasticityReceptorRecord>,
    pub synapse_learning_metadata: Vec<GpuSynapseLearningMetadata>,
    pub decoder_eligibility_metadata: Vec<GpuDecoderEligibilityMetadata>,
    pub replay_capture_local_synapse_ids: Vec<u32>,
    pub replay_capture_identity: GpuReplayCaptureIdentityRecord,
    pub sleep_parameters: Vec<GpuSleepParameterRecord>,
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
            candidate_decoder_synapses: self.decoder_weight_indices.len(),
            decoder_synapses: self.decoder_eligibility_metadata.len(),
            plasticity_receptors: self.plasticity_receptors.len(),
            replay_capture_synapses: self.replay_capture_local_synapse_ids.len(),
            sleep_parameters: self.sleep_parameters.len(),
        }
    }
    pub fn exact_byte_plan(&self) -> GpuPhenotypeBytePlan {
        let c = self.exact_count_plan();
        GpuPhenotypeBytePlan {
            immutable_weight_bytes: c.synapses * 8,
            mutable_weight_bytes: c.synapses * 24,
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
            .enumerate()
            .filter(|(_, row)| matches!(row.kind(), CompiledSynapseKind::Recurrent))
            .collect::<Vec<_>>();
        recurrent.sort_by_key(|(_, row)| (row.target(), row.source(), row.route_index()));
        let mut target_offsets = vec![0_u32; neuron_count as usize + 1];
        for (_, row) in &recurrent {
            target_offsets[row.target() as usize + 1] += 1;
        }
        prefix_sum(&mut target_offsets)?;
        let source_indices = recurrent
            .iter()
            .map(|(_, row)| row.source())
            .collect::<Vec<_>>();
        let route_indices = recurrent
            .iter()
            .map(|(_, row)| row.route_index() as u32)
            .collect::<Vec<_>>();

        let mut all_decoders = phenotype
            .synapses()
            .iter()
            .enumerate()
            .filter_map(|(canonical_id, row)| match row.kind() {
                CompiledSynapseKind::Decoder(coord) => Some((canonical_id, row, coord)),
                CompiledSynapseKind::Recurrent => None,
            })
            .collect::<Vec<_>>();
        all_decoders.sort_by_key(|(_, row, c)| {
            (
                c.head().raw(),
                c.family().raw(),
                c.input_lane(),
                c.motor_index(),
                row.source(),
                row.target(),
            )
        });
        let candidate_decoders = all_decoders
            .iter()
            .copied()
            .filter(|(_, _, coordinate)| {
                coordinate.head() == alife_core::DecoderHeadKind::ActionCandidate
            })
            .collect::<Vec<_>>();
        let memory_decoders = all_decoders
            .iter()
            .copied()
            .filter(|(_, _, coordinate)| {
                coordinate.head() == alife_core::DecoderHeadKind::MemoryContext
            })
            .collect::<Vec<_>>();
        let recurrent_count = recurrent.len() as u32;
        let ordered = recurrent
            .iter()
            .map(|(canonical_id, row)| (*canonical_id, *row))
            .chain(
                all_decoders
                    .iter()
                    .map(|(canonical_id, row, _)| (*canonical_id, *row)),
            )
            .collect::<Vec<_>>();
        let mut canonical_to_local = vec![u32::MAX; phenotype.synapses().len()];
        for (local_id, (canonical_id, _)) in ordered.iter().enumerate() {
            canonical_to_local[*canonical_id] = local_id as u32;
        }
        if canonical_to_local.contains(&u32::MAX) {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let decoder_weight_indices = candidate_decoders
            .iter()
            .map(|(canonical_id, _, c)| GpuDecoderWeightIndexRecord {
                global_synapse_id: canonical_to_local[*canonical_id],
                input_lane: c.input_lane() as u32,
                motor_index: c.motor_index() as u32,
                reserved0: 0,
            })
            .collect::<Vec<_>>();
        let memory_weight_indices = memory_decoders
            .iter()
            .map(|(canonical_id, _, _)| canonical_to_local[*canonical_id])
            .collect::<Vec<_>>();
        let memory_channel_plans = phenotype
            .candidate_decoder()
            .memory_channel()
            .map(GpuMemoryChannelPlan::try_from)
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();
        match memory_channel_plans.as_slice() {
            [] if memory_weight_indices.is_empty() => {}
            [plan] if plan.memory_decoder_synapse_count as usize == memory_weight_indices.len() => {
            }
            _ => return Err(GpuClosedLoopError::MalformedUpload),
        }
        let (genetic_weights, alpha): (Vec<_>, Vec<_>) = ordered
            .iter()
            .map(|(_, row)| (row.genetic_weight(), row.alpha()))
            .unzip();
        if genetic_weights.iter().chain(&alpha).any(|v| !v.is_finite()) {
            return Err(GpuClosedLoopError::NonFinitePayload);
        }

        let plasticity_receptors = phenotype
            .plasticity_receptors()
            .iter()
            .map(|receptor| {
                let (fast_min, fast_max) = receptor.fast_weight_bounds();
                GpuPlasticityReceptorRecord {
                    eligibility_decay: receptor.eligibility_decay(),
                    learning_rate: receptor.learning_rate(),
                    sleep_replay_rate: receptor.sleep_replay_rate(),
                    normalization_rate: receptor.normalization_rate(),
                    modulator_sign: receptor.modulator_sign(),
                    fast_min,
                    fast_max,
                    reserved: 0.0,
                }
            })
            .collect::<Vec<_>>();
        if plasticity_receptors.iter().any(|row| {
            row.reserved.to_bits() != 0
                || [
                    row.eligibility_decay,
                    row.learning_rate,
                    row.sleep_replay_rate,
                    row.normalization_rate,
                    row.modulator_sign,
                    row.fast_min,
                    row.fast_max,
                ]
                .into_iter()
                .any(|value| !value.is_finite())
        }) {
            return Err(GpuClosedLoopError::NonFinitePayload);
        }

        let decoder_eligibility_metadata = all_decoders
            .iter()
            .enumerate()
            .map(
                |(eligibility_local_index, (canonical_id, row, coordinate))| {
                    GpuDecoderEligibilityMetadata {
                        global_synapse_id: canonical_to_local[*canonical_id],
                        decoder_head: coordinate.head().raw(),
                        family: u32::from(coordinate.family().raw()),
                        input_lane: u32::from(coordinate.input_lane()),
                        // Eligibility reads the final activation heap directly, so this
                        // runtime-local lane is the absolute compiled source neuron rather
                        // than the candidate decoder's lobe-local motor ordinal.
                        motor_index: row.source(),
                        receptor_index: u32::from(row.receptor_index()),
                        eligibility_local_index: eligibility_local_index as u32,
                        reserved: 0,
                    }
                },
            )
            .collect::<Vec<_>>();
        let synapse_learning_metadata = ordered
            .iter()
            .enumerate()
            .map(|(local_id, (_, row))| {
                let (eligibility_local_index, decoder_metadata_local_or_max) = match row.kind() {
                    CompiledSynapseKind::Recurrent => (local_id as u32, u32::MAX),
                    CompiledSynapseKind::Decoder(_) => {
                        let decoder_local = (local_id as u32)
                            .checked_sub(recurrent_count)
                            .ok_or(GpuClosedLoopError::MalformedUpload)?;
                        (decoder_local, decoder_local)
                    }
                };
                Ok(GpuSynapseLearningMetadata {
                    global_synapse_id: local_id as u32,
                    kind: row.kind().kind_raw(),
                    source_neuron: row.source(),
                    target_neuron: row.target(),
                    receptor_index: u32::from(row.receptor_index()),
                    eligibility_local_index,
                    decoder_metadata_local_or_max,
                    reserved: 0,
                })
            })
            .collect::<Result<Vec<_>, GpuClosedLoopError>>()?;
        let mut replay_capture_local_synapse_ids = phenotype
            .replay_capture_plan()
            .global_synapse_ids()
            .iter()
            .map(|canonical_id| {
                canonical_to_local
                    .get(*canonical_id as usize)
                    .copied()
                    .filter(|local| *local != u32::MAX)
                    .ok_or(GpuClosedLoopError::MalformedUpload)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // The portable replay batch ABI is ordered by executable, GPU-local
        // synapse identity. Canonical phenotype order is not preserved by the
        // recurrent/decoder packing translation (notably for N2048), so sort
        // only after every persistent ID has resolved to its runtime-local ID.
        replay_capture_local_synapse_ids.sort_unstable();
        if replay_capture_local_synapse_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let replay_capture_identity = GpuReplayCaptureIdentityRecord {
            replay_capture_plan_digest: split_digest(
                phenotype.replay_capture_plan().canonical_digest(),
            ),
        };
        let sleep = phenotype.sleep_consolidation_plan();
        sleep
            .validate_contract()
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
        let sleep_parameter = GpuSleepParameterRecord {
            schema_version: u32::from(sleep.schema_version()),
            staging_rate: sleep.staging_rate(),
            weight_limit: sleep.weight_limit(),
            fast_decay_rate: sleep.fast_decay_rate(),
            eligibility_reset_policy: u32::from(sleep.eligibility_reset_policy_raw()),
            replay_consume_policy: u32::from(sleep.replay_consume_policy_raw()),
            reserved: [0; 2],
        };
        if [
            sleep_parameter.staging_rate,
            sleep_parameter.weight_limit,
            sleep_parameter.fast_decay_rate,
        ]
        .into_iter()
        .any(|value| !value.is_finite())
            || sleep_parameter.eligibility_reset_policy != 1
            || sleep_parameter.replay_consume_policy != 1
            || sleep_parameter.reserved != [0; 2]
        {
            return Err(GpuClosedLoopError::MalformedUpload);
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
            memory_channel_plans,
            memory_weight_indices,
            plasticity_receptors,
            synapse_learning_metadata,
            decoder_eligibility_metadata,
            replay_capture_local_synapse_ids,
            replay_capture_identity,
            sleep_parameters: vec![sleep_parameter],
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

fn split_digest(values: [u64; 4]) -> [u32; 8] {
    let mut split = [0_u32; 8];
    for (index, value) in values.into_iter().enumerate() {
        split[index * 2] = value as u32;
        split[index * 2 + 1] = (value >> 32) as u32;
    }
    split
}
