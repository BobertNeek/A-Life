//! Bounded, deliberately approximate recall for ordinary live cognition.
//! Cached values are somatic scratch state; decision queries are always fresh.
use super::candidate_recall::{
    recall_family_channel_with_budget, recall_target_channel_with_budget,
};
use super::*;

pub const RUNTIME_MEMORY_SEARCH_CAP: usize = 16;

/// Both audited and cached recall bind recognition to a creature visible now.
pub(super) fn retain_observed_social_recognition(
    draft: &PerceptionFrameDraft,
    candidate: &crate::ActionCandidate,
    query: &CandidateMemoryQueryV2,
    emission: Option<crate::NeuralEmission>,
    recognition: &mut std::collections::BTreeMap<crate::TrackedObjectId, crate::NeuralEmission>,
) -> Result<(), ScaffoldContractError> {
    if let (Some(tracked), Some(emission), crate::CandidateObservationRef::ObjectSlot(slot)) =
        (query.tracked_object_id(), emission, candidate.observation)
    {
        let observed = candidate.target.entity.is_some_and(|entity| {
            draft
                .sensory()
                .social_context
                .nearest_agents
                .iter()
                .flatten()
                .any(|agent| agent.body_entity == Some(entity))
        });
        let confidence = emission.confidence
            * draft.grounded_object_slots()[usize::from(slot)]
                .confidence
                .raw();
        if observed && confidence > 0.0 {
            let emission = crate::NeuralEmission::new(
                crate::NeuralEmissionClass::SocialState,
                emission.activity,
                confidence,
            )?;
            let retained = recognition.entry(tracked).or_insert(emission);
            if (emission.activity * emission.confidence)
                .total_cmp(&(retained.activity * retained.confidence))
                .then_with(|| emission.activity.total_cmp(&retained.activity))
                .then_with(|| emission.confidence.total_cmp(&retained.confidence))
                .is_gt()
            {
                *retained = emission;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecallKey {
    target: Option<crate::TrackedObjectId>,
    untracked_entity: Option<crate::WorldEntityId>,
    family: u8,
    action: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CachedCandidate {
    key: RecallKey,
    context: CandidateMemoryContextV1,
    social_emission: Option<crate::NeuralEmission>,
}

impl RecallKey {
    fn from_candidate(query: &CandidateMemoryQueryV2, candidate: &crate::ActionCandidate) -> Self {
        Self {
            target: query.tracked_object_id(),
            // PrivilegedAffordanceV1 has no object slots. Keep its distinct
            // current entities separate without changing durable query keys.
            untracked_entity: query
                .tracked_object_id()
                .is_none()
                .then_some(candidate.target.entity)
                .flatten(),
            family: query.action_family().raw(),
            action: query.action_id().raw(),
        }
    }
}

/// Small reusable value bank. It intentionally excludes exact query features:
/// a cached expectancy may survive modest movement or drive changes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuntimeMemoryRecallCache {
    owner: u64,
    profile: Option<crate::SensorProfileIdentity>,
    entries: Vec<CachedCandidate>,
    attention: Vec<crate::FinalizedMemoryAttentionEvidence>,
}

impl RuntimeMemoryRecallCache {
    pub fn attention_evidence(&self) -> &[crate::FinalizedMemoryAttentionEvidence] {
        &self.attention
    }

    fn from_entries(
        owner: u64,
        profile: crate::SensorProfileIdentity,
        entries: Vec<CachedCandidate>,
    ) -> Result<Self, ScaffoldContractError> {
        let attention = entries
            .iter()
            .map(|entry| {
                crate::memory_query::memory_attention_evidence_for_context(
                    entry.key.target,
                    &entry.context,
                )
            })
            .collect::<Result<_, ScaffoldContractError>>()?;
        Ok(Self {
            owner,
            profile: Some(profile),
            entries,
            attention,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl PreparedMemoryRecall {
    pub(super) fn validate_runtime_binding(
        &self,
        draft: &PerceptionFrameDraft,
    ) -> Result<(), ScaffoldContractError> {
        if self.receipt.organism_id_raw != draft.organism_id().raw()
            || self.context.tick != draft.tick()
            || self.context.profile != draft.profile_provenance().identity()
            || self.base_frame_digest != draft.base_digest()
            || self.candidate_queries.len() != draft.candidates().len()
            || self.context.candidates.len() != draft.candidates().len()
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }
}

impl MemoryBank {
    pub fn runtime_generation_binding(&self, owner: OrganismId) -> [u64; 4] {
        [
            owner.raw(),
            self.candidate_store.generation,
            self.candidate_store.records.len() as u64,
            1,
        ]
    }

    /// Fresh keys for current perception/action credit, with bounded cached
    /// recall values. New target/action identities always cause a real search.
    pub fn recall_frame_runtime(
        &self,
        draft: &PerceptionFrameDraft,
        cache: &RuntimeMemoryRecallCache,
        refresh: bool,
    ) -> Result<(PreparedMemoryRecall, Option<RuntimeMemoryRecallCache>), ScaffoldContractError>
    {
        let profile = draft.profile_provenance().identity();
        let owner = draft.organism_id().raw();
        let refresh = refresh || cache.owner != owner || cache.profile != Some(profile);

        let mut queries = Vec::with_capacity(draft.candidates().len());
        let mut contexts = Vec::with_capacity(draft.candidates().len());
        let mut emissions = Vec::with_capacity(draft.candidates().len());
        let mut social_recognition = std::collections::BTreeMap::new();

        let mut evaluations = 0_u32;
        let mut searched_any = refresh;
        for candidate in draft.candidates() {
            let query = MemoryQueryEncoderV2::encode_validated_candidate(draft, candidate)?;
            let key = RecallKey::from_candidate(&query, candidate);
            let (target_bucket, family_bucket) = keys_for_query(&query);
            let cached = (!refresh)
                .then(|| cache.entries.iter().find(|entry| entry.key == key))
                .flatten()
                .filter(|entry| {
                    let context = &entry.context;
                    [context.best_target_source, context.best_family_source]
                        .into_iter()
                        .flatten()
                        .all(|id| self.candidate_store.records.contains_key(&id.raw()))
                });
            let (mut context, emission) = if let Some(entry) = cached {
                (entry.context, entry.social_emission)
            } else {
                searched_any = true;
                let target = recall_target_channel_with_budget(
                    &self.candidate_store,
                    &query,
                    &target_bucket,
                    RUNTIME_MEMORY_SEARCH_CAP,
                )?;
                let family = recall_family_channel_with_budget(
                    &self.candidate_store,
                    &query,
                    &family_bucket,
                    RUNTIME_MEMORY_SEARCH_CAP,
                )?;
                evaluations += target.searched + family.searched;
                (
                    CandidateMemoryContextV1 {
                        candidate_index: candidate.candidate_index,
                        target_latent: target.values,
                        family_value: family.values,
                        target_confidence: target.confidence,
                        family_confidence: family.confidence,
                        target_source_count: target.source_count,
                        family_source_count: family.source_count,
                        best_target_source: target.best_source,
                        best_family_source: family.best_source,
                    },
                    target.social_emission,
                )
            };
            context.candidate_index = candidate.candidate_index;
            retain_observed_social_recognition(
                draft,
                candidate,
                &query,
                emission,
                &mut social_recognition,
            )?;

            contexts.push(context);
            queries.push(query);
            emissions.push(emission);
        }
        let next = searched_any
            .then(|| {
                RuntimeMemoryRecallCache::from_entries(
                    owner,
                    profile,
                    queries
                        .iter()
                        .zip(draft.candidates())
                        .zip(&contexts)
                        .zip(emissions)
                        .map(
                            |(((query, candidate), context), social_emission)| CachedCandidate {
                                key: RecallKey::from_candidate(query, candidate),
                                context: *context,
                                social_emission,
                            },
                        )
                        .collect(),
                )
            })
            .transpose()?;
        let context = EpisodicRetrievalContextV1::new(draft.tick(), profile, contexts)?;
        let context_digest = context.to_perception_context_block()?.canonical_digest();
        let receipt = MemoryRecallReceipt {
            runtime_approximate: true,
            schema_version: MEMORY_RECALL_SCHEMA_VERSION,
            organism_id_raw: owner,
            input_generation: self.candidate_store.generation,
            // Explicitly marked above: this is a runtime generation binding,
            // never a portable bank checksum or a training audit.
            bank_digest: [
                owner,
                self.candidate_store.generation,
                self.candidate_store.records.len() as u64,
                1,
            ],
            base_frame_digest: draft.base_digest(),
            context_digest,
            candidate_count: draft.candidates().len() as u16,
            exact_bucket_reads: 0,
            neighbor_bucket_reads: 0,
            similarity_evaluations: evaluations,
            candidates: Vec::new(),
            degradations: Vec::new(),
        };
        Ok((
            PreparedMemoryRecall {
                context,
                candidate_queries: queries,
                base_frame_digest: draft.base_digest(),
                receipt,
                cognitive_context: None,
                social_recognition,
            },
            next,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_attention_prefers_supported_evidence_over_weak_extremes() {
        let entries = [(1.0, 0.01, 1), (0.4, 0.9, 4)]
            .into_iter()
            .enumerate()
            .map(|(index, (signal, confidence, sources))| CachedCandidate {
                key: RecallKey {
                    target: Some(crate::TrackedObjectId(index as u64 + 1)),
                    untracked_entity: None,
                    family: CandidateActionFamily::Ingest.raw(),
                    action: 200,
                },
                context: CandidateMemoryContextV1 {
                    candidate_index: index as u16,
                    target_latent: [signal; MEMORY_LATENT_V1_COUNT],
                    family_value: [signal; MEMORY_VALUE_V1_COUNT],
                    target_confidence: Confidence::new(confidence).unwrap(),
                    family_confidence: Confidence::new(confidence).unwrap(),
                    target_source_count: sources,
                    family_source_count: sources,
                    best_target_source: Some(MemoryId(index as u64 + 1)),
                    best_family_source: Some(MemoryId(index as u64 + 1)),
                },
                social_emission: None,
            })
            .collect();
        let profile = crate::SensorProfileIdentity {
            profile_id: crate::SensorProfile::GroundedObjectSlotsV1.into(),
            profile_schema_version: 1,
            sensory_abi_version: crate::SensoryAbiVersion::CURRENT.raw(),
        };
        let cache = RuntimeMemoryRecallCache::from_entries(811, profile, entries).unwrap();
        let cues = cache.attention_evidence();
        assert!(cues[1].salience.raw() > cues[0].salience.raw());
        assert_eq!(cues[0].confidence.raw(), 0.01);
        assert_eq!(cues[1].source_count, 4);
    }
}
