//! Contract-only immutable aggregate for one compiler-authored production phenotype.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    BrainCapacityClass, BrainClassId, CanonicalDigestBuilder, LobeLayout, PhenotypeHash,
    ScaffoldContractError, SensorProfile,
};

use super::{
    CandidateDecoderPlan, CompiledBudgets, CompiledProjection, CompiledSynapse,
    CompiledSynapseKind, DecoderHeadKind, NeuronDynamics, PhenotypeCompilerInputs,
    SensorEncoderPlan,
};

const PHENOTYPE_SCHEMA_VERSION: u16 = 1;
const PHENOTYPE_DOMAIN: &[u8] = b"alife.brain.phenotype.v1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BrainPhenotype {
    schema_version: u16,
    compiler_inputs_digest: [u64; 4],
    brain_class_id: BrainClassId,
    neuron_count: u32,
    microstep_count: u8,
    sensor_profile: SensorProfile,
    lobe_layout: LobeLayout,
    projections: Vec<CompiledProjection>,
    synapses: Vec<CompiledSynapse>,
    neuron_dynamics: Vec<NeuronDynamics>,
    sensor_encoder: SensorEncoderPlan,
    decoder: CandidateDecoderPlan,
    budgets: CompiledBudgets,
    phenotype_hash: PhenotypeHash,
}

impl BrainPhenotype {
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn brain_class_id(&self) -> BrainClassId {
        self.brain_class_id
    }
    pub const fn neuron_count(&self) -> u32 {
        self.neuron_count
    }
    pub const fn microstep_count(&self) -> u8 {
        self.microstep_count
    }
    pub const fn sensor_profile(&self) -> SensorProfile {
        self.sensor_profile
    }
    pub const fn compiler_inputs_digest(&self) -> [u64; 4] {
        self.compiler_inputs_digest
    }
    pub const fn phenotype_hash(&self) -> PhenotypeHash {
        self.phenotype_hash
    }
    pub const fn lobe_layout(&self) -> &LobeLayout {
        &self.lobe_layout
    }
    pub fn projections(&self) -> &[CompiledProjection] {
        &self.projections
    }
    pub fn synapses(&self) -> &[CompiledSynapse] {
        &self.synapses
    }
    pub fn neuron_dynamics(&self) -> &[NeuronDynamics] {
        &self.neuron_dynamics
    }
    pub fn sensor_encoder(&self) -> &SensorEncoderPlan {
        &self.sensor_encoder
    }
    pub fn candidate_decoder(&self) -> &CandidateDecoderPlan {
        &self.decoder
    }
    pub const fn budgets(&self) -> &CompiledBudgets {
        &self.budgets
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_new(
        inputs: &PhenotypeCompilerInputs,
        capacity: &BrainCapacityClass,
        neuron_count: u32,
        microstep_count: u8,
        lobe_layout: LobeLayout,
        projections: Vec<CompiledProjection>,
        synapses: Vec<CompiledSynapse>,
        neuron_dynamics: Vec<NeuronDynamics>,
        sensor_encoder: SensorEncoderPlan,
        decoder: CandidateDecoderPlan,
        budgets: CompiledBudgets,
    ) -> Result<Self, ScaffoldContractError> {
        inputs.validate_against(capacity)?;
        let mut value = Self {
            schema_version: PHENOTYPE_SCHEMA_VERSION,
            compiler_inputs_digest: inputs.canonical_digest(),
            brain_class_id: capacity.id(),
            neuron_count,
            microstep_count,
            sensor_profile: inputs.sensor_profile(),
            lobe_layout,
            projections,
            synapses,
            neuron_dynamics,
            sensor_encoder,
            decoder,
            budgets,
            phenotype_hash: PhenotypeHash([0; 4]),
        };
        value
            .sensor_encoder
            .validate_against_inputs(&value, inputs)?;
        value.phenotype_hash = value.recompute_phenotype_hash()?;
        value.validate_against(capacity)?;
        Ok(value)
    }

    pub fn recompute_phenotype_hash(&self) -> Result<PhenotypeHash, ScaffoldContractError> {
        let mut d = CanonicalDigestBuilder::new(PHENOTYPE_DOMAIN);
        d.write_u16(self.schema_version);
        for word in self.compiler_inputs_digest {
            d.write_u64(word);
        }
        d.write_u16(self.brain_class_id.raw());
        d.write_u32(self.neuron_count);
        d.write_u8(self.microstep_count);
        d.write_u16(self.sensor_profile.raw());
        d.write_sequence_len(self.lobe_layout.regions.len());
        for region in &self.lobe_layout.regions {
            d.write_u16(region.id.0);
            d.write_u16(region.kind.raw());
            d.write_u32(region.start);
            d.write_u32(region.len);
            d.write_bool(region.enabled);
            d.write_u8(region.update_cadence.raw());
            d.write_u8(region.plasticity_policy as u8);
            d.write_u8(region.activation_policy as u8);
            d.write_u8(region.essentiality as u8);
            d.write_u8(region.throttle_priority as u8);
        }
        d.write_sequence_len(self.projections.len());
        for row in &self.projections {
            d.write_u16(row.route_index());
            d.write_u16(row.source_lobe().raw());
            d.write_u16(row.target_lobe().raw());
            d.write_u8(row.projection_type().raw());
            d.write_u8(row.active_tile_policy().raw());
            d.write_u8(row.update_cadence().raw());
            d.write_u8(row.priority().raw());
            d.write_u8(row.delay_microsteps());
            let (start, len) = row.synapse_range();
            d.write_u32(start);
            d.write_u32(len);
            d.write_u32(row.active_tile_count());
        }
        d.write_sequence_len(self.synapses.len());
        for row in &self.synapses {
            d.write_u32(row.source());
            d.write_u32(row.target());
            d.write_f32(row.genetic_weight())?;
            d.write_f32(row.alpha())?;
            d.write_u16(row.route_index());
            match row.kind() {
                CompiledSynapseKind::Recurrent => d.write_u8(0),
                CompiledSynapseKind::Decoder(c) => {
                    d.write_u8(1);
                    d.write_u8(c.head().raw());
                    d.write_u8(c.family().raw());
                    d.write_u16(c.input_lane());
                    d.write_u16(c.motor_index());
                }
            }
        }
        d.write_sequence_len(self.neuron_dynamics.len());
        for row in &self.neuron_dynamics {
            d.write_f32(row.bias())?;
            d.write_f32(row.leak())?;
            d.write_u8(row.activation().raw());
            d.write_f32(row.activity_ema_decay())?;
            d.write_f32(row.metabolic_decay())?;
            d.write_f32(row.homeostatic_gain())?;
        }
        for word in self.sensor_encoder.canonical_digest() {
            d.write_u64(word);
        }
        for word in self.decoder.canonical_digest() {
            d.write_u64(word);
        }
        encode_budgets(&mut d, &self.budgets);
        Ok(PhenotypeHash(d.finish256()))
    }

    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        capacity.validate_contract()?;
        let execution = capacity.execution();
        if self.schema_version != PHENOTYPE_SCHEMA_VERSION
            || self.brain_class_id != capacity.id()
            || self.neuron_count == 0
            || self.neuron_count > execution.max_neurons()
            || !self.neuron_count.is_multiple_of(16)
            || !(execution.microstep_range().0..=execution.microstep_range().1)
                .contains(&self.microstep_count)
            || SensorProfile::try_from_raw(self.sensor_profile.raw()).is_err()
            || self.projections.is_empty()
            || self.synapses.is_empty()
            || self.neuron_dynamics.len() != self.neuron_count as usize
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        self.lobe_layout
            .validate_for_neuron_count(self.neuron_count)?;
        self.budgets.validate_against(capacity)?;
        if self.budgets.global.neuron_count != self.neuron_count
            || self.budgets.global.total_synapses as usize != self.synapses.len()
            || self.projections.len() != self.budgets.routes.len()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut cursor = 0_u32;
        let mut recurrent_total = 0_u32;
        let mut action_decoder_total = 0_u32;
        let mut memory_decoder_total = 0_u32;
        let mut previous_recurrent_coordinate = None;
        let mut previous_decoder_coordinate = None;
        let mut recurrent_route_keys = std::collections::BTreeSet::new();
        let final_route_index = u16::try_from(self.projections.len() - 1)
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        for (index, (projection, receipt)) in self
            .projections
            .iter()
            .zip(&self.budgets.routes)
            .enumerate()
        {
            projection.validate_local()?;
            let route =
                u16::try_from(index).map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
            let (start, len) = projection.synapse_range();
            if projection.route_index() != route || receipt.route_index != route || start != cursor
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            cursor = start
                .checked_add(len)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            if cursor as usize > self.synapses.len()
                || receipt.active_tiles != projection.active_tile_count()
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            let mut recurrent = 0_u32;
            let mut action_decoder = 0_u32;
            let mut memory_decoder = 0_u32;
            let source_region = self
                .lobe_layout
                .region(projection.source_lobe())
                .filter(|region| region.enabled)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let target_region = self
                .lobe_layout
                .region(projection.target_lobe())
                .filter(|region| region.enabled)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let is_decoder_projection = route == final_route_index;
            if is_decoder_projection
                && (projection.source_lobe() != crate::LobeKind::MotorArbitration
                    || projection.target_lobe() != crate::LobeKind::MotorArbitration
                    || projection.projection_type() != crate::ProjectionType::MotorProposal)
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            if !is_decoder_projection
                && (!canonical_recurrent_projection(projection)
                    || !recurrent_route_keys.insert((
                        projection.source_lobe().raw(),
                        projection.target_lobe().raw(),
                    )))
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            let mut touched_tiles = std::collections::BTreeSet::new();
            for synapse in &self.synapses[start as usize..cursor as usize] {
                synapse.validate_local()?;
                if synapse.route_index() != route
                    || synapse.source() >= self.neuron_count
                    || synapse.target() >= self.neuron_count
                    || !source_region.contains_neuron(synapse.source())
                    || !target_region.contains_neuron(synapse.target())
                {
                    return Err(ScaffoldContractError::PhenotypeCompile);
                }
                match projection.projection_type() {
                    crate::ProjectionType::LateralInhibition if synapse.genetic_weight() >= 0.0 => {
                        return Err(ScaffoldContractError::PhenotypeCompile)
                    }
                    crate::ProjectionType::Homeostatic | crate::ProjectionType::MotorProposal
                        if synapse.genetic_weight() < 0.0 =>
                    {
                        return Err(ScaffoldContractError::PhenotypeCompile)
                    }
                    _ => {}
                }
                match synapse.kind() {
                    CompiledSynapseKind::Recurrent => {
                        if is_decoder_projection {
                            return Err(ScaffoldContractError::PhenotypeCompile);
                        }
                        let coordinate = (route, synapse.source(), synapse.target());
                        if previous_recurrent_coordinate.is_some_and(|prior| prior >= coordinate) {
                            return Err(ScaffoldContractError::PhenotypeCompile);
                        }
                        previous_recurrent_coordinate = Some(coordinate);
                        touched_tiles.insert((synapse.source() / 16, synapse.target() / 16));
                        recurrent = recurrent
                            .checked_add(1)
                            .ok_or(ScaffoldContractError::PhenotypeCompile)?
                    }
                    CompiledSynapseKind::Decoder(coordinate) => {
                        if !is_decoder_projection {
                            return Err(ScaffoldContractError::PhenotypeCompile);
                        }
                        let motor_neuron = self
                            .decoder
                            .motor_start()
                            .checked_add(u32::from(coordinate.motor_index()))
                            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
                        if coordinate.head() != DecoderHeadKind::ActionCandidate
                            || coordinate.input_lane() >= self.decoder.flattened_input_lane_count()
                            || coordinate.motor_index() >= self.decoder.motor_width()
                            || synapse.source() != motor_neuron
                            || synapse.target() != motor_neuron
                        {
                            return Err(ScaffoldContractError::PhenotypeCompile);
                        }
                        let identity = (
                            route,
                            coordinate.head().raw(),
                            coordinate.family().raw(),
                            coordinate.input_lane(),
                            coordinate.motor_index(),
                            synapse.source(),
                            synapse.target(),
                        );
                        if previous_decoder_coordinate.is_some_and(|prior| prior >= identity) {
                            return Err(ScaffoldContractError::PhenotypeCompile);
                        }
                        previous_decoder_coordinate = Some(identity);
                        match coordinate.head() {
                            DecoderHeadKind::ActionCandidate => {
                                action_decoder = action_decoder
                                    .checked_add(1)
                                    .ok_or(ScaffoldContractError::PhenotypeCompile)?
                            }
                            DecoderHeadKind::MemoryContext => {
                                memory_decoder = memory_decoder
                                    .checked_add(1)
                                    .ok_or(ScaffoldContractError::PhenotypeCompile)?
                            }
                        }
                    }
                }
            }
            if (
                receipt.recurrent_synapses,
                receipt.action_decoder_synapses,
                receipt.memory_decoder_synapses,
            ) != (recurrent, action_decoder, memory_decoder)
                || receipt.immutable_payload_words != len
                || u32::try_from(touched_tiles.len())
                    .map_err(|_| ScaffoldContractError::PhenotypeCompile)?
                    != projection.active_tile_count()
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            recurrent_total = recurrent_total
                .checked_add(recurrent)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            action_decoder_total = action_decoder_total
                .checked_add(action_decoder)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            memory_decoder_total = memory_decoder_total
                .checked_add(memory_decoder)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        }
        if cursor as usize != self.synapses.len() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if (
            self.budgets.global.recurrent_synapses,
            self.budgets.global.action_decoder_synapses,
            self.budgets.global.memory_decoder_synapses,
        ) != (recurrent_total, action_decoder_total, memory_decoder_total)
            || self.budgets.global.immutable_payload_words != self.budgets.global.total_synapses
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        for dynamics in &self.neuron_dynamics {
            dynamics.validate()?;
        }
        self.sensor_encoder.validate_against(self)?;
        self.decoder.validate_against(self)?;
        if self.budgets.routes.last().is_none_or(|receipt| {
            receipt.recurrent_synapses != 0
                || receipt.memory_decoder_synapses != 0
                || receipt.action_decoder_synapses == 0
        })
            || self.synapses.iter().any(|row| matches!(row.kind(), CompiledSynapseKind::Decoder(c) if c.head() != DecoderHeadKind::ActionCandidate))
            || self.recompute_phenotype_hash()? != self.phenotype_hash
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

fn canonical_recurrent_projection(projection: &CompiledProjection) -> bool {
    use crate::{ActiveTilePolicy, BiologicalPriority, LobeKind, ProjectionType, UpdateCadence};
    let expected = match (projection.source_lobe(), projection.target_lobe()) {
        (LobeKind::SensoryGrounding, LobeKind::CoreAssociation) => {
            (ProjectionType::FeedForward, UpdateCadence::Hot60Hz)
        }
        (LobeKind::CoreAssociation, LobeKind::MotorArbitration) => {
            (ProjectionType::MotorProposal, UpdateCadence::Hot60Hz)
        }
        (LobeKind::MetabolicDrive, LobeKind::HomeostaticRegulation) => {
            (ProjectionType::Homeostatic, UpdateCadence::Hot10To30Hz)
        }
        (LobeKind::MotorArbitration, LobeKind::MotorArbitration) => {
            (ProjectionType::LateralInhibition, UpdateCadence::Hot60Hz)
        }
        _ => return false,
    };
    projection.projection_type() == expected.0
        && projection.update_cadence() == expected.1
        && projection.active_tile_policy() == ActiveTilePolicy::EssentialReservation
        && projection.priority() == BiologicalPriority::Essential
}

impl<'de> Deserialize<'de> for BrainPhenotype {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            compiler_inputs_digest: [u64; 4],
            brain_class_id: BrainClassId,
            neuron_count: u32,
            microstep_count: u8,
            sensor_profile: SensorProfile,
            lobe_layout: LobeLayout,
            projections: Vec<CompiledProjection>,
            synapses: Vec<CompiledSynapse>,
            neuron_dynamics: Vec<NeuronDynamics>,
            sensor_encoder: SensorEncoderPlan,
            decoder: CandidateDecoderPlan,
            budgets: CompiledBudgets,
            phenotype_hash: PhenotypeHash,
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            compiler_inputs_digest: w.compiler_inputs_digest,
            brain_class_id: w.brain_class_id,
            neuron_count: w.neuron_count,
            microstep_count: w.microstep_count,
            sensor_profile: w.sensor_profile,
            lobe_layout: w.lobe_layout,
            projections: w.projections,
            synapses: w.synapses,
            neuron_dynamics: w.neuron_dynamics,
            sensor_encoder: w.sensor_encoder,
            decoder: w.decoder,
            budgets: w.budgets,
            phenotype_hash: w.phenotype_hash,
        };
        let capacity = BrainCapacityClass::production_for_id(value.brain_class_id)
            .map_err(D::Error::custom)?;
        value
            .validate_against(&capacity)
            .map_err(D::Error::custom)?;
        Ok(value)
    }
}

fn encode_budgets(d: &mut CanonicalDigestBuilder, budgets: &CompiledBudgets) {
    d.write_u16(budgets.capacity_class_id.raw());
    for word in budgets.execution_abi_digest {
        d.write_u64(word);
    }
    d.write_sequence_len(budgets.routes.len());
    for r in &budgets.routes {
        for value in [
            u32::from(r.route_index),
            r.active_tiles,
            r.recurrent_synapses,
            r.action_decoder_synapses,
            r.memory_decoder_synapses,
            r.immutable_payload_words,
            r.tile_ceiling,
            r.synapse_ceiling,
            r.payload_word_ceiling,
        ] {
            d.write_u32(value);
        }
    }
    let g = &budgets.global;
    for value in [
        g.neuron_count,
        g.active_tiles,
        g.recurrent_synapses,
        g.action_decoder_synapses,
        g.memory_decoder_synapses,
        g.total_synapses,
        g.immutable_payload_words,
        u32::from(g.candidate_capacity),
        u32::from(g.object_slot_capacity),
        u32::from(g.memory_context_capacity),
        u32::from(g.decoder_input_lanes),
        g.replay_event_capacity,
        g.replay_eligibility_sample_capacity,
    ] {
        d.write_u32(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler, Tick};

    #[test]
    fn self_consistently_rehashed_noncanonical_route_is_rejected() {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(0xF067_E001, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let mut phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        let original = phenotype.projections[0].clone();
        let (start, len) = original.synapse_range();
        phenotype.projections[0] = CompiledProjection::new(
            original.route_index(),
            original.source_lobe(),
            original.target_lobe(),
            crate::ProjectionType::Feedback,
            original.active_tile_policy(),
            original.update_cadence(),
            original.priority(),
            original.delay_microsteps(),
            start,
            len,
            original.active_tile_count(),
        );
        phenotype.phenotype_hash = phenotype.recompute_phenotype_hash().unwrap();
        assert_eq!(
            phenotype.validate_against(&capacity).unwrap_err(),
            ScaffoldContractError::PhenotypeCompile,
        );
    }
}
