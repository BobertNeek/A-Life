//! v0 scaffold: lossy packed logging derived from sealed ExperiencePatch records.
//!
//! Packed logs are export/replay data. They intentionally do not replace the
//! rich runtime `ExperiencePatch` contract used by cognition, learning, memory,
//! or topology systems.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, validate_finite, validate_finite_slice, ActionKind, BrainScaleTier,
    DriveSnapshot, EndocrineSnapshot, ExperiencePatch, PhysicalContactKind, ScaffoldContractError,
    SchemaKind, SchemaVersions, TeacherPerceptionChannel, Validate,
};

pub const PACKED_EXPERIENCE_SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.packed_log.0;
pub const PACKED_EXPERIENCE_FRAME_RESERVED_U32S: usize = 8;
pub const PACKED_DRIVE_SUMMARY_CHANNELS: usize = DriveSnapshot::CHANNEL_COUNT;
pub const PACKED_HORMONE_SUMMARY_CHANNELS: usize = EndocrineSnapshot::CHANNEL_COUNT;
pub const PACKED_SIDE_BUFFER_GROUP_COUNT: usize = 12;
pub const PACKED_LOG_DEFAULT_SIDE_BUFFER_CAPACITY_RECORDS: usize = 256;

pub const PACKED_FLAG_SUCCESS: u32 = 1 << 0;
pub const PACKED_FLAG_HAS_TARGET_ENTITY: u32 = 1 << 1;
pub const PACKED_FLAG_HAS_TARGET_POSITION: u32 = 1 << 2;
pub const PACKED_FLAG_HAS_TEACHER_FEEDBACK: u32 = 1 << 3;
pub const PACKED_FLAG_CONTRADICTION_OBSERVED: u32 = 1 << 4;
pub const PACKED_FLAG_HAS_SEMANTIC_CONTEXT: u32 = 1 << 5;
pub const PACKED_FLAG_HAS_GAUSSIAN_CONTEXT: u32 = 1 << 6;
pub const PACKED_FLAG_HAS_MOTOR_PAYLOAD: u32 = 1 << 7;
pub const PACKED_FLAG_HAS_TEACHER_LESSON: u32 = 1 << 8;

const PACKED_KNOWN_FLAGS: u32 = PACKED_FLAG_SUCCESS
    | PACKED_FLAG_HAS_TARGET_ENTITY
    | PACKED_FLAG_HAS_TARGET_POSITION
    | PACKED_FLAG_HAS_TEACHER_FEEDBACK
    | PACKED_FLAG_CONTRADICTION_OBSERVED
    | PACKED_FLAG_HAS_SEMANTIC_CONTEXT
    | PACKED_FLAG_HAS_GAUSSIAN_CONTEXT
    | PACKED_FLAG_HAS_MOTOR_PAYLOAD
    | PACKED_FLAG_HAS_TEACHER_LESSON;

const SIDE_RECORD_FLAG_REJECTED: u32 = 1 << 0;
const SIDE_RECORD_FLAG_CONTRADICTION: u32 = 1 << 1;
const SIDE_RECORD_FLAG_HAS_TEACHER_CHANNEL: u32 = 1 << 2;
const SIDE_RECORD_FLAG_ENTITY_ID: u32 = 1 << 3;
const SIDE_RECORD_FLAG_ORGANISM_ID: u32 = 1 << 4;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideBufferSpan {
    pub offset: u32,
    pub count: u32,
}

impl SideBufferSpan {
    pub const EMPTY: Self = Self {
        offset: 0,
        count: 0,
    };

    pub const fn new(offset: u32, count: u32) -> Self {
        Self { offset, count }
    }

    pub fn end(self) -> Result<u32, ScaffoldContractError> {
        self.offset
            .checked_add(self.count)
            .ok_or(ScaffoldContractError::PackedLogSideBufferOverflow)
    }

    pub fn validate_against_len(self, len: u32) -> Result<(), ScaffoldContractError> {
        if self.end()? <= len {
            Ok(())
        } else {
            Err(ScaffoldContractError::PackedLogSideBufferOverflow)
        }
    }
}

impl Validate for SideBufferSpan {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.end()?;
        Ok(())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackedSideBufferSpans {
    pub visible_entities: SideBufferSpan,
    pub touched_entities: SideBufferSpan,
    pub heard_tokens: SideBufferSpan,
    pub salience_clusters: SideBufferSpan,
    pub memory_links: SideBufferSpan,
    pub concept_links: SideBufferSpan,
    pub ranked_action_proposals: SideBufferSpan,
    pub arbitration_details: SideBufferSpan,
    pub semantic_codes: SideBufferSpan,
    pub gaussian_refs: SideBufferSpan,
    pub teacher_school_refs: SideBufferSpan,
    pub diagnostic_extras: SideBufferSpan,
}

impl PackedSideBufferSpans {
    pub const EMPTY: Self = Self {
        visible_entities: SideBufferSpan::EMPTY,
        touched_entities: SideBufferSpan::EMPTY,
        heard_tokens: SideBufferSpan::EMPTY,
        salience_clusters: SideBufferSpan::EMPTY,
        memory_links: SideBufferSpan::EMPTY,
        concept_links: SideBufferSpan::EMPTY,
        ranked_action_proposals: SideBufferSpan::EMPTY,
        arbitration_details: SideBufferSpan::EMPTY,
        semantic_codes: SideBufferSpan::EMPTY,
        gaussian_refs: SideBufferSpan::EMPTY,
        teacher_school_refs: SideBufferSpan::EMPTY,
        diagnostic_extras: SideBufferSpan::EMPTY,
    };

    pub const fn all(self) -> [SideBufferSpan; PACKED_SIDE_BUFFER_GROUP_COUNT] {
        [
            self.visible_entities,
            self.touched_entities,
            self.heard_tokens,
            self.salience_clusters,
            self.memory_links,
            self.concept_links,
            self.ranked_action_proposals,
            self.arbitration_details,
            self.semantic_codes,
            self.gaussian_refs,
            self.teacher_school_refs,
            self.diagnostic_extras,
        ]
    }

    pub fn validate_against_len(self, len: usize) -> Result<(), ScaffoldContractError> {
        let len =
            u32::try_from(len).map_err(|_| ScaffoldContractError::PackedLogSideBufferOverflow)?;
        for span in self.all() {
            span.validate_against_len(len)?;
        }
        Ok(())
    }
}

impl Validate for PackedSideBufferSpans {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let mut next_offset = 0;
        for span in (*self).all() {
            span.validate_contract()?;
            if span.offset != next_offset {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
            next_offset = span.end()?;
        }
        Ok(())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PackedExperienceFrame {
    pub schema_version: u16,
    pub experience_schema_version: u16,
    pub sensory_abi_version: u16,
    pub action_abi_version: u16,
    pub flags: u32,
    pub reserved_header: u32,
    pub organism_id: u64,
    pub sequence_id: u64,
    pub pre_action_tick: u64,
    pub decision_tick: u64,
    pub outcome_tick: u64,
    pub brain_class_id: u16,
    pub brain_scale_tier_code: u16,
    pub selected_action_kind_code: u16,
    pub reserved_kind: u16,
    pub selected_action_id: u32,
    pub action_duration_ticks: u32,
    pub action_source_mask: u32,
    pub target_entity_id: u64,
    pub position: [f32; 3],
    pub heading_quat: [f32; 4],
    pub target_position: [f32; 3],
    pub drive_summary: [f32; PACKED_DRIVE_SUMMARY_CHANNELS],
    pub hormone_summary: [f32; PACKED_HORMONE_SUMMARY_CHANNELS],
    pub action_intensity: f32,
    pub action_confidence: f32,
    pub decision_confidence: f32,
    pub reward_valence: f32,
    pub frustration_delta: f32,
    pub pain_delta: f32,
    pub energy_delta: f32,
    pub prediction_error: f32,
    pub salience_summary: f32,
    pub memory_expected_valence: f32,
    pub memory_salience_hint: f32,
    pub side_buffer_spans: PackedSideBufferSpans,
    pub reserved: [u32; PACKED_EXPERIENCE_FRAME_RESERVED_U32S],
}

impl PackedExperienceFrame {
    pub const SCHEMA_VERSION: u16 = PACKED_EXPERIENCE_SCHEMA_VERSION;
    pub const SIZE_BYTES: usize = core::mem::size_of::<Self>();

    pub fn require_schema_version(actual: u16) -> Result<(), ScaffoldContractError> {
        if actual == Self::SCHEMA_VERSION {
            Ok(())
        } else {
            Err(ScaffoldContractError::PackedLogSchemaMismatch {
                expected: Self::SCHEMA_VERSION,
                actual,
            })
        }
    }

    pub fn inspect_lossy(&self) -> Result<PackedExperienceSummary, ScaffoldContractError> {
        self.validate_contract()?;
        Ok(PackedExperienceSummary {
            schema_version: self.schema_version,
            organism_id: self.organism_id,
            sequence_id: self.sequence_id,
            pre_action_tick: self.pre_action_tick,
            decision_tick: self.decision_tick,
            outcome_tick: self.outcome_tick,
            brain_class_id: self.brain_class_id,
            selected_action_id: self.selected_action_id,
            selected_action_kind_code: self.selected_action_kind_code,
            success: self.flags & PACKED_FLAG_SUCCESS != 0,
            target_entity_id: if self.flags & PACKED_FLAG_HAS_TARGET_ENTITY != 0 {
                Some(self.target_entity_id)
            } else {
                None
            },
            reward_valence: self.reward_valence,
            salience_summary: self.salience_summary,
            side_buffer_record_count: self.side_buffer_spans.all().iter().try_fold(
                0u32,
                |acc, span| {
                    acc.checked_add(span.count)
                        .ok_or(ScaffoldContractError::PackedLogSideBufferOverflow)
                },
            )?,
        })
    }
}

impl Validate for PackedExperienceFrame {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Self::require_schema_version(self.schema_version)?;
        ensure_current_version(SchemaKind::Experience, self.experience_schema_version)?;
        ensure_current_version(SchemaKind::SensoryAbi, self.sensory_abi_version)?;
        ensure_current_version(SchemaKind::ActionAbi, self.action_abi_version)?;
        if self.flags & !PACKED_KNOWN_FLAGS != 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.pre_action_tick > self.decision_tick || self.decision_tick > self.outcome_tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        validate_finite_slice(&self.position)?;
        validate_finite_slice(&self.heading_quat)?;
        validate_finite_slice(&self.target_position)?;
        validate_unit_slice(&self.drive_summary)?;
        validate_unit_slice(&self.hormone_summary)?;
        validate_unit(self.action_intensity)?;
        validate_unit(self.action_confidence)?;
        validate_unit(self.decision_confidence)?;
        validate_signed_unit(self.reward_valence)?;
        validate_unit(self.frustration_delta)?;
        validate_unit(self.pain_delta)?;
        validate_signed_unit(self.energy_delta)?;
        validate_unit(self.prediction_error)?;
        validate_unit(self.salience_summary)?;
        validate_signed_unit(self.memory_expected_valence)?;
        validate_unit(self.memory_salience_hint)?;
        self.side_buffer_spans.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PackedExperienceSummary {
    pub schema_version: u16,
    pub organism_id: u64,
    pub sequence_id: u64,
    pub pre_action_tick: u64,
    pub decision_tick: u64,
    pub outcome_tick: u64,
    pub brain_class_id: u16,
    pub selected_action_id: u32,
    pub selected_action_kind_code: u16,
    pub success: bool,
    pub target_entity_id: Option<u64>,
    pub reward_valence: f32,
    pub salience_summary: f32,
    pub side_buffer_record_count: u32,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackedSideBufferKind {
    VisibleEntity = 1,
    TouchedEntity = 2,
    HeardToken = 3,
    SalienceCluster = 4,
    MemoryLink = 5,
    ConceptLink = 6,
    RankedActionProposal = 7,
    ArbitrationDetail = 8,
    SemanticCode = 9,
    GaussianRef = 10,
    TeacherSchoolRef = 11,
    DiagnosticExtra = 12,
}

impl PackedSideBufferKind {
    pub const fn code(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PackedSideBufferRecord {
    pub schema_version: u16,
    pub kind: PackedSideBufferKind,
    pub primary_id: u64,
    pub secondary_id: u64,
    pub values: [f32; 4],
    pub flags: u32,
}

impl PackedSideBufferRecord {
    pub fn new(
        kind: PackedSideBufferKind,
        primary_id: u64,
        secondary_id: u64,
        values: [f32; 4],
        flags: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let record = Self {
            schema_version: PACKED_EXPERIENCE_SCHEMA_VERSION,
            kind,
            primary_id,
            secondary_id,
            values,
            flags,
        };
        record.validate_contract()?;
        Ok(record)
    }
}

impl Validate for PackedSideBufferRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        PackedExperienceFrame::require_schema_version(self.schema_version)?;
        validate_finite_slice(&self.values)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedSideBuffers {
    records: Vec<PackedSideBufferRecord>,
}

impl PackedSideBuffers {
    pub fn from_records(
        records: Vec<PackedSideBufferRecord>,
    ) -> Result<Self, ScaffoldContractError> {
        let buffers = Self { records };
        buffers.validate_contract()?;
        Ok(buffers)
    }

    pub fn records(&self) -> &[PackedSideBufferRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl Validate for PackedSideBuffers {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for record in &self.records {
            record.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedExperienceRecord {
    pub frame: PackedExperienceFrame,
    pub side_buffers: PackedSideBuffers,
}

impl PackedExperienceRecord {
    pub fn inspect_lossy(&self) -> Result<PackedExperienceSummary, ScaffoldContractError> {
        self.validate_contract()?;
        self.frame.inspect_lossy()
    }
}

impl Validate for PackedExperienceRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.frame.validate_contract()?;
        self.side_buffers.validate_contract()?;
        self.frame
            .side_buffer_spans
            .validate_against_len(self.side_buffers.len())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperiencePacker {
    max_side_buffer_records: usize,
}

impl Default for ExperiencePacker {
    fn default() -> Self {
        Self {
            max_side_buffer_records: PACKED_LOG_DEFAULT_SIDE_BUFFER_CAPACITY_RECORDS,
        }
    }
}

impl ExperiencePacker {
    pub fn new(max_side_buffer_records: usize) -> Result<Self, ScaffoldContractError> {
        if max_side_buffer_records > u32::MAX as usize {
            return Err(ScaffoldContractError::PackedLogSideBufferOverflow);
        }
        Ok(Self {
            max_side_buffer_records,
        })
    }

    pub const fn max_side_buffer_records(self) -> usize {
        self.max_side_buffer_records
    }

    pub fn pack(
        &self,
        patch: &ExperiencePatch,
    ) -> Result<PackedExperienceRecord, ScaffoldContractError> {
        patch.validate_contract()?;
        let view = patch.as_learning_view();
        let pre = view.pre_action();
        let decision = view.decision();
        let outcome = view.outcome();
        let selected = decision.selected_action;

        let mut builder = SideBufferBuilder::new(self.max_side_buffer_records);
        let visible_entities = builder.capture(|builder| append_visible_entities(builder, pre))?;
        let touched_entities =
            builder.capture(|builder| append_touched_entities(builder, outcome))?;
        let heard_tokens = builder.capture(|builder| append_heard_tokens(builder, pre))?;
        let salience_clusters =
            builder.capture(|builder| append_salience_clusters(builder, pre))?;
        let memory_links = builder.capture(|builder| append_memory_links(builder, outcome))?;
        let concept_links = builder.capture(|builder| append_concept_links(builder, outcome))?;
        let ranked_action_proposals =
            builder.capture(|builder| append_ranked_action_proposals(builder, decision))?;
        let arbitration_details =
            builder.capture(|builder| append_arbitration_details(builder, decision))?;
        let semantic_codes = builder.capture(|builder| append_semantic_codes(builder, pre))?;
        let gaussian_refs = builder.capture(|builder| append_gaussian_refs(builder, pre))?;
        let teacher_school_refs =
            builder.capture(|builder| append_teacher_school_refs(builder, decision, outcome))?;
        let diagnostic_extras =
            builder.capture(|builder| append_diagnostic_extras(builder, decision, outcome))?;

        let side_buffer_spans = PackedSideBufferSpans {
            visible_entities,
            touched_entities,
            heard_tokens,
            salience_clusters,
            memory_links,
            concept_links,
            ranked_action_proposals,
            arbitration_details,
            semantic_codes,
            gaussian_refs,
            teacher_school_refs,
            diagnostic_extras,
        };

        let target_position = selected
            .target_position
            .unwrap_or(crate::Vec3f::ZERO)
            .to_array();
        let target_entity_id = selected.target_entity.map_or(0, |id| id.raw());
        let flags = build_flags(patch);
        let side_buffers = PackedSideBuffers::from_records(builder.into_records())?;
        let frame = PackedExperienceFrame {
            schema_version: PACKED_EXPERIENCE_SCHEMA_VERSION,
            experience_schema_version: patch.header().abi_version,
            sensory_abi_version: pre.sensory_abi_version.raw(),
            action_abi_version: decision.action_abi_version,
            flags,
            reserved_header: 0,
            organism_id: patch.header().organism_id.raw(),
            sequence_id: patch.header().sequence_id.raw(),
            pre_action_tick: pre.tick.raw(),
            decision_tick: decision.decision_tick.raw(),
            outcome_tick: outcome.outcome_tick.raw(),
            brain_class_id: pre.brain_class_id.raw(),
            brain_scale_tier_code: brain_scale_tier_code(pre.brain_scale_tier),
            selected_action_kind_code: action_kind_code(selected.kind),
            reserved_kind: 0,
            selected_action_id: selected.action_id.raw(),
            action_duration_ticks: selected.duration_ticks.raw(),
            action_source_mask: selected.source_mask,
            target_entity_id,
            position: pre.body_pose.translation.to_array(),
            heading_quat: [
                pre.body_pose.rotation.x,
                pre.body_pose.rotation.y,
                pre.body_pose.rotation.z,
                pre.body_pose.rotation.w,
            ],
            target_position,
            drive_summary: pre.homeostasis.drives.to_array(),
            hormone_summary: pre.homeostasis.hormones.to_array(),
            action_intensity: selected.intensity.raw(),
            action_confidence: selected.confidence.raw(),
            decision_confidence: decision.confidence.raw(),
            reward_valence: outcome.reward_valence.raw(),
            frustration_delta: outcome.frustration_delta.raw(),
            pain_delta: outcome.pain_delta.raw(),
            energy_delta: outcome.energy_delta.raw(),
            prediction_error: outcome.prediction_error.raw(),
            salience_summary: salience_summary(patch)?,
            memory_expected_valence: pre.memory_expectancy.expected_valence.raw(),
            memory_salience_hint: pre.memory_expectancy.salience_hint.raw(),
            side_buffer_spans,
            reserved: [0; PACKED_EXPERIENCE_FRAME_RESERVED_U32S],
        };

        let record = PackedExperienceRecord {
            frame,
            side_buffers,
        };
        record.validate_contract()?;
        Ok(record)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackedLogEntryRef {
    pub frame_index: u32,
    pub side_buffer_offset: u32,
    pub side_buffer_count: u32,
}

pub trait PackedExperienceSink {
    fn append(
        &mut self,
        record: PackedExperienceRecord,
    ) -> Result<PackedLogEntryRef, ScaffoldContractError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InMemoryPackedExperienceLog {
    frames: Vec<PackedExperienceFrame>,
    side_records: Vec<PackedSideBufferRecord>,
    max_frames: usize,
    max_side_records: usize,
}

impl InMemoryPackedExperienceLog {
    pub fn bounded(
        max_frames: usize,
        max_side_records: usize,
    ) -> Result<Self, ScaffoldContractError> {
        if max_frames > u32::MAX as usize {
            return Err(ScaffoldContractError::PackedLogFrameCapacityExceeded);
        }
        if max_side_records > u32::MAX as usize {
            return Err(ScaffoldContractError::PackedLogSideBufferOverflow);
        }
        Ok(Self {
            frames: Vec::new(),
            side_records: Vec::new(),
            max_frames,
            max_side_records,
        })
    }

    pub fn frames(&self) -> &[PackedExperienceFrame] {
        &self.frames
    }

    pub fn side_records(&self) -> &[PackedSideBufferRecord] {
        &self.side_records
    }
}

impl PackedExperienceSink for InMemoryPackedExperienceLog {
    fn append(
        &mut self,
        record: PackedExperienceRecord,
    ) -> Result<PackedLogEntryRef, ScaffoldContractError> {
        record.validate_contract()?;
        if self.frames.len() == self.max_frames {
            return Err(ScaffoldContractError::PackedLogFrameCapacityExceeded);
        }
        let side_count = record.side_buffers.len();
        if self.side_records.len().saturating_add(side_count) > self.max_side_records {
            return Err(ScaffoldContractError::PackedLogSideBufferOverflow);
        }

        let frame_index = u32::try_from(self.frames.len())
            .map_err(|_| ScaffoldContractError::PackedLogFrameCapacityExceeded)?;
        let side_buffer_offset = u32::try_from(self.side_records.len())
            .map_err(|_| ScaffoldContractError::PackedLogSideBufferOverflow)?;
        let side_buffer_count = u32::try_from(side_count)
            .map_err(|_| ScaffoldContractError::PackedLogSideBufferOverflow)?;

        self.frames.push(record.frame);
        self.side_records
            .extend_from_slice(record.side_buffers.records());

        Ok(PackedLogEntryRef {
            frame_index,
            side_buffer_offset,
            side_buffer_count,
        })
    }
}

struct SideBufferBuilder {
    records: Vec<PackedSideBufferRecord>,
    capacity: usize,
}

impl SideBufferBuilder {
    fn new(capacity: usize) -> Self {
        Self {
            records: Vec::new(),
            capacity,
        }
    }

    fn capture(
        &mut self,
        append: impl FnOnce(&mut Self) -> Result<(), ScaffoldContractError>,
    ) -> Result<SideBufferSpan, ScaffoldContractError> {
        let offset = u32::try_from(self.records.len())
            .map_err(|_| ScaffoldContractError::PackedLogSideBufferOverflow)?;
        append(self)?;
        let end = u32::try_from(self.records.len())
            .map_err(|_| ScaffoldContractError::PackedLogSideBufferOverflow)?;
        Ok(SideBufferSpan::new(offset, end.saturating_sub(offset)))
    }

    fn push(&mut self, record: PackedSideBufferRecord) -> Result<(), ScaffoldContractError> {
        if self.records.len() == self.capacity {
            return Err(ScaffoldContractError::PackedLogSideBufferOverflow);
        }
        self.records.push(record);
        Ok(())
    }

    fn into_records(self) -> Vec<PackedSideBufferRecord> {
        self.records
    }
}

fn append_visible_entities(
    builder: &mut SideBufferBuilder,
    pre: &crate::PreActionSnapshot,
) -> Result<(), ScaffoldContractError> {
    for agent in pre.sensory.social_context.nearest_agents.iter().flatten() {
        let (primary_id, flags) = if let Some(entity) = agent.body_entity {
            (entity.raw(), SIDE_RECORD_FLAG_ENTITY_ID)
        } else {
            (agent.agent_id.raw(), SIDE_RECORD_FLAG_ORGANISM_ID)
        };
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::VisibleEntity,
            primary_id,
            agent.agent_id.raw(),
            [
                agent.relative_position.x,
                agent.relative_position.y,
                agent.relative_position.z,
                agent.proximity.raw(),
            ],
            flags,
        )?)?;
    }
    Ok(())
}

fn append_touched_entities(
    builder: &mut SideBufferBuilder,
    outcome: &crate::PostActionOutcome,
) -> Result<(), ScaffoldContractError> {
    if outcome.physical.contact == PhysicalContactKind::None {
        return Ok(());
    }
    if let Some(target) = outcome.physical.target_entity {
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::TouchedEntity,
            target.raw(),
            physical_contact_code(outcome.physical.contact).into(),
            [
                outcome.physical.displacement.x,
                outcome.physical.displacement.y,
                outcome.physical.displacement.z,
                outcome.physical.energy_cost.raw(),
            ],
            SIDE_RECORD_FLAG_ENTITY_ID,
        )?)?;
    }
    Ok(())
}

fn append_heard_tokens(
    builder: &mut SideBufferBuilder,
    pre: &crate::PreActionSnapshot,
) -> Result<(), ScaffoldContractError> {
    for token in pre.sensory.context_streams.vocal_tokens.iter().flatten() {
        append_heard_token(builder, *token)?;
    }
    for token in pre.sensory.language_context.heard_tokens.iter().flatten() {
        append_heard_token(builder, *token)?;
    }
    Ok(())
}

fn append_heard_token(
    builder: &mut SideBufferBuilder,
    token: crate::HeardToken,
) -> Result<(), ScaffoldContractError> {
    let mut flags = 0;
    let secondary_id = if let Some(speaker_id) = token.speaker_id {
        flags |= SIDE_RECORD_FLAG_ORGANISM_ID;
        speaker_id.raw()
    } else if let Some(source_entity) = token.source_entity {
        flags |= SIDE_RECORD_FLAG_ENTITY_ID;
        source_entity.raw()
    } else {
        0
    };
    if let Some(channel) = token.teacher_channel {
        flags |=
            SIDE_RECORD_FLAG_HAS_TEACHER_CHANNEL | (u32::from(teacher_channel_code(channel)) << 16);
    }
    builder.push(PackedSideBufferRecord::new(
        PackedSideBufferKind::HeardToken,
        u64::from(token.token_id),
        secondary_id,
        [
            token.source_position.x,
            token.source_position.y,
            token.source_position.z,
            token.confidence.raw(),
        ],
        flags,
    )?)
}

fn append_salience_clusters(
    builder: &mut SideBufferBuilder,
    pre: &crate::PreActionSnapshot,
) -> Result<(), ScaffoldContractError> {
    if let Some(context) = &pre.sensory.semantic_context {
        for entry in &context.salience {
            builder.push(PackedSideBufferRecord::new(
                PackedSideBufferKind::SalienceCluster,
                entry.concept_id.raw(),
                0,
                [entry.salience.raw(), context.confidence.raw(), 0.0, 0.0],
                PackedSideBufferKind::SemanticCode.code().into(),
            )?)?;
        }
    }
    if let Some(context) = &pre.sensory.gaussian_context {
        for cluster in &context.clusters {
            builder.push(PackedSideBufferRecord::new(
                PackedSideBufferKind::SalienceCluster,
                cluster.cluster_id.raw(),
                context.egocentric_bin_hash,
                [
                    cluster.salience.raw(),
                    cluster.distance_meters,
                    context.confidence.raw(),
                    0.0,
                ],
                PackedSideBufferKind::GaussianRef.code().into(),
            )?)?;
        }
    }
    Ok(())
}

fn append_memory_links(
    builder: &mut SideBufferBuilder,
    outcome: &crate::PostActionOutcome,
) -> Result<(), ScaffoldContractError> {
    for hint in &outcome.memory_hints {
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::MemoryLink,
            hint.memory_id.raw(),
            0,
            [hint.salience.raw(), 0.0, 0.0, 0.0],
            0,
        )?)?;
    }
    Ok(())
}

fn append_concept_links(
    builder: &mut SideBufferBuilder,
    outcome: &crate::PostActionOutcome,
) -> Result<(), ScaffoldContractError> {
    for hint in &outcome.concept_hints {
        let flags = if hint.contradiction_observed {
            SIDE_RECORD_FLAG_CONTRADICTION
        } else {
            0
        };
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::ConceptLink,
            hint.concept_id.raw(),
            0,
            [hint.salience.raw(), 0.0, 0.0, 0.0],
            flags,
        )?)?;
    }
    Ok(())
}

fn append_ranked_action_proposals(
    builder: &mut SideBufferBuilder,
    decision: &crate::DecisionSnapshot,
) -> Result<(), ScaffoldContractError> {
    for ranked in &decision.ranked_top_proposals {
        builder.push(ranked_action_record(*ranked, 0)?)?;
    }
    if let Some(rejected) = decision.rejected_top_proposal {
        builder.push(ranked_action_record(rejected, SIDE_RECORD_FLAG_REJECTED)?)?;
    }
    Ok(())
}

fn ranked_action_record(
    ranked: crate::RankedActionProposal,
    flags: u32,
) -> Result<PackedSideBufferRecord, ScaffoldContractError> {
    PackedSideBufferRecord::new(
        PackedSideBufferKind::RankedActionProposal,
        u64::from(ranked.proposal.action_id.raw()),
        ranked.proposal_index as u64,
        [
            ranked.final_score,
            ranked.proposal.score,
            ranked.proposal.confidence.raw(),
            ranked.proposal.salience.raw(),
        ],
        flags | (u32::from(action_kind_code(ranked.proposal.kind)) << 16),
    )
}

fn append_arbitration_details(
    builder: &mut SideBufferBuilder,
    decision: &crate::DecisionSnapshot,
) -> Result<(), ScaffoldContractError> {
    let trace = &decision.arbitration_trace;
    builder.push(PackedSideBufferRecord::new(
        PackedSideBufferKind::ArbitrationDetail,
        trace.trace_ref.raw(),
        trace
            .wta_result
            .selected_action_id
            .map_or(0, |id| id.raw())
            .into(),
        [
            trace.wta_result.selected_score,
            trace.score_threshold,
            trace.confidence_threshold,
            trace.tied_proposal_indices.len() as f32,
        ],
        trace
            .wta_result
            .selected_proposal_index
            .map_or(0, |index| index as u32),
    )?)?;
    for suppressed in &trace.suppressed_proposals {
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::ArbitrationDetail,
            trace.trace_ref.raw(),
            suppressed.proposal_index as u64,
            [
                suppression_reason_code(suppressed.reason).into(),
                0.0,
                0.0,
                0.0,
            ],
            SIDE_RECORD_FLAG_REJECTED,
        )?)?;
    }
    Ok(())
}

fn append_semantic_codes(
    builder: &mut SideBufferBuilder,
    pre: &crate::PreActionSnapshot,
) -> Result<(), ScaffoldContractError> {
    if let Some(context) = &pre.sensory.semantic_context {
        for code in &context.compressed_codes {
            builder.push(PackedSideBufferRecord::new(
                PackedSideBufferKind::SemanticCode,
                u64::from(code.codebook_id),
                u64::from(code.code),
                [code.salience.raw(), context.confidence.raw(), 0.0, 0.0],
                context.feature_flags.raw(),
            )?)?;
        }
    }
    Ok(())
}

fn append_gaussian_refs(
    builder: &mut SideBufferBuilder,
    pre: &crate::PreActionSnapshot,
) -> Result<(), ScaffoldContractError> {
    if let Some(context) = &pre.sensory.gaussian_context {
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::GaussianRef,
            context.egocentric_bin_hash,
            u64::from(context.feature_flags.raw()),
            [
                context.confidence.raw(),
                context.clusters.len() as f32,
                0.0,
                0.0,
            ],
            0,
        )?)?;
        for cluster in &context.clusters {
            builder.push(PackedSideBufferRecord::new(
                PackedSideBufferKind::GaussianRef,
                cluster.cluster_id.raw(),
                context.egocentric_bin_hash,
                [
                    cluster.salience.raw(),
                    cluster.distance_meters,
                    context.confidence.raw(),
                    0.0,
                ],
                context.feature_flags.raw(),
            )?)?;
        }
    }
    Ok(())
}

fn append_teacher_school_refs(
    builder: &mut SideBufferBuilder,
    decision: &crate::DecisionSnapshot,
    outcome: &crate::PostActionOutcome,
) -> Result<(), ScaffoldContractError> {
    if let Some(lesson) = decision.selected_action.teacher_lesson {
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::TeacherSchoolRef,
            lesson.teacher_entity.map_or(0, |id| id.raw()),
            lesson.lesson_id,
            [
                teacher_response_channel_code(lesson.response_channel).into(),
                0.0,
                0.0,
                0.0,
            ],
            0,
        )?)?;
    }
    if let Some(feedback) = outcome.teacher_feedback {
        builder.push(PackedSideBufferRecord::new(
            PackedSideBufferKind::TeacherSchoolRef,
            feedback.source_entity.map_or(0, |id| id.raw()),
            u64::from(teacher_channel_code(feedback.channel)),
            [feedback.valence.raw(), feedback.confidence.raw(), 0.0, 0.0],
            SIDE_RECORD_FLAG_HAS_TEACHER_CHANNEL,
        )?)?;
    }
    Ok(())
}

fn append_diagnostic_extras(
    builder: &mut SideBufferBuilder,
    decision: &crate::DecisionSnapshot,
    outcome: &crate::PostActionOutcome,
) -> Result<(), ScaffoldContractError> {
    builder.push(PackedSideBufferRecord::new(
        PackedSideBufferKind::DiagnosticExtra,
        decision.arbitration_trace.trace_ref.raw(),
        0,
        [
            decision.arbitration_trace.suppressed_proposals.len() as f32,
            decision.arbitration_trace.tied_proposal_indices.len() as f32,
            outcome.prediction_error.raw(),
            if outcome.contradiction_observed {
                1.0
            } else {
                0.0
            },
        ],
        if outcome.contradiction_observed {
            SIDE_RECORD_FLAG_CONTRADICTION
        } else {
            0
        },
    )?)?;
    Ok(())
}

fn build_flags(patch: &ExperiencePatch) -> u32 {
    let view = patch.as_learning_view();
    let decision = view.decision();
    let outcome = view.outcome();
    let selected = decision.selected_action;
    let pre = view.pre_action();

    let mut flags = 0;
    if outcome.success {
        flags |= PACKED_FLAG_SUCCESS;
    }
    if selected.target_entity.is_some() {
        flags |= PACKED_FLAG_HAS_TARGET_ENTITY;
    }
    if selected.target_position.is_some() {
        flags |= PACKED_FLAG_HAS_TARGET_POSITION;
    }
    if outcome.teacher_feedback.is_some() {
        flags |= PACKED_FLAG_HAS_TEACHER_FEEDBACK;
    }
    if outcome.contradiction_observed {
        flags |= PACKED_FLAG_CONTRADICTION_OBSERVED;
    }
    if pre.sensory.semantic_context.is_some() {
        flags |= PACKED_FLAG_HAS_SEMANTIC_CONTEXT;
    }
    if pre.sensory.gaussian_context.is_some() {
        flags |= PACKED_FLAG_HAS_GAUSSIAN_CONTEXT;
    }
    if selected.motor_payload.is_some() {
        flags |= PACKED_FLAG_HAS_MOTOR_PAYLOAD;
    }
    if selected.teacher_lesson.is_some() {
        flags |= PACKED_FLAG_HAS_TEACHER_LESSON;
    }
    flags
}

fn salience_summary(patch: &ExperiencePatch) -> Result<f32, ScaffoldContractError> {
    let view = patch.as_learning_view();
    let mut sum = view.pre_action().sensory.channels.novelty_signal.raw()
        + view.pre_action().sensory.channels.pain_signal.raw()
        + view.pre_action().memory_expectancy.salience_hint.raw();
    let mut count = 3.0f32;

    for proposal in &view.decision().proposals {
        sum += proposal.salience.raw();
        count += 1.0;
    }
    for hint in &view.outcome().concept_hints {
        sum += hint.salience.raw();
        count += 1.0;
    }
    for hint in &view.outcome().memory_hints {
        sum += hint.salience.raw();
        count += 1.0;
    }
    if let Some(context) = &view.pre_action().sensory.semantic_context {
        for entry in &context.salience {
            sum += entry.salience.raw();
            count += 1.0;
        }
    }
    if let Some(context) = &view.pre_action().sensory.gaussian_context {
        for cluster in &context.clusters {
            sum += cluster.salience.raw();
            count += 1.0;
        }
    }

    validate_finite(sum)?;
    let average = sum / count;
    validate_unit(average)?;
    Ok(average)
}

fn validate_unit_slice(values: &[f32]) -> Result<(), ScaffoldContractError> {
    for value in values {
        validate_unit(*value)?;
    }
    Ok(())
}

fn validate_unit(value: f32) -> Result<(), ScaffoldContractError> {
    validate_finite(value)?;
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn validate_signed_unit(value: f32) -> Result<(), ScaffoldContractError> {
    validate_finite(value)?;
    if (-1.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn action_kind_code(kind: ActionKind) -> u16 {
    match kind {
        ActionKind::Idle => 1,
        ActionKind::Hold => 2,
        ActionKind::Rest => 3,
        ActionKind::Inspect => 4,
        ActionKind::Move => 100,
        ActionKind::Interact => 200,
        ActionKind::Gesture => 300,
        ActionKind::Vocalize => 400,
        ActionKind::Write => 500,
    }
}

fn brain_scale_tier_code(tier: BrainScaleTier) -> u16 {
    match tier {
        BrainScaleTier::Nano512 => 1,
        BrainScaleTier::Small1024 => 2,
        BrainScaleTier::Standard2048 => 3,
        BrainScaleTier::Large4096 => 4,
        BrainScaleTier::Cognitive32768 => 5,
        BrainScaleTier::Student131k => 6,
        BrainScaleTier::Ascended1M => 7,
        BrainScaleTier::Ascended5M => 8,
        BrainScaleTier::ResearchCustom => u16::MAX,
    }
}

fn physical_contact_code(contact: PhysicalContactKind) -> u16 {
    match contact {
        PhysicalContactKind::None => 0,
        PhysicalContactKind::Touch => 1,
        PhysicalContactKind::Collision => 2,
        PhysicalContactKind::Blocked => 3,
        PhysicalContactKind::Consumed => 4,
        PhysicalContactKind::Moved => 5,
    }
}

fn teacher_channel_code(channel: TeacherPerceptionChannel) -> u16 {
    match channel {
        TeacherPerceptionChannel::Hearing => 1,
        TeacherPerceptionChannel::Vision => 2,
        TeacherPerceptionChannel::Writing => 3,
        TeacherPerceptionChannel::Gesture => 4,
        TeacherPerceptionChannel::Object => 5,
    }
}

fn teacher_response_channel_code(channel: crate::TeacherLessonResponseChannel) -> u16 {
    match channel {
        crate::TeacherLessonResponseChannel::Speech => 1,
        crate::TeacherLessonResponseChannel::Writing => 2,
        crate::TeacherLessonResponseChannel::Gesture => 3,
        crate::TeacherLessonResponseChannel::Demonstration => 4,
        crate::TeacherLessonResponseChannel::Feedback => 5,
    }
}

fn suppression_reason_code(reason: crate::SuppressionReason) -> u16 {
    match reason {
        crate::SuppressionReason::InvalidActionId => 1,
        crate::SuppressionReason::InvalidTarget => 2,
        crate::SuppressionReason::InvalidConfidence => 3,
        crate::SuppressionReason::InvalidIntensity => 4,
        crate::SuppressionReason::InvalidTeacherLesson => 5,
        crate::SuppressionReason::InvalidMotorPayload => 6,
        crate::SuppressionReason::NonFiniteScore => 7,
        crate::SuppressionReason::BelowScoreThreshold => 8,
        crate::SuppressionReason::BelowConfidenceThreshold => 9,
    }
}
