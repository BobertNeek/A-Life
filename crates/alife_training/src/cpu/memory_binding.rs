//! Frozen CPU research construction extension, never a legacy asset mutation.
use super::*;
use alife_core::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenMemoryReadoutConstructionV1 {
    pub schema_version: u16,
    pub readout_abi_version: u16,
    pub work_policy_version: u16,
    /// Full unchanged construction genome, including the embedded asset.
    pub parent_genome: CreatureGenome,
    pub source_family: CandidateActionFamily,
    pub destination_family: CandidateActionFamily,
    /// Immutable for this life; no Adam moments or eligibility are invented.
    pub coefficient: f32,
}
impl FrozenMemoryReadoutConstructionV1 {
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.parent_genome.validate_contract()?;
        if self.schema_version != 1
            || self.readout_abi_version != 1
            || self.work_policy_version != 1
            || !self.coefficient.is_finite()
            || self.coefficient.abs() > 4.
            || self.source_family == self.destination_family
            || self.parent_genome.n2048_foundation_candidate.is_none()
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn validate_parent(
        &self,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), TrainingError> {
        let asset = self
            .parent_genome
            .n2048_foundation_candidate
            .as_ref()
            .ok_or_else(invalid)?;
        if asset.digest() != founder.source_foundation.digest()
            || founder
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .is_none_or(|p| p.family_value_lane_start() + 3 != 35)
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn digest(&self) -> Result<[u64; 4], CpuHostError> {
        let mut d = CanonicalDigestBuilder::new(b"ALIFE-FROZEN-MEMORY-READOUT-CONSTRUCTION-V1");
        d.write_bytes(&serde_json::to_vec(self)?);
        Ok(d.finish256())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuMemoryReadoutReceiptV1 {
    pub schema_version: u16,
    pub readout_abi_version: u16,
    pub work_policy_version: u16,
    pub construction_digest: [u64; 4],
    pub source_family: CandidateActionFamily,
    pub destination_family: CandidateActionFamily,
    pub coefficient: f32,
    pub additional_work_ops: u64,
}

pub(super) fn serialize_research_row<T: Serialize + ?Sized, S: serde::Serializer>(
    flat: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Envelope<'a, T: ?Sized> {
        memory_readout_research_record_v1: &'a T,
    }
    Envelope {
        memory_readout_research_record_v1: flat,
    }
    .serialize(serializer)
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MemoryReadoutWorkV1 {
    pub total_ops: u64,
}

pub(super) fn bind_prepared_memory_readout(
    config: &FrozenMemoryReadoutConstructionV1,
    frame: &PerceptionFrame,
    recall: &FinalizedMemoryRecall,
    tick: &mut crate::TrainingReplayTick,
) -> Result<MemoryReadoutWorkV1, CpuHostError> {
    use crate::{FrozenMemoryReadoutTermV1, PreparedMemoryBindingStatusV1 as Status};
    // The canonical encoder has already validated these immutable objects.
    // Recheck their binding, without another recall, export or digest pass.
    if recall.final_frame_digest() != frame.frame_digest()
        || recall.base_frame_digest() != frame.base_digest()
        || recall.context().tick != frame.tick()
        || recall.receipt().organism_id_raw != frame.organism_id().raw()
        || recall.context().profile != frame.profile_provenance().identity()
        || recall.candidate_keys().len() != frame.candidates().len()
    {
        return Err(CpuHostError::State(
            "research readout owner/frame/profile mismatch",
        ));
    }
    let provenance = recall.family_sources().ok_or(CpuHostError::State(
        "research readout lacks prepared family provenance",
    ))?;
    if provenance.len() != frame.candidates().len()
        || tick.candidates.len() != frame.candidates().len()
    {
        return Err(CpuHostError::State(
            "research readout candidate count mismatch",
        ));
    }
    for (native, encoded) in frame.candidates().iter().zip(&tick.candidates) {
        if native.family != encoded.family || encoded.memory_readout.is_some() {
            return Err(CpuHostError::State(
                "research readout source/destination row mismatch",
            ));
        }
    }
    // Research logical tariff, not an instruction/hardware measurement: 16
    // metadata words per source, 16 binding checks plus one family check per
    // candidate, six identity comparisons per join, four scalar readout ops,
    // and three bounded term validations (here, organism admission, replay).
    // Allocator and ordinary host validation overhead are not measured here.
    let mut work = MemoryReadoutWorkV1 {
        total_ops: 16
            + tick.candidates.len() as u64
            + 16 * provenance
                .iter()
                .map(|p| p.sources.iter().flatten().count() as u64)
                .sum::<u64>(),
    };
    for (destination, candidate) in frame.candidates().iter().enumerate() {
        if candidate.family != config.destination_family {
            continue;
        }
        let query = recall.candidate_keys()[destination].query();
        let mut term = FrozenMemoryReadoutTermV1 {
            schema_version: 1,
            destination_family: config.destination_family,
            source_family: config.source_family,
            source_candidate_index: None,
            organism_id: frame.organism_id(),
            current_entity: candidate.target.entity,
            current_tracked_target: query.tracked_object_id(),
            query_tick: frame.tick(),
            profile: query.profile(),
            frame_digest: frame.frame_digest(),
            source_confidence: 0.,
            source_energy: 0.,
            coefficient: config.coefficient,
            status: Status::NoTrackedTarget,
            provenance: None,
        };
        work.total_ops += 4;
        if let (Some(entity), Some(tracked)) = (candidate.target.entity, query.tracked_object_id())
        {
            let mut matching = Vec::new();
            for (index, source) in frame.candidates().iter().enumerate() {
                work.total_ops += 6;
                let key = recall.candidate_keys()[index].query();
                if source.family == config.source_family
                    && source.target.entity == Some(entity)
                    && key.tracked_object_id() == Some(tracked)
                    && key.profile() == query.profile()
                {
                    matching.push(index);
                }
            }
            term.status = match matching.as_slice() {
                [] => Status::MissingSourceCandidate,
                [index] => {
                    term.source_candidate_index = Some(*index as u16);
                    let memory = &recall.context().candidates[*index];
                    if memory.family_source_count == 0 || memory.family_confidence.raw() == 0. {
                        Status::MissingMemory
                    } else {
                        term.source_energy = memory.family_value[3];
                        term.source_confidence = memory.family_confidence.raw();
                        term.provenance = Some(
                            (provenance
                                .get(*index)
                                .ok_or(CpuHostError::State("missing source annotation"))?)
                            .clone(),
                        );
                        Status::Bound
                    }
                }
                _ => Status::AmbiguousSourceCandidate,
            };
        }
        work.total_ops += 3
            * (32
                + 16 * term
                    .provenance
                    .as_ref()
                    .map_or(0, |p| p.sources.iter().flatten().count() as u64));
        tick.candidates[destination].memory_readout = Some(Box::new(term));
    }
    for (index, candidate) in tick.candidates.iter().enumerate() {
        if let Some(term) = &candidate.memory_readout {
            term.validate(index, &tick.candidates)?;
        }
    }
    Ok(work)
}

pub(super) fn charge_memory_readout_work(
    activity: &mut CpuActivityBudget,
    work: MemoryReadoutWorkV1,
) -> Result<(), CpuHostError> {
    activity.work.memory_context_ops = activity
        .work
        .memory_context_ops
        .checked_add(work.total_ops)
        .ok_or(CpuHostError::State("memory readout work overflow"))?;
    let policy = BrainActivityPolicyV1::production_v1();
    activity.debit_atp_q16 = policy
        .cost
        .q24_to_atp_q16_round_half_up(policy.cost.neural_cost_q24(&activity.work)?)?;
    activity.affordable = activity.debit_atp_q16 <= activity.available_atp_q16;
    if !activity.affordable {
        return Err(CpuHostError::State(
            "research memory readout exceeds ATP availability",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "memory_binding_tests.rs"]
mod tests;
