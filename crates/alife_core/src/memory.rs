//! Candidate-conditioned bounded episodic memory and legacy diagnostic records.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    validate_finite_slice, ActionId, ActionKind, CandidateActionFamily, CandidateMemoryContextV1,
    CandidateMemoryQueryV2, CanonicalDigestBuilder, Confidence, DriveDelta, EpisodicDecisionKeyV2,
    EpisodicRetrievalContextV1, ExperiencePatch, ExperienceSequenceId, MemoryId,
    MemoryQueryEncoderV2, NormalizedScalar, OrganismId, PerceptionBaseDigest,
    PerceptionContextDigest, PerceptionFrame, PerceptionFrameDigest, PerceptionFrameDraft,
    PhysicalContactKind, PreActionSnapshot, ScaffoldContractError, SignedValence, Tick, Validate,
    CANDIDATE_FEATURE_COUNT, MAX_ACTION_CANDIDATES, MEMORY_LATENT_V1_COUNT, MEMORY_TARGET_RANGE,
    MEMORY_VALUE_V1_COUNT,
};

pub const MEMORY_FEATURE_VECTOR_MAX_LEN: usize = 64;
pub const MEMORY_BANK_MAX_CAPACITY: usize = 1_000_000;
pub const MEMORY_RECALL_SCHEMA_VERSION: u16 = 2;
pub const MEMORY_FAMILY_SEARCH_CAP: usize = 64;
pub const MEMORY_TARGET_SEARCH_CAP: usize = 64;
pub const MEMORY_TOTAL_SEARCH_CAP: usize = MEMORY_FAMILY_SEARCH_CAP + MEMORY_TARGET_SEARCH_CAP;
pub const MEMORY_RECALL_TOP_K: usize = 4;
pub const MEMORY_MIN_SIMILARITY: f32 = 0.72;
pub const MEMORY_MERGE_SIMILARITY: f32 = 0.98;

const CANDIDATE_MEMORY_BANK_DOMAIN: &[u8] = b"ALIFE-CANDIDATE-MEMORY-BANK-V2";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecallChannel {
    TargetLatent = 1,
    FamilyValue = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBucketReceiptKey {
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub query_version_raw: u16,
    pub tracked_object_id_raw: u64,
    pub family_raw: u16,
    pub other_action_id_raw: u32,
    pub target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetMemoryBucketReceiptKey {
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub query_version_raw: u16,
    pub tracked_object_id_raw: u64,
    pub target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecallDegradation {
    SearchShortlisted {
        candidate_index: u16,
        channel: MemoryRecallChannel,
        eligible: u32,
        searched: u32,
    },
    EmptyAfterCapacityPressure {
        candidate_index: u16,
        channel: MemoryRecallChannel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateMemoryRecallReceipt {
    pub candidate_index: u16,
    pub query_digest: [u64; 4],
    pub target_bucket: TargetMemoryBucketReceiptKey,
    pub family_bucket: MemoryBucketReceiptKey,
    pub target_eligible: u32,
    pub target_searched: u32,
    pub target_matches: u16,
    pub family_eligible: u32,
    pub family_searched: u32,
    pub family_matches: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecallReceipt {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub input_generation: u64,
    pub bank_digest: [u64; 4],
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub candidate_count: u16,
    pub exact_bucket_reads: u32,
    pub neighbor_bucket_reads: u32,
    pub similarity_evaluations: u32,
    pub candidates: Vec<CandidateMemoryRecallReceipt>,
    pub degradations: Vec<MemoryRecallDegradation>,
}

impl MemoryRecallReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != MEMORY_RECALL_SCHEMA_VERSION
            || self.organism_id_raw == 0
            || usize::from(self.candidate_count) != self.candidates.len()
            || self.candidates.is_empty()
            || self.candidates.len() > MAX_ACTION_CANDIDATES
            || self.similarity_evaluations
                > u32::from(self.candidate_count) * MEMORY_TOTAL_SEARCH_CAP as u32
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        for (index, candidate) in self.candidates.iter().enumerate() {
            if usize::from(candidate.candidate_index) != index
                || candidate.target_searched > candidate.target_eligible
                || candidate.family_searched > candidate.family_eligible
                || usize::from(candidate.target_matches) > MEMORY_RECALL_TOP_K
                || usize::from(candidate.family_matches) > MEMORY_RECALL_TOP_K
                || u32::from(candidate.target_matches) > candidate.target_searched
                || u32::from(candidate.family_matches) > candidate.family_searched
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedMemoryRecall {
    context: EpisodicRetrievalContextV1,
    candidate_queries: Vec<CandidateMemoryQueryV2>,
    base_frame_digest: PerceptionBaseDigest,
    receipt: MemoryRecallReceipt,
}

impl PreparedMemoryRecall {
    pub fn finalize(
        self,
        draft: PerceptionFrameDraft,
    ) -> Result<(PerceptionFrame, FinalizedMemoryRecall), ScaffoldContractError> {
        self.validate_for_draft(&draft)?;
        let context_block = self.context.to_perception_context_block()?;
        if context_block.canonical_digest() != self.receipt.context_digest {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let context_digest = context_block.canonical_digest();
        let frame = draft.finalize(context_block)?;
        let final_frame_digest = frame.frame_digest();
        let candidate_keys = self
            .candidate_queries
            .into_iter()
            .map(|query| EpisodicDecisionKeyV2::try_new(query, context_digest, final_frame_digest))
            .collect::<Result<Vec<_>, _>>()?;
        let finalized = FinalizedMemoryRecall {
            context: self.context,
            base_frame_digest: self.base_frame_digest,
            context_digest,
            final_frame_digest,
            candidate_keys,
            receipt: self.receipt,
        };
        finalized.validate_for_frame(&frame)?;
        Ok((frame, finalized))
    }

    pub fn validate_for_draft(
        &self,
        draft: &PerceptionFrameDraft,
    ) -> Result<(), ScaffoldContractError> {
        draft
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        self.context.validate_contract()?;
        self.receipt.validate_contract()?;
        let profile = draft.profile_provenance().identity();
        if self.base_frame_digest != draft.base_digest()
            || self.receipt.base_frame_digest != draft.base_digest()
            || self.receipt.organism_id_raw != draft.organism_id().raw()
            || self.context.tick != draft.tick()
            || self.context.profile != profile
            || self.context.candidates.len() != draft.candidates().len()
            || self.candidate_queries.len() != draft.candidates().len()
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let expected_context_digest = self
            .context
            .to_perception_context_block()?
            .canonical_digest();
        if self.receipt.context_digest != expected_context_digest {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        for (index, (query, candidate)) in self
            .candidate_queries
            .iter()
            .zip(draft.candidates())
            .enumerate()
        {
            let expected = MemoryQueryEncoderV2::encode_candidate(draft, candidate)?;
            let (target_key, family_key) = keys_for_query(query);
            let receipt = &self.receipt.candidates[index];
            if *query != expected
                || usize::from(query.candidate_index()) != index
                || receipt.query_digest != query.canonical_digest()
                || receipt.target_bucket != target_key.receipt()
                || receipt.family_bucket != family_key.receipt()
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        let similarity_evaluations = self
            .receipt
            .candidates
            .iter()
            .map(|candidate| candidate.target_searched + candidate.family_searched)
            .sum::<u32>();
        if similarity_evaluations != self.receipt.similarity_evaluations {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }

    pub const fn base_frame_digest(&self) -> PerceptionBaseDigest {
        self.base_frame_digest
    }

    pub const fn context(&self) -> &EpisodicRetrievalContextV1 {
        &self.context
    }

    pub const fn receipt(&self) -> &MemoryRecallReceipt {
        &self.receipt
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedMemoryRecall {
    context: EpisodicRetrievalContextV1,
    base_frame_digest: PerceptionBaseDigest,
    context_digest: PerceptionContextDigest,
    final_frame_digest: PerceptionFrameDigest,
    candidate_keys: Vec<EpisodicDecisionKeyV2>,
    receipt: MemoryRecallReceipt,
}

impl FinalizedMemoryRecall {
    pub fn validate_for_frame(&self, frame: &PerceptionFrame) -> Result<(), ScaffoldContractError> {
        frame
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        self.context.validate_contract()?;
        self.receipt.validate_contract()?;
        if self.base_frame_digest != frame.base_digest()
            || self.context_digest != frame.context().canonical_digest()
            || self.final_frame_digest != frame.frame_digest()
            || self.receipt.base_frame_digest != self.base_frame_digest
            || self.receipt.context_digest != self.context_digest
            || self.receipt.organism_id_raw != frame.organism_id().raw()
            || self.context.tick != frame.tick()
            || self.context.profile != frame.profile_provenance().identity()
            || self.context.candidates.len() != frame.candidates().len()
            || self.candidate_keys.len() != frame.candidates().len()
            || self
                .context
                .to_perception_context_block()?
                .canonical_digest()
                != self.context_digest
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        for (index, (key, candidate)) in self
            .candidate_keys
            .iter()
            .zip(frame.candidates())
            .enumerate()
        {
            key.validate_contract()?;
            key.query().validate_against_frame(frame, candidate)?;
            if usize::from(key.query().candidate_index()) != index
                || key.retrieval_context_digest() != self.context_digest
                || key.final_frame_digest() != self.final_frame_digest
                || self.receipt.candidates[index].query_digest != key.query().canonical_digest()
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        Ok(())
    }

    pub const fn base_frame_digest(&self) -> PerceptionBaseDigest {
        self.base_frame_digest
    }

    pub const fn context_digest(&self) -> PerceptionContextDigest {
        self.context_digest
    }

    pub const fn final_frame_digest(&self) -> PerceptionFrameDigest {
        self.final_frame_digest
    }

    pub const fn context(&self) -> &EpisodicRetrievalContextV1 {
        &self.context
    }

    pub fn candidate_keys(&self) -> &[EpisodicDecisionKeyV2] {
        &self.candidate_keys
    }

    pub const fn receipt(&self) -> &MemoryRecallReceipt {
        &self.receipt
    }
}

mod sidecar;

pub use sidecar::{
    MemoryCompactionCheckpoint, MemoryCompactionIdentity, MemoryCompactionPhase,
    MemoryCompactionReceipt, MemorySidecarState, MemoryUpdateKind, MemoryUpdateReceipt,
    PortableMemoryBankAssetV2, PortableMemoryRecordV2, PreparedMemoryCompaction,
    PORTABLE_MEMORY_BANK_ASSET_SCHEMA_VERSION,
};

mod candidate_index;

use candidate_index::{
    keys_for_query, CandidateMemoryRecordV2, CandidateMemoryStoreV2, MemoryBucketKey,
    TargetMemoryBucketKey,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryOutcomeSummary {
    pub success_likelihood: NormalizedScalar,
    pub contact_likelihood: NormalizedScalar,
    pub prediction_error: NormalizedScalar,
    pub pain_delta: NormalizedScalar,
    pub energy_delta: SignedValence,
}

impl MemoryOutcomeSummary {
    pub const fn neutral() -> Self {
        Self {
            success_likelihood: NormalizedScalar(0.0),
            contact_likelihood: NormalizedScalar(0.0),
            prediction_error: NormalizedScalar(0.0),
            pain_delta: NormalizedScalar(0.0),
            energy_delta: SignedValence(0.0),
        }
    }
}

impl Validate for MemoryOutcomeSummary {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        NormalizedScalar::new(self.success_likelihood.raw())?;
        NormalizedScalar::new(self.contact_likelihood.raw())?;
        NormalizedScalar::new(self.prediction_error.raw())?;
        NormalizedScalar::new(self.pain_delta.raw())?;
        SignedValence::new(self.energy_delta.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryExpectancy {
    pub expected_valence: SignedValence,
    pub predicted_drive_delta: DriveDelta,
    pub predicted_sensory_outcome: MemoryOutcomeSummary,
    pub affordance_bias: NormalizedScalar,
    pub danger_bias: NormalizedScalar,
    pub safety_bias: NormalizedScalar,
    pub social_trust_bias: NormalizedScalar,
    pub social_fear_bias: NormalizedScalar,
    pub novelty_bias: NormalizedScalar,
    pub curiosity_bias: NormalizedScalar,
    pub confidence: Confidence,
    pub source_memory_ids: Vec<MemoryId>,
}

impl MemoryExpectancy {
    pub fn neutral(confidence: Confidence) -> Result<Self, ScaffoldContractError> {
        Confidence::new(confidence.raw())?;
        Ok(Self {
            expected_valence: SignedValence(0.0),
            predicted_drive_delta: DriveDelta::zero(),
            predicted_sensory_outcome: MemoryOutcomeSummary::neutral(),
            affordance_bias: NormalizedScalar(0.0),
            danger_bias: NormalizedScalar(0.0),
            safety_bias: NormalizedScalar(0.0),
            social_trust_bias: NormalizedScalar(0.0),
            social_fear_bias: NormalizedScalar(0.0),
            novelty_bias: NormalizedScalar(0.0),
            curiosity_bias: NormalizedScalar(0.0),
            confidence,
            source_memory_ids: Vec::new(),
        })
    }
}

impl Validate for MemoryExpectancy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        SignedValence::new(self.expected_valence.raw())?;
        self.predicted_drive_delta.validate_contract()?;
        self.predicted_sensory_outcome.validate_contract()?;
        NormalizedScalar::new(self.affordance_bias.raw())?;
        NormalizedScalar::new(self.danger_bias.raw())?;
        NormalizedScalar::new(self.safety_bias.raw())?;
        NormalizedScalar::new(self.social_trust_bias.raw())?;
        NormalizedScalar::new(self.social_fear_bias.raw())?;
        NormalizedScalar::new(self.novelty_bias.raw())?;
        NormalizedScalar::new(self.curiosity_bias.raw())?;
        Confidence::new(self.confidence.raw())?;
        for memory_id in &self.source_memory_ids {
            memory_id.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub features: Vec<f32>,
}

impl MemoryQuery {
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        features: Vec<f32>,
    ) -> Result<Self, ScaffoldContractError> {
        let query = Self {
            organism_id,
            tick,
            features,
        };
        query.validate_contract()?;
        Ok(query)
    }
}

impl Validate for MemoryQuery {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        validate_feature_vector(&self.features)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryMatch {
    pub memory_id: MemoryId,
    pub score: f32,
    pub source_tick: Tick,
}

impl Validate for MemoryMatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.memory_id.validate()?;
        crate::validate_finite(self.score)?;
        if (0.0..=1.0).contains(&self.score) {
            Ok(())
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryBankConfig {
    pub capacity: usize,
    pub max_feature_len: usize,
    pub max_match_count: usize,
    pub min_match_score: f32,
    pub empty_confidence: Confidence,
}

impl MemoryBankConfig {
    pub fn new(
        capacity: usize,
        max_feature_len: usize,
        max_match_count: usize,
        min_match_score: f32,
        empty_confidence: Confidence,
    ) -> Result<Self, ScaffoldContractError> {
        let config = Self {
            capacity,
            max_feature_len,
            max_match_count,
            min_match_score,
            empty_confidence,
        };
        config.validate_contract()?;
        Ok(config)
    }
}

impl Validate for MemoryBankConfig {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.capacity == 0
            || self.capacity > MEMORY_BANK_MAX_CAPACITY
            || self.max_match_count == 0
            || self.max_match_count > self.capacity
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        validate_feature_cap(self.max_feature_len)?;
        crate::validate_finite(self.min_match_score)?;
        if !(0.0..=1.0).contains(&self.min_match_score) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Confidence::new(self.empty_confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub memory_id: MemoryId,
    pub organism_id: OrganismId,
    pub source_sequence_id: ExperienceSequenceId,
    pub source_tick: Tick,
    pub features: Vec<f32>,
    pub expected_valence: SignedValence,
    pub predicted_drive_delta: DriveDelta,
    pub outcome_summary: MemoryOutcomeSummary,
    pub affordance_bias: NormalizedScalar,
    pub danger_bias: NormalizedScalar,
    pub safety_bias: NormalizedScalar,
    pub social_trust_bias: NormalizedScalar,
    pub social_fear_bias: NormalizedScalar,
    pub novelty_bias: NormalizedScalar,
    pub curiosity_bias: NormalizedScalar,
    pub selected_action_id: Option<ActionId>,
    pub selected_action_kind: Option<ActionKind>,
}

impl MemoryRecord {
    pub fn from_sealed_patch(
        memory_id: MemoryId,
        patch: &ExperiencePatch,
        max_feature_len: usize,
    ) -> Result<Self, ScaffoldContractError> {
        memory_id.validate()?;
        patch.validate_contract()?;
        validate_feature_cap(max_feature_len)?;

        let pre_action = patch.pre_action();
        let decision = patch.decision();
        let outcome = patch.outcome();
        let contact_likelihood = match outcome.physical.contact {
            PhysicalContactKind::None => 0.0,
            _ => 1.0,
        };
        let positive_reward = outcome.reward_valence.raw().max(0.0);
        let negative_reward = (-outcome.reward_valence.raw()).max(0.0);
        let social_bias = social_biases(pre_action);
        let record = Self {
            memory_id,
            organism_id: pre_action.organism_id,
            source_sequence_id: pre_action.sequence_id,
            source_tick: pre_action.tick,
            features: legacy_diagnostic_features(patch, max_feature_len)?,
            expected_valence: outcome.reward_valence,
            predicted_drive_delta: outcome.homeostatic_delta.drives,
            outcome_summary: MemoryOutcomeSummary {
                success_likelihood: NormalizedScalar(if outcome.success { 1.0 } else { 0.0 }),
                contact_likelihood: NormalizedScalar(contact_likelihood),
                prediction_error: outcome.prediction_error,
                pain_delta: outcome.pain_delta,
                energy_delta: outcome.energy_delta,
            },
            affordance_bias: NormalizedScalar::new(max_affordance(pre_action))?,
            danger_bias: NormalizedScalar::new(
                negative_reward
                    .max(outcome.pain_delta.raw())
                    .max(outcome.frustration_delta.raw()),
            )?,
            safety_bias: NormalizedScalar::new(if outcome.success {
                positive_reward.max(0.25)
            } else {
                0.0
            })?,
            social_trust_bias: NormalizedScalar::new(social_bias.0)?,
            social_fear_bias: NormalizedScalar::new(social_bias.1)?,
            novelty_bias: pre_action.sensory().channels.novelty_signal,
            curiosity_bias: NormalizedScalar::new(pre_action.homeostasis().drives.curiosity)?,
            selected_action_id: Some(decision.selected_action.action_id),
            selected_action_kind: Some(decision.selected_action.kind),
        };
        record.validate_contract()?;
        Ok(record)
    }
}

impl Validate for MemoryRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.memory_id.validate()?;
        self.organism_id.validate()?;
        self.source_sequence_id.validate()?;
        validate_feature_vector(&self.features)?;
        SignedValence::new(self.expected_valence.raw())?;
        self.predicted_drive_delta.validate_contract()?;
        self.outcome_summary.validate_contract()?;
        NormalizedScalar::new(self.affordance_bias.raw())?;
        NormalizedScalar::new(self.danger_bias.raw())?;
        NormalizedScalar::new(self.safety_bias.raw())?;
        NormalizedScalar::new(self.social_trust_bias.raw())?;
        NormalizedScalar::new(self.social_fear_bias.raw())?;
        NormalizedScalar::new(self.novelty_bias.raw())?;
        NormalizedScalar::new(self.curiosity_bias.raw())?;
        if let Some(action_id) = self.selected_action_id {
            action_id.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MemoryBank {
    config: MemoryBankConfig,
    #[serde(default)]
    candidate_store: CandidateMemoryStoreV2,
    records: Vec<Option<MemoryRecord>>,
    next_write_index: usize,
    len: usize,
    next_memory_id: u64,
    last_inserted_ticks: Vec<(OrganismId, Tick)>,
}

impl<'de> Deserialize<'de> for MemoryBank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            config: MemoryBankConfig,
            #[serde(default)]
            candidate_store: CandidateMemoryStoreV2,
            records: Vec<Option<MemoryRecord>>,
            next_write_index: usize,
            len: usize,
            next_memory_id: u64,
            last_inserted_ticks: Vec<(OrganismId, Tick)>,
        }

        let wire = Wire::deserialize(deserializer)?;
        wire.config.validate_contract().map_err(D::Error::custom)?;
        wire.candidate_store
            .validate_for_capacity(wire.config.capacity)
            .map_err(D::Error::custom)?;
        let legacy_count = wire.records.iter().flatten().count();
        if wire.records.len() != wire.config.capacity
            || wire.next_write_index >= wire.config.capacity
            || wire.len > wire.config.capacity
            || legacy_count != wire.len
            || wire.next_memory_id == 0
            || (wire.len != 0 && !wire.candidate_store.records.is_empty())
        {
            return Err(D::Error::custom("invalid or mixed memory-bank storage"));
        }
        let mut highest_legacy_id = 0_u64;
        for record in wire.records.iter().flatten() {
            record.validate_contract().map_err(D::Error::custom)?;
            if record.features.len() > wire.config.max_feature_len {
                return Err(D::Error::custom("legacy memory feature width exceeds bank"));
            }
            highest_legacy_id = highest_legacy_id.max(record.memory_id.raw());
        }
        if wire.next_memory_id <= highest_legacy_id {
            return Err(D::Error::custom("legacy memory identity would be reused"));
        }
        let mut seen_organisms = std::collections::BTreeSet::new();
        for (organism, _) in &wire.last_inserted_ticks {
            organism.validate().map_err(D::Error::custom)?;
            if !seen_organisms.insert(organism.raw()) {
                return Err(D::Error::custom("duplicate legacy memory tick guard"));
            }
        }

        Ok(Self {
            config: wire.config,
            candidate_store: wire.candidate_store,
            records: wire.records,
            next_write_index: wire.next_write_index,
            len: wire.len,
            next_memory_id: wire.next_memory_id,
            last_inserted_ticks: wire.last_inserted_ticks,
        })
    }
}

impl MemoryBank {
    pub fn new(config: MemoryBankConfig) -> Result<Self, ScaffoldContractError> {
        config.validate_contract()?;
        Ok(Self {
            records: vec![None; config.capacity],
            config,
            candidate_store: CandidateMemoryStoreV2::default(),
            next_write_index: 0,
            len: 0,
            next_memory_id: 1,
            last_inserted_ticks: Vec::new(),
        })
    }

    pub const fn capacity(&self) -> usize {
        self.config.capacity
    }

    pub fn len(&self) -> usize {
        self.len + self.candidate_store.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0 && self.candidate_store.records.is_empty()
    }

    pub fn recall_frame(
        &self,
        draft: &PerceptionFrameDraft,
    ) -> Result<PreparedMemoryRecall, ScaffoldContractError> {
        self.config.validate_contract()?;
        if self.len != 0 {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        draft
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let profile = draft.profile_provenance().identity();
        let mut candidate_queries = Vec::with_capacity(draft.candidates().len());
        let mut candidate_contexts = Vec::with_capacity(draft.candidates().len());
        let mut candidate_receipts = Vec::with_capacity(draft.candidates().len());
        let mut exact_bucket_reads = 0_u32;
        let mut neighbor_bucket_reads = 0_u32;
        let mut similarity_evaluations = 0_u32;
        let mut degradations = Vec::new();

        for candidate in draft.candidates() {
            let query = MemoryQueryEncoderV2::encode_candidate(draft, candidate)?;
            let (target_bucket, family_bucket) = keys_for_query(&query);
            let has_target = query.tracked_object_id().is_some();
            exact_bucket_reads = exact_bucket_reads.saturating_add(if has_target { 2 } else { 1 });
            let family_neighbors = neighbor_family_keys(&family_bucket).len().saturating_sub(1);
            let target_neighbors = if has_target {
                neighbor_target_keys(&target_bucket).len().saturating_sub(1)
            } else {
                0
            };
            neighbor_bucket_reads = neighbor_bucket_reads.saturating_add(
                u32::try_from(family_neighbors + target_neighbors).unwrap_or(u32::MAX),
            );
            let target = recall_target_channel(&self.candidate_store, &query, &target_bucket)?;
            let family = recall_family_channel(&self.candidate_store, &query, &family_bucket)?;
            similarity_evaluations = similarity_evaluations
                .saturating_add(target.searched)
                .saturating_add(family.searched);
            if target.eligible > target.searched {
                degradations.push(MemoryRecallDegradation::SearchShortlisted {
                    candidate_index: candidate.candidate_index,
                    channel: MemoryRecallChannel::TargetLatent,
                    eligible: target.eligible,
                    searched: target.searched,
                });
            }
            if family.eligible > family.searched {
                degradations.push(MemoryRecallDegradation::SearchShortlisted {
                    candidate_index: candidate.candidate_index,
                    channel: MemoryRecallChannel::FamilyValue,
                    eligible: family.eligible,
                    searched: family.searched,
                });
            }
            candidate_contexts.push(CandidateMemoryContextV1 {
                candidate_index: candidate.candidate_index,
                target_latent: target.values,
                family_value: family.values,
                target_confidence: target.confidence,
                family_confidence: family.confidence,
                target_source_count: target.source_count,
                family_source_count: family.source_count,
                best_target_source: target.best_source,
                best_family_source: family.best_source,
            });
            candidate_receipts.push(CandidateMemoryRecallReceipt {
                candidate_index: candidate.candidate_index,
                query_digest: query.canonical_digest(),
                target_bucket: target_bucket.receipt(),
                family_bucket: family_bucket.receipt(),
                target_eligible: target.eligible,
                target_searched: target.searched,
                target_matches: target.matches,
                family_eligible: family.eligible,
                family_searched: family.searched,
                family_matches: family.matches,
            });
            candidate_queries.push(query);
        }

        let context = EpisodicRetrievalContextV1::new(draft.tick(), profile, candidate_contexts)?;
        let context_digest = context.to_perception_context_block()?.canonical_digest();
        let receipt = MemoryRecallReceipt {
            schema_version: MEMORY_RECALL_SCHEMA_VERSION,
            organism_id_raw: draft.organism_id().raw(),
            input_generation: self.candidate_store.generation,
            bank_digest: self.candidate_store.digest(self.capacity())?,
            base_frame_digest: draft.base_digest(),
            context_digest,
            candidate_count: u16::try_from(draft.candidates().len())
                .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
            exact_bucket_reads,
            neighbor_bucket_reads,
            similarity_evaluations,
            candidates: candidate_receipts,
            degradations,
        };
        receipt.validate_contract()?;
        let prepared = PreparedMemoryRecall {
            context,
            candidate_queries,
            base_frame_digest: draft.base_digest(),
            receipt,
        };
        prepared.validate_for_draft(draft)?;
        Ok(prepared)
    }

    pub fn observe_sealed_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<MemoryUpdateReceipt, ScaffoldContractError> {
        patch.validate_contract()?;
        if self.len != 0 {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        let key = patch
            .decision()
            .episodic_key()
            .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
        key.validate_contract()?;
        let query = key.query();
        let organism_id_raw = patch.pre_action().organism_id.raw();
        let sequence_raw = patch.header().sequence_id.raw();
        if query.organism_id().raw() != organism_id_raw {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if self
            .candidate_store
            .last_sequence_by_organism
            .get(&organism_id_raw)
            .is_some_and(|last| sequence_raw <= *last)
        {
            return Err(ScaffoldContractError::MemoryReplayRejected);
        }

        let before_digest = self.candidate_store.digest(self.config.capacity)?;
        let input_generation = self.candidate_store.generation;
        let output_generation = input_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        let capacity = u32::try_from(self.config.capacity)
            .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        let candidate_memory_id = MemoryId(self.candidate_store.next_memory_id);
        candidate_memory_id.validate()?;
        let candidate_record = candidate_record_from_patch(candidate_memory_id, patch)?;
        let (target_key, family_key) = keys_for_query(query);

        let merge_id = self
            .candidate_store
            .family_index
            .get(&family_key)
            .into_iter()
            .flatten()
            .filter_map(|id| {
                self.candidate_store
                    .records
                    .get(&id.raw())
                    .filter(|record| {
                        record.identity() == candidate_record.identity()
                            && family_similarity(
                                &candidate_record.query_features,
                                &record.query_features,
                            ) >= MEMORY_MERGE_SIMILARITY
                    })
                    .map(|_| *id)
            })
            .min_by_key(|id| id.raw());

        enum MutationPlan {
            Merge {
                memory_id: MemoryId,
                old: CandidateMemoryRecordV2,
                merged: CandidateMemoryRecordV2,
                merge_count: u64,
            },
            Insert {
                record: CandidateMemoryRecordV2,
                next_memory_id: u64,
            },
            EvictAndInsert {
                removed: CandidateMemoryRecordV2,
                record: CandidateMemoryRecordV2,
                next_memory_id: u64,
                eviction_count: u64,
            },
        }

        let (kind, plan) = if let Some(memory_id) = merge_id {
            let old = self.candidate_store.records[&memory_id.raw()].clone();
            let merged = merge_candidate_records(&old, &candidate_record)?;
            let merge_count = self
                .candidate_store
                .merge_count
                .checked_add(1)
                .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
            (
                MemoryUpdateKind::Merged { into: memory_id },
                MutationPlan::Merge {
                    memory_id,
                    old,
                    merged,
                    merge_count,
                },
            )
        } else {
            let next_memory_id = self
                .candidate_store
                .next_memory_id
                .checked_add(1)
                .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
            if self.candidate_store.records.len() == self.config.capacity {
                let removed = self
                    .candidate_store
                    .records
                    .values()
                    .min_by_key(|record| {
                        (
                            record.salience_q16,
                            record.last_tick.raw(),
                            record.memory_id.raw(),
                        )
                    })
                    .cloned()
                    .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
                let eviction_count = self
                    .candidate_store
                    .eviction_count
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                (
                    MemoryUpdateKind::Evicted {
                        removed: removed.memory_id,
                        inserted: candidate_memory_id,
                    },
                    MutationPlan::EvictAndInsert {
                        removed,
                        record: candidate_record,
                        next_memory_id,
                        eviction_count,
                    },
                )
            } else {
                (
                    MemoryUpdateKind::Inserted {
                        inserted: candidate_memory_id,
                    },
                    MutationPlan::Insert {
                        record: candidate_record,
                        next_memory_id,
                    },
                )
            }
        };

        let record_count_after = match &plan {
            MutationPlan::Merge { .. } | MutationPlan::EvictAndInsert { .. } => {
                self.candidate_store.records.len()
            }
            MutationPlan::Insert { .. } => self
                .candidate_store
                .records
                .len()
                .checked_add(1)
                .ok_or(ScaffoldContractError::ScalarOutOfRange)?,
        };
        let record_count = u32::try_from(record_count_after)
            .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        let previous_last_sequence = self
            .candidate_store
            .last_sequence_by_organism
            .get(&organism_id_raw)
            .copied();
        let previous_next_memory_id = self.candidate_store.next_memory_id;
        let previous_merge_count = self.candidate_store.merge_count;
        let previous_eviction_count = self.candidate_store.eviction_count;

        match &plan {
            MutationPlan::Merge {
                memory_id,
                old,
                merged,
                merge_count,
            } => {
                self.candidate_store.remove_record_from_indices(old);
                self.candidate_store
                    .records
                    .insert(memory_id.raw(), merged.clone());
                self.candidate_store.insert_record_into_indices(*memory_id);
                self.candidate_store.merge_count = *merge_count;
            }
            MutationPlan::Insert {
                record,
                next_memory_id,
            } => {
                self.candidate_store
                    .records
                    .insert(record.memory_id.raw(), record.clone());
                self.candidate_store
                    .insert_record_into_indices(record.memory_id);
                self.candidate_store.next_memory_id = *next_memory_id;
            }
            MutationPlan::EvictAndInsert {
                removed,
                record,
                next_memory_id,
                eviction_count,
            } => {
                self.candidate_store
                    .records
                    .remove(&removed.memory_id.raw());
                self.candidate_store.remove_record_from_indices(removed);
                self.candidate_store
                    .records
                    .insert(record.memory_id.raw(), record.clone());
                self.candidate_store
                    .insert_record_into_indices(record.memory_id);
                self.candidate_store.next_memory_id = *next_memory_id;
                self.candidate_store.eviction_count = *eviction_count;
            }
        }
        self.candidate_store.generation = output_generation;
        self.candidate_store
            .last_sequence_by_organism
            .insert(organism_id_raw, sequence_raw);

        let after_digest = match self.candidate_store.digest(self.config.capacity) {
            Ok(digest) => digest,
            Err(error) => {
                self.candidate_store.generation = input_generation;
                self.candidate_store.next_memory_id = previous_next_memory_id;
                self.candidate_store.merge_count = previous_merge_count;
                self.candidate_store.eviction_count = previous_eviction_count;
                match previous_last_sequence {
                    Some(sequence) => {
                        self.candidate_store
                            .last_sequence_by_organism
                            .insert(organism_id_raw, sequence);
                    }
                    None => {
                        self.candidate_store
                            .last_sequence_by_organism
                            .remove(&organism_id_raw);
                    }
                }
                match &plan {
                    MutationPlan::Merge { memory_id, old, .. } => {
                        self.candidate_store
                            .records
                            .insert(memory_id.raw(), old.clone());
                    }
                    MutationPlan::Insert { record, .. } => {
                        self.candidate_store.records.remove(&record.memory_id.raw());
                    }
                    MutationPlan::EvictAndInsert {
                        removed, record, ..
                    } => {
                        self.candidate_store.records.remove(&record.memory_id.raw());
                        self.candidate_store
                            .records
                            .insert(removed.memory_id.raw(), removed.clone());
                    }
                }
                self.candidate_store.rebuild_indices();
                return Err(error);
            }
        };

        Ok(MemoryUpdateReceipt {
            sealed_sequence_id: patch.header().sequence_id,
            organism_id_raw,
            bucket: family_key.receipt(),
            target_bucket: target_key.receipt(),
            input_generation,
            output_generation,
            kind,
            record_count,
            capacity,
            merge_count: self.candidate_store.merge_count,
            eviction_count: self.candidate_store.eviction_count,
            before_digest,
            after_digest,
        })
    }

    fn compact_candidate_records(
        &mut self,
        organism_id: OrganismId,
        cycle_id: u64,
        max_records_after: u32,
        policy_version: u16,
    ) -> Result<MemoryCompactionReceipt, ScaffoldContractError> {
        organism_id.validate()?;
        if cycle_id == 0
            || max_records_after == 0
            || usize::try_from(max_records_after).unwrap_or(usize::MAX) > self.config.capacity
            || policy_version == 0
            || self.len != 0
            || self
                .candidate_store
                .records
                .values()
                .any(|record| record.organism_id_raw != organism_id.raw())
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let input_generation = self.candidate_store.generation;
        let input_digest = self.candidate_store.digest(self.config.capacity)?;
        let identity = MemoryCompactionIdentity {
            organism_id_raw: organism_id.raw(),
            cycle_id,
            policy_version,
            max_records_after,
            input_generation,
            input_digest,
        };
        let output_generation = input_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;

        let mut records = self
            .candidate_store
            .records
            .values()
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.identity()
                .cmp(&right.identity())
                .then_with(|| left.memory_id.raw().cmp(&right.memory_id.raw()))
        });
        let mut folded: Vec<CandidateMemoryRecordV2> = Vec::with_capacity(records.len());
        let mut merged_count = 0_u32;
        for record in records {
            if let Some(previous) = folded.last_mut() {
                if previous.identity() == record.identity()
                    && family_similarity(&previous.query_features, &record.query_features)
                        >= MEMORY_MERGE_SIMILARITY
                {
                    *previous = merge_candidate_records(previous, &record)?;
                    merged_count = merged_count.saturating_add(1);
                    continue;
                }
            }
            folded.push(record);
        }
        folded.sort_by(|left, right| {
            right
                .salience_q16
                .cmp(&left.salience_q16)
                .then_with(|| right.last_tick.raw().cmp(&left.last_tick.raw()))
                .then_with(|| right.observation_count.cmp(&left.observation_count))
                .then_with(|| left.memory_id.raw().cmp(&right.memory_id.raw()))
        });
        let keep = usize::try_from(max_records_after)
            .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        let evicted_count = u32::try_from(folded.len().saturating_sub(keep))
            .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        folded.truncate(keep);
        folded.sort_by_key(|record| (record.last_tick.raw(), record.memory_id.raw()));
        self.candidate_store.records = folded
            .into_iter()
            .map(|record| (record.memory_id.raw(), record))
            .collect();
        self.candidate_store.merge_count = self
            .candidate_store
            .merge_count
            .checked_add(u64::from(merged_count))
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        self.candidate_store.eviction_count = self
            .candidate_store
            .eviction_count
            .checked_add(u64::from(evicted_count))
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        self.candidate_store.generation = output_generation;
        self.candidate_store.rebuild_indices();
        let output_digest = self.candidate_store.digest(self.config.capacity)?;
        Ok(MemoryCompactionReceipt {
            identity,
            output_generation,
            output_digest,
            merged: merged_count,
            evicted: evicted_count,
            record_count: u32::try_from(self.candidate_store.records.len())
                .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?,
            capacity: u32::try_from(self.config.capacity)
                .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?,
        })
    }

    pub fn insert_from_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<MemoryId, ScaffoldContractError> {
        if !self.candidate_store.records.is_empty() {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        patch.validate_contract()?;
        let organism_id = patch.pre_action().organism_id;
        let source_tick = patch.pre_action().tick;
        self.validate_monotonic_insert(organism_id, source_tick)?;

        let memory_id = MemoryId(self.next_memory_id);
        let record =
            MemoryRecord::from_sealed_patch(memory_id, patch, self.config.max_feature_len)?;
        self.insert_record(record)?;
        self.next_memory_id = self
            .next_memory_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        self.record_last_tick(organism_id, source_tick);
        Ok(memory_id)
    }

    pub fn insert_record(
        &mut self,
        record: MemoryRecord,
    ) -> Result<MemoryId, ScaffoldContractError> {
        if !self.candidate_store.records.is_empty() {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        record.validate_contract()?;
        if record.features.len() > self.config.max_feature_len {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let memory_id = record.memory_id;
        self.records[self.next_write_index] = Some(record);
        self.next_write_index = (self.next_write_index + 1) % self.config.capacity;
        self.len = (self.len + 1).min(self.config.capacity);
        Ok(memory_id)
    }

    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryMatch>, ScaffoldContractError> {
        if !self.candidate_store.records.is_empty() {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        query.validate_contract()?;
        if query.features.len() > self.config.max_feature_len {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }

        let mut matches = Vec::new();
        for record in self.records_chronological() {
            if record.organism_id != query.organism_id {
                continue;
            }
            let score = normalized_dot(&query.features, &record.features)?;
            if score >= self.config.min_match_score {
                matches.push(MemoryMatch {
                    memory_id: record.memory_id,
                    score,
                    source_tick: record.source_tick,
                });
            }
        }

        matches.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.memory_id.raw().cmp(&b.memory_id.raw()))
        });
        matches.truncate(self.config.max_match_count);
        for memory_match in &matches {
            memory_match.validate_contract()?;
        }
        Ok(matches)
    }

    pub fn recall(&self, query: &MemoryQuery) -> Result<MemoryExpectancy, ScaffoldContractError> {
        if !self.candidate_store.records.is_empty() {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        let matches = self.query(query)?;
        if matches.is_empty() {
            return MemoryExpectancy::neutral(self.config.empty_confidence);
        }

        let total_weight: f32 = matches.iter().map(|memory_match| memory_match.score).sum();
        if total_weight <= 0.0 {
            return MemoryExpectancy::neutral(self.config.empty_confidence);
        }

        let mut expected_valence = 0.0;
        let mut drive_delta = DriveDelta::zero();
        let mut outcome = WeightedOutcomeSummary::default();
        let mut affordance_bias = 0.0;
        let mut danger_bias = 0.0;
        let mut safety_bias = 0.0;
        let mut social_trust_bias = 0.0;
        let mut social_fear_bias = 0.0;
        let mut novelty_bias = 0.0;
        let mut curiosity_bias = 0.0;
        let mut source_memory_ids = Vec::with_capacity(matches.len());

        for memory_match in &matches {
            let Some(record) = self.record_by_id(memory_match.memory_id) else {
                return Err(ScaffoldContractError::InvalidId);
            };
            let weight = memory_match.score / total_weight;
            expected_valence += record.expected_valence.raw() * weight;
            drive_delta = weighted_drive_delta(drive_delta, record.predicted_drive_delta, weight);
            outcome.add(record.outcome_summary, weight);
            affordance_bias += record.affordance_bias.raw() * weight;
            danger_bias += record.danger_bias.raw() * weight;
            safety_bias += record.safety_bias.raw() * weight;
            social_trust_bias += record.social_trust_bias.raw() * weight;
            social_fear_bias += record.social_fear_bias.raw() * weight;
            novelty_bias += record.novelty_bias.raw() * weight;
            curiosity_bias += record.curiosity_bias.raw() * weight;
            source_memory_ids.push(record.memory_id);
        }

        let average_score = total_weight / matches.len() as f32;
        let empty_confidence = self.config.empty_confidence.raw();
        let confidence = empty_confidence + average_score * (1.0 - empty_confidence);
        let expectancy = MemoryExpectancy {
            expected_valence: SignedValence::new(expected_valence.clamp(-1.0, 1.0))?,
            predicted_drive_delta: drive_delta,
            predicted_sensory_outcome: outcome.finish()?,
            affordance_bias: NormalizedScalar::new(affordance_bias.clamp(0.0, 1.0))?,
            danger_bias: NormalizedScalar::new(danger_bias.clamp(0.0, 1.0))?,
            safety_bias: NormalizedScalar::new(safety_bias.clamp(0.0, 1.0))?,
            social_trust_bias: NormalizedScalar::new(social_trust_bias.clamp(0.0, 1.0))?,
            social_fear_bias: NormalizedScalar::new(social_fear_bias.clamp(0.0, 1.0))?,
            novelty_bias: NormalizedScalar::new(novelty_bias.clamp(0.0, 1.0))?,
            curiosity_bias: NormalizedScalar::new(curiosity_bias.clamp(0.0, 1.0))?,
            confidence: Confidence::new(confidence.clamp(0.0, 1.0))?,
            source_memory_ids,
        };
        expectancy.validate_contract()?;
        Ok(expectancy)
    }

    pub fn records_chronological(&self) -> Vec<&MemoryRecord> {
        let mut records = Vec::with_capacity(self.len);
        if self.len == 0 {
            return records;
        }

        let start = if self.len == self.config.capacity {
            self.next_write_index
        } else {
            0
        };
        for offset in 0..self.len {
            let index = (start + offset) % self.config.capacity;
            if let Some(record) = &self.records[index] {
                records.push(record);
            }
        }
        records
    }

    pub fn replace_with_consolidated_records(
        &mut self,
        records: Vec<MemoryRecord>,
    ) -> Result<(), ScaffoldContractError> {
        if !self.candidate_store.records.is_empty() {
            return Err(ScaffoldContractError::MemoryModeConflict);
        }
        if records.len() > self.config.capacity {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.records.fill(None);
        self.next_write_index = 0;
        self.len = 0;
        for record in records {
            self.insert_record(record)?;
        }
        Ok(())
    }

    fn record_by_id(&self, memory_id: MemoryId) -> Option<&MemoryRecord> {
        self.records
            .iter()
            .flatten()
            .find(|record| record.memory_id == memory_id)
    }

    fn validate_monotonic_insert(
        &self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<(), ScaffoldContractError> {
        organism_id.validate()?;
        if let Some((_, previous)) = self
            .last_inserted_ticks
            .iter()
            .find(|(known_organism, _)| *known_organism == organism_id)
        {
            Tick::validate_monotonic(*previous, tick)?;
        }
        Ok(())
    }

    fn record_last_tick(&mut self, organism_id: OrganismId, tick: Tick) {
        if let Some((_, previous)) = self
            .last_inserted_ticks
            .iter_mut()
            .find(|(known_organism, _)| *known_organism == organism_id)
        {
            *previous = tick;
        } else {
            self.last_inserted_ticks.push((organism_id, tick));
        }
    }
}

mod candidate_recall;

use candidate_recall::{
    candidate_record_from_patch, family_similarity, merge_candidate_records, neighbor_family_keys,
    neighbor_target_keys, recall_family_channel, recall_target_channel,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryConsolidationBatch {
    pub records: Vec<MemoryRecord>,
    pub max_records_after: usize,
}

impl Validate for MemoryConsolidationBatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.max_records_after == 0 || self.records.len() > self.max_records_after {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for record in &self.records {
            record.validate_contract()?;
        }
        Ok(())
    }
}

pub trait MemoryConsolidator {
    fn consolidate(
        &self,
        batch: MemoryConsolidationBatch,
    ) -> Result<Vec<MemoryRecord>, ScaffoldContractError>;
}

#[derive(Default)]
struct WeightedOutcomeSummary {
    success_likelihood: f32,
    contact_likelihood: f32,
    prediction_error: f32,
    pain_delta: f32,
    energy_delta: f32,
}

impl WeightedOutcomeSummary {
    fn add(&mut self, summary: MemoryOutcomeSummary, weight: f32) {
        self.success_likelihood += summary.success_likelihood.raw() * weight;
        self.contact_likelihood += summary.contact_likelihood.raw() * weight;
        self.prediction_error += summary.prediction_error.raw() * weight;
        self.pain_delta += summary.pain_delta.raw() * weight;
        self.energy_delta += summary.energy_delta.raw() * weight;
    }

    fn finish(self) -> Result<MemoryOutcomeSummary, ScaffoldContractError> {
        Ok(MemoryOutcomeSummary {
            success_likelihood: NormalizedScalar::new(self.success_likelihood.clamp(0.0, 1.0))?,
            contact_likelihood: NormalizedScalar::new(self.contact_likelihood.clamp(0.0, 1.0))?,
            prediction_error: NormalizedScalar::new(self.prediction_error.clamp(0.0, 1.0))?,
            pain_delta: NormalizedScalar::new(self.pain_delta.clamp(0.0, 1.0))?,
            energy_delta: SignedValence::new(self.energy_delta.clamp(-1.0, 1.0))?,
        })
    }
}

fn validate_feature_cap(max_feature_len: usize) -> Result<(), ScaffoldContractError> {
    if max_feature_len == 0 || max_feature_len > MEMORY_FEATURE_VECTOR_MAX_LEN {
        Err(ScaffoldContractError::ScalarOutOfRange)
    } else {
        Ok(())
    }
}

fn validate_feature_vector(features: &[f32]) -> Result<(), ScaffoldContractError> {
    if features.is_empty() || features.len() > MEMORY_FEATURE_VECTOR_MAX_LEN {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    validate_finite_slice(features)?;
    if features.iter().all(|value| (-1.0..=1.0).contains(value)) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn legacy_diagnostic_features(
    patch: &ExperiencePatch,
    max_feature_len: usize,
) -> Result<Vec<f32>, ScaffoldContractError> {
    validate_feature_cap(max_feature_len)?;
    patch.validate_contract()?;
    let pre_action = patch.pre_action();
    let channels = &pre_action.sensory().channels;
    let mut encoded = Vec::with_capacity(MEMORY_FEATURE_VECTOR_MAX_LEN);
    for values in [
        channels.visual_affordance.as_slice(),
        channels.auditory_acoustic.as_slice(),
        channels.smell_chemistry.as_slice(),
        channels.tactile_contact.as_slice(),
    ] {
        let mean = values.iter().copied().sum::<f32>() / values.len() as f32;
        let maximum = values.iter().copied().fold(0.0, f32::max);
        encoded.extend([mean, maximum]);
    }
    encoded.extend([
        channels.pain_signal.raw(),
        channels.novelty_signal.raw(),
        (channels.nearby_affordances.raw().count_ones() as f32 / 10.0).min(1.0),
        patch.decision().confidence.raw(),
    ]);
    encoded.extend(pre_action.homeostasis().drives.to_array());
    encoded.extend(pre_action.homeostasis().hormones.to_array());
    let velocity = pre_action.body().velocity;
    encoded.extend([
        velocity.linear.x.clamp(-1.0, 1.0),
        velocity.linear.y.clamp(-1.0, 1.0),
        velocity.linear.z.clamp(-1.0, 1.0),
        velocity.angular.x.clamp(-1.0, 1.0),
        velocity.angular.y.clamp(-1.0, 1.0),
        velocity.angular.z.clamp(-1.0, 1.0),
    ]);
    for raw_kind in 0..9 {
        encoded.push(if patch.decision().selected_action.kind.raw() == raw_kind {
            1.0
        } else {
            0.0
        });
    }
    if let Some(candidate) = pre_action
        .perception()
        .candidates()
        .iter()
        .find(|candidate| candidate.action_id == patch.decision().selected_action.action_id)
    {
        encoded.extend(candidate.features.0);
    }
    encoded.truncate(max_feature_len);
    let features = encoded;
    validate_feature_vector(&features)?;
    Ok(features)
}

fn normalized_dot(query: &[f32], record: &[f32]) -> Result<f32, ScaffoldContractError> {
    validate_feature_vector(query)?;
    validate_feature_vector(record)?;
    let len = query.len().min(record.len());
    let mut dot = 0.0;
    let mut query_norm = 0.0;
    let mut record_norm = 0.0;
    for index in 0..len {
        dot += query[index] * record[index];
        query_norm += query[index] * query[index];
        record_norm += record[index] * record[index];
    }
    if query_norm == 0.0 || record_norm == 0.0 {
        return Ok(0.0);
    }
    let score = dot / (query_norm.sqrt() * record_norm.sqrt());
    crate::validate_finite(score)?;
    Ok(score.clamp(0.0, 1.0))
}

fn max_affordance(pre_action: &PreActionSnapshot) -> f32 {
    pre_action
        .sensory()
        .channels
        .visual_affordance
        .iter()
        .copied()
        .fold(0.0, f32::max)
}

fn social_biases(pre_action: &PreActionSnapshot) -> (f32, f32) {
    let mut trust = 0.0_f32;
    let mut fear = 0.0_f32;
    for agent in pre_action
        .sensory()
        .social_context
        .nearest_agents
        .iter()
        .flatten()
    {
        let weighted_affinity = agent.affinity.raw() * agent.proximity.raw();
        trust = trust.max(weighted_affinity.max(0.0));
        fear = fear.max((-weighted_affinity).max(0.0));
    }
    (trust.clamp(0.0, 1.0), fear.clamp(0.0, 1.0))
}

fn weighted_drive_delta(current: DriveDelta, next: DriveDelta, weight: f32) -> DriveDelta {
    DriveDelta {
        hunger: current.hunger + next.hunger * weight,
        fatigue: current.fatigue + next.fatigue * weight,
        fear: current.fear + next.fear * weight,
        pain: current.pain + next.pain * weight,
        loneliness: current.loneliness + next.loneliness * weight,
        curiosity: current.curiosity + next.curiosity * weight,
        brain_atp: current.brain_atp + next.brain_atp * weight,
        temperature_stress: current.temperature_stress + next.temperature_stress * weight,
        reproductive_drive: current.reproductive_drive + next.reproductive_drive * weight,
        extension: [
            current.extension[0] + next.extension[0] * weight,
            current.extension[1] + next.extension[1] * weight,
        ],
    }
}
