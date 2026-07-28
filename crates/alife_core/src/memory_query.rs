//! Contract-only candidate-conditioned episodic query and retrieval-context records.

use std::ops::Range;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    ensure_current_version, ActionCandidate, ActionId, ActionKind, BodySnapshot,
    CandidateActionFamily, CandidateFeatureDigest, CandidateObservationRef, CanonicalDigestBuilder,
    Confidence, HomeostaticSnapshot, MemoryId, OrganismId, PerceptionBaseDigest,
    PerceptionContextBlock, PerceptionContextDigest, PerceptionContextKind, PerceptionFrame,
    PerceptionFrameDigest, PerceptionFrameDraft, ScaffoldContractError, SchemaKind, SchemaVersions,
    SensorProfileIdentity, SensorySnapshot, Tick, TrackedObjectId, Validate, MAX_ACTION_CANDIDATES,
};

pub const MEMORY_QUERY_V2_FEATURE_COUNT: usize = 96;
pub const MEMORY_STATE_SENSORY_RANGE: Range<usize> = 0..12;
pub const MEMORY_DRIVE_RANGE: Range<usize> = 12..23;
pub const MEMORY_HORMONE_RANGE: Range<usize> = 23..34;
pub const MEMORY_BODY_RANGE: Range<usize> = 34..40;
pub const MEMORY_ACTION_KIND_RANGE: Range<usize> = 40..49;
pub const MEMORY_ACTION_FAMILY_RANGE: Range<usize> = 49..57;
pub const MEMORY_TARGET_RANGE: Range<usize> = 57..81;
pub const MEMORY_PROFILE_RANGE: Range<usize> = 81..83;
pub const MEMORY_RESERVED_RANGE: Range<usize> = 83..96;
pub const MEMORY_LATENT_V1_COUNT: usize = 8;
pub const MEMORY_VALUE_V1_COUNT: usize = 4;
pub const MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE: usize = 16;
pub const MEMORY_CONTEXT_V1_MAX_SOURCES: u16 = 4;
pub const EPISODIC_RETRIEVAL_CONTEXT_SCHEMA_VERSION: u16 = 1;

const MEMORY_QUERY_DOMAIN: &[u8] = b"ALIFE-MEMORY-QUERY-V2";
const EPISODIC_DECISION_KEY_DOMAIN: &[u8] = b"ALIFE-EPISODIC-DECISION-KEY-V2";

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryQueryVersion {
    StateActionTargetV2 = 2,
}

impl MemoryQueryVersion {
    pub const fn raw(self) -> u16 {
        match self {
            Self::StateActionTargetV2 => 2,
        }
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            2 => Ok(Self::StateActionTargetV2),
            _ => Err(ScaffoldContractError::InvalidMemoryQuery),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateMemoryQueryV2 {
    schema_version: u16,
    organism_id: OrganismId,
    tick: Tick,
    profile: SensorProfileIdentity,
    candidate_index: u16,
    action_id: ActionId,
    action_kind: ActionKind,
    action_family: CandidateActionFamily,
    base_frame_digest: PerceptionBaseDigest,
    candidate_feature_digest: CandidateFeatureDigest,
    tracked_object_id: Option<TrackedObjectId>,
    features: [f32; MEMORY_QUERY_V2_FEATURE_COUNT],
    canonical_digest: [u64; 4],
}

impl Serialize for CandidateMemoryQueryV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wire<'a> {
            schema_version: u16,
            organism_id: OrganismId,
            tick: Tick,
            profile: SensorProfileIdentity,
            candidate_index: u16,
            action_id: ActionId,
            action_kind: ActionKind,
            action_family: CandidateActionFamily,
            base_frame_digest: PerceptionBaseDigest,
            candidate_feature_digest: CandidateFeatureDigest,
            tracked_object_id: Option<TrackedObjectId>,
            features: &'a [f32],
            canonical_digest: [u64; 4],
        }

        Wire {
            schema_version: self.schema_version,
            organism_id: self.organism_id,
            tick: self.tick,
            profile: self.profile,
            candidate_index: self.candidate_index,
            action_id: self.action_id,
            action_kind: self.action_kind,
            action_family: self.action_family,
            base_frame_digest: self.base_frame_digest,
            candidate_feature_digest: self.candidate_feature_digest,
            tracked_object_id: self.tracked_object_id,
            features: &self.features,
            canonical_digest: self.canonical_digest,
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EpisodicDecisionKeyV2 {
    query: CandidateMemoryQueryV2,
    retrieval_context_digest: PerceptionContextDigest,
    final_frame_digest: PerceptionFrameDigest,
    canonical_digest: [u64; 4],
}

struct EncodedCandidateMemoryQueryV2 {
    organism_id: OrganismId,
    tick: Tick,
    profile: SensorProfileIdentity,
    candidate_index: u16,
    action_id: ActionId,
    action_kind: ActionKind,
    action_family: CandidateActionFamily,
    base_frame_digest: PerceptionBaseDigest,
    candidate_feature_digest: CandidateFeatureDigest,
    tracked_object_id: Option<TrackedObjectId>,
    features: [f32; MEMORY_QUERY_V2_FEATURE_COUNT],
}

impl CandidateMemoryQueryV2 {
    fn try_new(encoded: EncodedCandidateMemoryQueryV2) -> Result<Self, ScaffoldContractError> {
        let mut query = Self {
            schema_version: MemoryQueryVersion::StateActionTargetV2.raw(),
            organism_id: encoded.organism_id,
            tick: encoded.tick,
            profile: encoded.profile,
            candidate_index: encoded.candidate_index,
            action_id: encoded.action_id,
            action_kind: encoded.action_kind,
            action_family: encoded.action_family,
            base_frame_digest: encoded.base_frame_digest,
            candidate_feature_digest: encoded.candidate_feature_digest,
            tracked_object_id: encoded.tracked_object_id,
            features: encoded.features,
            canonical_digest: [0; 4],
        };
        validate_query_fields(&query)?;
        query.canonical_digest = compute_query_digest(&query)?;
        Ok(query)
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_query_fields(self)?;
        if compute_query_digest(self)? == self.canonical_digest {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidMemoryQuery)
        }
    }

    pub fn validate_against_frame(
        &self,
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
    ) -> Result<(), ScaffoldContractError> {
        frame
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let expected = MemoryQueryEncoderV2::encode_from_frame(frame, candidate)?;
        if &expected == self {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidMemoryQuery)
        }
    }

    pub const fn version(&self) -> MemoryQueryVersion {
        MemoryQueryVersion::StateActionTargetV2
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn tick(&self) -> Tick {
        self.tick
    }

    pub const fn profile(&self) -> SensorProfileIdentity {
        self.profile
    }

    pub const fn candidate_index(&self) -> u16 {
        self.candidate_index
    }

    pub const fn action_id(&self) -> ActionId {
        self.action_id
    }

    pub const fn action_kind(&self) -> ActionKind {
        self.action_kind
    }

    pub const fn action_family(&self) -> CandidateActionFamily {
        self.action_family
    }

    pub const fn base_frame_digest(&self) -> PerceptionBaseDigest {
        self.base_frame_digest
    }

    pub const fn candidate_feature_digest(&self) -> CandidateFeatureDigest {
        self.candidate_feature_digest
    }

    pub const fn tracked_object_id(&self) -> Option<TrackedObjectId> {
        self.tracked_object_id
    }

    pub fn features(&self) -> &[f32; MEMORY_QUERY_V2_FEATURE_COUNT] {
        &self.features
    }

    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }
}

impl Validate for CandidateMemoryQueryV2 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Self::validate_contract(self)
    }
}

impl<'de> Deserialize<'de> for CandidateMemoryQueryV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            organism_id: OrganismId,
            tick: Tick,
            profile: SensorProfileIdentity,
            candidate_index: u16,
            action_id: ActionId,
            action_kind: ActionKind,
            action_family: CandidateActionFamily,
            base_frame_digest: PerceptionBaseDigest,
            candidate_feature_digest: CandidateFeatureDigest,
            tracked_object_id: Option<TrackedObjectId>,
            features: Vec<f32>,
            canonical_digest: [u64; 4],
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.schema_version != MemoryQueryVersion::StateActionTargetV2.raw() {
            return Err(D::Error::custom("unsupported memory query schema"));
        }
        let features: [f32; MEMORY_QUERY_V2_FEATURE_COUNT] = wire
            .features
            .try_into()
            .map_err(|features: Vec<f32>| {
                D::Error::custom(format_args!(
                    "memory query features must contain exactly {MEMORY_QUERY_V2_FEATURE_COUNT} lanes, got {}",
                    features.len()
                ))
            })?;
        let query = Self::try_new(EncodedCandidateMemoryQueryV2 {
            organism_id: wire.organism_id,
            tick: wire.tick,
            profile: wire.profile,
            candidate_index: wire.candidate_index,
            action_id: wire.action_id,
            action_kind: wire.action_kind,
            action_family: wire.action_family,
            base_frame_digest: wire.base_frame_digest,
            candidate_feature_digest: wire.candidate_feature_digest,
            tracked_object_id: wire.tracked_object_id,
            features,
        })
        .map_err(D::Error::custom)?;
        if query.canonical_digest != wire.canonical_digest {
            return Err(D::Error::custom("memory query digest mismatch"));
        }
        Ok(query)
    }
}

impl EpisodicDecisionKeyV2 {
    pub(crate) fn try_new(
        query: CandidateMemoryQueryV2,
        retrieval_context_digest: PerceptionContextDigest,
        final_frame_digest: PerceptionFrameDigest,
    ) -> Result<Self, ScaffoldContractError> {
        query.validate_contract()?;
        if query.base_frame_digest.0 == final_frame_digest.0 {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let canonical_digest = compute_decision_key_digest(
            query.canonical_digest,
            retrieval_context_digest,
            final_frame_digest,
        );
        Ok(Self {
            query,
            retrieval_context_digest,
            final_frame_digest,
            canonical_digest,
        })
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.query.validate_contract()?;
        if self.query.base_frame_digest.0 == self.final_frame_digest.0
            || compute_decision_key_digest(
                self.query.canonical_digest,
                self.retrieval_context_digest,
                self.final_frame_digest,
            ) != self.canonical_digest
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }

    pub fn query(&self) -> &CandidateMemoryQueryV2 {
        &self.query
    }

    pub const fn retrieval_context_digest(&self) -> PerceptionContextDigest {
        self.retrieval_context_digest
    }

    pub const fn final_frame_digest(&self) -> PerceptionFrameDigest {
        self.final_frame_digest
    }

    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }
}

impl Validate for EpisodicDecisionKeyV2 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Self::validate_contract(self)
    }
}

impl<'de> Deserialize<'de> for EpisodicDecisionKeyV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            query: CandidateMemoryQueryV2,
            retrieval_context_digest: PerceptionContextDigest,
            final_frame_digest: PerceptionFrameDigest,
            canonical_digest: [u64; 4],
        }

        let wire = Wire::deserialize(deserializer)?;
        let key = Self::try_new(
            wire.query,
            wire.retrieval_context_digest,
            wire.final_frame_digest,
        )
        .map_err(D::Error::custom)?;
        if key.canonical_digest != wire.canonical_digest {
            return Err(D::Error::custom("episodic decision key digest mismatch"));
        }
        Ok(key)
    }
}

pub struct MemoryQueryEncoderV2;

impl MemoryQueryEncoderV2 {
    pub fn encode_candidate(
        draft: &PerceptionFrameDraft,
        candidate: &ActionCandidate,
    ) -> Result<CandidateMemoryQueryV2, ScaffoldContractError> {
        draft
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        encode_query(
            draft.organism_id(),
            draft.tick(),
            draft.profile_provenance().identity(),
            draft.sensory(),
            draft.body(),
            draft.homeostasis(),
            draft.grounded_object_slots(),
            draft.base_digest(),
            draft.candidates().len(),
            candidate,
        )
    }

    fn encode_from_frame(
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
    ) -> Result<CandidateMemoryQueryV2, ScaffoldContractError> {
        encode_query(
            frame.organism_id(),
            frame.tick(),
            frame.profile_provenance().identity(),
            frame.sensory(),
            frame.body(),
            frame.homeostasis(),
            frame.grounded_object_slots(),
            frame.base_digest(),
            frame.candidates().len(),
            candidate,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_query(
    organism_id: OrganismId,
    tick: Tick,
    profile: SensorProfileIdentity,
    sensory: &SensorySnapshot,
    body: BodySnapshot,
    homeostasis: &HomeostaticSnapshot,
    slots: &[crate::GroundedObjectSlotV1],
    base_frame_digest: PerceptionBaseDigest,
    candidate_count: usize,
    candidate: &ActionCandidate,
) -> Result<CandidateMemoryQueryV2, ScaffoldContractError> {
    candidate
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    profile
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    if usize::from(candidate.candidate_index) >= candidate_count {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    let tracked_object_id = match candidate.observation {
        CandidateObservationRef::None => None,
        CandidateObservationRef::ObjectSlot(index) => {
            let slot = slots
                .get(usize::from(index))
                .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
            if slot.slot_index != index {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
            Some(slot.tracked_object_id)
        }
    };

    let mut features = [0.0; MEMORY_QUERY_V2_FEATURE_COUNT];
    let channels = &sensory.channels;
    write_mean_max(&mut features[0..2], &channels.visual_affordance);
    write_mean_max(&mut features[2..4], &channels.auditory_acoustic);
    write_mean_max(&mut features[4..6], &channels.smell_chemistry);
    write_mean_max(&mut features[6..8], &channels.tactile_contact);
    features[8] = channels.pain_signal.raw();
    features[9] = channels.novelty_signal.raw();
    features[10] = (channels.nearby_affordances.raw().count_ones() as f32 / 10.0).min(1.0);
    features[11] = candidate.sensor_confidence.raw();
    features[MEMORY_DRIVE_RANGE].copy_from_slice(&homeostasis.drives.to_array());
    features[MEMORY_HORMONE_RANGE].copy_from_slice(&homeostasis.hormones.to_array());
    features[MEMORY_BODY_RANGE].copy_from_slice(&[
        body.velocity.linear.x.clamp(-1.0, 1.0),
        body.velocity.linear.y.clamp(-1.0, 1.0),
        body.velocity.linear.z.clamp(-1.0, 1.0),
        body.velocity.angular.x.clamp(-1.0, 1.0),
        body.velocity.angular.y.clamp(-1.0, 1.0),
        body.velocity.angular.z.clamp(-1.0, 1.0),
    ]);
    features[MEMORY_ACTION_KIND_RANGE.start + usize::from(candidate.kind.raw())] = 1.0;
    features[MEMORY_ACTION_FAMILY_RANGE.start + usize::from(candidate.family.raw())] = 1.0;
    features[MEMORY_TARGET_RANGE].copy_from_slice(&candidate.features.0);
    let profile_offset = match profile.profile_id.raw() {
        1 => 0,
        2 => 1,
        _ => return Err(ScaffoldContractError::InvalidMemoryQuery),
    };
    features[MEMORY_PROFILE_RANGE.start + profile_offset] = 1.0;

    CandidateMemoryQueryV2::try_new(EncodedCandidateMemoryQueryV2 {
        organism_id,
        tick,
        profile,
        candidate_index: candidate.candidate_index,
        action_id: candidate.action_id,
        action_kind: candidate.kind,
        action_family: candidate.family,
        base_frame_digest,
        candidate_feature_digest: candidate.feature_digest()?,
        tracked_object_id,
        features,
    })
}

fn write_mean_max(output: &mut [f32], values: &[f32]) {
    debug_assert_eq!(output.len(), 2);
    let sum = values.iter().copied().sum::<f32>();
    output[0] = if values.is_empty() {
        0.0
    } else {
        sum / values.len() as f32
    };
    output[1] = values.iter().copied().fold(0.0, f32::max);
}

fn validate_query_fields(query: &CandidateMemoryQueryV2) -> Result<(), ScaffoldContractError> {
    ensure_current_version(SchemaKind::MemoryQuery, query.schema_version)
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    MemoryQueryVersion::try_from_raw(query.schema_version)?;
    query
        .organism_id
        .validate()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    query
        .profile
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    query
        .action_id
        .validate()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    ActionKind::try_from_raw(query.action_kind.raw())?;
    CandidateActionFamily::try_from_raw(query.action_family.raw())?;
    if !query.action_family.is_compatible_with(query.action_kind) {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    if let Some(tracked_object_id) = query.tracked_object_id {
        tracked_object_id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    }
    if query
        .features
        .iter()
        .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        || query.features[MEMORY_RESERVED_RANGE]
            .iter()
            .any(|value| *value != 0.0)
    {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    require_exact_one_hot(
        &query.features[MEMORY_ACTION_KIND_RANGE],
        usize::from(query.action_kind.raw()),
    )?;
    require_exact_one_hot(
        &query.features[MEMORY_ACTION_FAMILY_RANGE],
        usize::from(query.action_family.raw()),
    )?;
    let profile_index = match query.profile.profile_id.raw() {
        1 => 0,
        2 => 1,
        _ => return Err(ScaffoldContractError::InvalidMemoryQuery),
    };
    require_exact_one_hot(&query.features[MEMORY_PROFILE_RANGE], profile_index)
}

fn require_exact_one_hot(values: &[f32], expected: usize) -> Result<(), ScaffoldContractError> {
    if values
        .iter()
        .enumerate()
        .all(|(index, value)| *value == f32::from(index == expected))
    {
        Ok(())
    } else {
        Err(ScaffoldContractError::InvalidMemoryQuery)
    }
}

fn compute_query_digest(query: &CandidateMemoryQueryV2) -> Result<[u64; 4], ScaffoldContractError> {
    let mut builder = CanonicalDigestBuilder::new(MEMORY_QUERY_DOMAIN);
    builder.write_u16(query.schema_version);
    builder.write_u64(query.organism_id.raw());
    builder.write_u64(query.tick.raw());
    builder.write_u16(query.profile.profile_id.raw());
    builder.write_u16(query.profile.profile_schema_version);
    builder.write_u16(query.profile.sensory_abi_version);
    builder.write_u16(query.candidate_index);
    builder.write_u32(query.action_id.raw());
    builder.write_u8(query.action_kind.raw());
    builder.write_u8(query.action_family.raw());
    write_u64_digest(&mut builder, query.base_frame_digest.0);
    write_u64_digest_128(&mut builder, query.candidate_feature_digest.0);
    match query.tracked_object_id {
        Some(id) => {
            builder.write_some();
            builder.write_u64(id.raw());
        }
        None => builder.write_none(),
    }
    builder.write_sequence_len(query.features.len());
    for value in query.features {
        builder.write_f32(value)?;
    }
    Ok(builder.finish256())
}

fn compute_decision_key_digest(
    query_digest: [u64; 4],
    context_digest: PerceptionContextDigest,
    frame_digest: PerceptionFrameDigest,
) -> [u64; 4] {
    let mut builder = CanonicalDigestBuilder::new(EPISODIC_DECISION_KEY_DOMAIN);
    write_u64_digest(&mut builder, query_digest);
    write_u64_digest(&mut builder, context_digest.0);
    write_u64_digest(&mut builder, frame_digest.0);
    builder.finish256()
}

fn write_u64_digest(builder: &mut CanonicalDigestBuilder, digest: [u64; 4]) {
    builder.write_sequence_len(digest.len());
    for value in digest {
        builder.write_u64(value);
    }
}

fn write_u64_digest_128(builder: &mut CanonicalDigestBuilder, digest: [u64; 2]) {
    builder.write_sequence_len(digest.len());
    for value in digest {
        builder.write_u64(value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandidateMemoryContextV1 {
    pub candidate_index: u16,
    pub target_latent: [f32; MEMORY_LATENT_V1_COUNT],
    pub family_value: [f32; MEMORY_VALUE_V1_COUNT],
    pub target_confidence: Confidence,
    pub family_confidence: Confidence,
    pub target_source_count: u16,
    pub family_source_count: u16,
    pub best_target_source: Option<MemoryId>,
    pub best_family_source: Option<MemoryId>,
}

impl Validate for CandidateMemoryContextV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self
            .target_latent
            .iter()
            .chain(self.family_value.iter())
            .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Confidence::new(self.target_confidence.raw())?;
        Confidence::new(self.family_confidence.raw())?;
        for source in [self.best_target_source, self.best_family_source]
            .into_iter()
            .flatten()
        {
            source.validate()?;
        }
        validate_recall_channel_sources(
            &self.target_latent,
            self.target_confidence,
            self.target_source_count,
            self.best_target_source,
        )?;
        validate_recall_channel_sources(
            &self.family_value,
            self.family_confidence,
            self.family_source_count,
            self.best_family_source,
        )?;
        Ok(())
    }
}

fn validate_recall_channel_sources(
    values: &[f32],
    confidence: Confidence,
    source_count: u16,
    best_source: Option<MemoryId>,
) -> Result<(), ScaffoldContractError> {
    if source_count > MEMORY_CONTEXT_V1_MAX_SOURCES {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    if source_count == 0 {
        if best_source.is_some()
            || confidence.raw().to_bits() != 0.0_f32.to_bits()
            || values
                .iter()
                .any(|value| value.to_bits() != 0.0_f32.to_bits())
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
    } else if best_source.is_none() || confidence.raw() <= 0.0 {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodicRetrievalContextV1 {
    pub schema_version: u16,
    pub tick: Tick,
    pub profile: SensorProfileIdentity,
    pub candidates: Vec<CandidateMemoryContextV1>,
}

impl EpisodicRetrievalContextV1 {
    pub fn new(
        tick: Tick,
        profile: SensorProfileIdentity,
        candidates: Vec<CandidateMemoryContextV1>,
    ) -> Result<Self, ScaffoldContractError> {
        let context = Self {
            schema_version: EPISODIC_RETRIEVAL_CONTEXT_SCHEMA_VERSION,
            tick,
            profile,
            candidates,
        };
        context.validate_contract()?;
        Ok(context)
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != EPISODIC_RETRIEVAL_CONTEXT_SCHEMA_VERSION
            || self.candidates.is_empty()
            || self.candidates.len() > MAX_ACTION_CANDIDATES
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        self.profile
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        for (index, candidate) in self.candidates.iter().enumerate() {
            candidate.validate_contract()?;
            if usize::from(candidate.candidate_index) != index {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
        Ok(())
    }

    pub fn to_perception_context_block(
        &self,
    ) -> Result<PerceptionContextBlock, ScaffoldContractError> {
        self.validate_contract()?;
        let mut values =
            Vec::with_capacity(self.candidates.len() * MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE);
        for candidate in &self.candidates {
            values.extend_from_slice(&candidate.target_latent);
            values.extend_from_slice(&candidate.family_value);
            values.push(candidate.target_confidence.raw());
            values.push(candidate.family_confidence.raw());
            values.push(f32::from(candidate.target_source_count));
            values.push(f32::from(candidate.family_source_count));
        }
        PerceptionContextBlock::try_new(
            SchemaVersions::CURRENT.perception.raw(),
            PerceptionContextKind::EpisodicCandidateV1,
            values,
        )
    }
}

impl Validate for EpisodicRetrievalContextV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Self::validate_contract(self)
    }
}
