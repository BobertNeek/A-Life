//! Bounded, deliberately approximate recall for ordinary live cognition.
//! Cached values are somatic scratch state; decision queries are always fresh.
use super::candidate_recall::{
    recall_family_channel_with_budget, recall_target_channel_with_budget,
};
use super::*;

pub const RUNTIME_MEMORY_SEARCH_CAP: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecallKey {
    target: Option<crate::TrackedObjectId>,
    family: u8,
    action: u32,
}

impl RecallKey {
    fn from_query(query: &CandidateMemoryQueryV2) -> Self {
        Self {
            target: query.tracked_object_id(),
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
    entries: Vec<(RecallKey, CandidateMemoryContextV1)>,
    attention: Vec<crate::FinalizedMemoryAttentionEvidence>,
}

impl RuntimeMemoryRecallCache {
    pub fn attention_evidence(&self) -> &[crate::FinalizedMemoryAttentionEvidence] {
        &self.attention
    }

    fn from_entries(
        owner: u64,
        profile: crate::SensorProfileIdentity,
        entries: Vec<(RecallKey, CandidateMemoryContextV1)>,
    ) -> Result<Self, ScaffoldContractError> {
        let attention = entries
            .iter()
            .map(|(key, context)| {
                let confidence = context
                    .target_confidence
                    .raw()
                    .max(context.family_confidence.raw());
                let salience = context
                    .target_latent
                    .iter()
                    .chain(context.family_value.iter())
                    .map(|value| value.abs())
                    .fold(0.0, f32::max);
                Ok(crate::FinalizedMemoryAttentionEvidence {
                    tracked_object_id: key.target,
                    salience: crate::NormalizedScalar::new(salience.clamp(0.0, 1.0))?,
                    confidence: Confidence::new(confidence)?,
                    source_count: context.target_source_count.max(context.family_source_count),
                })
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

        let mut evaluations = 0_u32;
        let mut searched_any = refresh;
        for candidate in draft.candidates() {
            let query = MemoryQueryEncoderV2::encode_validated_candidate(draft, candidate)?;
            let key = RecallKey::from_query(&query);
            let (target_bucket, family_bucket) = keys_for_query(&query);
            let cached = (!refresh)
                .then(|| cache.entries.iter().find(|(old, _)| *old == key))
                .flatten()
                .filter(|(_, context)| {
                    [context.best_target_source, context.best_family_source]
                        .into_iter()
                        .flatten()
                        .all(|id| self.candidate_store.records.contains_key(&id.raw()))
                });
            let (
                mut context,
                _target_eligible,
                _target_searched,
                _target_matches,
                _family_eligible,
                _family_searched,
                _family_matches,
            ) = if let Some((_, context)) = cached {
                (*context, 0, 0, 0, 0, 0, 0)
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
                    target.eligible,
                    target.searched,
                    target.matches,
                    family.eligible,
                    family.searched,
                    family.matches,
                )
            };
            context.candidate_index = candidate.candidate_index;

            contexts.push(context);
            queries.push(query);
        }
        let next = searched_any
            .then(|| {
                RuntimeMemoryRecallCache::from_entries(
                    owner,
                    profile,
                    queries
                        .iter()
                        .zip(&contexts)
                        .map(|(query, context)| (RecallKey::from_query(query), *context))
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
            },
            next,
        ))
    }
}
