//! CPU research-only, frozen additional readout inside the existing memory cap.
//! The versioned construction coefficient has no Adam/eligibility coordinates.
//! Legacy foundation weights, decoder lanes and plasticity retain their meanings.
use alife_core::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreparedMemoryBindingStatusV1 {
    Bound,
    NoTrackedTarget,
    MissingSourceCandidate,
    AmbiguousSourceCandidate,
    MissingMemory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenMemoryReadoutTermV1 {
    pub schema_version: u16,
    pub destination_family: CandidateActionFamily,
    pub source_family: CandidateActionFamily,
    pub source_candidate_index: Option<u16>,
    pub organism_id: OrganismId,
    pub current_entity: Option<WorldEntityId>,
    pub current_tracked_target: Option<TrackedObjectId>,
    pub query_tick: Tick,
    pub profile: SensorProfileIdentity,
    pub frame_digest: PerceptionFrameDigest,
    /// Native family retention is already incorporated here. No second decay.
    pub source_confidence: f32,
    pub source_energy: f32,
    /// Frozen CPU research parameter, outside the legacy genome/asset/Adam.
    pub coefficient: f32,
    pub status: PreparedMemoryBindingStatusV1,
    pub provenance: Option<MemoryFamilySourcesV1>,
}
impl FrozenMemoryReadoutTermV1 {
    pub fn contribution(&self) -> f32 {
        self.coefficient * (self.source_energy * self.source_confidence)
    }
    pub fn validate(
        &self,
        destination: usize,
        candidates: &[crate::TrainingReplayCandidate],
    ) -> Result<(), ScaffoldContractError> {
        let error = || ScaffoldContractError::InvalidDecisionEvidence;
        self.profile.validate_contract()?;
        if self.schema_version != 1
            || !self.coefficient.is_finite()
            || self.coefficient.abs() > 4.
            || !self.source_energy.is_finite()
            || self.source_energy.abs() > 1.
            || !self.source_confidence.is_finite()
            || !(0.0..=1.).contains(&self.source_confidence)
            || self.source_family == self.destination_family
            || candidates
                .get(destination)
                .is_none_or(|c| c.family != self.destination_family)
        {
            return Err(error());
        }
        if self.status != PreparedMemoryBindingStatusV1::Bound {
            if self.source_confidence != 0. || self.source_energy != 0. || self.provenance.is_some()
            {
                return Err(error());
            }
            return Ok(());
        }
        let index = self.source_candidate_index.ok_or_else(error)? as usize;
        let source = candidates.get(index).ok_or_else(error)?;
        if index == destination
            || source.family != self.source_family
            || self.current_entity.is_none()
            || self.current_tracked_target.is_none()
            || self.source_confidence <= 0.
            || source.decoder_inputs[35].to_bits()
                != (self.source_energy * self.source_confidence).to_bits()
        {
            return Err(error());
        }
        let records = self.provenance.as_ref().ok_or_else(error)?;
        let mut count = 0;
        let mut weight = 0.;
        for item in records.sources.iter().flatten() {
            weight += item.aggregate_weight;
            count += 1;
            if item.organism_id != self.organism_id
                || item.profile != self.profile
                || item.family != self.source_family
                || item.last_tick > self.query_tick
                || item.first_tick > item.last_tick
                || item.memory_id.raw() == 0
                || item.observation_count == 0
                || !item.episode_confidence.is_finite()
                || !(0.0..=1.).contains(&item.episode_confidence)
                || !item.similarity.is_finite()
                || !(0.0..=1.).contains(&item.similarity)
                || !item.aggregate_weight.is_finite()
                || !(0.0..=1.).contains(&item.aggregate_weight)
            {
                return Err(error());
            }
        }
        if count == 0 || (weight - 1.).abs() > 1e-5 {
            return Err(error());
        }
        Ok(())
    }
}

pub(crate) mod wire {
    use super::*;
    use crate::TrainingReplayCandidate;
    #[derive(Serialize)]
    struct LegacyRef<'a> {
        family: CandidateActionFamily,
        #[serde(with = "crate::types::decoder_input_array")]
        decoder_inputs: &'a [f32; 54],
        innate_bias: f32,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Legacy {
        family: CandidateActionFamily,
        #[serde(with = "crate::types::decoder_input_array")]
        decoder_inputs: [f32; 54],
        #[serde(default)]
        innate_bias: f32,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Research<T> {
        memory_readout_research_v1: T,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Payload {
        ordinary: Legacy,
        term: FrozenMemoryReadoutTermV1,
    }
    impl Serialize for TrainingReplayCandidate {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let ordinary = LegacyRef {
                family: self.family,
                decoder_inputs: &self.decoder_inputs,
                innate_bias: self.innate_bias,
            };
            match &self.memory_readout {
                None => ordinary.serialize(serializer),
                Some(term) => {
                    if !serializer.is_human_readable() {
                        return Err(serde::ser::Error::custom(
                            "research memory readout requires explicit JSON envelope",
                        ));
                    }
                    #[derive(Serialize)]
                    struct Borrowed<'a> {
                        ordinary: LegacyRef<'a>,
                        term: &'a FrozenMemoryReadoutTermV1,
                    }
                    Research {
                        memory_readout_research_v1: Borrowed { ordinary, term },
                    }
                    .serialize(serializer)
                }
            }
        }
    }
    impl<'de> Deserialize<'de> for TrainingReplayCandidate {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let (ordinary, memory_readout) = if deserializer.is_human_readable() {
                #[derive(Deserialize)]
                #[serde(untagged)]
                // Keep legacy candidate decoding free of a per-row box.
                #[allow(clippy::large_enum_variant)]
                enum Any {
                    Research(Box<Research<Payload>>),
                    Legacy(Legacy),
                }
                match Any::deserialize(deserializer)? {
                    Any::Legacy(x) => (x, None),
                    Any::Research(x) => (
                        x.memory_readout_research_v1.ordinary,
                        Some(Box::new(x.memory_readout_research_v1.term)),
                    ),
                }
            } else {
                (Legacy::deserialize(deserializer)?, None)
            };
            Ok(Self {
                family: ordinary.family,
                decoder_inputs: ordinary.decoder_inputs,
                innate_bias: ordinary.innate_bias,
                memory_readout,
            })
        }
    }
}
