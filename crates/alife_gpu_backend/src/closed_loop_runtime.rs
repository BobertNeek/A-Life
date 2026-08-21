//! Shared required-GPU ownership and evidence contracts for the closed loop.
//!
//! The world supplies current perception and unscored candidates. This module
//! owns the one authoritative device, fixed class arenas, generation-checked
//! capabilities, bounded selection readback, and fail-stop transaction state.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use alife_core::{
    ActionId, ActionTarget, Blake3Digest, BrainActivityPolicyV1, BrainCapacityClass, BrainClassId,
    BrainDispatchIdentity, BrainPhenotype, BrainWorkCounters, BrainWorkReceipt,
    CandidateActionFamily, CanonicalDigestBuilder, CoactivationEvidence, Confidence,
    DendriticBranchSet, ExperiencePatch, FinalizedMemoryRecall, GpuPressureSample,
    GpuPressureSampleInput, LearningCommitToken, LearningSequenceGuard, NeuralActionSelection,
    NeuralThrottleDecision, NeuralThrottleLevel, OrganismId, OutcomeCreditPacket,
    PerceptionBaseDigest, PerceptionFrame, PerceptionFrameDigest, PhenotypeHash,
    ScaffoldContractError, SensorProfile, SpeechMotorPayload, BRAIN_ATP_BASAL_DEBIT_Q16,
    BRAIN_ATP_Q16_MAX, BRAIN_ATP_SLEEP_RECOVERY_Q16, REQUIRED_GPU_FEATURE_MASK,
};
use serde::{Deserialize, Serialize};

use crate::closed_loop_buffers::GpuFixedSlotUpload;
use crate::closed_loop_pipeline::{
    GpuDecodeMappedRecordsDiagnostic as PipelineDecodeMappedRecordsDiagnostic,
    GpuDecodeMappedRecordsSubstage as PipelineDecodeMappedRecordsSubstage,
    GpuFastPlasticityMalformedField as PipelineFastPlasticityMalformedField,
    GpuSelectionValidationFailure as PipelineSelectionValidationFailure,
    GpuSelectionValidationField as PipelineSelectionValidationField,
    GpuSelectorDiagnosticEnableError as PipelineSelectorDiagnosticEnableError,
    GpuSelectorDiagnosticErrorReceipt as PipelineSelectorDiagnosticErrorReceipt,
    GpuSelectorDiagnosticFailureClass as PipelineSelectorDiagnosticFailureClass,
    GPU_SELECTOR_DIAGNOSTIC_RECORD_WORDS,
};
use crate::{
    derive_executed_work, AddLifetimeSynapse, GpuActiveBatchUpload, GpuAdmissionReceipt,
    GpuAllocationEventKind, GpuAllocationEventReceipt, GpuBrainSlot, GpuClosedLoopError,
    GpuClosedLoopKernelSet, GpuClosedLoopPipelines, GpuCompactMapTicket,
    GpuFastPlasticityBatchEntry, GpuFixedActiveBatchEntry, GpuFixedClassArenaBuffers,
    GpuFixedClassArenaPlan, GpuFixedSlotRanges, GpuLearningReceipt,
    GpuMemoryContextDispatchReceipt, GpuMemoryContextUpload, GpuOutcomeCreditRecord,
    GpuPendingEligibilityRecord, GpuPerceptionUpload, GpuPreparedActiveBatch, GpuRuntimeBudget,
    GpuRuntimeProfile, GpuSelectorLogitCapture, GpuTimestampQueryResources, GpuV11CausalState,
    GpuV11Checkpoint, GpuV11WorkReceipt, GpuValidatedClassBatch, PendingEligibilityDiscardReceipt,
    PendingEligibilityIdentity, PendingEligibilityReceipt, GPU_CLOSED_LOOP_LAYOUT_VERSION,
};

pub const GPU_HARDWARE_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const GPU_DRIVER_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.driver.v1";
pub const GPU_FEATURE_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.features.v1";
pub const GPU_LIMITS_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.limits.v1";

const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
static NEXT_BACKEND_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_HARDWARE_RECEIPT_GENERATION: AtomicU64 = AtomicU64::new(1);
const GPU_TIMESTAMP_QUERY_COUNT: u32 = 2;
const GPU_TIMESTAMP_READBACK_BYTES: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExactGpuTimestampPeriod {
    significand: u32,
    binary_exponent: i16,
}

impl ExactGpuTimestampPeriod {
    fn try_from_f32_bits(bits: u32) -> Result<Self, ScaffoldContractError> {
        let sign = bits >> 31;
        let exponent_bits = (bits >> 23) & 0xff;
        let mantissa = bits & 0x7f_ffff;
        if sign != 0 || exponent_bits == 0xff || (exponent_bits == 0 && mantissa == 0) {
            return Err(ScaffoldContractError::GpuTimestampQueryUnavailable);
        }
        let (significand, binary_exponent) = if exponent_bits == 0 {
            (mantissa, -149)
        } else {
            (
                (1 << 23) | mantissa,
                i16::try_from(exponent_bits)
                    .map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)?
                    - 127
                    - 23,
            )
        };
        Ok(Self {
            significand,
            binary_exponent,
        })
    }

    fn elapsed_ns(self, begin: u64, end: u64) -> Result<u64, ScaffoldContractError> {
        let ticks = self.delta_ticks(begin, end)?;
        let scaled = u128::from(ticks)
            .checked_mul(u128::from(self.significand))
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?;
        let nanoseconds = if self.binary_exponent >= 0 {
            scaled
                .checked_shl(u32::from(self.binary_exponent.unsigned_abs()))
                .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
        } else {
            let shift = u32::from(self.binary_exponent.unsigned_abs());
            if shift >= u128::BITS {
                1
            } else {
                let quotient = scaled >> shift;
                let remainder_mask = (1_u128 << shift) - 1;
                quotient
                    .checked_add(u128::from(scaled & remainder_mask != 0))
                    .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
            }
        };
        u64::try_from(nanoseconds).map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)
    }

    fn delta_ticks(self, begin: u64, end: u64) -> Result<u64, ScaffoldContractError> {
        end.checked_sub(begin)
            .filter(|ticks| *ticks != 0)
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)
    }

    fn period_ns_q24(self) -> Result<u64, ScaffoldContractError> {
        let exponent = i32::from(self.binary_exponent) + 24;
        let scaled = u128::from(self.significand);
        let rounded = if exponent >= 0 {
            scaled
                .checked_shl(exponent as u32)
                .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
        } else {
            let shift = exponent.unsigned_abs();
            if shift >= u128::BITS {
                0
            } else {
                scaled
                    .checked_add(1_u128 << (shift - 1))
                    .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
                    >> shift
            }
        };
        u64::try_from(rounded)
            .ok()
            .filter(|value| *value != 0)
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)
    }
}

fn timestamp_mapping_completed(
    receiver: &std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
) -> bool {
    matches!(receiver.try_recv(), Ok(Ok(())))
}

struct GpuTimestampResources {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    period: ExactGpuTimestampPeriod,
}

impl GpuTimestampResources {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, ScaffoldContractError> {
        validate_required_device_features(device.features())?;
        let period =
            ExactGpuTimestampPeriod::try_from_f32_bits(queue.get_timestamp_period().to_bits())?;
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("closed-loop-runtime-timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: GPU_TIMESTAMP_QUERY_COUNT,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("closed-loop-runtime-timestamp-resolve"),
            size: GPU_TIMESTAMP_READBACK_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("closed-loop-runtime-timestamp-readback"),
            size: GPU_TIMESTAMP_READBACK_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            query_set,
            resolve_buffer,
            readback_buffer,
            period,
        })
    }

    fn read_delta_and_elapsed_ns(&self) -> Result<(u64, u64), ScaffoldContractError> {
        let mapped = self
            .readback_buffer
            .slice(..GPU_TIMESTAMP_READBACK_BYTES)
            .get_mapped_range();
        let bytes: &[u8] = &mapped;
        let begin = u64::from_le_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)?,
        );
        let end = u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)?,
        );
        drop(mapped);
        self.readback_buffer.unmap();
        Ok((
            self.period.delta_ticks(begin, end)?,
            self.period.elapsed_ns(begin, end)?,
        ))
    }

    fn period_ns_q24(&self) -> Result<u64, ScaffoldContractError> {
        self.period.period_ns_q24()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuHardwareReceipt {
    pub schema_version: u16,
    pub generation: u64,
    pub backend_api: String,
    pub adapter_name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub gpu_layout_version: u16,
    pub backend_version: String,
}

/// Ephemeral capture of Task 3 activity state. Runtime handle fields are
/// validated before the app canonicalizes this into portable world records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActivityRuntimeSnapshot {
    pub next_sequence_cursor: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
    pub next_completed_gpu_time_ns: u64,
    pub pressure: Option<GpuPressureSample>,
    pub throttle: Option<NeuralThrottleDecision>,
    pub work: Option<BrainWorkReceipt>,
}

/// Portable activity record accepted only as data for rebinding to a newly
/// allocated handle. It carries no backend instance, slot, or generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuPortableActivityRestoreRecord {
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub source_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub completed_gpu_time_ns: u64,
    pub queue_depth: u32,
    pub logical_heap_pressure_q16: u32,
    pub brain_atp_fraction_q16: u32,
    pub level: NeuralThrottleLevel,
    pub microsteps: u8,
    pub enabled_route_ids: Vec<u16>,
    pub route_schedule_digest: [u64; 4],
    pub work: BrainWorkCounters,
    pub neural_cost_q24: u64,
    pub atp_before_q16: u32,
    pub atp_debit_q16: u32,
    pub atp_after_q16: u32,
    pub policy_digest: [u64; 4],
}

/// Portable activity continuation rebound only after a new opaque handle exists.
/// Runtime slots and generations are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActivityRestoreInput {
    pub next_sequence_cursor: u64,
    pub checkpoint_tick: u64,
    pub next_completed_gpu_time_ns: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
    pub record: Option<GpuPortableActivityRestoreRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuBackendState {
    Ready,
    DeviceLost {
        last_checkpoint_digest: Option<[u64; 4]>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuBrainHandle {
    backend_instance_id: NonZeroU64,
    class_id: BrainClassId,
    slot: u32,
    generation: u32,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
}

impl GpuBrainHandle {
    pub const fn class_id(self) -> BrainClassId {
        self.class_id
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn organism_id(self) -> OrganismId {
        self.organism_id
    }

    pub const fn phenotype_hash(self) -> PhenotypeHash {
        self.phenotype_hash
    }
}

/// First value that disagrees between a sealed outcome packet and the
/// decision evidence currently installed for its resident GPU brain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuLearningEvidenceMismatchField {
    OutcomeTickAfterOriginating,
    ActiveActivationSide,
    DispatchGeneration,
    PhenotypeHashNonZero,
    FrameDigestNonZero,
    CandidateFeatureDigestNonZero,
    RewardPredictionErrorRange,
    PainRange,
    HomeostaticImprovementRange,
    FrustrationRange,
    NoveltyRange,
    ModulatorValueRange,
    PendingEligibilityPresent,
    PendingEligibilityRecordPresent,
    OrganismId,
    PhenotypeHash,
    HandleGeneration,
    PendingPhenotypeHash,
    DispatchGenerationIdentity,
    OriginatingTick,
    FrameDigest,
    ActiveActivationSideIdentity,
    CandidateIndex,
    ActionId,
    ActionFamily,
    CandidateFeatureDigest,
    ActiveEligibilityGeneration,
    StagingEligibilityGeneration,
    ActiveWeightGenerationNonZero,
    ReplayJournalGenerationNonZero,
    TransactionGenerationNonZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuLearningEvidenceMismatchReceipt {
    pub field: GpuLearningEvidenceMismatchField,
    pub expected: [u64; 4],
    pub actual: [u64; 4],
}

impl GpuLearningEvidenceMismatchReceipt {
    const fn scalar(field: GpuLearningEvidenceMismatchField, expected: u64, actual: u64) -> Self {
        Self {
            field,
            expected: [expected, 0, 0, 0],
            actual: [actual, 0, 0, 0],
        }
    }

    const fn words(
        field: GpuLearningEvidenceMismatchField,
        expected: [u64; 4],
        actual: [u64; 4],
    ) -> Self {
        Self {
            field,
            expected,
            actual,
        }
    }
}

impl std::fmt::Display for GpuLearningEvidenceMismatchReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "sealed outcome credit mismatch at {:?}: expected={:x?}, actual={:x?}",
            self.field, self.expected, self.actual
        )
    }
}

/// The bounded failure classes produced by the GPU fast-plasticity apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeApplyFastPlasticityFailureClass {
    MalformedUpload,
    StaleOrForeignHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeApplyFastPlasticityMalformedField {
    BrainSlotIndex,
    DuplicateSlotGeneration,
    PendingSlot,
    PendingSlotGeneration,
    RecordSchemaVersion,
    SelectionOffset,
    SynapseCountNonZero,
    RecurrentSynapseCountNonZero,
    RecurrentSynapseCountLessThanTotal,
    OutcomeSchemaVersion,
    OutcomeActiveActivationSideRange,
    OutcomeActiveActivationSide,
    OutcomeOrganismId,
    OutcomePhenotypeHash,
    OutcomeDispatchGeneration,
    OutcomeOriginatingTick,
    OutcomeFrameDigest,
    OutcomeCandidateAndFamily,
    OutcomeActionId,
    OutcomeCandidateFeatureDigest,
    PendingSchemaVersion,
    StagingEligibilityGeneration,
    ActiveWeightGenerationNonZero,
    ReplayGenerationNonZero,
    TransactionGenerationNonZero,
    ReplaySpanEndAtLeastStart,
    ReplaySpanNonZero,
    ReplaySpanMultipleOfFour,
    CommitRecordWordCount,
    CommitSchemaVersion,
    CommitSlot,
    CommitSlotGeneration,
    CommitStatus,
    CommitInputFastGeneration,
    CommitOutputFastGeneration,
    CommitOutputEligibilityGeneration,
    CommitReplayGeneration,
    CommitTransactionGeneration,
    CommitFastWeightsChangedRange,
    CommitMaxAbsDeltaFinite,
    CommitMaxAbsDeltaNonNegative,
    CommitZeroChangeDelta,
    CommitPositiveChangeDelta,
}

impl From<PipelineFastPlasticityMalformedField> for GpuRuntimeApplyFastPlasticityMalformedField {
    fn from(field: PipelineFastPlasticityMalformedField) -> Self {
        use GpuRuntimeApplyFastPlasticityMalformedField as Runtime;
        use PipelineFastPlasticityMalformedField as Pipeline;
        match field {
            Pipeline::BrainSlotIndex => Runtime::BrainSlotIndex,
            Pipeline::DuplicateSlotGeneration => Runtime::DuplicateSlotGeneration,
            Pipeline::PendingSlot => Runtime::PendingSlot,
            Pipeline::PendingSlotGeneration => Runtime::PendingSlotGeneration,
            Pipeline::RecordSchemaVersion => Runtime::RecordSchemaVersion,
            Pipeline::SelectionOffset => Runtime::SelectionOffset,
            Pipeline::SynapseCountNonZero => Runtime::SynapseCountNonZero,
            Pipeline::RecurrentSynapseCountNonZero => Runtime::RecurrentSynapseCountNonZero,
            Pipeline::RecurrentSynapseCountLessThanTotal => {
                Runtime::RecurrentSynapseCountLessThanTotal
            }
            Pipeline::OutcomeSchemaVersion => Runtime::OutcomeSchemaVersion,
            Pipeline::OutcomeActiveActivationSideRange => Runtime::OutcomeActiveActivationSideRange,
            Pipeline::OutcomeActiveActivationSide => Runtime::OutcomeActiveActivationSide,
            Pipeline::OutcomeOrganismId => Runtime::OutcomeOrganismId,
            Pipeline::OutcomePhenotypeHash => Runtime::OutcomePhenotypeHash,
            Pipeline::OutcomeDispatchGeneration => Runtime::OutcomeDispatchGeneration,
            Pipeline::OutcomeOriginatingTick => Runtime::OutcomeOriginatingTick,
            Pipeline::OutcomeFrameDigest => Runtime::OutcomeFrameDigest,
            Pipeline::OutcomeCandidateAndFamily => Runtime::OutcomeCandidateAndFamily,
            Pipeline::OutcomeActionId => Runtime::OutcomeActionId,
            Pipeline::OutcomeCandidateFeatureDigest => Runtime::OutcomeCandidateFeatureDigest,
            Pipeline::PendingSchemaVersion => Runtime::PendingSchemaVersion,
            Pipeline::StagingEligibilityGeneration => Runtime::StagingEligibilityGeneration,
            Pipeline::ActiveWeightGenerationNonZero => Runtime::ActiveWeightGenerationNonZero,
            Pipeline::ReplayGenerationNonZero => Runtime::ReplayGenerationNonZero,
            Pipeline::TransactionGenerationNonZero => Runtime::TransactionGenerationNonZero,
            Pipeline::ReplaySpanEndAtLeastStart => Runtime::ReplaySpanEndAtLeastStart,
            Pipeline::ReplaySpanNonZero => Runtime::ReplaySpanNonZero,
            Pipeline::ReplaySpanMultipleOfFour => Runtime::ReplaySpanMultipleOfFour,
            Pipeline::CommitRecordWordCount => Runtime::CommitRecordWordCount,
            Pipeline::CommitSchemaVersion => Runtime::CommitSchemaVersion,
            Pipeline::CommitSlot => Runtime::CommitSlot,
            Pipeline::CommitSlotGeneration => Runtime::CommitSlotGeneration,
            Pipeline::CommitStatus => Runtime::CommitStatus,
            Pipeline::CommitInputFastGeneration => Runtime::CommitInputFastGeneration,
            Pipeline::CommitOutputFastGeneration => Runtime::CommitOutputFastGeneration,
            Pipeline::CommitOutputEligibilityGeneration => {
                Runtime::CommitOutputEligibilityGeneration
            }
            Pipeline::CommitReplayGeneration => Runtime::CommitReplayGeneration,
            Pipeline::CommitTransactionGeneration => Runtime::CommitTransactionGeneration,
            Pipeline::CommitFastWeightsChangedRange => Runtime::CommitFastWeightsChangedRange,
            Pipeline::CommitMaxAbsDeltaFinite => Runtime::CommitMaxAbsDeltaFinite,
            Pipeline::CommitMaxAbsDeltaNonNegative => Runtime::CommitMaxAbsDeltaNonNegative,
            Pipeline::CommitZeroChangeDelta => Runtime::CommitZeroChangeDelta,
            Pipeline::CommitPositiveChangeDelta => Runtime::CommitPositiveChangeDelta,
        }
    }
}

/// Lossless identity for a rejected fast-plasticity submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeApplyFastPlasticityFailureReceipt {
    pub class: GpuRuntimeApplyFastPlasticityFailureClass,
    pub class_id: u16,
    pub chunk_index: usize,
    /// Original batch index of the first entry in the rejected GPU submission.
    pub submitted_entry: usize,
    pub malformed_field: Option<GpuRuntimeApplyFastPlasticityMalformedField>,
    pub expected: Option<[u64; 4]>,
    pub actual: Option<[u64; 4]>,
}

impl std::fmt::Display for GpuRuntimeApplyFastPlasticityFailureReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "apply_fast_plasticity {:?}: class_id={}, chunk_index={}, submitted_entry={}",
            self.class, self.class_id, self.chunk_index, self.submitted_entry
        )?;
        if let (Some(field), Some(expected), Some(actual)) =
            (self.malformed_field, self.expected, self.actual)
        {
            write!(
                formatter,
                ", field={field:?}, expected={expected:x?}, actual={actual:x?}"
            )?;
        }
        Ok(())
    }
}

/// Stable target identity carried opaquely through the backend residency
/// transaction. The backend only validates non-zero and uniqueness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpuCuratedResidencyTargetIdentity(pub u64);

impl GpuCuratedResidencyTargetIdentity {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuCuratedResidencyEntry {
    pub organism_id: OrganismId,
    pub opaque_target_identity: GpuCuratedResidencyTargetIdentity,
    pub phenotype: BrainPhenotype,
    pub exact_phenotype_hash: PhenotypeHash,
    pub exact_foundation_hash: Blake3Digest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuCuratedResidencyCohort {
    pub expected_old_generation: u64,
    pub new_generation_fingerprint: [u64; 4],
    pub ordered_entries: Vec<GpuCuratedResidencyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCuratedResidentReceipt {
    pub organism_id: OrganismId,
    pub opaque_target_identity: GpuCuratedResidencyTargetIdentity,
    pub exact_phenotype_hash: PhenotypeHash,
    pub exact_foundation_hash: Blake3Digest,
    pub handle: GpuBrainHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCuratedResidencyReceipt {
    pub generation_fingerprint: [u64; 4],
    pub ordered_residents: Vec<GpuCuratedResidentReceipt>,
    pub submission_completed: bool,
    pub backend_hardware_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GpuCuratedResidencyOutcome {
    Committed(GpuCuratedResidencyReceipt),
    PreSubmitFailure {
        error: ScaffoldContractError,
        retryable: bool,
    },
    Unknown {
        error: ScaffoldContractError,
        fail_stop: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuClosedLoopTick {
    pub handle: GpuBrainHandle,
    pub dispatch_generation: u64,
    pub base_digest: PerceptionBaseDigest,
    pub frame_digest: PerceptionFrameDigest,
    pub memory_context_binding: Option<GpuMemoryContextDispatchReceipt>,
    pub active_activation_side: u8,
    pub selection: NeuralActionSelection,
    pub speech_payload: Option<SpeechMotorPayload>,
    pub factorized_motor_candidates: [u16; crate::GPU_MOTOR_CHANNEL_SLOT_COUNT],
    pub pending_eligibility: PendingEligibilityReceipt,
    pub pressure: GpuPressureSample,
    pub throttle: NeuralThrottleDecision,
    pub work: BrainWorkReceipt,
    pub v11_work: GpuV11WorkReceipt,
    pub compact_readback_bytes: usize,
    pub hardware_receipt_generation: u64,
    pub selector_diagnostic: Option<GpuSelectorDiagnosticReceipt>,
}

pub const GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION: u16 = 3;
const GPU_SELECTOR_INVALID_LOGIT_BITS: u32 = 0x7fc0_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuSelectorCandidateValidity {
    Valid,
    InvalidLogit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuSelectorTieBreak {
    LowestCandidateIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuSelectorExplorationMode {
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSelectorPolicyIdentity {
    pub schema_version: u16,
    pub invalid_logit_bits: u32,
    pub tie_break: GpuSelectorTieBreak,
    pub exploration_mode: GpuSelectorExplorationMode,
}

impl GpuSelectorPolicyIdentity {
    pub const PRODUCTION_V1: Self = Self {
        schema_version: GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION,
        invalid_logit_bits: GPU_SELECTOR_INVALID_LOGIT_BITS,
        tie_break: GpuSelectorTieBreak::LowestCandidateIndex,
        exploration_mode: GpuSelectorExplorationMode::Disabled,
    };
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSelectorCandidateDiagnostic {
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub family: CandidateActionFamily,
    pub target: ActionTarget,
    pub validity: GpuSelectorCandidateValidity,
    pub decoder_family_bias: f32,
    pub binding: Option<GpuSelectorBindingIdentity>,
    pub contributions: Vec<GpuSelectorSynapseContribution>,
    pub pre_context_logit: Option<f32>,
    pub memory_context_delta: Option<f32>,
    pub final_logit: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSelectorBindingIdentity {
    pub decoder_plan_offset: u32,
    pub decoder_family_offset: u32,
    pub decoder_family_start: u32,
    pub decoder_family_count: u32,
    pub weight_index_start: u32,
    pub weight_index_count: u32,
    pub activation_side: u8,
    pub activation_offset: u32,
    pub motor_start: u32,
    pub feature_offset: u32,
    pub genetic_weight_offset: u32,
    pub alpha_offset: u32,
    pub lifetime_weight_offset: u32,
    pub fast_weight_offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuSelectorSynapseContribution {
    pub synapse_index: u32,
    pub global_synapse_id: u32,
    pub input_lane: u16,
    pub motor_index: u16,
    pub motor: f32,
    pub feature: f32,
    pub genetic: f32,
    pub lifetime: f32,
    pub alpha: f32,
    pub fast: f32,
    pub effective_weight: f32,
    pub signed_contribution: f32,
    pub running_logit: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSelectorDiagnosticReceipt {
    pub schema_version: u16,
    pub frame_digest: PerceptionFrameDigest,
    pub phenotype_hash: PhenotypeHash,
    pub dispatch_generation: u64,
    pub policy: GpuSelectorPolicyIdentity,
    pub requested_candidate_indices: Vec<u16>,
    pub candidates: Vec<GpuSelectorCandidateDiagnostic>,
    pub argmax_candidate_index: u16,
    pub equal_max_candidate_indices: Vec<u16>,
    pub chosen_candidate_index: u16,
}

/// Stable class for a selector-diagnostic enable failure. This mirrors the
/// crate-private pipeline boundary at the public runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticFailureClass {
    CapacityExceeded,
    ArithmeticOverflow,
    SubmissionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticFailureStage {
    SelectorDiagnosticBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSelectorDiagnosticFailureReceipt {
    pub stage: GpuRuntimeSelectorDiagnosticFailureStage,
    pub class: GpuRuntimeSelectorDiagnosticFailureClass,
    pub class_id: u16,
    pub chunk_index: usize,
}

impl GpuRuntimeSelectorDiagnosticFailureReceipt {
    pub const fn from_gpu_error(
        error: GpuClosedLoopError,
        class_id: u16,
        chunk_index: usize,
    ) -> Option<Self> {
        let class = match error {
            GpuClosedLoopError::CapacityExceeded => {
                GpuRuntimeSelectorDiagnosticFailureClass::CapacityExceeded
            }
            GpuClosedLoopError::ArithmeticOverflow => {
                GpuRuntimeSelectorDiagnosticFailureClass::ArithmeticOverflow
            }
            GpuClosedLoopError::SubmissionFailed => {
                GpuRuntimeSelectorDiagnosticFailureClass::SubmissionFailed
            }
            _ => return None,
        };
        Some(Self {
            stage: GpuRuntimeSelectorDiagnosticFailureStage::SelectorDiagnosticBytes,
            class,
            class_id,
            chunk_index,
        })
    }

    pub const fn gpu_error(self) -> GpuClosedLoopError {
        match self.class {
            GpuRuntimeSelectorDiagnosticFailureClass::CapacityExceeded => {
                GpuClosedLoopError::CapacityExceeded
            }
            GpuRuntimeSelectorDiagnosticFailureClass::ArithmeticOverflow => {
                GpuClosedLoopError::ArithmeticOverflow
            }
            GpuRuntimeSelectorDiagnosticFailureClass::SubmissionFailed => {
                GpuClosedLoopError::SubmissionFailed
            }
        }
    }

    pub fn mapped_contract_error(self) -> ScaffoldContractError {
        map_gpu_contract_error(self.gpu_error())
    }
}

impl std::fmt::Display for GpuRuntimeSelectorDiagnosticFailureReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "selector diagnostic failure: stage={:?} class={:?} class_id={} chunk_index={}",
            self.stage, self.class, self.class_id, self.chunk_index,
        )
    }
}

impl std::error::Error for GpuRuntimeSelectorDiagnosticFailureReceipt {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass {
    StaleOrForeignHandle,
    SubmissionFailed,
    MalformedUpload,
    ArithmeticOverflow,
    CapacityExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage {
    AuthorityPoisoned,
    CompactWordCount,
    SelectionValidation,
    SpeechValidation,
    FactorizedValidation,
    PendingEligibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticSelectionValidationField {
    RecordCount,
    Slot,
    SlotGeneration,
    DispatchGenerationNonZero,
    DispatchGeneration,
    ActiveActivationSide,
    ActiveTilesNonZero,
    ActiveSynapsesNonZero,
    DendriticGatedBranches,
    Status,
    CandidateIndex,
    LogitFinite,
    CandidateRecord,
    ConfidenceQ16,
    EmptyCandidateIndex,
    EmptyLogitBits,
    EmptyConfidenceQ16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSelectorDiagnosticSelectionValidationFailureReceipt {
    pub field: GpuRuntimeSelectorDiagnosticSelectionValidationField,
    pub expected: u64,
    pub actual: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstageReceipt {
    pub substage: GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage,
    pub expected_words: Option<usize>,
    pub actual_words: Option<usize>,
    pub selection_failure: Option<GpuRuntimeSelectorDiagnosticSelectionValidationFailureReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt {
    pub class: GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass,
    pub class_id: u16,
    pub chunk_index: usize,
    pub substage: Option<GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstageReceipt>,
}

impl GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt {
    const fn from_gpu_error(
        error: GpuClosedLoopError,
        class_id: u16,
        chunk_index: usize,
        diagnostic: PipelineDecodeMappedRecordsDiagnostic,
    ) -> Option<Self> {
        let class = match error {
            GpuClosedLoopError::StaleOrForeignHandle => {
                GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::StaleOrForeignHandle
            }
            GpuClosedLoopError::SubmissionFailed => {
                GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::SubmissionFailed
            }
            GpuClosedLoopError::MalformedUpload => {
                GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::MalformedUpload
            }
            GpuClosedLoopError::ArithmeticOverflow => {
                GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::ArithmeticOverflow
            }
            GpuClosedLoopError::CapacityExceeded => {
                GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::CapacityExceeded
            }
            _ => return None,
        };
        Some(Self {
            class,
            class_id,
            chunk_index,
            substage: match diagnostic.substage {
                Some(substage) => Some(GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstageReceipt {
                    substage: match substage {
                        PipelineDecodeMappedRecordsSubstage::AuthorityPoisoned => GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage::AuthorityPoisoned,
                        PipelineDecodeMappedRecordsSubstage::CompactWordCount => GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage::CompactWordCount,
                        PipelineDecodeMappedRecordsSubstage::SelectionValidation => GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage::SelectionValidation,
                        PipelineDecodeMappedRecordsSubstage::SpeechValidation => GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage::SpeechValidation,
                        PipelineDecodeMappedRecordsSubstage::FactorizedValidation => GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage::FactorizedValidation,
                        PipelineDecodeMappedRecordsSubstage::PendingEligibility => GpuRuntimeSelectorDiagnosticDecodeMappedRecordsSubstage::PendingEligibility,
                    },
                    expected_words: diagnostic.expected_words,
                    actual_words: diagnostic.actual_words,
                    selection_failure: match diagnostic.selection_failure {
                        Some(PipelineSelectionValidationFailure {
                            field,
                            expected,
                            actual,
                        }) => Some(
                            GpuRuntimeSelectorDiagnosticSelectionValidationFailureReceipt {
                                field: match field {
                                    PipelineSelectionValidationField::RecordCount => GpuRuntimeSelectorDiagnosticSelectionValidationField::RecordCount,
                                    PipelineSelectionValidationField::Slot => GpuRuntimeSelectorDiagnosticSelectionValidationField::Slot,
                                    PipelineSelectionValidationField::SlotGeneration => GpuRuntimeSelectorDiagnosticSelectionValidationField::SlotGeneration,
                                    PipelineSelectionValidationField::DispatchGenerationNonZero => GpuRuntimeSelectorDiagnosticSelectionValidationField::DispatchGenerationNonZero,
                                    PipelineSelectionValidationField::DispatchGeneration => GpuRuntimeSelectorDiagnosticSelectionValidationField::DispatchGeneration,
                                    PipelineSelectionValidationField::ActiveActivationSide => GpuRuntimeSelectorDiagnosticSelectionValidationField::ActiveActivationSide,
                                    PipelineSelectionValidationField::ActiveTilesNonZero => GpuRuntimeSelectorDiagnosticSelectionValidationField::ActiveTilesNonZero,
                                    PipelineSelectionValidationField::ActiveSynapsesNonZero => GpuRuntimeSelectorDiagnosticSelectionValidationField::ActiveSynapsesNonZero,
                                    PipelineSelectionValidationField::DendriticGatedBranches => GpuRuntimeSelectorDiagnosticSelectionValidationField::DendriticGatedBranches,
                                    PipelineSelectionValidationField::Status => GpuRuntimeSelectorDiagnosticSelectionValidationField::Status,
                                    PipelineSelectionValidationField::CandidateIndex => GpuRuntimeSelectorDiagnosticSelectionValidationField::CandidateIndex,
                                    PipelineSelectionValidationField::LogitFinite => GpuRuntimeSelectorDiagnosticSelectionValidationField::LogitFinite,
                                    PipelineSelectionValidationField::CandidateRecord => GpuRuntimeSelectorDiagnosticSelectionValidationField::CandidateRecord,
                                    PipelineSelectionValidationField::ConfidenceQ16 => GpuRuntimeSelectorDiagnosticSelectionValidationField::ConfidenceQ16,
                                    PipelineSelectionValidationField::EmptyCandidateIndex => GpuRuntimeSelectorDiagnosticSelectionValidationField::EmptyCandidateIndex,
                                    PipelineSelectionValidationField::EmptyLogitBits => GpuRuntimeSelectorDiagnosticSelectionValidationField::EmptyLogitBits,
                                    PipelineSelectionValidationField::EmptyConfidenceQ16 => GpuRuntimeSelectorDiagnosticSelectionValidationField::EmptyConfidenceQ16,
                                },
                                expected,
                                actual,
                            },
                        ),
                        None => None,
                    },
                }),
                None => None,
            },
        })
    }

    pub const fn gpu_error(self) -> GpuClosedLoopError {
        match self.class {
            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::StaleOrForeignHandle => {
                GpuClosedLoopError::StaleOrForeignHandle
            }
            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::SubmissionFailed => {
                GpuClosedLoopError::SubmissionFailed
            }
            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::MalformedUpload => {
                GpuClosedLoopError::MalformedUpload
            }
            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::ArithmeticOverflow => {
                GpuClosedLoopError::ArithmeticOverflow
            }
            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureClass::CapacityExceeded => {
                GpuClosedLoopError::CapacityExceeded
            }
        }
    }

    pub fn mapped_contract_error(self) -> ScaffoldContractError {
        map_gpu_contract_error(self.gpu_error())
    }
}

impl std::fmt::Display for GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "DecodeMappedRecords GPU failure {:?} (class_id={}, chunk_index={}, substage={:?})",
            self.class, self.class_id, self.chunk_index, self.substage
        )
    }
}

impl std::error::Error for GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticBuildFailureClass {
    InvalidDecisionEvidence,
    BrainOwnershipMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticBuildFailureField {
    MissingCapture,
    CaptureCount,
    MissingSelectionRecord,
    ChosenCandidateIndex,
    ResidentBrainOwnership,
    CandidateLogitShape,
    RequestedContributionShape,
    DecoderFamily,
    ContributionDetailWord,
    ActivationSide,
    FamilyStart,
    WeightIndexStart,
    BindingIdentity,
    FamilyCount,
    ContributionRecordIdentity,
    InputLane,
    MotorIndex,
    Argmax,
    ReceiptContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSelectorDiagnosticBuildFailureReceipt {
    pub class: GpuRuntimeSelectorDiagnosticBuildFailureClass,
    pub field: GpuRuntimeSelectorDiagnosticBuildFailureField,
    pub expected_binding_identity: Option<GpuSelectorBindingIdentity>,
    pub actual_binding_identity: Option<GpuSelectorBindingIdentity>,
}

impl GpuRuntimeSelectorDiagnosticBuildFailureReceipt {
    pub const fn mapped_contract_error(self) -> ScaffoldContractError {
        match self.class {
            GpuRuntimeSelectorDiagnosticBuildFailureClass::InvalidDecisionEvidence => {
                ScaffoldContractError::InvalidDecisionEvidence
            }
            GpuRuntimeSelectorDiagnosticBuildFailureClass::BrainOwnershipMismatch => {
                ScaffoldContractError::BrainOwnershipMismatch
            }
        }
    }
}

impl std::fmt::Display for GpuRuntimeSelectorDiagnosticBuildFailureReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "BuildSelectorDiagnostic {:?} at {:?}",
            self.class, self.field
        )?;
        if let (Some(expected), Some(actual)) =
            (self.expected_binding_identity, self.actual_binding_identity)
        {
            write!(formatter, ": expected={expected:?} actual={actual:?}")?;
        }
        Ok(())
    }
}

impl std::error::Error for GpuRuntimeSelectorDiagnosticBuildFailureReceipt {}

/// Complete selector-diagnostic enable receipt translated out of the
/// crate-private pipeline module without dropping any planning inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeSelectorDiagnosticErrorReceipt {
    pub class: GpuRuntimeSelectorDiagnosticFailureClass,
    pub class_id: u16,
    pub chunk_index: usize,
    pub row: usize,
    pub base_words: usize,
    pub candidate_count: u32,
    pub decoder_synapse_count: u32,
    pub record_words: usize,
    pub detail_words: u128,
    pub frame_payload_capacity_words: usize,
}

impl From<PipelineSelectorDiagnosticFailureClass> for GpuRuntimeSelectorDiagnosticFailureClass {
    fn from(class: PipelineSelectorDiagnosticFailureClass) -> Self {
        match class {
            PipelineSelectorDiagnosticFailureClass::CapacityExceeded => Self::CapacityExceeded,
            PipelineSelectorDiagnosticFailureClass::ArithmeticOverflow => Self::ArithmeticOverflow,
        }
    }
}

impl From<PipelineSelectorDiagnosticErrorReceipt> for GpuRuntimeSelectorDiagnosticErrorReceipt {
    fn from(receipt: PipelineSelectorDiagnosticErrorReceipt) -> Self {
        Self {
            class: receipt.class.into(),
            class_id: receipt.class_id,
            chunk_index: receipt.chunk_index,
            row: receipt.row,
            base_words: receipt.base_words,
            candidate_count: receipt.candidate_count,
            decoder_synapse_count: receipt.decoder_synapse_count,
            record_words: receipt.record_words,
            detail_words: receipt.detail_words,
            frame_payload_capacity_words: receipt.frame_payload_capacity_words,
        }
    }
}

impl std::fmt::Display for GpuRuntimeSelectorDiagnosticErrorReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "selector diagnostic {:?}: class_id={} chunk_index={} row={} base_words={} candidate_count={} decoder_synapse_count={} record_words={} detail_words={} frame_payload_capacity_words={}",
            self.class,
            self.class_id,
            self.chunk_index,
            self.row,
            self.base_words,
            self.candidate_count,
            self.decoder_synapse_count,
            self.record_words,
            self.detail_words,
            self.frame_payload_capacity_words,
        )
    }
}

impl std::error::Error for GpuRuntimeSelectorDiagnosticErrorReceipt {}

/// Typed failure from the opt-in selector-diagnostic enable boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRuntimeSelectorDiagnosticEnableFailure {
    Contract(GpuClosedLoopError),
    Receipt(GpuRuntimeSelectorDiagnosticErrorReceipt),
}

impl std::fmt::Display for GpuRuntimeSelectorDiagnosticEnableFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => {
                write!(formatter, "selector diagnostic enable contract: {error}")
            }
            Self::Receipt(receipt) => receipt.fmt(formatter),
        }
    }
}

impl std::error::Error for GpuRuntimeSelectorDiagnosticEnableFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Contract(error) => Some(error),
            Self::Receipt(receipt) => Some(receipt),
        }
    }
}

impl GpuRuntimeSelectorDiagnosticEnableFailure {
    pub const fn receipt(self) -> Option<GpuRuntimeSelectorDiagnosticErrorReceipt> {
        match self {
            Self::Contract(_) => None,
            Self::Receipt(receipt) => Some(receipt),
        }
    }

    pub const fn gpu_error(self) -> GpuClosedLoopError {
        match self {
            Self::Contract(error) => error,
            Self::Receipt(receipt) => match receipt.class {
                GpuRuntimeSelectorDiagnosticFailureClass::CapacityExceeded => {
                    GpuClosedLoopError::CapacityExceeded
                }
                GpuRuntimeSelectorDiagnosticFailureClass::ArithmeticOverflow => {
                    GpuClosedLoopError::ArithmeticOverflow
                }
                GpuRuntimeSelectorDiagnosticFailureClass::SubmissionFailed => {
                    GpuClosedLoopError::SubmissionFailed
                }
            },
        }
    }

    pub fn mapped_contract_error(self) -> ScaffoldContractError {
        map_gpu_contract_error(self.gpu_error())
    }
}

/// Ordered production stage reached by the opt-in selector-diagnostic path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRuntimeSelectorDiagnosticStage {
    WriteStagedUploads,
    RecordDispatch,
    RegisterCompactMapping,
    RegisterSelectorDiagnosticMapping,
    DevicePoll,
    CompactMappingCompletion,
    SelectorMappingCompletion,
    TimestampMappingCompletion,
    DeviceLostAfterSubmit,
    TimestampReadback,
    DecodeSelectorDiagnostics,
    DecodeMappedRecords,
    PrevalidateCommit,
    ValidateReceiptIdentity,
    BuildSelectorDiagnostic,
    AccountActivityWork,
    PrepareTicks,
    ComputeReadbackBytes,
    CommitValidatedBatch,
    ValidateCommitShape,
    ValidateCommitContents,
    ValidateHostPrecommit,
    ConvertPopulation,
}

/// Failure from the opt-in diagnostic path, retaining whether it happened
/// before enable, while enabling, or after the GPU batch was staged.
#[derive(Debug, PartialEq, Eq)]
pub enum GpuRuntimeSelectorDiagnosticError {
    Preflight(ScaffoldContractError),
    Enable(GpuRuntimeSelectorDiagnosticEnableFailure),
    LaterStage(GpuRuntimeSelectorDiagnosticFailureReceipt),
    DecodeMappedRecords(GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt),
    BuildSelectorDiagnostic(GpuRuntimeSelectorDiagnosticBuildFailureReceipt),
    LaterStageContract {
        stage: GpuRuntimeSelectorDiagnosticStage,
        error: ScaffoldContractError,
    },
}

impl std::fmt::Display for GpuRuntimeSelectorDiagnosticError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preflight(error) => {
                write!(formatter, "selector diagnostic preflight failed: {error}")
            }
            Self::Enable(error) => write!(
                formatter,
                "selector diagnostic enable-stage failure: {error}"
            ),
            Self::LaterStage(error) => write!(
                formatter,
                "selector diagnostic later-stage GPU failure: {error}"
            ),
            Self::DecodeMappedRecords(error) => write!(
                formatter,
                "selector diagnostic DecodeMappedRecords GPU failure: {error}"
            ),
            Self::BuildSelectorDiagnostic(error) => error.fmt(formatter),
            Self::LaterStageContract { stage, error } => write!(
                formatter,
                "selector diagnostic later-stage GPU failure at {stage:?}: {error}"
            ),
        }
    }
}

impl std::error::Error for GpuRuntimeSelectorDiagnosticError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(error) => Some(error),
            Self::LaterStageContract { error, .. } => Some(error),
            Self::LaterStage(error) => Some(error),
            Self::DecodeMappedRecords(error) => Some(error),
            Self::BuildSelectorDiagnostic(error) => Some(error),
            Self::Enable(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuNeuralTimingSample {
    pub dispatch_generation: u64,
    pub class_id_raw: u16,
    pub population: u32,
    pub inference_timestamp_ticks: u64,
    pub plasticity_timestamp_ticks: u64,
    pub timestamp_period_ns_q24: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingInferenceTiming {
    dispatch_generation: u64,
    class_id_raw: Option<u16>,
    population: u32,
    inference_timestamp_ticks: u64,
}

/// One finalized candidate-memory context bound to an exact live brain handle
/// and immutable perception frame.
#[derive(Debug, Clone, Copy)]
pub struct GpuClosedLoopMemoryTickInput<'a> {
    handle: GpuBrainHandle,
    frame: &'a PerceptionFrame,
    memory_upload: &'a GpuMemoryContextUpload,
}

impl<'a> GpuClosedLoopMemoryTickInput<'a> {
    pub fn try_new(
        handle: GpuBrainHandle,
        frame: &'a PerceptionFrame,
        memory_upload: &'a GpuMemoryContextUpload,
    ) -> Result<Self, ScaffoldContractError> {
        frame.validate()?;
        if handle.organism_id != frame.organism_id()
            || memory_upload.header.class_id != u32::from(handle.class_id.raw())
            || memory_upload.header.slot != handle.slot
            || memory_upload.header.slot_generation != handle.generation
            || memory_upload.header.tick() != frame.tick().raw()
            || memory_upload.base_frame_digest != frame.base_digest()
            || memory_upload.context_digest != frame.context().canonical_digest()
            || memory_upload.final_frame_digest != frame.frame_digest()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(Self {
            handle,
            frame,
            memory_upload,
        })
    }
}

/// Mixed-class memory-aware runtime tick input. The backend still groups rows
/// by class internally and submits all class pipelines in one command buffer.
#[derive(Debug)]
pub struct GpuClosedLoopMemoryBatchInput<'a> {
    members: Vec<GpuClosedLoopMemoryTickInput<'a>>,
}

impl<'a> GpuClosedLoopMemoryBatchInput<'a> {
    pub fn try_new(
        members: Vec<GpuClosedLoopMemoryTickInput<'a>>,
    ) -> Result<Self, ScaffoldContractError> {
        if members.is_empty() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(Self { members })
    }
}

#[derive(Clone, Copy)]
struct GpuRuntimeTickInput<'a> {
    handle: GpuBrainHandle,
    frame: &'a PerceptionFrame,
    memory_upload: Option<&'a GpuMemoryContextUpload>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CheckedNonZeroAllocator {
    next: Option<NonZeroU64>,
}

#[cfg(test)]
impl CheckedNonZeroAllocator {
    const fn new(next: u64) -> Self {
        Self {
            next: NonZeroU64::new(next),
        }
    }

    fn take(&mut self) -> Result<NonZeroU64, GpuClosedLoopError> {
        let value = self.next.ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        self.next = value.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(value)
    }
}

#[cfg(test)]
thread_local! {
    static TEST_ALLOCATION_STATE: std::cell::RefCell<Option<(CheckedNonZeroAllocator, CheckedNonZeroAllocator)>> = const { std::cell::RefCell::new(None) };
}

fn take_atomic_nonzero(allocator: &AtomicU64) -> Result<NonZeroU64, GpuClosedLoopError> {
    let value = allocator
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    NonZeroU64::new(value).ok_or(GpuClosedLoopError::ArithmeticOverflow)
}

fn next_backend_instance_id() -> Result<NonZeroU64, GpuClosedLoopError> {
    #[cfg(test)]
    if let Some(value) = TEST_ALLOCATION_STATE.with(|state| {
        state
            .borrow_mut()
            .as_mut()
            .map(|(backend, _)| backend.take())
    }) {
        return value;
    }
    take_atomic_nonzero(&NEXT_BACKEND_INSTANCE_ID)
}

fn next_hardware_receipt_generation() -> Result<NonZeroU64, GpuClosedLoopError> {
    #[cfg(test)]
    if let Some(value) = TEST_ALLOCATION_STATE.with(|state| {
        state
            .borrow_mut()
            .as_mut()
            .map(|(_, receipt)| receipt.take())
    }) {
        return value;
    }
    take_atomic_nonzero(&NEXT_HARDWARE_RECEIPT_GENERATION)
}

#[cfg(test)]
fn with_runtime_allocation_state_for_test<R>(
    backend_next: u64,
    receipt_next: u64,
    operation: impl FnOnce() -> R,
) -> R {
    TEST_ALLOCATION_STATE.with(|state| {
        assert!(state.borrow().is_none(), "nested allocation test state");
        *state.borrow_mut() = Some((
            CheckedNonZeroAllocator::new(backend_next),
            CheckedNonZeroAllocator::new(receipt_next),
        ));
    });
    let result = operation();
    TEST_ALLOCATION_STATE.with(|state| *state.borrow_mut() = None);
    result
}

fn canonical_driver_digest(driver: &str, driver_info: &str) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(GPU_DRIVER_DIGEST_DOMAIN);
    digest.write_sequence_len(2);
    digest.write_utf8(driver);
    digest.write_utf8(driver_info);
    digest.finish256()
}

fn canonical_feature_digest(requested: wgpu::Features, enabled: wgpu::Features) -> [u64; 4] {
    let requested = requested.bits().0;
    let enabled = enabled.bits().0;
    let mut digest = CanonicalDigestBuilder::new(GPU_FEATURE_DIGEST_DOMAIN);
    digest.write_sequence_len(4);
    for word in [requested[0], requested[1], enabled[0], enabled[1]] {
        digest.write_u64(word);
    }
    digest.finish256()
}

fn canonical_limit_words_for_test(limits: &wgpu::Limits) -> [u64; 51] {
    [
        u64::from(limits.max_texture_dimension_1d),
        u64::from(limits.max_texture_dimension_2d),
        u64::from(limits.max_texture_dimension_3d),
        u64::from(limits.max_texture_array_layers),
        u64::from(limits.max_bind_groups),
        u64::from(limits.max_bindings_per_bind_group),
        u64::from(limits.max_dynamic_uniform_buffers_per_pipeline_layout),
        u64::from(limits.max_dynamic_storage_buffers_per_pipeline_layout),
        u64::from(limits.max_sampled_textures_per_shader_stage),
        u64::from(limits.max_samplers_per_shader_stage),
        u64::from(limits.max_storage_buffers_per_shader_stage),
        u64::from(limits.max_storage_textures_per_shader_stage),
        u64::from(limits.max_uniform_buffers_per_shader_stage),
        u64::from(limits.max_binding_array_elements_per_shader_stage),
        u64::from(limits.max_binding_array_acceleration_structure_elements_per_shader_stage),
        u64::from(limits.max_binding_array_sampler_elements_per_shader_stage),
        limits.max_uniform_buffer_binding_size,
        limits.max_storage_buffer_binding_size,
        u64::from(limits.max_vertex_buffers),
        limits.max_buffer_size,
        u64::from(limits.max_vertex_attributes),
        u64::from(limits.max_vertex_buffer_array_stride),
        u64::from(limits.max_inter_stage_shader_variables),
        u64::from(limits.min_uniform_buffer_offset_alignment),
        u64::from(limits.min_storage_buffer_offset_alignment),
        u64::from(limits.max_color_attachments),
        u64::from(limits.max_color_attachment_bytes_per_sample),
        u64::from(limits.max_compute_workgroup_storage_size),
        u64::from(limits.max_compute_invocations_per_workgroup),
        u64::from(limits.max_compute_workgroup_size_x),
        u64::from(limits.max_compute_workgroup_size_y),
        u64::from(limits.max_compute_workgroup_size_z),
        u64::from(limits.max_compute_workgroups_per_dimension),
        u64::from(limits.max_immediate_size),
        u64::from(limits.max_non_sampler_bindings),
        u64::from(limits.max_task_mesh_workgroup_total_count),
        u64::from(limits.max_task_mesh_workgroups_per_dimension),
        u64::from(limits.max_task_invocations_per_workgroup),
        u64::from(limits.max_task_invocations_per_dimension),
        u64::from(limits.max_mesh_invocations_per_workgroup),
        u64::from(limits.max_mesh_invocations_per_dimension),
        u64::from(limits.max_task_payload_size),
        u64::from(limits.max_mesh_output_vertices),
        u64::from(limits.max_mesh_output_primitives),
        u64::from(limits.max_mesh_output_layers),
        u64::from(limits.max_mesh_multiview_view_count),
        u64::from(limits.max_blas_primitive_count),
        u64::from(limits.max_blas_geometry_count),
        u64::from(limits.max_tlas_instance_count),
        u64::from(limits.max_acceleration_structures_per_shader_stage),
        u64::from(limits.max_multiview_view_count),
    ]
}

fn canonical_limits_digest(limits: &wgpu::Limits) -> [u64; 4] {
    let words = canonical_limit_words_for_test(limits);
    let mut digest = CanonicalDigestBuilder::new(GPU_LIMITS_DIGEST_DOMAIN);
    digest.write_sequence_len(words.len());
    for word in words {
        digest.write_u64(word);
    }
    digest.finish256()
}

fn validate_required_gpu_layout_version(version: u32) -> Result<(), ScaffoldContractError> {
    if version == GPU_CLOSED_LOOP_LAYOUT_VERSION {
        Ok(())
    } else {
        Err(ScaffoldContractError::GpuLayoutMismatch)
    }
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn backend_slug(backend: wgpu::Backend) -> Result<&'static str, ScaffoldContractError> {
    match backend {
        wgpu::Backend::Vulkan => Ok("vulkan"),
        wgpu::Backend::Metal => Ok("metal"),
        wgpu::Backend::Dx12 => Ok("dx12"),
        wgpu::Backend::Gl => Ok("gl"),
        wgpu::Backend::BrowserWebGpu => Ok("webgpu"),
        wgpu::Backend::Noop => Err(ScaffoldContractError::NeuralBackendUnavailable),
    }
}

fn build_hardware_receipt(
    info: &wgpu::AdapterInfo,
    requested_features: wgpu::Features,
    enabled_features: wgpu::Features,
    enabled_limits: &wgpu::Limits,
) -> Result<GpuHardwareReceipt, ScaffoldContractError> {
    // Cargo validates CARGO_PKG_VERSION as SemVer; the receipt additionally
    // enforces its transport bound before consuming a process-local ID.
    if !BACKEND_VERSION.is_ascii() || BACKEND_VERSION.len() > 64 {
        return Err(ScaffoldContractError::NeuralBackendUnavailable);
    }
    let generation = next_hardware_receipt_generation()
        .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
    Ok(GpuHardwareReceipt {
        schema_version: GPU_HARDWARE_RECEIPT_SCHEMA_VERSION,
        generation: generation.get(),
        backend_api: backend_slug(info.backend)?.to_owned(),
        adapter_name: truncate_utf8(&info.name, 256),
        vendor_id: info.vendor,
        device_id: info.device,
        driver_digest: canonical_driver_digest(
            &truncate_utf8(&info.driver, 256),
            &truncate_utf8(&info.driver_info, 256),
        ),
        feature_digest: canonical_feature_digest(requested_features, enabled_features),
        limits_digest: canonical_limits_digest(enabled_limits),
        gpu_layout_version: GPU_CLOSED_LOOP_LAYOUT_VERSION as u16,
        backend_version: BACKEND_VERSION.to_owned(),
    })
}

enum GpuAdapterCandidate {
    Hardware(wgpu::Adapter),
    #[cfg(test)]
    Software,
}

trait GpuDeviceFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError>;

    fn request_device(
        &self,
        adapter: &wgpu::Adapter,
        descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError>;
}

struct WgpuDeviceFactory;

impl GpuDeviceFactory for WgpuDeviceFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
        pollster::block_on(async {
            let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            descriptor.backends = wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY;
            let instance = wgpu::Instance::new(descriptor);
            let required_features = wgpu::Features::empty();
            let required_limits = required_device_limits();
            let mut adapters = instance
                .enumerate_adapters(wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY)
                .await
                .into_iter()
                .filter(|adapter| {
                    let info = adapter.get_info();
                    info.device_type != wgpu::DeviceType::Cpu
                        && info.backend != wgpu::Backend::Noop
                        && backend_slug(info.backend).is_ok()
                        && adapter.features().contains(required_features)
                        && required_limits.check_limits(&adapter.limits())
                })
                .collect::<Vec<_>>();
            adapters.sort_by_key(|adapter| {
                let info = adapter.get_info();
                let backend_rank = match info.backend {
                    wgpu::Backend::Vulkan => 0,
                    wgpu::Backend::Metal => 1,
                    wgpu::Backend::Dx12 => 2,
                    wgpu::Backend::BrowserWebGpu => 3,
                    wgpu::Backend::Gl => 4,
                    wgpu::Backend::Noop => 5,
                };
                let device_rank = match info.device_type {
                    wgpu::DeviceType::DiscreteGpu => 0,
                    wgpu::DeviceType::IntegratedGpu => 1,
                    _ => 2,
                };
                (
                    backend_rank,
                    device_rank,
                    info.vendor,
                    info.device,
                    info.device_pci_bus_id.clone(),
                    info.name.clone(),
                    info.driver.clone(),
                    info.driver_info.clone(),
                )
            });
            if adapters.is_empty() {
                Err(ScaffoldContractError::NeuralBackendUnavailable)
            } else {
                Ok(adapters
                    .into_iter()
                    .map(GpuAdapterCandidate::Hardware)
                    .collect())
            }
        })
    }

    fn request_device(
        &self,
        adapter: &wgpu::Adapter,
        descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
        pollster::block_on(adapter.request_device(descriptor))
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)
    }
}

struct RequiredGpuDevice {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    hardware: GpuHardwareReceipt,
    lost: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GpuBrainSlotOwnership {
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    sensor_profile: SensorProfile,
}

pub(crate) struct ResidentBrainSlot {
    ownership: GpuBrainSlotOwnership,
    pub(crate) phenotype: BrainPhenotype,
    pub(crate) brain_slot: GpuBrainSlot,
    pub(crate) ranges: GpuFixedSlotRanges,
    pub(crate) active_eligibility_bank: u8,
    pub(crate) active_eligibility_generation: u64,
    pub(crate) active_weight_bank: u8,
    pub(crate) active_weight_generation: u64,
    pub(crate) replay_journal_generation: u64,
    pub(crate) transaction_generation: u64,
    pub(crate) logical_dispatch_generation: u64,
    pub(crate) activity_sequence_cursor: u64,
    pub(crate) brain_atp_q16: u32,
    pub(crate) last_world_atp_tick: Option<u64>,
    pub(crate) last_activity_dispatch_generation: u64,
    pub(crate) last_activity_frame_digest: [u64; 4],
    pub(crate) last_completed_gpu_time_ns: u64,
    pub(crate) last_pressure: Option<GpuPressureSample>,
    pub(crate) last_throttle: Option<NeuralThrottleDecision>,
    pub(crate) last_work: Option<BrainWorkReceipt>,
    pub(crate) v11: GpuV11CausalState,
    pub(crate) sleep_plan: alife_core::SleepConsolidationPlan,
    pub(crate) learning_sequence_guard: LearningSequenceGuard,
    pub(crate) pending_eligibility: Option<PendingEligibilityReceipt>,
    pub(crate) pending_eligibility_record: Option<GpuPendingEligibilityRecord>,
}

struct PreparedLearningApply {
    chunk_index: usize,
    handle: GpuBrainHandle,
    packet: OutcomeCreditPacket,
    outcome: GpuOutcomeCreditRecord,
    brain_slot: GpuBrainSlot,
    pending_receipt: PendingEligibilityReceipt,
    pending_record: GpuPendingEligibilityRecord,
    active_weight_generation: u64,
    active_eligibility_generation: u64,
    replay_journal_generation: u64,
    transaction_generation: u64,
    expected_last_committed: Option<alife_core::OutcomeCreditReplayKey>,
    commit_token: LearningCommitToken,
}

pub(crate) struct ClassBucketRuntime {
    pub(crate) plan: GpuFixedClassArenaPlan,
    pub(crate) buffers: GpuFixedClassArenaBuffers,
    pub(crate) pipelines: GpuClosedLoopPipelines,
    pub(crate) slots: Vec<Option<ResidentBrainSlot>>,
    pub(crate) generations: Vec<u32>,
    pub(crate) retired: BTreeSet<u32>,
    pub(crate) free_slots: Vec<u32>,
}

impl ClassBucketRuntime {
    fn from_plan(
        device: &wgpu::Device,
        kernels: Arc<GpuClosedLoopKernelSet>,
        plan: GpuFixedClassArenaPlan,
    ) -> Result<Self, GpuClosedLoopError> {
        let slot_capacity = plan.slot_capacity();
        let buffers = GpuFixedClassArenaBuffers::allocate(device, &plan)?;
        let pipelines = GpuClosedLoopPipelines::from_shared_kernel_set_for_fixed_arena(
            device, &buffers, kernels,
        )?;
        let slot_count =
            usize::try_from(slot_capacity).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        let mut free_slots = (0..slot_capacity).collect::<Vec<_>>();
        free_slots.reverse();
        Ok(Self {
            plan,
            buffers,
            pipelines,
            slots: (0..slot_count).map(|_| None).collect(),
            generations: vec![0; slot_count],
            retired: BTreeSet::new(),
            free_slots,
        })
    }

    pub(crate) fn contains(&self, handle: GpuBrainHandle) -> bool {
        self.slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .is_some_and(|resident| {
                resident.brain_slot.record().slot_generation == handle.generation
                    && resident.ownership.organism_id == handle.organism_id
                    && resident.ownership.phenotype_hash == handle.phenotype_hash
            })
    }
}

#[derive(Default)]
pub(crate) struct ClassBucketPool {
    pub(crate) chunks: Vec<ClassBucketRuntime>,
}

impl ClassBucketPool {
    pub(crate) fn bucket_index_for_handle(&self, handle: GpuBrainHandle) -> Option<usize> {
        self.chunks
            .iter()
            .position(|bucket| bucket.contains(handle))
    }

    pub(crate) fn bucket_for_handle(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<&ClassBucketRuntime, ScaffoldContractError> {
        self.bucket_index_for_handle(handle)
            .and_then(|index| self.chunks.get(index))
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn bucket_for_handle_mut(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<&mut ClassBucketRuntime, ScaffoldContractError> {
        let index = self
            .bucket_index_for_handle(handle)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        self.chunks
            .get_mut(index)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn resident(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<&ResidentBrainSlot, ScaffoldContractError> {
        self.bucket_for_handle(handle)?
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn resident_mut(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<&mut ResidentBrainSlot, ScaffoldContractError> {
        self.bucket_for_handle_mut(handle)?
            .slots
            .get_mut(handle.slot as usize)
            .and_then(Option::as_mut)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    fn reusable_slot(
        &self,
        class_raw: u16,
        watermarks: &BTreeMap<(u16, u32), u32>,
    ) -> Option<(usize, u32, u32)> {
        self.chunks
            .iter()
            .enumerate()
            .find_map(|(chunk_index, bucket)| {
                let slot = *bucket.free_slots.last()?;
                let generation = watermarks
                    .get(&(class_raw, slot))
                    .copied()
                    .unwrap_or(0)
                    .checked_add(1)?;
                Some((chunk_index, slot, generation))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CuratedResidencySlotState {
    class_id: BrainClassId,
    chunk_index: usize,
    slot: u32,
    generation_watermark: u32,
    reserved_generation: u32,
    occupied: bool,
    retired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CuratedResidencyPortSnapshot {
    backend_instance_id: NonZeroU64,
    generation: u64,
    admission_generation: u64,
    live_brains: u32,
    max_hot_brains: u32,
    logical_committed_bytes: u64,
    logical_budget_bytes: u64,
    logical_slot_commit_bytes: u64,
    physical_allocated_bytes: u64,
    transient_new_physical_bytes: u64,
    physical_ceiling_bytes: u64,
    slots: Vec<CuratedResidencySlotState>,
    old_residents: Vec<GpuCuratedResidentReceipt>,
    backend_hardware_generation: u64,
}

trait CuratedResidencyTransactionPort {
    type StagedEntry;

    fn classify_pre_submit(&mut self, error: ScaffoldContractError) -> GpuCuratedResidencyOutcome {
        curated_residency_pre_submit(error)
    }
    fn snapshot(&mut self) -> Result<CuratedResidencyPortSnapshot, ScaffoldContractError>;
    fn prepare_entry(
        &mut self,
        entry_index: usize,
        entry: &GpuCuratedResidencyEntry,
        reservation: CuratedResidencySlotState,
    ) -> Result<Self::StagedEntry, ScaffoldContractError>;
    fn record_old_slot_scrub(
        &mut self,
        resident: &GpuCuratedResidentReceipt,
    ) -> Result<(), ScaffoldContractError>;
    fn record_new_slot_initialization(
        &mut self,
        staged: &Self::StagedEntry,
    ) -> Result<(), ScaffoldContractError>;
    fn submit_once(&mut self) -> Result<(), ScaffoldContractError>;
    fn poll_completion(&mut self) -> Result<(), ScaffoldContractError>;
    fn commit(
        &mut self,
        cohort: &GpuCuratedResidencyCohort,
        reservations: &[CuratedResidencySlotState],
        staged: Vec<Self::StagedEntry>,
        receipt: &GpuCuratedResidencyReceipt,
    ) -> Result<(), ScaffoldContractError>;
    fn mark_unknown(&mut self);
}

fn curated_residency_pre_submit(error: ScaffoldContractError) -> GpuCuratedResidencyOutcome {
    GpuCuratedResidencyOutcome::PreSubmitFailure {
        error,
        retryable: true,
    }
}

fn curated_residency_unknown<P: CuratedResidencyTransactionPort>(
    port: &mut P,
    error: ScaffoldContractError,
) -> GpuCuratedResidencyOutcome {
    port.mark_unknown();
    GpuCuratedResidencyOutcome::Unknown {
        error,
        fail_stop: true,
    }
}

fn validate_curated_residency_cohort(
    cohort: &GpuCuratedResidencyCohort,
    snapshot: &CuratedResidencyPortSnapshot,
) -> Result<BrainClassId, ScaffoldContractError> {
    if cohort.ordered_entries.is_empty()
        || cohort.new_generation_fingerprint == [0; 4]
        || cohort.expected_old_generation != snapshot.generation
    {
        return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
    }
    let first = cohort
        .ordered_entries
        .first()
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    let class_id = first.phenotype.brain_class_id();
    if class_id != BrainCapacityClass::N512_ID {
        return Err(ScaffoldContractError::UnsupportedProductionBrainClass);
    }
    let capacity = BrainCapacityClass::production_for_id(class_id)?;
    validate_required_gpu_layout_version(u32::from(capacity.execution().gpu_layout_version()))?;
    first
        .phenotype
        .validate_against(&capacity)
        .map_err(|_| ScaffoldContractError::GpuLayoutMismatch)?;
    if first.organism_id.validate().is_err()
        || first.opaque_target_identity.raw() == 0
        || first.exact_phenotype_hash != first.phenotype.phenotype_hash()
        || first.exact_foundation_hash.bytes() == &[0; 32]
        || first.phenotype.foundation_abi().foundation_payload_digest()
            != Some(first.exact_foundation_hash)
    {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    let first_profile = first.phenotype.sensor_profile();
    let first_foundation = first.exact_foundation_hash;
    let mut organism_ids = BTreeSet::new();
    let mut target_ids = BTreeSet::new();
    for entry in &cohort.ordered_entries {
        let capacity = BrainCapacityClass::production_for_id(entry.phenotype.brain_class_id())?;
        validate_required_gpu_layout_version(u32::from(capacity.execution().gpu_layout_version()))?;
        entry
            .phenotype
            .validate_against(&capacity)
            .map_err(|_| ScaffoldContractError::GpuLayoutMismatch)?;
        if entry.phenotype.brain_class_id() != class_id
            || entry.phenotype.sensor_profile() != first_profile
            || entry.exact_foundation_hash != first_foundation
            || entry.organism_id.validate().is_err()
            || entry.opaque_target_identity.raw() == 0
            || !organism_ids.insert(entry.organism_id.raw())
            || !target_ids.insert(entry.opaque_target_identity)
            || entry.exact_phenotype_hash != entry.phenotype.phenotype_hash()
            || entry.exact_foundation_hash.bytes() == &[0; 32]
            || entry.phenotype.foundation_abi().foundation_payload_digest()
                != Some(entry.exact_foundation_hash)
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
    }
    Ok(class_id)
}

fn run_curated_residency_transaction<P: CuratedResidencyTransactionPort>(
    port: &mut P,
    cohort: &GpuCuratedResidencyCohort,
) -> GpuCuratedResidencyOutcome {
    let snapshot = match port.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => return port.classify_pre_submit(error),
    };
    let class_id = match validate_curated_residency_cohort(cohort, &snapshot) {
        Ok(class_id) => class_id,
        Err(error) => return port.classify_pre_submit(error),
    };
    let entry_count = match u32::try_from(cohort.ordered_entries.len()) {
        Ok(count) => count,
        Err(_) => {
            return port.classify_pre_submit(ScaffoldContractError::NeuralBackendUnavailable);
        }
    };
    if snapshot
        .live_brains
        .checked_add(entry_count)
        .is_none_or(|live| live > snapshot.max_hot_brains)
        || snapshot
            .logical_committed_bytes
            .checked_add(
                snapshot
                    .logical_slot_commit_bytes
                    .checked_mul(u64::from(entry_count))
                    .unwrap_or(u64::MAX),
            )
            .is_none_or(|bytes| bytes > snapshot.logical_budget_bytes)
        || snapshot
            .physical_allocated_bytes
            .checked_add(snapshot.transient_new_physical_bytes)
            .is_none_or(|bytes| bytes > snapshot.physical_ceiling_bytes)
    {
        return port.classify_pre_submit(ScaffoldContractError::NeuralBackendUnavailable);
    }

    let mut reservations = Vec::with_capacity(cohort.ordered_entries.len());
    for slot in snapshot.slots.iter().copied() {
        if slot.class_id != class_id || slot.occupied || slot.retired {
            continue;
        }
        let generation = match slot.generation_watermark.checked_add(1) {
            Some(generation) if generation != 0 => generation,
            _ => {
                return port.classify_pre_submit(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        reservations.push(CuratedResidencySlotState {
            reserved_generation: generation,
            ..slot
        });
        if reservations.len() == cohort.ordered_entries.len() {
            break;
        }
    }
    if reservations.len() != cohort.ordered_entries.len() {
        return port.classify_pre_submit(ScaffoldContractError::NeuralBackendUnavailable);
    }

    let mut staged = Vec::with_capacity(cohort.ordered_entries.len());
    for (entry_index, (entry, reservation)) in cohort
        .ordered_entries
        .iter()
        .zip(reservations.iter().copied())
        .enumerate()
    {
        match port.prepare_entry(entry_index, entry, reservation) {
            Ok(prepared) => staged.push(prepared),
            Err(error) => return port.classify_pre_submit(error),
        }
    }
    for resident in &snapshot.old_residents {
        if let Err(error) = port.record_old_slot_scrub(resident) {
            return port.classify_pre_submit(error);
        }
    }
    for prepared in &staged {
        if let Err(error) = port.record_new_slot_initialization(prepared) {
            return port.classify_pre_submit(error);
        }
    }
    if let Err(error) = port.submit_once() {
        return curated_residency_unknown(port, error);
    }
    if let Err(error) = port.poll_completion() {
        return curated_residency_unknown(port, error);
    }

    let ordered_residents = cohort
        .ordered_entries
        .iter()
        .zip(reservations.iter())
        .map(|(entry, reservation)| GpuCuratedResidentReceipt {
            organism_id: entry.organism_id,
            opaque_target_identity: entry.opaque_target_identity,
            exact_phenotype_hash: entry.exact_phenotype_hash,
            exact_foundation_hash: entry.exact_foundation_hash,
            handle: GpuBrainHandle {
                backend_instance_id: snapshot.backend_instance_id,
                class_id,
                slot: reservation.slot,
                generation: reservation.reserved_generation,
                organism_id: entry.organism_id,
                phenotype_hash: entry.exact_phenotype_hash,
            },
        })
        .collect::<Vec<_>>();
    let receipt = GpuCuratedResidencyReceipt {
        generation_fingerprint: cohort.new_generation_fingerprint,
        ordered_residents,
        submission_completed: true,
        backend_hardware_generation: snapshot.backend_hardware_generation,
    };
    if let Err(error) = port.commit(cohort, &reservations, staged, &receipt) {
        return curated_residency_unknown(port, error);
    }
    GpuCuratedResidencyOutcome::Committed(receipt)
}

struct PreparedClassDispatch {
    class_id: u16,
    chunk_index: usize,
    original_indices: Vec<usize>,
    prepared: Option<GpuPreparedActiveBatch>,
    batch: Option<GpuActiveBatchUpload>,
    recorded: bool,
    map_ticket: Option<GpuCompactMapTicket>,
    selector_readback: Option<wgpu::Buffer>,
    selector_map_ticket: Option<GpuCompactMapTicket>,
    selector_captures: Option<Vec<GpuSelectorLogitCapture>>,
    validated: Option<GpuValidatedClassBatch>,
}

#[derive(Default)]
struct SelectorDiagnosticErrorCapture {
    enable_error: Option<GpuRuntimeSelectorDiagnosticEnableFailure>,
    later_stage_receipt: Option<GpuRuntimeSelectorDiagnosticFailureReceipt>,
    decode_mapped_records_receipt:
        Option<GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt>,
    build_selector_diagnostic_receipt: Option<GpuRuntimeSelectorDiagnosticBuildFailureReceipt>,
    enable_completed: bool,
    later_stage: Option<GpuRuntimeSelectorDiagnosticStage>,
}

impl GpuSelectorDiagnosticReceipt {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION
            || self.frame_digest.0 == [0; 4]
            || self.phenotype_hash.0 == [0; 4]
            || self.dispatch_generation == 0
            || self.policy != GpuSelectorPolicyIdentity::PRODUCTION_V1
            || self.requested_candidate_indices.is_empty()
            || self
                .requested_candidate_indices
                .windows(2)
                .any(|window| window[0] >= window[1])
            || self
                .requested_candidate_indices
                .iter()
                .any(|index| usize::from(*index) >= self.candidates.len())
            || self.candidates.is_empty()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for (index, candidate) in self.candidates.iter().enumerate() {
            if usize::from(candidate.candidate_index) != index
                || !candidate.decoder_family_bias.is_finite()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            let requested = self
                .requested_candidate_indices
                .binary_search(&candidate.candidate_index)
                .is_ok();
            match candidate.validity {
                GpuSelectorCandidateValidity::Valid => {
                    let (Some(pre), Some(delta), Some(final_logit)) = (
                        candidate.pre_context_logit,
                        candidate.memory_context_delta,
                        candidate.final_logit,
                    ) else {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    };
                    if !pre.is_finite()
                        || !delta.is_finite()
                        || !final_logit.is_finite()
                        || (final_logit - pre).to_bits() != delta.to_bits()
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    if !requested {
                        if candidate.binding.is_some() || !candidate.contributions.is_empty() {
                            return Err(ScaffoldContractError::InvalidDecisionEvidence);
                        }
                        continue;
                    }
                    let binding = candidate
                        .binding
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    if binding.activation_side > 1
                        || candidate.contributions.len()
                            != usize::try_from(binding.weight_index_count)
                                .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    let mut reconstructed = candidate.decoder_family_bias;
                    for (index, contribution) in candidate.contributions.iter().enumerate() {
                        if contribution.synapse_index
                            != u32::try_from(index)
                                .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
                            || [
                                contribution.motor,
                                contribution.feature,
                                contribution.genetic,
                                contribution.lifetime,
                                contribution.alpha,
                                contribution.fast,
                                contribution.effective_weight,
                                contribution.signed_contribution,
                                contribution.running_logit,
                            ]
                            .iter()
                            .any(|value| !value.is_finite())
                        {
                            return Err(ScaffoldContractError::InvalidDecisionEvidence);
                        }
                        let expected_effective = contribution.genetic
                            + contribution.lifetime
                            + contribution.alpha * contribution.fast;
                        let expected_contribution =
                            contribution.motor * contribution.feature * expected_effective;
                        if !selector_receipt_close(
                            expected_effective,
                            contribution.effective_weight,
                        ) || !selector_receipt_close(
                            expected_contribution,
                            contribution.signed_contribution,
                        ) {
                            return Err(ScaffoldContractError::InvalidDecisionEvidence);
                        }
                        reconstructed += contribution.signed_contribution;
                        if !selector_receipt_close(reconstructed, contribution.running_logit) {
                            return Err(ScaffoldContractError::InvalidDecisionEvidence);
                        }
                    }
                    if !selector_receipt_close(reconstructed, pre) {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                }
                GpuSelectorCandidateValidity::InvalidLogit => {
                    if candidate.pre_context_logit.is_some()
                        || candidate.memory_context_delta.is_some()
                        || candidate.final_logit.is_some()
                        || candidate.binding.is_some()
                        || !candidate.contributions.is_empty()
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                }
            }
        }
        let mut argmax = None::<(u16, f32)>;
        let mut equal_max = Vec::new();
        for candidate in &self.candidates {
            let Some(logit) = candidate.final_logit else {
                continue;
            };
            match argmax {
                None => {
                    argmax = Some((candidate.candidate_index, logit));
                    equal_max.push(candidate.candidate_index);
                }
                Some((_, maximum)) if logit > maximum => {
                    argmax = Some((candidate.candidate_index, logit));
                    equal_max.clear();
                    equal_max.push(candidate.candidate_index);
                }
                Some((_, maximum)) if logit == maximum => {
                    equal_max.push(candidate.candidate_index);
                }
                _ => {}
            }
        }
        let (argmax_index, _) = argmax.ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if self.argmax_candidate_index != argmax_index
            || self.equal_max_candidate_indices != equal_max
            || self.chosen_candidate_index != argmax_index
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn selector_receipt_close(left: f32, right: f32) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-4 * scale
}

fn selector_logit(bits: u32) -> Option<f32> {
    let value = f32::from_bits(bits);
    (bits != GPU_SELECTOR_INVALID_LOGIT_BITS && value.is_finite()).then_some(value)
}

fn selector_diagnostic_family_binding_offsets(
    recurrent_synapse_count: u32,
    decoder_weight_indices_offset: u32,
    decoder_synapse_start: u32,
) -> Result<(u32, u32), ScaffoldContractError> {
    let local_decoder_synapse_start = decoder_synapse_start
        .checked_sub(recurrent_synapse_count)
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    let weight_index_start = decoder_weight_indices_offset
        .checked_add(
            local_decoder_synapse_start
                .checked_mul(4)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?,
        )
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    Ok((decoder_synapse_start, weight_index_start))
}

fn selector_detail_word(
    words: &[u32],
    base: usize,
    offset: usize,
) -> Result<u32, ScaffoldContractError> {
    words
        .get(
            base.checked_add(offset)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?,
        )
        .copied()
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)
}

fn selector_detail_f32(
    words: &[u32],
    base: usize,
    offset: usize,
) -> Result<f32, ScaffoldContractError> {
    Ok(f32::from_bits(selector_detail_word(words, base, offset)?))
}

fn build_selector_diagnostic(
    frame: &PerceptionFrame,
    phenotype: &BrainPhenotype,
    slot: &GpuBrainSlot,
    active_weight_bank: u8,
    dispatch_generation: u64,
    chosen_candidate_index: u16,
    capture: &GpuSelectorLogitCapture,
    failure_field: &mut GpuRuntimeSelectorDiagnosticBuildFailureField,
    binding_identity_failure: &mut Option<(GpuSelectorBindingIdentity, GpuSelectorBindingIdentity)>,
) -> Result<GpuSelectorDiagnosticReceipt, ScaffoldContractError> {
    *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::CandidateLogitShape;
    if capture.pre_context_logit_bits.len() != frame.candidates().len()
        || capture.final_logit_bits.len() != frame.candidates().len()
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::RequestedContributionShape;
    if capture.requested_candidate_indices.is_empty()
        || capture.contributions.len() != capture.requested_candidate_indices.len()
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let lifetime_weight_offset = if active_weight_bank == 0 {
        slot.record().lifetime_weight_offset
    } else {
        slot.word_ranges().lifetime_weight_bank_1_words.start
    };
    let fast_weight_offset = if active_weight_bank == 0 {
        slot.record().fast_weight_offset
    } else {
        slot.word_ranges().fast_weight_bank_1_words.start
    };
    let candidates = frame
        .candidates()
        .iter()
        .zip(&capture.pre_context_logit_bits)
        .zip(&capture.final_logit_bits)
        .map(|((candidate, pre_bits), final_bits)| {
            *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::DecoderFamily;
            let family_bias = phenotype
                .candidate_decoder()
                .families()
                .iter()
                .find(|family| family.family() == candidate.family)
                .map(|family| family.bias())
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            let family_plan = phenotype
                .candidate_decoder()
                .families()
                .iter()
                .find(|family| family.family() == candidate.family)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            let contribution_capture = capture
                .contributions
                .iter()
                .find(|detail| detail.candidate_index == candidate.candidate_index);
            let pre = selector_logit(*pre_bits);
            let final_logit = selector_logit(*final_bits);
            let (validity, pre_context_logit, memory_context_delta, final_logit) =
                match (pre, final_logit) {
                    (Some(pre), Some(final_logit)) => (
                        GpuSelectorCandidateValidity::Valid,
                        Some(pre),
                        Some(final_logit - pre),
                        Some(final_logit),
                    ),
                    _ => (GpuSelectorCandidateValidity::InvalidLogit, None, None, None),
                };
            let (binding, contributions) = if validity == GpuSelectorCandidateValidity::Valid
                && contribution_capture.is_some()
            {
                *failure_field =
                    GpuRuntimeSelectorDiagnosticBuildFailureField::ContributionDetailWord;
                let words = &contribution_capture
                    .expect("checked sparse contribution capture")
                    .synapse_words;
                let first = 0;
                let family_start = selector_detail_word(words, first, 19)?;
                let family_count = selector_detail_word(words, first, 20)?;
                let binding = GpuSelectorBindingIdentity {
                    decoder_plan_offset: selector_detail_word(words, first, 18)?,
                    decoder_family_offset: selector_detail_word(words, first, 28)?,
                    decoder_family_start: family_start,
                    decoder_family_count: family_count,
                    weight_index_start: selector_detail_word(words, first, 21)?,
                    weight_index_count: selector_detail_word(words, first, 22)?,
                    activation_side: u8::try_from(selector_detail_word(words, first, 14)?)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
                    activation_offset: selector_detail_word(words, first, 15)?,
                    motor_start: selector_detail_word(words, first, 16)?,
                    feature_offset: selector_detail_word(words, first, 17)?,
                    genetic_weight_offset: selector_detail_word(words, first, 23)?,
                    alpha_offset: selector_detail_word(words, first, 24)?,
                    lifetime_weight_offset: selector_detail_word(words, first, 25)?,
                    fast_weight_offset: selector_detail_word(words, first, 26)?,
                };
                *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::FamilyStart;
                let (expected_family_start, expected_weight_index_start) =
                    selector_diagnostic_family_binding_offsets(
                        slot.record().recurrent_synapse_count,
                        slot.record().decoder_weight_indices_offset,
                        family_plan.decoder_synapse_start(),
                    )?;
                *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::WeightIndexStart;
                let expected_activation_offset = if binding.activation_side == 0 {
                    slot.record().activation_a_offset
                } else {
                    slot.record().activation_b_offset
                };
                let expected_binding = GpuSelectorBindingIdentity {
                    decoder_plan_offset: slot.record().decoder_plan_offset,
                    decoder_family_offset: slot.record().decoder_family_offset,
                    decoder_family_start: expected_family_start,
                    decoder_family_count: family_plan.decoder_synapse_count(),
                    weight_index_start: expected_weight_index_start,
                    weight_index_count: family_plan.decoder_synapse_count(),
                    activation_side: binding.activation_side,
                    activation_offset: expected_activation_offset,
                    motor_start: phenotype.candidate_decoder().motor_start(),
                    feature_offset: binding.feature_offset,
                    genetic_weight_offset: slot.record().genetic_weight_offset,
                    alpha_offset: slot.record().alpha_offset,
                    lifetime_weight_offset,
                    fast_weight_offset,
                };
                *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::BindingIdentity;
                if binding.decoder_plan_offset != slot.record().decoder_plan_offset
                    || binding.decoder_family_offset != slot.record().decoder_family_offset
                    || binding.decoder_family_start != expected_family_start
                    || binding.decoder_family_count != family_plan.decoder_synapse_count()
                    || binding.weight_index_start != expected_weight_index_start
                    || binding.weight_index_count != family_plan.decoder_synapse_count()
                    || binding.activation_offset != expected_activation_offset
                    || binding.motor_start != phenotype.candidate_decoder().motor_start()
                    || binding.genetic_weight_offset != slot.record().genetic_weight_offset
                    || binding.alpha_offset != slot.record().alpha_offset
                    || binding.lifetime_weight_offset != lifetime_weight_offset
                    || binding.fast_weight_offset != fast_weight_offset
                {
                    *binding_identity_failure = Some((expected_binding, binding));
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::FamilyCount;
                let family_count_usize = usize::try_from(family_count)
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                let mut contributions = Vec::with_capacity(family_count_usize);
                for synapse_index in 0..family_count_usize {
                    *failure_field =
                        GpuRuntimeSelectorDiagnosticBuildFailureField::ContributionRecordIdentity;
                    let base = synapse_index
                        .checked_mul(GPU_SELECTOR_DIAGNOSTIC_RECORD_WORDS)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    if selector_detail_word(words, base, 0)? != u32::from(candidate.candidate_index)
                        || selector_detail_word(words, base, 1)?
                            != u32::try_from(synapse_index)
                                .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
                        || selector_detail_word(words, base, 14)?
                            != u32::from(binding.activation_side)
                        || selector_detail_word(words, base, 15)? != binding.activation_offset
                        || selector_detail_word(words, base, 16)? != binding.motor_start
                        || selector_detail_word(words, base, 17)? != binding.feature_offset
                        || selector_detail_word(words, base, 18)? != binding.decoder_plan_offset
                        || selector_detail_word(words, base, 19)? != binding.decoder_family_start
                        || selector_detail_word(words, base, 20)? != binding.decoder_family_count
                        || selector_detail_word(words, base, 21)? != binding.weight_index_start
                        || selector_detail_word(words, base, 22)? != binding.weight_index_count
                        || selector_detail_word(words, base, 23)? != binding.genetic_weight_offset
                        || selector_detail_word(words, base, 24)? != binding.alpha_offset
                        || selector_detail_word(words, base, 25)? != binding.lifetime_weight_offset
                        || selector_detail_word(words, base, 26)? != binding.fast_weight_offset
                        || selector_detail_word(words, base, 27)?
                            != u32::from(candidate.family.raw())
                        || selector_detail_word(words, base, 28)? != binding.decoder_family_offset
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::InputLane;
                    let input_lane = u16::try_from(selector_detail_word(words, base, 3)?)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::MotorIndex;
                    let motor_index = u16::try_from(selector_detail_word(words, base, 4)?)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    *failure_field =
                        GpuRuntimeSelectorDiagnosticBuildFailureField::ContributionDetailWord;
                    contributions.push(GpuSelectorSynapseContribution {
                        synapse_index: selector_detail_word(words, base, 1)?,
                        global_synapse_id: selector_detail_word(words, base, 2)?,
                        input_lane,
                        motor_index,
                        motor: selector_detail_f32(words, base, 5)?,
                        feature: selector_detail_f32(words, base, 6)?,
                        genetic: selector_detail_f32(words, base, 7)?,
                        lifetime: selector_detail_f32(words, base, 8)?,
                        alpha: selector_detail_f32(words, base, 9)?,
                        fast: selector_detail_f32(words, base, 10)?,
                        effective_weight: selector_detail_f32(words, base, 11)?,
                        signed_contribution: selector_detail_f32(words, base, 12)?,
                        running_logit: selector_detail_f32(words, base, 13)?,
                    });
                }
                (Some(binding), contributions)
            } else {
                (None, Vec::new())
            };
            Ok(GpuSelectorCandidateDiagnostic {
                candidate_index: candidate.candidate_index,
                action_id: candidate.action_id,
                family: candidate.family,
                target: candidate.target,
                validity,
                decoder_family_bias: family_bias,
                binding,
                contributions,
                pre_context_logit,
                memory_context_delta,
                final_logit,
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::Argmax;
    let mut argmax = None::<(u16, f32)>;
    let mut equal_max_candidate_indices = Vec::new();
    for candidate in &candidates {
        let Some(logit) = candidate.final_logit else {
            continue;
        };
        match argmax {
            None => {
                argmax = Some((candidate.candidate_index, logit));
                equal_max_candidate_indices.push(candidate.candidate_index);
            }
            Some((_, maximum)) if logit > maximum => {
                argmax = Some((candidate.candidate_index, logit));
                equal_max_candidate_indices.clear();
                equal_max_candidate_indices.push(candidate.candidate_index);
            }
            Some((_, maximum)) if logit == maximum => {
                equal_max_candidate_indices.push(candidate.candidate_index);
            }
            _ => {}
        }
    }
    let argmax_candidate_index = argmax
        .map(|(index, _)| index)
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    let receipt = GpuSelectorDiagnosticReceipt {
        schema_version: GPU_SELECTOR_DIAGNOSTIC_SCHEMA_VERSION,
        frame_digest: frame.frame_digest(),
        phenotype_hash: phenotype.phenotype_hash(),
        dispatch_generation,
        policy: GpuSelectorPolicyIdentity::PRODUCTION_V1,
        requested_candidate_indices: capture.requested_candidate_indices.clone(),
        candidates,
        argmax_candidate_index,
        equal_max_candidate_indices,
        chosen_candidate_index,
    };
    *failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::ReceiptContract;
    receipt.validate_contract()?;
    Ok(receipt)
}

fn capacity_for_promoted_class(
    class_id: BrainClassId,
) -> Result<BrainCapacityClass, ScaffoldContractError> {
    BrainCapacityClass::production_for_id(class_id)
}

fn capacity_for_gpu_class(
    class_id: BrainClassId,
) -> Result<BrainCapacityClass, ScaffoldContractError> {
    BrainCapacityClass::supported_for_id(class_id)
}

fn live_pressure_sample(
    policy: &BrainActivityPolicyV1,
    identity: BrainDispatchIdentity,
    resident: &ResidentBrainSlot,
    admission: &GpuAdmissionReceipt,
    runtime_budget: &GpuRuntimeBudget,
) -> Result<GpuPressureSample, ScaffoldContractError> {
    GpuPressureSample::try_new(
        policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: resident.last_activity_dispatch_generation,
            source_frame_digest: resident.last_activity_frame_digest,
            completed_gpu_time_ns: resident.last_completed_gpu_time_ns,
            // The runtime waits for the submitted mixed-class batch before it
            // accepts another neural dispatch, so no older neural work is queued.
            queue_depth: 0,
            logical_heap_used: admission.logical_committed_bytes,
            logical_heap_capacity: runtime_budget.logical_neural_heap_budget_bytes,
            brain_atp_remaining_q16: resident.brain_atp_q16,
            brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
        },
    )
}

pub(crate) fn map_gpu_contract_error(error: GpuClosedLoopError) -> ScaffoldContractError {
    match error {
        GpuClosedLoopError::LayoutMismatch => ScaffoldContractError::GpuLayoutMismatch,
        GpuClosedLoopError::StaleOrForeignHandle => ScaffoldContractError::BrainOwnershipMismatch,
        GpuClosedLoopError::MalformedUpload
        | GpuClosedLoopError::NonFinitePayload
        | GpuClosedLoopError::InvalidOffsetDomain => ScaffoldContractError::InvalidPerceptionFrame,
        GpuClosedLoopError::CapacityExceeded
        | GpuClosedLoopError::ArithmeticOverflow
        | GpuClosedLoopError::SubmissionFailed => ScaffoldContractError::NeuralBackendUnavailable,
    }
}

fn translate_selector_diagnostic_enable_error(
    error: PipelineSelectorDiagnosticEnableError,
) -> GpuRuntimeSelectorDiagnosticEnableFailure {
    match error {
        PipelineSelectorDiagnosticEnableError::Contract(error) => {
            GpuRuntimeSelectorDiagnosticEnableFailure::Contract(error)
        }
        PipelineSelectorDiagnosticEnableError::Receipt(receipt) => {
            GpuRuntimeSelectorDiagnosticEnableFailure::Receipt(receipt.into())
        }
    }
}

fn compile_v11_slot_upload(
    plan: &GpuFixedClassArenaPlan,
    slot: &GpuBrainSlot,
    phenotype: &BrainPhenotype,
    state: &GpuV11CausalState,
) -> Result<GpuFixedSlotUpload, GpuClosedLoopError> {
    let mut upload =
        plan.prepare_slot_upload(slot.record().slot, slot.record().slot_generation, phenotype)?;
    upload = upload.with_dendritic_branches(state.dendritic_branches())?;
    for span in state.sparse_spans() {
        for edge in &span.edges {
            upload = upload.with_added_lifetime_synapse(&AddLifetimeSynapse {
                source: edge.source,
                target: edge.target,
                route: edge.route,
                initial_weight: edge.weight,
                evidence: CoactivationEvidence {
                    region: u16::try_from(edge.route)
                        .map_err(|_| GpuClosedLoopError::MalformedUpload)?,
                    source: edge.source,
                    target: edge.target,
                    coactivation: 1,
                    eligibility: 0,
                    concept_gap_support: 0,
                },
            })?;
        }
    }
    Ok(upload)
}

pub struct GpuClosedLoopBackend {
    backend_instance_id: NonZeroU64,
    pub(crate) hardware: GpuHardwareReceipt,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    timestamp_resources: GpuTimestampResources,
    plasticity_timestamp_resources: GpuTimestampResources,
    pub(crate) device_lost: Arc<AtomicBool>,
    kernels: Arc<GpuClosedLoopKernelSet>,
    pub(crate) state: GpuBackendState,
    runtime_profile: GpuRuntimeProfile,
    runtime_budget: GpuRuntimeBudget,
    activity_policy: BrainActivityPolicyV1,
    admission: GpuAdmissionReceipt,
    pub(crate) class_buckets: BTreeMap<u16, ClassBucketPool>,
    slot_generation_watermarks: BTreeMap<(u16, u32), u32>,
    organisms: BTreeMap<u64, GpuBrainHandle>,
    curated_residency_generation: u64,
    curated_residency_generation_fingerprint: [u64; 4],
    pub(crate) next_dispatch_generation: u64,
    force_device_lost_after_submit: bool,
    #[cfg(feature = "gpu-tests")]
    forced_learning_rejections_remaining: u8,
    #[cfg(feature = "gpu-tests")]
    forced_discard_rejections_remaining: u8,
    recorded_pressure_replay: VecDeque<GpuPressureSample>,
    completed_dispatch_count: u64,
    perception_upload_count: u64,
    completed_selection_count: u64,
    last_compact_readback_bytes: usize,
    pending_inference_timing: Option<PendingInferenceTiming>,
    completed_neural_timing: Option<GpuNeuralTimingSample>,
    last_apply_fast_plasticity_failure: Option<GpuRuntimeApplyFastPlasticityFailureReceipt>,
    pub(crate) next_sleep_job_id: u64,
    pub(crate) sleep_jobs: BTreeMap<u64, crate::GpuSleepJobState>,
    pub(crate) committed_sleep: BTreeMap<(u16, u32, u32, u64), crate::GpuSleepConsolidationReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EphemeralBackendStatePlan {
    backend_instance_id: NonZeroU64,
    state: GpuBackendState,
    next_dispatch_generation: u64,
    next_sleep_job_id: u64,
    curated_residency_generation: u64,
    #[cfg(test)]
    recorded_pressure_replay_empty: bool,
}

fn new_ephemeral_backend_state_plan() -> Result<EphemeralBackendStatePlan, ScaffoldContractError> {
    let backend_instance_id =
        next_backend_instance_id().map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
    Ok(EphemeralBackendStatePlan {
        backend_instance_id,
        state: GpuBackendState::Ready,
        next_dispatch_generation: 1,
        next_sleep_job_id: 1,
        curated_residency_generation: 0,
        #[cfg(test)]
        recorded_pressure_replay_empty: true,
    })
}

impl GpuClosedLoopBackend {
    pub fn new_required(profile: GpuRuntimeProfile) -> Result<Self, ScaffoldContractError> {
        Self::new_with_factory_and_profile(&WgpuDeviceFactory, profile)
    }

    #[cfg(test)]
    fn new_with_factory(factory: &impl GpuDeviceFactory) -> Result<Self, ScaffoldContractError> {
        Self::new_with_factory_and_profile(factory, GpuRuntimeProfile::production_v1())
    }

    fn new_with_factory_and_profile(
        factory: &impl GpuDeviceFactory,
        profile: GpuRuntimeProfile,
    ) -> Result<Self, ScaffoldContractError> {
        profile.validate_contract()?;
        let required = acquire_required_gpu(factory)?;
        let backend_instance_id = next_backend_instance_id()
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
        let runtime_budget = GpuRuntimeBudget::from_device(
            profile,
            required.device.features(),
            &required.device.limits(),
            required.hardware.limits_digest,
        )?;
        let admission = GpuAdmissionReceipt::empty(runtime_budget);
        let activity_policy = BrainActivityPolicyV1::production_v1();
        activity_policy.validate_contract()?;
        let timestamp_resources = GpuTimestampResources::new(&required.device, &required.queue)?;
        let plasticity_timestamp_resources =
            GpuTimestampResources::new(&required.device, &required.queue)?;
        let kernels =
            GpuClosedLoopKernelSet::new(&required.device).map_err(map_gpu_contract_error)?;
        Ok(Self {
            backend_instance_id,
            hardware: required.hardware,
            adapter: required.adapter,
            device: required.device,
            queue: required.queue,
            timestamp_resources,
            plasticity_timestamp_resources,
            device_lost: required.lost,
            kernels,
            state: GpuBackendState::Ready,
            runtime_profile: profile,
            runtime_budget,
            activity_policy,
            admission,
            class_buckets: BTreeMap::new(),
            slot_generation_watermarks: BTreeMap::new(),
            organisms: BTreeMap::new(),
            curated_residency_generation: 0,
            curated_residency_generation_fingerprint: [0; 4],
            next_dispatch_generation: 1,
            force_device_lost_after_submit: false,
            #[cfg(feature = "gpu-tests")]
            forced_learning_rejections_remaining: 0,
            #[cfg(feature = "gpu-tests")]
            forced_discard_rejections_remaining: 0,
            recorded_pressure_replay: VecDeque::new(),
            completed_dispatch_count: 0,
            perception_upload_count: 0,
            completed_selection_count: 0,
            last_compact_readback_bytes: 0,
            pending_inference_timing: None,
            completed_neural_timing: None,
            last_apply_fast_plasticity_failure: None,
            next_sleep_job_id: 1,
            sleep_jobs: BTreeMap::new(),
            committed_sleep: BTreeMap::new(),
        })
    }

    /// Creates a fresh restore target on this backend's exact GPU context.
    ///
    /// The adapter, device, queue, hardware receipt, immutable kernel set, and
    /// device-loss signal remain tied to this backend. Resident brains,
    /// admission, generations, counters, and replay state are new and cannot
    /// accept handles issued by the live backend.
    pub fn new_staging_like_live(&self) -> Result<Self, ScaffoldContractError> {
        if !matches!(self.state, GpuBackendState::Ready) || self.device_lost.load(Ordering::Acquire)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let plan = new_ephemeral_backend_state_plan()?;
        let timestamp_resources = GpuTimestampResources::new(&self.device, &self.queue)?;
        let plasticity_timestamp_resources = GpuTimestampResources::new(&self.device, &self.queue)?;
        Ok(Self {
            backend_instance_id: plan.backend_instance_id,
            hardware: self.hardware.clone(),
            adapter: self.adapter.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
            timestamp_resources,
            plasticity_timestamp_resources,
            device_lost: Arc::clone(&self.device_lost),
            kernels: Arc::clone(&self.kernels),
            state: plan.state.clone(),
            runtime_profile: self.runtime_profile.clone(),
            runtime_budget: self.runtime_budget.clone(),
            activity_policy: self.activity_policy.clone(),
            admission: GpuAdmissionReceipt::empty(self.runtime_budget.clone()),
            class_buckets: BTreeMap::new(),
            slot_generation_watermarks: BTreeMap::new(),
            organisms: BTreeMap::new(),
            curated_residency_generation: plan.curated_residency_generation,
            curated_residency_generation_fingerprint: [0; 4],
            next_dispatch_generation: plan.next_dispatch_generation,
            force_device_lost_after_submit: false,
            #[cfg(feature = "gpu-tests")]
            forced_learning_rejections_remaining: 0,
            #[cfg(feature = "gpu-tests")]
            forced_discard_rejections_remaining: 0,
            recorded_pressure_replay: VecDeque::new(),
            completed_dispatch_count: 0,
            perception_upload_count: 0,
            completed_selection_count: 0,
            last_compact_readback_bytes: 0,
            pending_inference_timing: None,
            completed_neural_timing: None,
            last_apply_fast_plasticity_failure: None,
            next_sleep_job_id: plan.next_sleep_job_id,
            sleep_jobs: BTreeMap::new(),
            committed_sleep: BTreeMap::new(),
        })
    }

    pub const fn hardware_receipt(&self) -> &GpuHardwareReceipt {
        &self.hardware
    }

    /// Returns the shared authoritative device only for quiescent offline
    /// training work. Training pipelines remain owned by `alife_training` and
    /// are never packaged into the gameplay backend.
    pub fn offline_training_device_queue(
        &self,
    ) -> Result<(&wgpu::Device, &wgpu::Queue), ScaffoldContractError> {
        if !matches!(self.state, GpuBackendState::Ready) || self.device_lost.load(Ordering::Acquire)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok((&self.device, &self.queue))
    }

    /// Installs an exact pressure sequence for same-adapter evidence replay.
    ///
    /// This exact-replay boundary replaces only the host pressure sample;
    /// perception, recurrent execution, logits, selection, world outcomes, and
    /// learning remain GPU-authoritative and run through the production path.
    pub fn install_recorded_pressure_replay(
        &mut self,
        samples: Vec<GpuPressureSample>,
    ) -> Result<(), ScaffoldContractError> {
        if samples.is_empty() || !self.recorded_pressure_replay.is_empty() {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        for sample in &samples {
            sample.validate_for(&self.activity_policy)?;
        }
        self.recorded_pressure_replay = samples.into();
        Ok(())
    }

    pub fn recorded_pressure_replay_remaining(&self) -> usize {
        self.recorded_pressure_replay.len()
    }

    pub const fn runtime_profile(&self) -> &GpuRuntimeProfile {
        &self.runtime_profile
    }

    pub const fn runtime_budget(&self) -> &GpuRuntimeBudget {
        &self.runtime_budget
    }

    pub const fn activity_policy(&self) -> &BrainActivityPolicyV1 {
        &self.activity_policy
    }

    pub const fn admission_receipt(&self) -> &GpuAdmissionReceipt {
        &self.admission
    }

    pub const fn state(&self) -> &GpuBackendState {
        &self.state
    }

    pub const fn completed_dispatch_count(&self) -> u64 {
        self.completed_dispatch_count
    }

    pub const fn perception_upload_count(&self) -> u64 {
        self.perception_upload_count
    }

    pub const fn completed_selection_count(&self) -> u64 {
        self.completed_selection_count
    }

    pub fn take_completed_neural_timing_sample(&mut self) -> Option<GpuNeuralTimingSample> {
        self.completed_neural_timing.take()
    }

    pub fn brain_atp_q16(&self, handle: GpuBrainHandle) -> Result<u32, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        self.class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .map(|resident| resident.brain_atp_q16)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    /// Returns the bounded v1.1 work receipt attached to a resident brain.
    pub fn v11_work(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<GpuV11WorkReceipt, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        self.class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .map(|resident| resident.v11.last_work())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn read_v11_mutable_state_for_test(
        &mut self,
        handle: GpuBrainHandle,
        decoder_local: u32,
        neuron: u32,
    ) -> Result<crate::GpuV11MutableStateProbe, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let (phenotype, slot, state) = {
            let resident = pool.resident(handle)?;
            (
                resident.phenotype.clone(),
                resident.brain_slot.clone(),
                resident.v11.clone(),
            )
        };
        let bucket = pool.bucket_for_handle_mut(handle)?;
        let upload = compile_v11_slot_upload(&bucket.plan, &slot, &phenotype, &state)
            .map_err(map_gpu_contract_error)?;
        bucket
            .buffers
            .read_v11_mutable_probe(&self.device, &self.queue, &upload, decoder_local, neuron)
            .map_err(map_gpu_contract_error)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn seed_v11_mutable_state_for_test(
        &mut self,
        handle: GpuBrainHandle,
        decoder_local: u32,
        neuron: u32,
        probe: crate::GpuV11MutableStateProbe,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let (phenotype, slot, state) = {
            let resident = pool.resident(handle)?;
            (
                resident.phenotype.clone(),
                resident.brain_slot.clone(),
                resident.v11.clone(),
            )
        };
        let bucket = pool.bucket_for_handle_mut(handle)?;
        let upload = compile_v11_slot_upload(&bucket.plan, &slot, &phenotype, &state)
            .map_err(map_gpu_contract_error)?;
        bucket
            .buffers
            .seed_v11_mutable_probe(&self.queue, &upload, decoder_local, neuron, probe)
            .map_err(map_gpu_contract_error)
    }

    /// Runs bounded sparse structural plasticity from the live sleep replay.
    pub fn apply_v11_sleep_structural_phase(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<(), ScaffoldContractError> {
        let replay = self.build_sleep_replay_batch(handle)?;
        let phenotype = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .resident(handle)?
            .phenotype
            .clone();
        let base_pairs = phenotype
            .synapses()
            .iter()
            .map(|synapse| (synapse.source(), synapse.target()))
            .collect::<std::collections::BTreeSet<_>>();
        let active = replay
            .synapse_spans
            .iter()
            .filter_map(|span| {
                let start = usize::try_from(span.sample_start).ok()?;
                let count = usize::try_from(span.sample_count).ok()?;
                let samples = replay
                    .eligibility_samples
                    .get(start..start.checked_add(count)?)?;
                let eligibility = samples
                    .iter()
                    .map(|sample| u32::from(sample.eligibility_q15.unsigned_abs()))
                    .max()
                    .unwrap_or(0);
                if eligibility == 0 {
                    return None;
                }
                let synapse = phenotype
                    .synapses()
                    .get(usize::try_from(span.local_synapse_id).ok()?)?;
                Some((
                    synapse.source(),
                    synapse.target(),
                    synapse.route_index(),
                    eligibility,
                ))
            })
            .take(32)
            .collect::<Vec<_>>();
        let mut evidence = Vec::new();
        for pair in active.windows(2).take(32) {
            let (source, target, route, eligibility) = (pair[0].0, pair[1].1, pair[0].2, pair[0].3);
            if source == target || base_pairs.contains(&(source, target)) {
                continue;
            }
            let Some(region) = u16::try_from(route).ok() else {
                continue;
            };
            evidence.push(CoactivationEvidence {
                region,
                source,
                target,
                coactivation: 1,
                eligibility,
                concept_gap_support: 0,
            });
        }
        if evidence.is_empty() {
            if let Some((source, target, route, eligibility)) = active.first().copied() {
                for offset in 1..=8_u32 {
                    let candidate_target = (target % phenotype.neuron_count()).wrapping_add(offset)
                        % phenotype.neuron_count();
                    if candidate_target == source
                        || base_pairs.contains(&(source, candidate_target))
                    {
                        continue;
                    }
                    let Some(region) = u16::try_from(route).ok() else {
                        break;
                    };
                    evidence.push(CoactivationEvidence {
                        region,
                        source,
                        target: candidate_target,
                        coactivation: 1,
                        eligibility,
                        concept_gap_support: 0,
                    });
                    break;
                }
            }
        }
        self.apply_v11_structural_phase(handle, &evidence)?;
        Ok(())
    }

    pub fn set_v11_dendritic_branches(
        &mut self,
        handle: GpuBrainHandle,
        branches: DendriticBranchSet,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let (phenotype, brain_slot, mut next) = {
            let resident = pool.resident(handle)?;
            (
                resident.phenotype.clone(),
                resident.brain_slot.clone(),
                resident.v11.clone(),
            )
        };
        if branches.branches().len()
            > usize::from(
                phenotype
                    .cognitive_architecture()
                    .dendritic_branch_capacity(),
            )
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        next.set_dendritic_branches(branches)?;
        let upload = {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            compile_v11_slot_upload(&bucket.plan, &brain_slot, &phenotype, &next)
                .map_err(map_gpu_contract_error)?
        };
        {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            bucket
                .buffers
                .write_slot_upload(&self.queue, &upload)
                .map_err(map_gpu_contract_error)?;
        }
        let resident = pool.resident_mut(handle)?;
        resident.brain_slot = upload.brain_slot().clone();
        resident.ranges = upload.ranges().clone();
        resident.v11 = next;
        Ok(())
    }

    /// Applies bounded local structural evidence and atomically rebuilds the
    /// resident's affected sparse spans.
    pub fn apply_v11_structural_phase(
        &mut self,
        handle: GpuBrainHandle,
        evidence: &[CoactivationEvidence],
    ) -> Result<GpuV11WorkReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let (phenotype, brain_slot, previous, mut next) = {
            let resident = pool.resident(handle)?;
            (
                resident.phenotype.clone(),
                resident.brain_slot.clone(),
                resident.v11.clone(),
                resident.v11.clone(),
            )
        };
        let work = next.apply_structural_phase(evidence)?;
        if let Some(pending) = next.pending_lifetime_synapse() {
            next.clear_pending_lifetime_synapse(&pending)?;
        }
        let (previous_upload, upload) = {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            let previous_upload =
                compile_v11_slot_upload(&bucket.plan, &brain_slot, &phenotype, &previous)
                    .map_err(map_gpu_contract_error)?;
            let upload = compile_v11_slot_upload(&bucket.plan, &brain_slot, &phenotype, &next)
                .map_err(map_gpu_contract_error)?;
            (previous_upload, upload)
        };
        let upload = {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            let live = bucket
                .buffers
                .read_live_mutable_slot(&self.device, &self.queue, upload.ranges())
                .map_err(map_gpu_contract_error)?;
            upload
                .with_remapped_live_mutable_state(&previous_upload, live)
                .map_err(map_gpu_contract_error)?
        };
        {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            bucket
                .buffers
                .write_v11_topology_upload(&self.queue, &upload)
                .map_err(map_gpu_contract_error)?;
        }
        let resident = pool.resident_mut(handle)?;
        resident.brain_slot = upload.brain_slot().clone();
        resident.ranges = upload.ranges().clone();
        resident.v11 = next;
        Ok(work)
    }

    pub fn checkpoint_v11(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<GpuV11Checkpoint, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        self.class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .map(|resident| resident.v11.checkpoint())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub fn restore_v11(
        &mut self,
        handle: GpuBrainHandle,
        checkpoint: GpuV11Checkpoint,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let (phenotype, brain_slot, previous) = {
            let resident = pool.resident(handle)?;
            (
                resident.phenotype.clone(),
                resident.brain_slot.clone(),
                resident.v11.clone(),
            )
        };
        if checkpoint.neuron_count != phenotype.neuron_count() {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        if checkpoint.dendritic_branches.branches().len()
            > usize::from(
                phenotype
                    .cognitive_architecture()
                    .dendritic_branch_capacity(),
            )
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let next = GpuV11CausalState::restore(checkpoint)?;
        let (previous_upload, upload) = {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            let previous_upload =
                compile_v11_slot_upload(&bucket.plan, &brain_slot, &phenotype, &previous)
                    .map_err(map_gpu_contract_error)?;
            let upload = compile_v11_slot_upload(&bucket.plan, &brain_slot, &phenotype, &next)
                .map_err(map_gpu_contract_error)?;
            (previous_upload, upload)
        };
        let upload = {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            let live = bucket
                .buffers
                .read_live_mutable_slot(&self.device, &self.queue, upload.ranges())
                .map_err(map_gpu_contract_error)?;
            upload
                .with_remapped_live_mutable_state(&previous_upload, live)
                .map_err(map_gpu_contract_error)?
        };
        {
            let bucket = pool.bucket_for_handle_mut(handle)?;
            bucket
                .buffers
                .write_v11_topology_upload(&self.queue, &upload)
                .map_err(map_gpu_contract_error)?;
        }
        let resident = pool.resident_mut(handle)?;
        resident.brain_slot = upload.brain_slot().clone();
        resident.ranges = upload.ranges().clone();
        resident.v11 = next;
        Ok(())
    }

    pub fn snapshot_activity_state(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<GpuActivityRuntimeSnapshot, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        let resident = self
            .class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let all_absent = resident.last_pressure.is_none()
            && resident.last_throttle.is_none()
            && resident.last_work.is_none();
        let all_present = resident.last_pressure.is_some()
            && resident.last_throttle.is_some()
            && resident.last_work.is_some();
        if !(all_absent || all_present)
            || resident.activity_sequence_cursor == 0
            || resident.brain_atp_q16 > BRAIN_ATP_Q16_MAX
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        if let (Some(pressure), Some(throttle), Some(work)) = (
            resident.last_pressure,
            resident.last_throttle.as_ref(),
            resident.last_work.as_ref(),
        ) {
            pressure.validate_for(&self.activity_policy)?;
            let capacity = capacity_for_gpu_class(handle.class_id)?;
            throttle.validate_for(&resident.phenotype, capacity.execution())?;
            work.validate_for(&self.activity_policy, throttle)?;
            if pressure.handle_slot != handle.slot
                || pressure.handle_generation != handle.generation
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            throttle.validate_runtime_binding(handle.slot, handle.generation)?;
            work.validate_runtime_binding(handle.slot, handle.generation)?;
            if resident.activity_sequence_cursor
                != pressure.sequence_cursor.checked_add(1).unwrap_or(0)
                || resident.last_activity_dispatch_generation != pressure.dispatch_generation
                || resident.last_activity_frame_digest != pressure.frame_digest
            {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
        } else if resident.activity_sequence_cursor != 1
            || resident.last_activity_dispatch_generation != 0
            || resident.last_activity_frame_digest != [0; 4]
            || resident.last_completed_gpu_time_ns != 0
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(GpuActivityRuntimeSnapshot {
            next_sequence_cursor: resident.activity_sequence_cursor,
            brain_atp_q16: resident.brain_atp_q16,
            last_world_atp_tick: resident.last_world_atp_tick,
            next_completed_gpu_time_ns: resident.last_completed_gpu_time_ns,
            pressure: resident.last_pressure,
            throttle: resident.last_throttle.clone(),
            work: resident.last_work.clone(),
        })
    }

    pub fn restore_activity_state(
        &mut self,
        handle: GpuBrainHandle,
        input: GpuActivityRestoreInput,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        if input.brain_atp_q16 > BRAIN_ATP_Q16_MAX
            || input
                .last_world_atp_tick
                .is_some_and(|tick| tick > input.checkpoint_tick)
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let phenotype = self
            .class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .map(|resident| resident.phenotype.clone())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;

        let rebound = match input.record {
            Some(record) => {
                if record.policy_version != self.activity_policy.policy_version
                    || record.policy_digest != self.activity_policy.policy_digest
                    || record.organism_id_raw != handle.organism_id.raw()
                    || record.class_id_raw != handle.class_id.raw()
                    || record.tick > input.checkpoint_tick
                    || input.next_sequence_cursor
                        != record.sequence_cursor.checked_add(1).unwrap_or(0)
                    || record.brain_atp_fraction_q16 > BRAIN_ATP_Q16_MAX
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                let identity = BrainDispatchIdentity {
                    organism_id_raw: record.organism_id_raw,
                    tick: record.tick,
                    class_id_raw: record.class_id_raw,
                    handle_slot: handle.slot,
                    handle_generation: handle.generation,
                    sequence_cursor: record.sequence_cursor,
                    dispatch_generation: record.dispatch_generation,
                    frame_digest: record.frame_digest,
                };
                let pressure = GpuPressureSample::try_new(
                    &self.activity_policy,
                    GpuPressureSampleInput {
                        identity,
                        source_dispatch_generation: record.source_dispatch_generation,
                        source_frame_digest: record.source_frame_digest,
                        completed_gpu_time_ns: record.completed_gpu_time_ns,
                        queue_depth: record.queue_depth,
                        logical_heap_used: u64::from(record.logical_heap_pressure_q16),
                        logical_heap_capacity: u64::from(BRAIN_ATP_Q16_MAX),
                        brain_atp_remaining_q16: record.brain_atp_fraction_q16,
                        brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
                    },
                )?;
                let capacity = capacity_for_gpu_class(handle.class_id)?;
                let throttle = NeuralThrottleDecision::derive(
                    &self.activity_policy,
                    &phenotype,
                    capacity.execution(),
                    identity,
                    pressure,
                )?;
                if throttle.level != record.level
                    || throttle.microsteps != record.microsteps
                    || throttle.enabled_route_ids != record.enabled_route_ids
                    || throttle.route_schedule_digest != record.route_schedule_digest
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                let work = BrainWorkReceipt::try_new(
                    &self.activity_policy,
                    &throttle,
                    record.work,
                    record.atp_before_q16,
                )?;
                if work.neural_cost_q24 != record.neural_cost_q24
                    || work.atp_debit_q16 != record.atp_debit_q16
                    || work.atp_after_q16 != record.atp_after_q16
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                Some((pressure, throttle, work))
            }
            None if input.next_sequence_cursor == 1 && input.next_completed_gpu_time_ns == 0 => {
                None
            }
            None => return Err(ScaffoldContractError::BrainActivitySequenceMismatch),
        };

        let resident = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .and_then(|pool| pool.resident_mut(handle).ok())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        resident.activity_sequence_cursor = input.next_sequence_cursor;
        match rebound {
            Some((pressure, throttle, work)) => {
                resident.brain_atp_q16 = input.brain_atp_q16;
                resident.last_world_atp_tick = input.last_world_atp_tick;
                resident.last_activity_dispatch_generation = pressure.dispatch_generation;
                resident.last_activity_frame_digest = pressure.frame_digest;
                resident.last_completed_gpu_time_ns = input.next_completed_gpu_time_ns;
                resident.last_pressure = Some(pressure);
                resident.last_throttle = Some(throttle);
                resident.last_work = Some(work);
            }
            None => {
                resident.brain_atp_q16 = input.brain_atp_q16;
                resident.last_world_atp_tick = input.last_world_atp_tick;
                resident.last_activity_dispatch_generation = 0;
                resident.last_activity_frame_digest = [0; 4];
                resident.last_completed_gpu_time_ns = 0;
                resident.last_pressure = None;
                resident.last_throttle = None;
                resident.last_work = None;
            }
        }
        Ok(())
    }

    /// Charges the exact world-owned ATP term before neural dispatch.
    ///
    /// The monotonic tick guard makes basal cost replay-safe. Sleep recovery is
    /// a distinct credit in the same fixed-point transaction and never alters
    /// the neural work receipt's independently computed debit.
    pub fn charge_world_brain_atp_tick(
        &mut self,
        handle: GpuBrainHandle,
        world_tick: u64,
        began_tick_asleep: bool,
    ) -> Result<u32, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident_mut(handle)?;
        if let Some(last) = resident.last_world_atp_tick {
            if last == world_tick {
                return Ok(resident.brain_atp_q16);
            }
            if last.checked_add(1) != Some(world_tick) {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
        }
        let after_basal = resident
            .brain_atp_q16
            .saturating_sub(BRAIN_ATP_BASAL_DEBIT_Q16);
        resident.brain_atp_q16 = if began_tick_asleep {
            after_basal
                .saturating_add(BRAIN_ATP_SLEEP_RECOVERY_Q16)
                .min(BRAIN_ATP_Q16_MAX)
        } else {
            after_basal
        };
        resident.last_world_atp_tick = Some(world_tick);
        Ok(resident.brain_atp_q16)
    }

    pub fn pending_eligibility(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<Option<PendingEligibilityReceipt>, ScaffoldContractError> {
        if self.device_lost.load(Ordering::Acquire) || !matches!(self.state, GpuBackendState::Ready)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident(handle)?;
        Ok(resident.pending_eligibility)
    }

    /// Apply one sealed measured outcome to the pending waking eligibility.
    pub fn apply_sealed_outcome(
        &mut self,
        handle: GpuBrainHandle,
        patch: &ExperiencePatch,
    ) -> Result<GpuLearningReceipt, ScaffoldContractError> {
        let mut receipts = self.apply_sealed_outcome_batch(&[(handle, patch)])?;
        receipts
            .pop()
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)
    }

    /// Takes the diagnostic receipt from the most recent sealed-outcome apply.
    pub fn take_apply_fast_plasticity_failure_receipt(
        &mut self,
    ) -> Option<GpuRuntimeApplyFastPlasticityFailureReceipt> {
        self.last_apply_fast_plasticity_failure.take()
    }

    /// Read-only field receipt for the exact evidence contract enforced by
    /// `apply_sealed_outcome`. `None` means the sealed packet matches the
    /// currently installed pending decision evidence.
    pub fn sealed_outcome_credit_mismatch_receipt(
        &self,
        handle: GpuBrainHandle,
        patch: &ExperiencePatch,
    ) -> Result<Option<GpuLearningEvidenceMismatchReceipt>, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        let packet = OutcomeCreditPacket::from_sealed_patch(patch)?;

        let scalar = GpuLearningEvidenceMismatchReceipt::scalar;
        let words = GpuLearningEvidenceMismatchReceipt::words;
        let originating_tick = packet.originating_tick().raw();
        let outcome_tick = packet.outcome_tick().raw();
        if outcome_tick <= originating_tick {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::OutcomeTickAfterOriginating,
                originating_tick.saturating_add(1),
                outcome_tick,
            )));
        }
        if packet.active_activation_side() > 1 {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::ActiveActivationSide,
                1,
                u64::from(packet.active_activation_side()),
            )));
        }
        if packet.dispatch_generation() == 0 {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::DispatchGeneration,
                1,
                0,
            )));
        }
        if packet.phenotype_hash() == PhenotypeHash([0; 4]) {
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::PhenotypeHashNonZero,
                [1, 0, 0, 0],
                packet.phenotype_hash().0,
            )));
        }
        if packet.frame_digest() == PerceptionFrameDigest([0; 4]) {
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::FrameDigestNonZero,
                [1, 0, 0, 0],
                packet.frame_digest().0,
            )));
        }
        if packet.candidate_feature_digest().0 == [0; 2] {
            let digest = packet.candidate_feature_digest().0;
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::CandidateFeatureDigestNonZero,
                [1, 0, 0, 0],
                [digest[0], digest[1], 0, 0],
            )));
        }

        let modulator = packet.modulator();
        for (field, value) in [
            (
                GpuLearningEvidenceMismatchField::RewardPredictionErrorRange,
                modulator.reward_prediction_error(),
            ),
            (
                GpuLearningEvidenceMismatchField::PainRange,
                modulator.pain(),
            ),
            (
                GpuLearningEvidenceMismatchField::HomeostaticImprovementRange,
                modulator.homeostatic_improvement(),
            ),
            (
                GpuLearningEvidenceMismatchField::FrustrationRange,
                modulator.frustration(),
            ),
            (
                GpuLearningEvidenceMismatchField::NoveltyRange,
                modulator.novelty(),
            ),
            (
                GpuLearningEvidenceMismatchField::ModulatorValueRange,
                modulator.value(),
            ),
        ] {
            if !(-1.0..=1.0).contains(&value) {
                return Ok(Some(words(
                    field,
                    [
                        u64::from((-1.0_f32).to_bits()),
                        u64::from(1.0_f32.to_bits()),
                        0,
                        0,
                    ],
                    [u64::from(value.to_bits()), 0, 0, 0],
                )));
            }
        }

        // This conversion now cannot collapse one of its learning-evidence
        // checks without the receipt above naming the same first bad value.
        let _ = GpuOutcomeCreditRecord::try_from(&packet)?;
        let pool = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident(handle)?;
        if packet.organism_id() != handle.organism_id {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::OrganismId,
                handle.organism_id.raw(),
                packet.organism_id().raw(),
            )));
        }
        if packet.phenotype_hash() != handle.phenotype_hash {
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::PhenotypeHash,
                handle.phenotype_hash.0,
                packet.phenotype_hash().0,
            )));
        }
        resident
            .learning_sequence_guard
            .validate_next(packet.replay_key())?;
        let Some(pending_receipt) = resident.pending_eligibility else {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::PendingEligibilityPresent,
                1,
                0,
            )));
        };
        if resident.pending_eligibility_record.is_none() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::PendingEligibilityRecordPresent,
                1,
                0,
            )));
        }
        let identity = pending_receipt.identity();
        if identity.handle_generation() != handle.generation {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::HandleGeneration,
                u64::from(handle.generation),
                u64::from(identity.handle_generation()),
            )));
        }
        if identity.phenotype_hash() != packet.phenotype_hash() {
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::PendingPhenotypeHash,
                identity.phenotype_hash().0,
                packet.phenotype_hash().0,
            )));
        }
        if identity.dispatch_generation() != packet.dispatch_generation() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::DispatchGenerationIdentity,
                identity.dispatch_generation(),
                packet.dispatch_generation(),
            )));
        }
        if identity.originating_tick() != packet.originating_tick() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::OriginatingTick,
                identity.originating_tick().raw(),
                packet.originating_tick().raw(),
            )));
        }
        if identity.frame_digest() != packet.frame_digest() {
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::FrameDigest,
                identity.frame_digest().0,
                packet.frame_digest().0,
            )));
        }
        if identity.active_activation_side() != packet.active_activation_side() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::ActiveActivationSideIdentity,
                u64::from(identity.active_activation_side()),
                u64::from(packet.active_activation_side()),
            )));
        }
        if identity.candidate_index() != packet.selected_candidate() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::CandidateIndex,
                u64::from(identity.candidate_index()),
                u64::from(packet.selected_candidate()),
            )));
        }
        if identity.action_id() != packet.selected_action() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::ActionId,
                u64::from(identity.action_id().raw()),
                u64::from(packet.selected_action().raw()),
            )));
        }
        if identity.action_family() != packet.selected_family() {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::ActionFamily,
                u64::from(identity.action_family().raw()),
                u64::from(packet.selected_family().raw()),
            )));
        }
        if identity.candidate_feature_digest() != packet.candidate_feature_digest() {
            let expected = identity.candidate_feature_digest().0;
            let actual = packet.candidate_feature_digest().0;
            return Ok(Some(words(
                GpuLearningEvidenceMismatchField::CandidateFeatureDigest,
                [expected[0], expected[1], 0, 0],
                [actual[0], actual[1], 0, 0],
            )));
        }
        if identity.active_eligibility_generation() != resident.active_eligibility_generation {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::ActiveEligibilityGeneration,
                resident.active_eligibility_generation,
                identity.active_eligibility_generation(),
            )));
        }
        let expected_staging_generation = resident
            .active_eligibility_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
        if identity.staging_eligibility_generation() != expected_staging_generation {
            return Ok(Some(scalar(
                GpuLearningEvidenceMismatchField::StagingEligibilityGeneration,
                expected_staging_generation,
                identity.staging_eligibility_generation(),
            )));
        }
        for (field, actual) in [
            (
                GpuLearningEvidenceMismatchField::ActiveWeightGenerationNonZero,
                resident.active_weight_generation,
            ),
            (
                GpuLearningEvidenceMismatchField::ReplayJournalGenerationNonZero,
                resident.replay_journal_generation,
            ),
            (
                GpuLearningEvidenceMismatchField::TransactionGenerationNonZero,
                resident.transaction_generation,
            ),
        ] {
            if actual == 0 {
                return Ok(Some(scalar(field, 1, actual)));
            }
        }
        Ok(None)
    }

    /// Apply a same-class batch. Rows may span fixed arenas, but every row is
    /// bound to its arena-local slot, durable pending eligibility, and a
    /// core-owned sequence token before any command is submitted.
    pub fn apply_sealed_outcome_batch(
        &mut self,
        batch: &[(GpuBrainHandle, &ExperiencePatch)],
    ) -> Result<Vec<GpuLearningReceipt>, ScaffoldContractError> {
        self.last_apply_fast_plasticity_failure = None;
        self.ensure_ready()?;
        if batch.is_empty() {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        #[cfg(feature = "gpu-tests")]
        if self.forced_learning_rejections_remaining > 0 {
            self.forced_learning_rejections_remaining -= 1;
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        let class_id = batch[0].0.class_id.raw();
        let mut seen = BTreeSet::new();
        let mut prepared = Vec::with_capacity(batch.len());
        for (handle, patch) in batch {
            self.validate_handle_backend(*handle)?;
            if handle.class_id.raw() != class_id
                || !seen.insert((handle.slot, handle.generation, handle.organism_id.raw()))
            {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
            let packet = OutcomeCreditPacket::from_sealed_patch(patch)?;
            let outcome = GpuOutcomeCreditRecord::try_from(&packet)?;
            let pool = self
                .class_buckets
                .get(&class_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let chunk_index = pool
                .bucket_index_for_handle(*handle)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(*handle)?;
            let expected_last_committed = resident.learning_sequence_guard.last_committed();
            let commit_token = resident
                .learning_sequence_guard
                .validate_next(packet.replay_key())?;
            let pending_receipt = resident
                .pending_eligibility
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            let pending_record = resident
                .pending_eligibility_record
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            let identity = pending_receipt.identity();
            if packet.organism_id() != handle.organism_id
                || packet.phenotype_hash() != handle.phenotype_hash
                || identity.handle_generation() != handle.generation
                || identity.phenotype_hash() != packet.phenotype_hash()
                || identity.dispatch_generation() != packet.dispatch_generation()
                || identity.originating_tick() != packet.originating_tick()
                || identity.frame_digest() != packet.frame_digest()
                || identity.active_activation_side() != packet.active_activation_side()
                || identity.candidate_index() != packet.selected_candidate()
                || identity.action_id() != packet.selected_action()
                || identity.action_family() != packet.selected_family()
                || identity.candidate_feature_digest() != packet.candidate_feature_digest()
                || identity.active_eligibility_generation()
                    != resident.active_eligibility_generation
                || identity.staging_eligibility_generation()
                    != resident
                        .active_eligibility_generation
                        .checked_add(1)
                        .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?
                || resident.active_weight_generation == 0
                || resident.replay_journal_generation == 0
                || resident.transaction_generation == 0
            {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
            prepared.push(PreparedLearningApply {
                chunk_index,
                handle: *handle,
                packet,
                outcome,
                brain_slot: resident.brain_slot.clone(),
                pending_receipt,
                pending_record,
                active_weight_generation: resident.active_weight_generation,
                active_eligibility_generation: resident.active_eligibility_generation,
                replay_journal_generation: resident.replay_journal_generation,
                transaction_generation: resident.transaction_generation,
                expected_last_committed,
                commit_token,
            });
        }
        let mut grouped_indices = BTreeMap::<usize, Vec<usize>>::new();
        for (index, entry) in prepared.iter().enumerate() {
            grouped_indices
                .entry(entry.chunk_index)
                .or_default()
                .push(index);
        }
        let mut ordered_gpu_records = vec![None; prepared.len()];
        let mut plasticity_timestamp_ticks = 0_u64;
        for (chunk_index, indices) in grouped_indices {
            let submitted_entry = indices
                .first()
                .copied()
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            let gpu_entries = indices
                .iter()
                .map(|index| {
                    let entry = &prepared[*index];
                    GpuFastPlasticityBatchEntry {
                        slot: &entry.brain_slot,
                        pending: &entry.pending_record,
                        outcome: entry.outcome,
                        active_weight_generation: entry.active_weight_generation,
                        replay_generation: entry.replay_journal_generation,
                        transaction_generation: entry.transaction_generation,
                    }
                })
                .collect::<Vec<_>>();
            let (gpu_result, malformed_receipt) = {
                let bucket = self
                    .class_buckets
                    .get_mut(&class_id)
                    .and_then(|pool| pool.chunks.get_mut(chunk_index))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let result = bucket.pipelines.apply_fast_plasticity(
                    &self.device,
                    &self.queue,
                    &bucket.buffers,
                    &gpu_entries,
                    GpuTimestampQueryResources::new(
                        &self.plasticity_timestamp_resources.query_set,
                        &self.plasticity_timestamp_resources.resolve_buffer,
                        &self.plasticity_timestamp_resources.readback_buffer,
                    ),
                );
                let malformed_receipt = bucket.pipelines.take_fast_plasticity_malformed_receipt();
                (result, malformed_receipt)
            };
            let gpu_timed_result = match gpu_result {
                Ok(result) => result,
                Err(
                    error @ (GpuClosedLoopError::MalformedUpload
                    | GpuClosedLoopError::StaleOrForeignHandle),
                ) => {
                    let class = match error {
                        GpuClosedLoopError::MalformedUpload => {
                            GpuRuntimeApplyFastPlasticityFailureClass::MalformedUpload
                        }
                        GpuClosedLoopError::StaleOrForeignHandle => {
                            GpuRuntimeApplyFastPlasticityFailureClass::StaleOrForeignHandle
                        }
                        _ => unreachable!("matched bounded fast-plasticity failure class"),
                    };
                    self.last_apply_fast_plasticity_failure =
                        Some(GpuRuntimeApplyFastPlasticityFailureReceipt {
                            class,
                            class_id,
                            chunk_index,
                            submitted_entry,
                            malformed_field: malformed_receipt.map(|receipt| receipt.field.into()),
                            expected: malformed_receipt.map(|receipt| receipt.expected),
                            actual: malformed_receipt.map(|receipt| receipt.actual),
                        });
                    return Err(ScaffoldContractError::LearningEvidenceMismatch);
                }
                Err(_) => {
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            };
            if gpu_timed_result.records.len() != indices.len() {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            plasticity_timestamp_ticks = plasticity_timestamp_ticks
                .checked_add(gpu_timed_result.timestamp_delta_ticks)
                .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?;
            for (index, record) in indices.into_iter().zip(gpu_timed_result.records) {
                ordered_gpu_records[index] = Some(record);
            }
        }
        let gpu_records = ordered_gpu_records
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        if gpu_records.len() != prepared.len() {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let host_precommit_valid = prepared.iter().zip(&gpu_records).all(|(entry, record)| {
            self.class_buckets
                .get(&class_id)
                .and_then(|pool| pool.resident(entry.handle).ok())
                .is_some_and(|resident| {
                    resident.pending_eligibility == Some(entry.pending_receipt)
                        && resident.pending_eligibility_record == Some(entry.pending_record)
                        && resident.active_weight_generation == entry.active_weight_generation
                        && resident.active_eligibility_generation
                            == entry.active_eligibility_generation
                        && resident.replay_journal_generation == entry.replay_journal_generation
                        && resident.transaction_generation == entry.transaction_generation
                        && resident.learning_sequence_guard.last_committed()
                            == entry.expected_last_committed
                        && record.input_fast_generation() == entry.active_weight_generation
                        && record.output_eligibility_generation()
                            == entry
                                .pending_receipt
                                .identity()
                                .staging_eligibility_generation()
                })
        });
        if !host_precommit_valid {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let hardware_receipt_generation = self.hardware.generation;
        let readback_bytes = prepared
            .len()
            .checked_mul(crate::GPU_FAST_PLASTICITY_COMMIT_BYTES)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let mut receipts = Vec::with_capacity(prepared.len());
        for (entry, record) in prepared.into_iter().zip(gpu_records) {
            let guard_commit = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.resident_mut(entry.handle).ok())
                .expect("learning host commit was prevalidated")
                .learning_sequence_guard
                .commit_validated(entry.commit_token);
            if guard_commit.is_err() {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            let resident = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.resident_mut(entry.handle).ok())
                .expect("learning guard commit retained the resident slot");
            resident.active_weight_bank ^= 1;
            resident.active_eligibility_bank ^= 1;
            resident.active_weight_generation = record.output_fast_generation();
            resident.active_eligibility_generation = record.output_eligibility_generation();
            resident.replay_journal_generation = record.replay_generation();
            resident.transaction_generation = record.transaction_generation();
            resident.pending_eligibility = None;
            resident.pending_eligibility_record = None;
            receipts.push(GpuLearningReceipt {
                handle: entry.handle,
                sequence_id: entry.packet.sequence_id(),
                dispatch_generation: entry.packet.dispatch_generation(),
                active_activation_side: entry.packet.active_activation_side(),
                input_fast_generation: record.input_fast_generation(),
                output_fast_generation: record.output_fast_generation(),
                output_eligibility_generation: record.output_eligibility_generation(),
                replay_journal_generation: record.replay_generation(),
                fast_weights_changed: record.fast_weights_changed,
                max_abs_delta: record.max_abs_delta(),
                hardware_receipt_generation,
            });
        }
        self.last_compact_readback_bytes = readback_bytes;
        if let Some(pending) = self.pending_inference_timing {
            let dispatch_generation = receipts
                .first()
                .map(|receipt| receipt.dispatch_generation)
                .unwrap_or(0);
            if pending.dispatch_generation == dispatch_generation
                && pending.class_id_raw == Some(class_id)
                && usize::try_from(pending.population).ok() == Some(receipts.len())
            {
                let inference_period_ns_q24 = self.timestamp_resources.period_ns_q24()?;
                let plasticity_period_ns_q24 =
                    self.plasticity_timestamp_resources.period_ns_q24()?;
                if inference_period_ns_q24 != plasticity_period_ns_q24 {
                    return Err(ScaffoldContractError::GpuTimestampQueryUnavailable);
                }
                self.completed_neural_timing = Some(GpuNeuralTimingSample {
                    dispatch_generation,
                    class_id_raw: class_id,
                    population: pending.population,
                    inference_timestamp_ticks: pending.inference_timestamp_ticks,
                    plasticity_timestamp_ticks,
                    timestamp_period_ns_q24: inference_period_ns_q24,
                });
                self.pending_inference_timing = None;
            }
        }
        Ok(receipts)
    }

    pub fn discard_pending_eligibility(
        &mut self,
        handle: GpuBrainHandle,
        identity: &PendingEligibilityIdentity,
    ) -> Result<PendingEligibilityDiscardReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let (brain_slot, pending_receipt, pending_record, transaction_generation) = {
            let pool = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(handle)?;
            (
                resident.brain_slot.clone(),
                resident
                    .pending_eligibility
                    .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?,
                resident
                    .pending_eligibility_record
                    .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?,
                resident.transaction_generation,
            )
        };
        if pending_receipt.identity() != identity
            || identity.handle_generation() != handle.generation
            || identity.phenotype_hash() != handle.phenotype_hash
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        #[cfg(feature = "gpu-tests")]
        if self.forced_discard_rejections_remaining > 0 {
            self.forced_discard_rejections_remaining -= 1;
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        let discard_result = {
            let bucket = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle_mut(handle)?;
            bucket.pipelines.discard_pending_eligibility(
                &self.device,
                &self.queue,
                &bucket.buffers,
                &brain_slot,
                &pending_record,
                transaction_generation,
            )
        };
        let discard_record = match discard_result {
            Ok(record) => record,
            Err(_) => {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        let next_transaction_generation = transaction_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
        if discard_record.active_eligibility_generation()
            != identity.active_eligibility_generation()
            || discard_record.discarded_staging_generation()
                != identity.staging_eligibility_generation()
            || discard_record.transaction_generation() != next_transaction_generation
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let resident = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .and_then(|pool| pool.resident_mut(handle).ok())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        resident.transaction_generation = next_transaction_generation;
        resident.pending_eligibility = None;
        resident.pending_eligibility_record = None;
        if self
            .pending_inference_timing
            .is_some_and(|pending| pending.dispatch_generation == identity.dispatch_generation())
        {
            self.pending_inference_timing = None;
        }
        Ok(PendingEligibilityDiscardReceipt::new(
            *identity,
            self.hardware.generation,
        ))
    }

    pub fn prepare_memory_context_upload(
        &mut self,
        handle: GpuBrainHandle,
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
    ) -> Result<GpuMemoryContextUpload, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        frame.validate()?;
        let pool = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident(handle)?;
        if resident.ownership.organism_id != frame.organism_id()
            || handle.organism_id != frame.organism_id()
            || resident.ownership.sensor_profile != frame.sensor_profile()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let perception = GpuPerceptionUpload::try_from_frame(frame, &resident.brain_slot, 0)
            .map_err(map_gpu_contract_error)?;
        GpuMemoryContextUpload::try_from_finalized(
            frame,
            recall,
            perception.frame_binding,
            &resident.brain_slot,
        )
        .map_err(map_gpu_contract_error)
    }

    pub fn tick_batch(
        &mut self,
        batch: &[(GpuBrainHandle, PerceptionFrame)],
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        let inputs = batch
            .iter()
            .map(|(handle, frame)| GpuRuntimeTickInput {
                handle: *handle,
                frame,
                memory_upload: None,
            })
            .collect::<Vec<_>>();
        self.tick_inputs(&inputs, None)
    }

    pub fn tick_memory_batch(
        &mut self,
        batch: &GpuClosedLoopMemoryBatchInput<'_>,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        let inputs = batch
            .members
            .iter()
            .map(|member| GpuRuntimeTickInput {
                handle: member.handle,
                frame: member.frame,
                memory_upload: Some(member.memory_upload),
            })
            .collect::<Vec<_>>();
        self.tick_inputs(&inputs, None)
    }

    pub fn tick_memory_batch_with_selector_diagnostics(
        &mut self,
        batch: &GpuClosedLoopMemoryBatchInput<'_>,
        requested_candidate_indices: &[u16],
    ) -> Result<Vec<GpuClosedLoopTick>, GpuRuntimeSelectorDiagnosticError> {
        let inputs = batch
            .members
            .iter()
            .map(|member| GpuRuntimeTickInput {
                handle: member.handle,
                frame: member.frame,
                memory_upload: Some(member.memory_upload),
            })
            .collect::<Vec<_>>();
        let mut capture = SelectorDiagnosticErrorCapture::default();
        match self.tick_inputs_with_selector_diagnostic_capture(
            &inputs,
            Some(requested_candidate_indices),
            Some(&mut capture),
        ) {
            Ok(ticks) => Ok(ticks),
            Err(error) => match capture.enable_error {
                Some(enable_error) => Err(GpuRuntimeSelectorDiagnosticError::Enable(enable_error)),
                None if let Some(receipt) = capture.later_stage_receipt.take() => {
                    Err(GpuRuntimeSelectorDiagnosticError::LaterStage(receipt))
                }
                None if let Some(receipt) = capture.decode_mapped_records_receipt.take() => Err(
                    GpuRuntimeSelectorDiagnosticError::DecodeMappedRecords(receipt),
                ),
                None if let Some(receipt) = capture.build_selector_diagnostic_receipt.take() => {
                    Err(GpuRuntimeSelectorDiagnosticError::BuildSelectorDiagnostic(
                        receipt,
                    ))
                }
                None if capture.enable_completed => {
                    Err(GpuRuntimeSelectorDiagnosticError::LaterStageContract {
                        stage: capture
                            .later_stage
                            .expect("enabled selector diagnostics record their current stage"),
                        error,
                    })
                }
                None => Err(GpuRuntimeSelectorDiagnosticError::Preflight(error)),
            },
        }
    }

    fn tick_inputs(
        &mut self,
        batch: &[GpuRuntimeTickInput<'_>],
        selector_diagnostic_candidate_indices: Option<&[u16]>,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        self.tick_inputs_with_selector_diagnostic_capture(
            batch,
            selector_diagnostic_candidate_indices,
            None,
        )
    }

    fn tick_inputs_with_selector_diagnostic_capture(
        &mut self,
        batch: &[GpuRuntimeTickInput<'_>],
        selector_diagnostic_candidate_indices: Option<&[u16]>,
        mut selector_diagnostic_error_capture: Option<&mut SelectorDiagnosticErrorCapture>,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        let capture_selector_diagnostics = selector_diagnostic_candidate_indices.is_some();
        self.ensure_ready()?;
        if batch.is_empty() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let mut seen_handles = BTreeSet::new();
        let mut seen_organisms = BTreeSet::new();
        let mut grouped = BTreeMap::<(u16, usize), Vec<usize>>::new();
        for (index, input) in batch.iter().enumerate() {
            let handle = input.handle;
            let frame = input.frame;
            self.validate_handle_backend(handle)?;
            if !seen_handles.insert((handle.class_id.raw(), handle.slot, handle.generation))
                || !seen_organisms.insert(handle.organism_id.0)
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            frame.validate()?;
            let pool = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let chunk_index = pool
                .bucket_index_for_handle(handle)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(handle)?;
            if resident.ownership.organism_id != frame.organism_id()
                || handle.organism_id != frame.organism_id()
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            if resident.ownership.sensor_profile != frame.sensor_profile() {
                return Err(ScaffoldContractError::SensorProfileMismatch);
            }
            if resident.pending_eligibility.is_some() {
                return Err(ScaffoldContractError::LearningReplayRejected);
            }
            if resident.activity_sequence_cursor.checked_add(1).is_none() {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
            grouped
                .entry((handle.class_id.raw(), chunk_index))
                .or_default()
                .push(index);
        }

        let dispatch_generation = NonZeroU64::new(self.next_dispatch_generation)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_dispatch_generation = self
            .next_dispatch_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_upload_count = self
            .perception_upload_count
            .checked_add(batch.len() as u64)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_completed_dispatch_count = self
            .completed_dispatch_count
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_completed_selection_count = self
            .completed_selection_count
            .checked_add(batch.len() as u64)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let replayed_pressure = if self.recorded_pressure_replay.is_empty() {
            None
        } else {
            if self.recorded_pressure_replay.len() < batch.len() {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
            Some(
                self.recorded_pressure_replay
                    .drain(..batch.len())
                    .collect::<Vec<_>>(),
            )
        };
        let mut replayed_pressure_iter = replayed_pressure.as_deref().map(<[_]>::iter);
        let activity_decisions = batch
            .iter()
            .map(|input| {
                let handle = input.handle;
                let resident = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let identity = BrainDispatchIdentity {
                    organism_id_raw: handle.organism_id.raw(),
                    tick: input.frame.tick().raw(),
                    class_id_raw: handle.class_id.raw(),
                    handle_slot: handle.slot,
                    handle_generation: handle.generation,
                    sequence_cursor: resident.activity_sequence_cursor,
                    dispatch_generation: dispatch_generation.get(),
                    frame_digest: input.frame.frame_digest().0,
                };
                let pressure = match replayed_pressure_iter
                    .as_mut()
                    .and_then(|samples| samples.next())
                    .copied()
                {
                    Some(sample) => {
                        sample.validate_for(&self.activity_policy)?;
                        if sample.dispatch_identity() != identity
                            || sample.source_dispatch_generation
                                != resident.last_activity_dispatch_generation
                            || sample.source_frame_digest != resident.last_activity_frame_digest
                        {
                            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                        }
                        sample
                    }
                    None => live_pressure_sample(
                        &self.activity_policy,
                        identity,
                        resident,
                        &self.admission,
                        &self.runtime_budget,
                    )?,
                };
                let capacity = capacity_for_gpu_class(handle.class_id)?;
                NeuralThrottleDecision::derive(
                    &self.activity_policy,
                    &resident.phenotype,
                    capacity.execution(),
                    identity,
                    pressure,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let first_class_id = batch[0].handle.class_id.raw();
        let timing_class_id = batch
            .iter()
            .all(|input| input.handle.class_id.raw() == first_class_id)
            .then_some(first_class_id);
        let mut dispatches = Vec::with_capacity(grouped.len());
        for ((class_id, chunk_index), original_indices) in grouped {
            let bucket = self
                .class_buckets
                .get(&class_id)
                .and_then(|pool| pool.chunks.get(chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let entries = original_indices
                .iter()
                .map(|index| {
                    let input = batch[*index];
                    let resident = bucket.slots[input.handle.slot as usize]
                        .as_ref()
                        .expect("complete preflight retained occupied slot");
                    match input.memory_upload {
                        Some(memory_upload) => GpuFixedActiveBatchEntry::with_memory(
                            input.frame,
                            &resident.brain_slot,
                            &resident.phenotype,
                            &activity_decisions[*index],
                            memory_upload,
                            resident.active_eligibility_generation,
                        ),
                        None => GpuFixedActiveBatchEntry::new(
                            input.frame,
                            &resident.brain_slot,
                            &resident.phenotype,
                            &activity_decisions[*index],
                            resident.active_eligibility_generation,
                        ),
                    }
                })
                .collect::<Vec<_>>();
            let prepared = bucket
                .pipelines
                .preflight_fixed_active_batch(&entries, 0, dispatch_generation)
                .map_err(map_gpu_contract_error)?;
            dispatches.push(PreparedClassDispatch {
                class_id,
                chunk_index,
                original_indices,
                prepared: Some(prepared),
                batch: None,
                recorded: false,
                map_ticket: None,
                selector_readback: None,
                selector_map_ticket: None,
                selector_captures: None,
                validated: None,
            });
        }

        for index in 0..dispatches.len() {
            let class_id = dispatches[index].class_id;
            let prepared = dispatches[index]
                .prepared
                .take()
                .expect("prepared exactly once");
            let result = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatches[index].chunk_index))
                .expect("preflight bucket exists")
                .pipelines
                .begin_prepared_batch(prepared);
            match result {
                Ok(mut active) => {
                    if capture_selector_diagnostics {
                        let capacity = self
                            .class_buckets
                            .get(&class_id)
                            .and_then(|pool| pool.chunks.get(dispatches[index].chunk_index))
                            .expect("preflight bucket exists")
                            .buffers
                            .frame_payload_capacity_words();
                        let enable_result = active.enable_selector_diagnostics(
                            class_id,
                            dispatches[index].chunk_index,
                            selector_diagnostic_candidate_indices
                                .expect("capture flag follows requested candidates"),
                            capacity,
                        );
                        if let Err(error) = enable_result {
                            let translated_error =
                                translate_selector_diagnostic_enable_error(error);
                            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut()
                            {
                                capture.enable_error = Some(translated_error.clone());
                            }
                            if let Some(receipt) = error.receipt() {
                                eprintln!("gpu_selector_diagnostic_error_receipt: {receipt}");
                            }
                            let _ = self
                                .class_buckets
                                .get_mut(&class_id)
                                .and_then(|pool| pool.chunks.get_mut(dispatches[index].chunk_index))
                                .expect("preflight bucket exists")
                                .pipelines
                                .abandon_unsubmitted_batch(active);
                            for prior in &mut dispatches[..index] {
                                if let Some(active) = prior.batch.take() {
                                    let _ = self
                                        .class_buckets
                                        .get_mut(&prior.class_id)
                                        .and_then(|pool| pool.chunks.get_mut(prior.chunk_index))
                                        .expect("prior bucket exists")
                                        .pipelines
                                        .abandon_unsubmitted_batch(active);
                                }
                            }
                            return Err(map_gpu_contract_error(error.gpu_error()));
                        }
                    }
                    dispatches[index].batch = Some(active)
                }
                Err(error) => {
                    for prior in &mut dispatches[..index] {
                        if let Some(active) = prior.batch.take() {
                            let _ = self
                                .class_buckets
                                .get_mut(&prior.class_id)
                                .and_then(|pool| pool.chunks.get_mut(prior.chunk_index))
                                .expect("prior bucket exists")
                                .pipelines
                                .abandon_unsubmitted_batch(active);
                        }
                    }
                    return Err(map_gpu_contract_error(error));
                }
            }
        }

        if capture_selector_diagnostics {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.enable_completed = true;
            }
        }

        if capture_selector_diagnostics {
            for dispatch in &mut dispatches {
                let bytes = dispatch
                    .batch
                    .as_ref()
                    .expect("begun batch")
                    .selector_diagnostic_bytes();
                let bytes = match bytes {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                            capture.later_stage_receipt =
                                GpuRuntimeSelectorDiagnosticFailureReceipt::from_gpu_error(
                                    error,
                                    dispatch.class_id,
                                    dispatch.chunk_index,
                                );
                        }
                        return Err(map_gpu_contract_error(error));
                    }
                };
                dispatch.selector_readback =
                    Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("closed-loop-selector-diagnostic-readback"),
                        size: bytes,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    }));
            }
        }

        for index in 0..dispatches.len() {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::WriteStagedUploads);
            }
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("prepared bucket exists");
            if let Err(error) = bucket.pipelines.write_staged_uploads(
                &self.queue,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("begun batch"),
            ) {
                self.cleanup_unsubmitted_dispatches(&mut dispatches);
                return Err(map_gpu_contract_error(error));
            }
        }
        self.perception_upload_count = next_upload_count;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-runtime-mixed-class-tick"),
            });
        {
            let _timestamp_start = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-runtime-timestamp-start"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timestamp_resources.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: None,
                }),
            });
        }
        for index in 0..dispatches.len() {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::RecordDispatch);
            }
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("begun bucket exists");
            let result = match dispatch.selector_readback.as_ref() {
                Some(readback) => bucket
                    .pipelines
                    .record_staged_closed_loop_with_selector_diagnostics(
                        &mut encoder,
                        &bucket.buffers,
                        dispatch.batch.as_ref().expect("begun batch"),
                        readback,
                    ),
                None => bucket.pipelines.record_staged_closed_loop(
                    &mut encoder,
                    &bucket.buffers,
                    dispatch.batch.as_ref().expect("begun batch"),
                ),
            };
            if let Err(error) = result {
                self.cleanup_unsubmitted_dispatches(&mut dispatches);
                return Err(map_gpu_contract_error(error));
            }
            dispatch.recorded = true;
        }
        {
            let _timestamp_end = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-runtime-timestamp-end"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timestamp_resources.query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(1),
                }),
            });
        }
        encoder.resolve_query_set(
            &self.timestamp_resources.query_set,
            0..GPU_TIMESTAMP_QUERY_COUNT,
            &self.timestamp_resources.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timestamp_resources.resolve_buffer,
            0,
            &self.timestamp_resources.readback_buffer,
            0,
            GPU_TIMESTAMP_READBACK_BYTES,
        );
        let command_buffer = encoder.finish();
        for index in 0..dispatches.len() {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::RegisterCompactMapping);
            }
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("recorded bucket exists");
            match bucket.pipelines.register_compact_mapping(
                &command_buffer,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("recorded batch"),
            ) {
                Ok(ticket) => dispatch.map_ticket = Some(ticket),
                Err(error) => {
                    self.cleanup_unsubmitted_dispatches(&mut dispatches);
                    return Err(map_gpu_contract_error(error));
                }
            }
            if let Some(readback) = dispatch.selector_readback.as_ref() {
                if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                    capture.later_stage =
                        Some(GpuRuntimeSelectorDiagnosticStage::RegisterSelectorDiagnosticMapping);
                }
                match bucket.pipelines.register_selector_diagnostic_mapping(
                    &command_buffer,
                    readback,
                    dispatch.batch.as_ref().expect("recorded batch"),
                ) {
                    Ok(ticket) => dispatch.selector_map_ticket = Some(ticket),
                    Err(error) => {
                        self.cleanup_unsubmitted_dispatches(&mut dispatches);
                        return Err(map_gpu_contract_error(error));
                    }
                }
            }
        }
        let (timestamp_sender, timestamp_receiver) = std::sync::mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &self.timestamp_resources.readback_buffer,
            wgpu::MapMode::Read,
            0..GPU_TIMESTAMP_READBACK_BYTES,
            move |result| {
                let _ = timestamp_sender.send(result);
            },
        );
        let submission = self.queue.submit(Some(command_buffer));
        let forced_loss = std::mem::take(&mut self.force_device_lost_after_submit);
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::DevicePoll);
        }
        let poll_failed = self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err();
        let mappings_succeeded = dispatches.iter_mut().all(|dispatch| {
            dispatch
                .map_ticket
                .take()
                .is_some_and(GpuCompactMapTicket::mapping_succeeded)
        });
        let selector_mappings_succeeded = dispatches.iter_mut().all(|dispatch| {
            dispatch.selector_readback.is_none()
                || dispatch
                    .selector_map_ticket
                    .take()
                    .is_some_and(GpuCompactMapTicket::mapping_succeeded)
        });
        let timestamp_mapping_succeeded = timestamp_mapping_completed(&timestamp_receiver);
        let post_submit_failure_stage = if forced_loss {
            Some(GpuRuntimeSelectorDiagnosticStage::DeviceLostAfterSubmit)
        } else if poll_failed {
            Some(GpuRuntimeSelectorDiagnosticStage::DevicePoll)
        } else if !mappings_succeeded {
            Some(GpuRuntimeSelectorDiagnosticStage::CompactMappingCompletion)
        } else if !selector_mappings_succeeded {
            Some(GpuRuntimeSelectorDiagnosticStage::SelectorMappingCompletion)
        } else if !timestamp_mapping_succeeded {
            Some(GpuRuntimeSelectorDiagnosticStage::TimestampMappingCompletion)
        } else if self.device_lost.load(Ordering::Acquire) {
            Some(GpuRuntimeSelectorDiagnosticStage::DeviceLostAfterSubmit)
        } else {
            None
        };
        if let Some(stage) = post_submit_failure_stage {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(stage);
            }
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("submitted bucket exists");
                bucket.buffers.compact_readback().unmap();
                if let Some(readback) = dispatch.selector_readback.as_ref() {
                    readback.unmap();
                }
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.timestamp_resources.readback_buffer.unmap();
            self.mark_device_lost();
            return Err(
                if stage == GpuRuntimeSelectorDiagnosticStage::TimestampMappingCompletion {
                    ScaffoldContractError::GpuTimestampQueryUnavailable
                } else {
                    ScaffoldContractError::NeuralBackendUnavailable
                },
            );
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::TimestampReadback);
        }
        let (inference_timestamp_ticks, completed_gpu_time_ns) =
            match self.timestamp_resources.read_delta_and_elapsed_ns() {
                Ok(timing) => timing,
                Err(error) => {
                    for dispatch in &dispatches {
                        self.class_buckets
                            .get(&dispatch.class_id)
                            .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                        if let Some(readback) = dispatch.selector_readback.as_ref() {
                            readback.unmap();
                        }
                    }
                    self.poison_submitted_dispatches(&dispatches);
                    return Err(error);
                }
            };

        if capture_selector_diagnostics {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::DecodeSelectorDiagnostics);
            }
            for index in 0..dispatches.len() {
                let result = {
                    let dispatch = &dispatches[index];
                    let bucket = self
                        .class_buckets
                        .get(&dispatch.class_id)
                        .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                        .expect("mapped bucket exists");
                    bucket.pipelines.decode_mapped_selector_diagnostics(
                        dispatch
                            .selector_readback
                            .as_ref()
                            .expect("diagnostic capture was requested"),
                        dispatch.batch.as_ref().expect("mapped batch"),
                    )
                };
                match result {
                    Ok(captures) => dispatches[index].selector_captures = Some(captures),
                    Err(_) => {
                        for still_mapped in &dispatches[index + 1..] {
                            still_mapped
                                .selector_readback
                                .as_ref()
                                .expect("diagnostic capture was requested")
                                .unmap();
                        }
                        for submitted in &dispatches {
                            let bucket = self
                                .class_buckets
                                .get_mut(&submitted.class_id)
                                .and_then(|pool| pool.chunks.get_mut(submitted.chunk_index))
                                .expect("submitted bucket exists");
                            bucket.buffers.compact_readback().unmap();
                            let _ = bucket.pipelines.mark_post_submit_poison(
                                submitted.batch.as_ref().expect("submitted batch"),
                            );
                        }
                        self.mark_device_lost();
                        return Err(ScaffoldContractError::NeuralBackendUnavailable);
                    }
                }
            }
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::DecodeMappedRecords);
        }
        for index in 0..dispatches.len() {
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("mapped bucket exists");
            let mut decode_diagnostic = PipelineDecodeMappedRecordsDiagnostic::default();
            let decoded = if selector_diagnostic_error_capture.is_some() {
                bucket
                    .pipelines
                    .decode_validate_mapped_records_with_diagnostic(
                        &bucket.buffers,
                        dispatch.batch.as_ref().expect("mapped batch"),
                        &mut decode_diagnostic,
                    )
            } else {
                bucket.pipelines.decode_validate_mapped_records(
                    &bucket.buffers,
                    dispatch.batch.as_ref().expect("mapped batch"),
                )
            };
            match decoded {
                Ok(validated) => dispatches[index].validated = Some(validated),
                Err(error) => {
                    if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                        capture.decode_mapped_records_receipt =
                            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt::from_gpu_error(
                                error,
                                dispatch.class_id,
                                dispatch.chunk_index,
                                decode_diagnostic,
                            );
                    }
                    for still_mapped in &dispatches[index + 1..] {
                        self.class_buckets
                            .get(&still_mapped.class_id)
                            .and_then(|pool| pool.chunks.get(still_mapped.chunk_index))
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                    }
                    for submitted in &dispatches {
                        let bucket = self
                            .class_buckets
                            .get_mut(&submitted.class_id)
                            .and_then(|pool| pool.chunks.get_mut(submitted.chunk_index))
                            .expect("submitted bucket exists");
                        let _ = bucket.pipelines.mark_post_submit_poison(
                            submitted.batch.as_ref().expect("submitted batch"),
                        );
                    }
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            }
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::PrevalidateCommit);
        }
        if dispatches.iter().any(|dispatch| {
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("validated bucket exists");
            bucket
                .pipelines
                .prevalidate_commit_validated_batch(
                    dispatch.validated.as_ref().expect("validated batch"),
                )
                .is_err()
        }) {
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("validated bucket exists");
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        let mut ordered_records = vec![None; batch.len()];
        let mut ordered_speech_payloads = vec![None; batch.len()];
        let mut ordered_factorized_motor_candidates =
            vec![[0_u16; crate::GPU_MOTOR_CHANNEL_SLOT_COUNT]; batch.len()];
        let mut ordered_pending_receipts = vec![None; batch.len()];
        let mut ordered_pending_records = vec![None; batch.len()];
        let mut ordered_next_transaction_generations = vec![None; batch.len()];
        let mut ordered_memory_receipts = vec![None; batch.len()];
        let mut ordered_selector_diagnostics = vec![None; batch.len()];
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ValidateReceiptIdentity);
        }
        let receipt_validation = (|| -> Result<(), ScaffoldContractError> {
            for dispatch in &dispatches {
                let validated = dispatch.validated.as_ref().expect("validated batch");
                let memory_bindings = dispatch
                    .batch
                    .as_ref()
                    .expect("validated batch retains its upload")
                    .memory_context_bindings();
                for (
                    (
                        (((original_index, selection), speech_payload), motor_candidates),
                        pending_record,
                    ),
                    memory_binding,
                ) in dispatch
                    .original_indices
                    .iter()
                    .zip(validated.records())
                    .zip(validated.speech_payloads())
                    .zip(validated.factorized_motor_candidates())
                    .zip(validated.pending_records())
                    .zip(memory_bindings)
                {
                    if selection.status != 1 {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    let input = batch[*original_index];
                    let handle = input.handle;
                    let frame = input.frame;
                    let candidate_index = u16::try_from(selection.candidate_index)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    let candidate = frame
                        .candidates()
                        .get(candidate_index as usize)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    let receipt = PendingEligibilityReceipt::from_gpu_record(
                        *pending_record,
                        handle.slot,
                        handle.organism_id,
                        handle.phenotype_hash,
                    )?;
                    let identity = receipt.identity();
                    let resident = self
                        .class_buckets
                        .get(&handle.class_id.raw())
                        .and_then(|pool| pool.resident(handle).ok())
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    if identity.handle_generation() != handle.generation
                        || identity.dispatch_generation() != dispatch_generation.get()
                        || identity.originating_tick() != frame.tick()
                        || identity.frame_digest() != frame.frame_digest()
                        || u32::from(identity.active_activation_side())
                            != selection.active_activation_side
                        || identity.candidate_index() != candidate_index
                        || identity.action_id() != candidate.action_id
                        || identity.action_family() != candidate.family
                        || identity.candidate_feature_digest() != candidate.feature_digest()?
                        || identity.active_eligibility_generation()
                            != resident.active_eligibility_generation
                        || identity.staging_eligibility_generation()
                            != resident
                                .active_eligibility_generation
                                .checked_add(1)
                                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    match (input.memory_upload, *memory_binding) {
                        (None, None) => {}
                        (Some(_), Some(memory_receipt))
                            if memory_receipt.slot == handle.slot
                                && memory_receipt.slot_generation == handle.generation
                                && memory_receipt.base_frame_digest == frame.base_digest()
                                && memory_receipt.context_digest
                                    == frame.context().canonical_digest()
                                && memory_receipt.final_frame_digest == frame.frame_digest()
                                && usize::from(memory_receipt.candidate_count)
                                    == frame.candidates().len() =>
                        {
                            ordered_memory_receipts[*original_index] = Some(memory_receipt);
                        }
                        _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
                    }
                    let next_transaction_generation = resident
                        .transaction_generation
                        .checked_add(1)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    ordered_records[*original_index] = Some(*selection);
                    ordered_speech_payloads[*original_index] = speech_payload.clone();
                    ordered_factorized_motor_candidates[*original_index] = *motor_candidates;
                    ordered_pending_receipts[*original_index] = Some(receipt);
                    ordered_pending_records[*original_index] = Some(*pending_record);
                    ordered_next_transaction_generations[*original_index] =
                        Some(next_transaction_generation);
                }
            }
            Ok(())
        })();
        if receipt_validation.is_err() {
            self.poison_submitted_dispatches(&dispatches);
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        if capture_selector_diagnostics {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::BuildSelectorDiagnostic);
            }
            let mut failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::MissingCapture;
            let mut binding_identity_failure = None;
            let selector_validation = (|| -> Result<(), ScaffoldContractError> {
                for dispatch in &dispatches {
                    failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::MissingCapture;
                    let captures = dispatch
                        .selector_captures
                        .as_ref()
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::CaptureCount;
                    if captures.len() != dispatch.original_indices.len() {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    for (original_index, capture) in dispatch.original_indices.iter().zip(captures)
                    {
                        let input = batch[*original_index];
                        failure_field =
                            GpuRuntimeSelectorDiagnosticBuildFailureField::MissingSelectionRecord;
                        let record = ordered_records[*original_index]
                            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                        failure_field =
                            GpuRuntimeSelectorDiagnosticBuildFailureField::ChosenCandidateIndex;
                        let chosen = u16::try_from(record.candidate_index)
                            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                        failure_field =
                            GpuRuntimeSelectorDiagnosticBuildFailureField::ResidentBrainOwnership;
                        let resident = self
                            .class_buckets
                            .get(&input.handle.class_id.raw())
                            .and_then(|pool| pool.resident(input.handle).ok())
                            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                        ordered_selector_diagnostics[*original_index] =
                            Some(build_selector_diagnostic(
                                input.frame,
                                &resident.phenotype,
                                &resident.brain_slot,
                                resident.active_weight_bank,
                                dispatch_generation.get(),
                                chosen,
                                capture,
                                &mut failure_field,
                                &mut binding_identity_failure,
                            )?);
                    }
                }
                Ok(())
            })();
            if let Err(error) = selector_validation {
                let class = match error {
                    ScaffoldContractError::InvalidDecisionEvidence => {
                        Some(GpuRuntimeSelectorDiagnosticBuildFailureClass::InvalidDecisionEvidence)
                    }
                    ScaffoldContractError::BrainOwnershipMismatch => {
                        Some(GpuRuntimeSelectorDiagnosticBuildFailureClass::BrainOwnershipMismatch)
                    }
                    _ => None,
                };
                if let (Some(capture), Some(class)) =
                    (selector_diagnostic_error_capture.as_deref_mut(), class)
                {
                    capture.build_selector_diagnostic_receipt =
                        Some(GpuRuntimeSelectorDiagnosticBuildFailureReceipt {
                            class,
                            field: failure_field,
                            expected_binding_identity: binding_identity_failure
                                .map(|(expected, _)| expected),
                            actual_binding_identity: binding_identity_failure
                                .map(|(_, actual)| actual),
                        });
                }
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::AccountActivityWork);
        }
        let activity_work_receipts: Result<Vec<BrainWorkReceipt>, ScaffoldContractError> = batch
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let handle = input.handle;
                let resident = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let decision = &activity_decisions[index];
                let candidate_count = u32::try_from(input.frame.candidates().len())
                    .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)?;
                let memory_context_count = input
                    .memory_upload
                    .map_or(0, |upload| upload.header.candidate_count);
                let work = derive_executed_work(
                    &resident.phenotype,
                    decision.microsteps,
                    &decision.enabled_route_ids,
                    candidate_count,
                    memory_context_count,
                )?;
                let record =
                    ordered_records[index].ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                if work.tile_visits != u64::from(record.active_tiles)
                    || work.synapse_ops != u64::from(record.active_synapses)
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                BrainWorkReceipt::try_new(
                    &self.activity_policy,
                    decision,
                    work,
                    resident.brain_atp_q16,
                )
            })
            .collect();
        let activity_work_receipts = match activity_work_receipts {
            Ok(receipts) => receipts,
            Err(error) => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(error);
            }
        };

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::PrepareTicks);
        }
        let prepared_ticks = (|| -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
            let mut ticks = Vec::with_capacity(batch.len());
            for (index, input) in batch.iter().enumerate() {
                let handle = input.handle;
                let frame = input.frame;
                let record =
                    ordered_records[index].ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let pending_eligibility = ordered_pending_receipts[index]
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let candidate_index = u16::try_from(record.candidate_index)
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                let candidate = frame
                    .candidates()
                    .get(candidate_index as usize)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let v11_work = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .v11
                    .gpu_recurrent_work_receipt(
                        record.dendritic_branches_evaluated,
                        record.dendritic_inputs_evaluated,
                        record.dendritic_gated_branches,
                        record.structural_edges_evaluated,
                    )?;
                ticks.push(GpuClosedLoopTick {
                    handle,
                    dispatch_generation: dispatch_generation.get(),
                    base_digest: frame.base_digest(),
                    frame_digest: frame.frame_digest(),
                    memory_context_binding: ordered_memory_receipts[index],
                    active_activation_side: u8::try_from(record.active_activation_side)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
                    selection: NeuralActionSelection {
                        candidate_index,
                        logit: f32::from_bits(record.logit_bits),
                        confidence: Confidence::new(candidate.sensor_confidence.raw())?,
                        active_tiles: record.active_tiles,
                        active_synapses: record.active_synapses,
                    },
                    speech_payload: ordered_speech_payloads[index].clone(),
                    factorized_motor_candidates: ordered_factorized_motor_candidates[index],
                    pending_eligibility,
                    pressure: activity_decisions[index].pressure,
                    throttle: activity_decisions[index].clone(),
                    work: activity_work_receipts[index].clone(),
                    v11_work,
                    compact_readback_bytes: crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
                    hardware_receipt_generation: self.hardware.generation,
                    selector_diagnostic: ordered_selector_diagnostics[index].clone(),
                });
            }
            Ok(ticks)
        })();
        let prepared_ticks = match prepared_ticks {
            Ok(ticks) => ticks,
            Err(_) => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ComputeReadbackBytes);
        }
        let total_readback_bytes = match batch
            .len()
            .checked_mul(crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES)
        {
            Some(bytes) => bytes,
            None => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        let mut commit_mismatch = false;
        for dispatch in &mut dispatches {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::CommitValidatedBatch);
            }
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("validated bucket exists");
            let commit = bucket
                .pipelines
                .commit_validated_batch(dispatch.validated.take().expect("validated batch"));
            let committed = match commit {
                Ok(committed) => committed,
                Err(_) => {
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            };
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ValidateCommitShape);
            }
            if committed.readback_bytes as usize
                != dispatch.original_indices.len() * crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES
                || committed.records.len() != dispatch.original_indices.len()
                || committed.speech_payloads.len() != dispatch.original_indices.len()
                || committed.factorized_motor_candidates.len() != dispatch.original_indices.len()
                || committed.pending_records.len() != dispatch.original_indices.len()
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::ValidateCommitContents);
            }
            for ((((original_index, record), speech_payload), motor_candidates), pending_record) in
                dispatch
                    .original_indices
                    .iter()
                    .zip(committed.records)
                    .zip(committed.speech_payloads)
                    .zip(committed.factorized_motor_candidates)
                    .zip(committed.pending_records)
            {
                commit_mismatch |= ordered_records[*original_index] != Some(record)
                    || ordered_speech_payloads[*original_index] != speech_payload
                    || ordered_factorized_motor_candidates[*original_index] != motor_candidates
                    || ordered_pending_records[*original_index] != Some(pending_record);
            }
        }
        if commit_mismatch {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ValidateHostPrecommit);
        }
        let host_precommit_valid = batch.iter().enumerate().all(|(index, input)| {
            let handle = input.handle;
            let Some(expected_generation) = ordered_next_transaction_generations[index] else {
                return false;
            };
            ordered_pending_receipts[index].is_some()
                && ordered_pending_records[index].is_some()
                && self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .is_some_and(|resident| {
                        resident.pending_eligibility.is_none()
                            && resident.pending_eligibility_record.is_none()
                            && resident.activity_sequence_cursor
                                == activity_decisions[index].sequence_cursor
                            && resident.brain_atp_q16
                                == activity_work_receipts[index].atp_before_q16
                            && activity_work_receipts[index]
                                .validate_for(&self.activity_policy, &activity_decisions[index])
                                .is_ok()
                            && resident.transaction_generation.checked_add(1)
                                == Some(expected_generation)
                    })
        });
        if !host_precommit_valid {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        for (index, input) in batch.iter().enumerate() {
            let handle = input.handle;
            let resident = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.resident_mut(handle).ok())
                .expect("host pending commit was prevalidated");
            resident.transaction_generation = ordered_next_transaction_generations[index]
                .expect("host transaction generation was prevalidated");
            resident.logical_dispatch_generation = dispatch_generation.get();
            resident.activity_sequence_cursor = resident
                .activity_sequence_cursor
                .checked_add(1)
                .expect("activity cursor was prevalidated");
            resident.brain_atp_q16 = activity_work_receipts[index].atp_after_q16;
            resident.last_activity_dispatch_generation = dispatch_generation.get();
            resident.last_activity_frame_digest = input.frame.frame_digest().0;
            resident.last_completed_gpu_time_ns = completed_gpu_time_ns;
            resident.last_pressure = Some(activity_decisions[index].pressure);
            resident.last_throttle = Some(activity_decisions[index].clone());
            resident.last_work = Some(activity_work_receipts[index].clone());
            resident
                .v11
                .record_gpu_recurrent_work(prepared_ticks[index].v11_work);
            resident.pending_eligibility = ordered_pending_receipts[index];
            resident.pending_eligibility_record = ordered_pending_records[index];
        }

        self.completed_dispatch_count = next_completed_dispatch_count;
        self.last_compact_readback_bytes = total_readback_bytes;
        self.next_dispatch_generation = next_dispatch_generation;
        self.completed_selection_count = next_completed_selection_count;
        self.completed_neural_timing = None;
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ConvertPopulation);
        }
        self.pending_inference_timing = Some(PendingInferenceTiming {
            dispatch_generation: dispatch_generation.get(),
            class_id_raw: timing_class_id,
            population: u32::try_from(batch.len())
                .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?,
            inference_timestamp_ticks,
        });
        Ok(prepared_ticks)
    }

    fn current_admission_snapshot(&self) -> Result<GpuAdmissionReceipt, ScaffoldContractError> {
        let mut logical_committed_bytes = 0_u64;
        let mut physical_allocated_bytes = 0_u64;
        let mut physical_unused_retained_bytes = 0_u64;
        let mut physical_shared_bytes = 0_u64;
        let mut physical_alignment_slack_bytes = 0_u64;
        let mut live_brains = 0_u32;
        for pool in self.class_buckets.values() {
            for bucket in &pool.chunks {
                let receipt = bucket
                    .plan
                    .slot_allocation_receipt()
                    .map_err(map_gpu_contract_error)?;
                let live = u64::try_from(bucket.slots.iter().filter(|slot| slot.is_some()).count())
                    .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
                let slots = u64::from(bucket.plan.slot_capacity());
                let unused = slots
                    .checked_sub(live)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                logical_committed_bytes = logical_committed_bytes
                    .checked_add(
                        receipt
                            .logical_slot_commit_bytes
                            .checked_mul(live)
                            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_unused_retained_bytes = physical_unused_retained_bytes
                    .checked_add(
                        receipt
                            .logical_slot_commit_bytes
                            .checked_mul(unused)
                            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_shared_bytes = physical_shared_bytes
                    .checked_add(receipt.shared_class_bytes)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_alignment_slack_bytes = physical_alignment_slack_bytes
                    .checked_add(
                        receipt
                            .alignment_padding_bytes
                            .checked_mul(slots)
                            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_allocated_bytes = physical_allocated_bytes
                    .checked_add(bucket.plan.aggregate_resident_bytes())
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                live_brains = live_brains
                    .checked_add(
                        u32::try_from(live)
                            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            }
        }
        let logical_available_bytes = self
            .runtime_budget
            .logical_neural_heap_budget_bytes
            .checked_sub(logical_committed_bytes)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let receipt = GpuAdmissionReceipt {
            schema_version: 1,
            runtime: self.runtime_budget,
            logical_committed_bytes,
            logical_available_bytes,
            physical_allocated_bytes,
            physical_unused_retained_bytes,
            physical_shared_bytes,
            physical_alignment_slack_bytes,
            peak_logical_committed_bytes: self
                .admission
                .peak_logical_committed_bytes
                .max(logical_committed_bytes),
            peak_physical_allocated_bytes: self
                .admission
                .peak_physical_allocated_bytes
                .max(physical_allocated_bytes),
            live_brains,
            max_hot_brains: self.runtime_budget.max_hot_brains,
            allocation_generation: 0,
            last_event: None,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    fn commit_admission_event(
        &mut self,
        kind: GpuAllocationEventKind,
        handle: GpuBrainHandle,
        transient_peak_physical_bytes: u64,
    ) -> Result<(), ScaffoldContractError> {
        let before = self.admission.clone();
        let mut after = self.current_admission_snapshot()?;
        after.allocation_generation = before
            .allocation_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        after.peak_logical_committed_bytes = before
            .peak_logical_committed_bytes
            .max(after.logical_committed_bytes);
        after.peak_physical_allocated_bytes = before
            .peak_physical_allocated_bytes
            .max(after.physical_allocated_bytes)
            .max(transient_peak_physical_bytes);
        after.last_event = Some(GpuAllocationEventReceipt::new(
            kind,
            handle.class_id.raw(),
            handle.slot,
            handle.generation,
            &before,
            &after,
        )?);
        after.validate_contract()?;
        self.admission = after;
        Ok(())
    }

    fn validate_logical_admission(
        &self,
        slot_receipt: &crate::GpuSlotAllocationReceipt,
    ) -> Result<(), ScaffoldContractError> {
        if self.admission.live_brains >= self.runtime_budget.max_hot_brains
            || self
                .admission
                .logical_committed_bytes
                .checked_add(slot_receipt.logical_slot_commit_bytes)
                .is_none_or(|bytes| bytes > self.runtime_budget.logical_neural_heap_budget_bytes)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub fn insert_brain(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.insert_brain_inner(organism_id, phenotype, false)
    }

    /// Research-only insertion used by sealed, equivalence-checked growth.
    pub fn insert_research_brain(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.insert_brain_inner(organism_id, phenotype, true)
    }

    pub const fn curated_residency_generation(&self) -> u64 {
        self.curated_residency_generation
    }

    pub const fn curated_residency_generation_fingerprint(&self) -> [u64; 4] {
        self.curated_residency_generation_fingerprint
    }

    /// Atomically replaces the active GPU cohort from one already-bound,
    /// caller-ordered plan. The port owns all GPU staging and host cutover;
    /// this method never routes through per-brain removal or insertion.
    pub fn replace_curated_cohort(
        &mut self,
        cohort: &GpuCuratedResidencyCohort,
    ) -> GpuCuratedResidencyOutcome {
        if let Err(error) = self.ensure_ready() {
            self.mark_device_lost();
            return GpuCuratedResidencyOutcome::Unknown {
                error,
                fail_stop: true,
            };
        }
        let mut port = GpuCuratedResidencyBackendPort::new(self);
        run_curated_residency_transaction(&mut port, cohort)
    }

    fn insert_brain_inner(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
        allow_research: bool,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.ensure_ready()?;
        organism_id
            .validate()
            .map_err(|_| ScaffoldContractError::BrainOwnershipMismatch)?;
        if self.organisms.contains_key(&organism_id.0) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let class_id = phenotype.brain_class_id();
        let capacity = if allow_research {
            if class_id != BrainCapacityClass::N4096_RESEARCH_ID {
                return Err(ScaffoldContractError::UnsupportedProductionBrainClass);
            }
            capacity_for_gpu_class(class_id)?
        } else {
            capacity_for_promoted_class(class_id)?
        };
        validate_required_gpu_layout_version(u32::from(capacity.execution().gpu_layout_version()))?;
        phenotype
            .validate_against(&capacity)
            .map_err(|_| ScaffoldContractError::GpuLayoutMismatch)?;
        self.runtime_budget.validate_for(capacity.execution())?;
        crate::GpuClassBucketPlan::validate_adapter(&phenotype, &self.runtime_budget)
            .map_err(map_gpu_contract_error)?;
        let slot_receipt = GpuFixedClassArenaPlan::new(
            capacity,
            1,
            self.runtime_budget.physical_allocation_ceiling_bytes,
        )
        .map_err(map_gpu_contract_error)?
        .slot_allocation_receipt()
        .map_err(map_gpu_contract_error)?;
        self.validate_logical_admission(&slot_receipt)?;
        let class_raw = class_id.raw();
        let current_physical = self.admission.physical_allocated_bytes;
        let reusable = self
            .class_buckets
            .get(&class_raw)
            .and_then(|pool| pool.reusable_slot(class_raw, &self.slot_generation_watermarks));
        let (chunk_index, slot, generation, upload, event_kind, transient_peak_physical_bytes) =
            if let Some((chunk_index, slot, generation)) = reusable {
                let bucket = self
                    .class_buckets
                    .get(&class_raw)
                    .and_then(|pool| pool.chunks.get(chunk_index))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let upload = bucket
                    .plan
                    .prepare_slot_upload(slot, generation, &phenotype)
                    .map_err(map_gpu_contract_error)?;
                (
                    chunk_index,
                    slot,
                    generation,
                    upload,
                    GpuAllocationEventKind::AdmitFromRetainedSlot,
                    current_physical,
                )
            } else {
                let remaining_hot = self
                    .runtime_profile
                    .max_hot_brains
                    .checked_sub(self.admission.live_brains)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let slot_capacity =
                    u32::from(self.runtime_profile.growth_chunk_slots).min(remaining_hot);
                if slot_capacity == 0 {
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
                let plan = GpuFixedClassArenaPlan::new(
                    capacity,
                    slot_capacity,
                    self.runtime_budget.physical_allocation_ceiling_bytes,
                )
                .map_err(map_gpu_contract_error)?;
                let transient_peak = current_physical
                    .checked_add(plan.aggregate_resident_bytes())
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                if transient_peak > self.runtime_budget.physical_allocation_ceiling_bytes {
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
                let mut bucket =
                    ClassBucketRuntime::from_plan(&self.device, Arc::clone(&self.kernels), plan)
                        .map_err(map_gpu_contract_error)?;
                for candidate_slot in 0..slot_capacity {
                    if let Some(previous) = self
                        .slot_generation_watermarks
                        .get(&(class_raw, candidate_slot))
                        .copied()
                    {
                        bucket.generations[candidate_slot as usize] = previous;
                        if previous == u32::MAX {
                            bucket.retired.insert(candidate_slot);
                            bucket
                                .free_slots
                                .retain(|free_slot| *free_slot != candidate_slot);
                        }
                    }
                }
                let slot = *bucket
                    .free_slots
                    .last()
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let generation = self
                    .slot_generation_watermarks
                    .get(&(class_raw, slot))
                    .copied()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let upload = bucket
                    .plan
                    .prepare_slot_upload(slot, generation, &phenotype)
                    .map_err(map_gpu_contract_error)?;
                let pool = self.class_buckets.entry(class_raw).or_default();
                let chunk_index = pool.chunks.len();
                pool.chunks.push(bucket);
                (
                    chunk_index,
                    slot,
                    generation,
                    upload,
                    GpuAllocationEventKind::AdmitFromNewChunk,
                    transient_peak,
                )
            };
        let v11 = GpuV11CausalState::for_phenotype(&phenotype)?;
        let upload = {
            let bucket = self
                .class_buckets
                .get(&class_raw)
                .and_then(|pool| pool.chunks.get(chunk_index))
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            compile_v11_slot_upload(&bucket.plan, upload.brain_slot(), &phenotype, &v11)
                .map_err(map_gpu_contract_error)?
        };
        let bucket = self
            .class_buckets
            .get_mut(&class_raw)
            .and_then(|pool| pool.chunks.get_mut(chunk_index))
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        bucket
            .buffers
            .write_slot_upload(&self.queue, &upload)
            .map_err(map_gpu_contract_error)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-runtime-slot-initialize"),
            });
        bucket
            .buffers
            .record_mutable_slot_reset(&mut encoder, upload.ranges())
            .map_err(map_gpu_contract_error)?;
        let submission = self.queue.submit(Some(encoder.finish()));
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || self.device_lost.load(Ordering::Acquire)
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        bucket
            .buffers
            .write_mutable_slot_upload(&self.queue, &upload)
            .map_err(map_gpu_contract_error)?;
        let initialization_submission = self.queue.submit(std::iter::empty());
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(initialization_submission),
                timeout: None,
            })
            .is_err()
            || self.device_lost.load(Ordering::Acquire)
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let handle = GpuBrainHandle {
            backend_instance_id: self.backend_instance_id,
            class_id,
            slot,
            generation,
            organism_id,
            phenotype_hash: phenotype.phenotype_hash(),
        };
        let popped = bucket.free_slots.pop();
        debug_assert_eq!(popped, Some(slot));
        bucket.generations[slot as usize] = generation;
        bucket.slots[slot as usize] = Some(ResidentBrainSlot {
            ownership: GpuBrainSlotOwnership {
                organism_id,
                phenotype_hash: phenotype.phenotype_hash(),
                sensor_profile: phenotype.sensor_profile(),
            },
            phenotype: phenotype.clone(),
            brain_slot: upload.brain_slot().clone(),
            ranges: upload.ranges().clone(),
            active_eligibility_generation: 1,
            active_eligibility_bank: 0,
            active_weight_bank: 0,
            active_weight_generation: 1,
            replay_journal_generation: 1,
            transaction_generation: 1,
            logical_dispatch_generation: self.next_dispatch_generation,
            activity_sequence_cursor: 1,
            brain_atp_q16: BRAIN_ATP_Q16_MAX,
            last_world_atp_tick: None,
            last_activity_dispatch_generation: 0,
            last_activity_frame_digest: [0; 4],
            last_completed_gpu_time_ns: 0,
            last_pressure: None,
            last_throttle: None,
            last_work: None,
            v11,
            sleep_plan: *phenotype.sleep_consolidation_plan(),
            learning_sequence_guard: LearningSequenceGuard::new(
                organism_id,
                phenotype.phenotype_hash(),
            ),
            pending_eligibility: None,
            pending_eligibility_record: None,
        });
        self.slot_generation_watermarks
            .insert((class_raw, slot), generation);
        self.organisms.insert(organism_id.0, handle);
        if self
            .commit_admission_event(event_kind, handle, transient_peak_physical_bytes)
            .is_err()
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(handle)
    }

    pub fn rebind_brain_for_restore(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.insert_brain(organism_id, phenotype)
    }

    pub fn remove_brain(&mut self, handle: GpuBrainHandle) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let class_raw = handle.class_id.raw();
        let transient_peak_physical_bytes = self.admission.physical_allocated_bytes;
        let chunk_index = self
            .class_buckets
            .get(&class_raw)
            .and_then(|pool| pool.bucket_index_for_handle(handle))
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        {
            let bucket = self
                .class_buckets
                .get_mut(&class_raw)
                .and_then(|pool| pool.chunks.get_mut(chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = bucket.slots[handle.slot as usize]
                .as_ref()
                .expect("validated occupied slot");
            if resident.pending_eligibility.is_some()
                || resident.pending_eligibility_record.is_some()
            {
                return Err(ScaffoldContractError::LearningReplayRejected);
            }
            let ranges = resident.ranges.clone();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("closed-loop-runtime-slot-scrub"),
                });
            bucket
                .buffers
                .record_full_slot_scrub(&mut encoder, &ranges)
                .map_err(map_gpu_contract_error)?;
            let submission = self.queue.submit(Some(encoder.finish()));
            if self
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .is_err()
                || self.device_lost.load(Ordering::Acquire)
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            if bucket
                .pipelines
                .retire_slot_active_side(handle.slot, handle.generation)
                .is_err()
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            bucket.slots[handle.slot as usize] = None;
            if handle.generation == u32::MAX {
                bucket.retired.insert(handle.slot);
            } else {
                bucket.free_slots.push(handle.slot);
            }
        }
        self.organisms.remove(&handle.organism_id.0);
        let drop_empty_chunk = self.runtime_profile.retain_empty_chunks == 0
            && self
                .class_buckets
                .get(&class_raw)
                .and_then(|pool| pool.chunks.get(chunk_index))
                .is_some_and(|bucket| bucket.slots.iter().all(Option::is_none));
        let event_kind = if drop_empty_chunk {
            let pool = self
                .class_buckets
                .get_mut(&class_raw)
                .expect("validated class pool exists");
            pool.chunks.remove(chunk_index);
            if pool.chunks.is_empty() {
                self.class_buckets.remove(&class_raw);
            }
            GpuAllocationEventKind::ReleaseAndDropEmptyChunk
        } else {
            GpuAllocationEventKind::ReleaseToRetainedSlot
        };
        if self
            .commit_admission_event(event_kind, handle, transient_peak_physical_bytes)
            .is_err()
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub(crate) fn ensure_ready(&mut self) -> Result<(), ScaffoldContractError> {
        if self.device_lost.load(Ordering::Acquire) {
            self.mark_device_lost();
        }
        if matches!(self.state, GpuBackendState::Ready) {
            Ok(())
        } else {
            Err(ScaffoldContractError::NeuralBackendUnavailable)
        }
    }

    pub(crate) fn validate_handle_backend(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<(), ScaffoldContractError> {
        if handle.backend_instance_id == self.backend_instance_id {
            Ok(())
        } else {
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        }
    }

    pub(crate) fn mark_device_lost(&mut self) {
        self.state = GpuBackendState::DeviceLost {
            last_checkpoint_digest: None,
        };
    }

    fn poison_submitted_dispatches(&mut self, dispatches: &[PreparedClassDispatch]) {
        for dispatch in dispatches {
            if let Some(batch) = dispatch.batch.as_ref() {
                if let Some(bucket) = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                {
                    let _ = bucket.pipelines.mark_post_submit_poison(batch);
                }
            }
        }
        self.mark_device_lost();
    }

    fn cleanup_unsubmitted_dispatches(&mut self, dispatches: &mut [PreparedClassDispatch]) {
        for dispatch in dispatches.iter_mut() {
            if dispatch.recorded {
                if let Some(batch) = dispatch.batch.as_ref() {
                    let _ = self
                        .class_buckets
                        .get_mut(&dispatch.class_id)
                        .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                        .expect("transaction bucket exists")
                        .pipelines
                        .rollback_recorded_batch(batch);
                }
                dispatch.recorded = false;
            }
        }
        for dispatch in dispatches.iter_mut() {
            if let Some(batch) = dispatch.batch.take() {
                let _ = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("transaction bucket exists")
                    .pipelines
                    .abandon_unsubmitted_batch(batch);
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn shared_resource_counts_for_test(&self) -> (usize, usize, usize) {
        let _ = (&self.adapter, &self.device, &self.queue);
        (1, 1, 1)
    }

    #[cfg(feature = "gpu-tests")]
    pub const fn shared_kernel_set_count_for_test(&self) -> usize {
        1
    }

    #[cfg(feature = "gpu-tests")]
    pub fn allocated_class_arena_count_for_test(&self) -> usize {
        self.class_buckets
            .values()
            .map(|pool| pool.chunks.len())
            .sum()
    }

    #[cfg(feature = "gpu-tests")]
    pub const fn runtime_counters_for_test(&self) -> (u64, u64, u64) {
        (
            self.completed_dispatch_count,
            self.perception_upload_count,
            self.completed_selection_count,
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn contains_organism_for_test(&self, organism_id: OrganismId) -> bool {
        self.organisms.contains_key(&organism_id.0)
    }

    #[cfg(feature = "gpu-tests")]
    pub const fn last_compact_readback_bytes_for_test(&self) -> usize {
        self.last_compact_readback_bytes
    }

    #[cfg(feature = "gpu-tests")]
    pub fn read_active_fast_weights_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let bucket = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .bucket_for_handle(handle)?;
        let resident = bucket
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let words = if resident.active_weight_bank == 0 {
            resident.ranges.layout.fast_weight_words.clone()
        } else {
            resident.ranges.layout.fast_weight_bank_1_words.clone()
        };
        let range_word_count = words
            .end
            .checked_sub(words.start)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let word_count = resident.brain_slot.record().synapse_count;
        if word_count == 0 || range_word_count < word_count {
            return Err(ScaffoldContractError::GpuLayoutMismatch);
        }
        let size = u64::from(word_count) * 4;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("closed-loop-test-active-fast-readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-test-active-fast-copy"),
            });
        encoder.copy_buffer_to_buffer(
            bucket.buffers.neural_buffers()[6],
            u64::from(words.start) * 4,
            &readback,
            0,
            size,
        );
        let command_buffer = encoder.finish();
        let (sender, receiver) = std::sync::mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &readback,
            wgpu::MapMode::Read,
            0..size,
            move |result| {
                let _ = sender.send(result);
            },
        );
        let submission = self.queue.submit(Some(command_buffer));
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || receiver.recv().ok().and_then(Result::ok).is_none()
        {
            readback.unmap();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let mapped = readback.slice(..size).get_mapped_range();
        let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
        drop(mapped);
        readback.unmap();
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        Ok(values)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_all_invalid_after_next_decode_for_test(&mut self, handle: GpuBrainHandle) {
        if handle.backend_instance_id == self.backend_instance_id {
            if let Some(bucket) = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.bucket_for_handle_mut(handle).ok())
            {
                bucket
                    .pipelines
                    .force_all_invalid_record_for_test(handle.slot, handle.generation);
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_pending_identity_mismatch_after_next_decode_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) {
        if handle.backend_instance_id == self.backend_instance_id {
            if let Some(bucket) = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.bucket_for_handle_mut(handle).ok())
            {
                bucket
                    .pipelines
                    .force_pending_identity_mismatch_for_test(handle.slot, handle.generation);
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_device_lost_after_next_submit_for_test(&mut self) {
        self.force_device_lost_after_submit = true;
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_activity_sequence_cursor_for_test(
        &mut self,
        handle: GpuBrainHandle,
        cursor: u64,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        pool.resident_mut(handle)?.activity_sequence_cursor = cursor;
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_learning_rejections_for_test(&mut self, rejection_count: u8) {
        self.forced_learning_rejections_remaining = rejection_count;
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_discard_rejections_for_test(&mut self, rejection_count: u8) {
        self.forced_discard_rejections_remaining = rejection_count;
    }
}

struct PreparedCuratedBackendEntry {
    handle: GpuBrainHandle,
    class_id: BrainClassId,
    chunk_index: usize,
    upload: GpuFixedSlotUpload,
    resident: ResidentBrainSlot,
}

struct GpuCuratedResidencyBackendPort<'a> {
    backend: &'a mut GpuClosedLoopBackend,
    encoder: Option<wgpu::CommandEncoder>,
    staging_buffers: Vec<wgpu::Buffer>,
    submitted: bool,
}

impl<'a> GpuCuratedResidencyBackendPort<'a> {
    fn new(backend: &'a mut GpuClosedLoopBackend) -> Self {
        Self {
            backend,
            encoder: None,
            staging_buffers: Vec::new(),
            submitted: false,
        }
    }

    fn ensure_encoder(&mut self) {
        if self.encoder.is_none() {
            self.encoder = Some(self.backend.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("closed-loop-runtime-curated-cohort-cutover"),
                },
            ));
        }
    }
}

impl CuratedResidencyTransactionPort for GpuCuratedResidencyBackendPort<'_> {
    type StagedEntry = PreparedCuratedBackendEntry;

    fn classify_pre_submit(&mut self, error: ScaffoldContractError) -> GpuCuratedResidencyOutcome {
        if self.backend.device_lost.load(Ordering::Acquire)
            || !matches!(self.backend.state, GpuBackendState::Ready)
        {
            curated_residency_unknown(self, error)
        } else {
            curated_residency_pre_submit(error)
        }
    }

    fn snapshot(&mut self) -> Result<CuratedResidencyPortSnapshot, ScaffoldContractError> {
        self.backend.ensure_ready()?;
        let capacity = BrainCapacityClass::n512();
        let slot_plan = GpuFixedClassArenaPlan::new(
            capacity,
            1,
            self.backend
                .runtime_budget
                .physical_allocation_ceiling_bytes,
        )
        .map_err(map_gpu_contract_error)?;
        let slot_receipt = slot_plan
            .slot_allocation_receipt()
            .map_err(map_gpu_contract_error)?;
        let mut slots = Vec::new();
        let mut old_residents = Vec::new();
        for (&class_raw, pool) in &self.backend.class_buckets {
            let class_id = BrainClassId(class_raw);
            for (chunk_index, bucket) in pool.chunks.iter().enumerate() {
                for (slot_index, resident) in bucket.slots.iter().enumerate() {
                    let slot = u32::try_from(slot_index)
                        .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
                    let watermark = self
                        .backend
                        .slot_generation_watermarks
                        .get(&(class_raw, slot))
                        .copied()
                        .unwrap_or_else(|| bucket.generations[slot_index]);
                    slots.push(CuratedResidencySlotState {
                        class_id,
                        chunk_index,
                        slot,
                        generation_watermark: watermark,
                        reserved_generation: 0,
                        occupied: resident.is_some(),
                        retired: bucket.retired.contains(&slot),
                    });
                    if let Some(resident) = resident {
                        let handle = GpuBrainHandle {
                            backend_instance_id: self.backend.backend_instance_id,
                            class_id,
                            slot,
                            generation: resident.brain_slot.record().slot_generation,
                            organism_id: resident.ownership.organism_id,
                            phenotype_hash: resident.ownership.phenotype_hash,
                        };
                        let foundation_hash = resident
                            .phenotype
                            .foundation_abi()
                            .foundation_payload_digest()
                            .unwrap_or_else(|| Blake3Digest::from_bytes([0; 32]));
                        old_residents.push(GpuCuratedResidentReceipt {
                            organism_id: resident.ownership.organism_id,
                            opaque_target_identity: GpuCuratedResidencyTargetIdentity::new(
                                resident.ownership.organism_id.raw(),
                            ),
                            exact_phenotype_hash: resident.ownership.phenotype_hash,
                            exact_foundation_hash: foundation_hash,
                            handle,
                        });
                    }
                }
            }
        }
        Ok(CuratedResidencyPortSnapshot {
            backend_instance_id: self.backend.backend_instance_id,
            generation: self.backend.curated_residency_generation,
            admission_generation: self.backend.admission.allocation_generation,
            live_brains: u32::try_from(old_residents.len())
                .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?,
            max_hot_brains: self.backend.runtime_budget.max_hot_brains,
            logical_committed_bytes: self.backend.admission.logical_committed_bytes,
            logical_budget_bytes: self.backend.runtime_budget.logical_neural_heap_budget_bytes,
            logical_slot_commit_bytes: slot_receipt.logical_slot_commit_bytes,
            physical_allocated_bytes: self.backend.admission.physical_allocated_bytes,
            transient_new_physical_bytes: 0,
            physical_ceiling_bytes: self
                .backend
                .runtime_budget
                .physical_allocation_ceiling_bytes,
            slots,
            old_residents,
            backend_hardware_generation: self.backend.hardware.generation,
        })
    }

    fn prepare_entry(
        &mut self,
        _entry_index: usize,
        entry: &GpuCuratedResidencyEntry,
        reservation: CuratedResidencySlotState,
    ) -> Result<Self::StagedEntry, ScaffoldContractError> {
        let class_raw = reservation.class_id.raw();
        let bucket = self
            .backend
            .class_buckets
            .get(&class_raw)
            .and_then(|pool| pool.chunks.get(reservation.chunk_index))
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if bucket
            .slots
            .get(reservation.slot as usize)
            .and_then(Option::as_ref)
            .is_some()
            || bucket.retired.contains(&reservation.slot)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let upload = bucket
            .plan
            .prepare_slot_upload(
                reservation.slot,
                reservation.reserved_generation,
                &entry.phenotype,
            )
            .map_err(map_gpu_contract_error)?;
        let v11 = GpuV11CausalState::for_phenotype(&entry.phenotype)?;
        let upload =
            compile_v11_slot_upload(&bucket.plan, upload.brain_slot(), &entry.phenotype, &v11)
                .map_err(map_gpu_contract_error)?;
        let handle = GpuBrainHandle {
            backend_instance_id: self.backend.backend_instance_id,
            class_id: reservation.class_id,
            slot: reservation.slot,
            generation: reservation.reserved_generation,
            organism_id: entry.organism_id,
            phenotype_hash: entry.exact_phenotype_hash,
        };
        let resident = ResidentBrainSlot {
            ownership: GpuBrainSlotOwnership {
                organism_id: entry.organism_id,
                phenotype_hash: entry.exact_phenotype_hash,
                sensor_profile: entry.phenotype.sensor_profile(),
            },
            phenotype: entry.phenotype.clone(),
            brain_slot: upload.brain_slot().clone(),
            ranges: upload.ranges().clone(),
            active_eligibility_generation: 1,
            active_eligibility_bank: 0,
            active_weight_bank: 0,
            active_weight_generation: 1,
            replay_journal_generation: 1,
            transaction_generation: 1,
            logical_dispatch_generation: self.backend.next_dispatch_generation,
            activity_sequence_cursor: 1,
            brain_atp_q16: BRAIN_ATP_Q16_MAX,
            last_world_atp_tick: None,
            last_activity_dispatch_generation: 0,
            last_activity_frame_digest: [0; 4],
            last_completed_gpu_time_ns: 0,
            last_pressure: None,
            last_throttle: None,
            last_work: None,
            v11,
            sleep_plan: *entry.phenotype.sleep_consolidation_plan(),
            learning_sequence_guard: LearningSequenceGuard::new(
                entry.organism_id,
                entry.exact_phenotype_hash,
            ),
            pending_eligibility: None,
            pending_eligibility_record: None,
        };
        Ok(PreparedCuratedBackendEntry {
            handle,
            class_id: reservation.class_id,
            chunk_index: reservation.chunk_index,
            upload,
            resident,
        })
    }

    fn record_old_slot_scrub(
        &mut self,
        resident: &GpuCuratedResidentReceipt,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_encoder();
        let class_raw = resident.handle.class_id().raw();
        let pool = self
            .backend
            .class_buckets
            .get(&class_raw)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let chunk_index = pool
            .bucket_index_for_handle(resident.handle)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let bucket = pool
            .chunks
            .get(chunk_index)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let slot = bucket
            .slots
            .get(resident.handle.slot() as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if slot.brain_slot.record().slot_generation != resident.handle.generation()
            || slot.ownership.organism_id != resident.handle.organism_id()
            || slot.ownership.phenotype_hash != resident.handle.phenotype_hash()
            || slot.pending_eligibility.is_some()
            || slot.pending_eligibility_record.is_some()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let ranges = slot.ranges.clone();
        let encoder = self.encoder.as_mut().expect("encoder was initialized");
        bucket
            .buffers
            .record_full_slot_scrub(encoder, &ranges)
            .map_err(map_gpu_contract_error)
    }

    fn record_new_slot_initialization(
        &mut self,
        staged: &Self::StagedEntry,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_encoder();
        let bucket = self
            .backend
            .class_buckets
            .get(&staged.class_id.raw())
            .and_then(|pool| pool.chunks.get(staged.chunk_index))
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        bucket
            .buffers
            .record_mutable_slot_reset(
                self.encoder.as_mut().expect("encoder was initialized"),
                staged.upload.ranges(),
            )
            .map_err(map_gpu_contract_error)?;
        bucket
            .buffers
            .record_slot_upload(
                &self.backend.device,
                self.encoder.as_mut().expect("encoder was initialized"),
                &staged.upload,
                &mut self.staging_buffers,
            )
            .map_err(map_gpu_contract_error)
    }

    fn submit_once(&mut self) -> Result<(), ScaffoldContractError> {
        if self.submitted {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let encoder = self
            .encoder
            .take()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        self.backend.queue.submit(Some(encoder.finish()));
        self.submitted = true;
        if self.backend.force_device_lost_after_submit {
            self.backend.force_device_lost_after_submit = false;
            self.backend.device_lost.store(true, Ordering::Release);
        }
        if self.backend.device_lost.load(Ordering::Acquire) {
            Err(ScaffoldContractError::NeuralBackendUnavailable)
        } else {
            Ok(())
        }
    }

    fn poll_completion(&mut self) -> Result<(), ScaffoldContractError> {
        if !self.submitted
            || self
                .backend
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .is_err()
            || self.backend.device_lost.load(Ordering::Acquire)
        {
            Err(ScaffoldContractError::NeuralBackendUnavailable)
        } else {
            Ok(())
        }
    }

    fn commit(
        &mut self,
        cohort: &GpuCuratedResidencyCohort,
        reservations: &[CuratedResidencySlotState],
        staged: Vec<Self::StagedEntry>,
        receipt: &GpuCuratedResidencyReceipt,
    ) -> Result<(), ScaffoldContractError> {
        if reservations.len() != staged.len()
            || staged.len() != receipt.ordered_residents.len()
            || receipt
                .ordered_residents
                .iter()
                .zip(staged.iter())
                .any(|(receipt_row, staged)| receipt_row.handle != staged.handle)
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        for (reservation, prepared) in reservations.iter().zip(staged.iter()) {
            let bucket = self
                .backend
                .class_buckets
                .get(&prepared.class_id.raw())
                .and_then(|pool| pool.chunks.get(prepared.chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if reservation.class_id != prepared.class_id
                || reservation.chunk_index != prepared.chunk_index
                || bucket
                    .slots
                    .get(reservation.slot as usize)
                    .and_then(Option::as_ref)
                    .is_some()
                || bucket.retired.contains(&reservation.slot)
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
        }

        let mut old_locations = Vec::new();
        for (&class_raw, pool) in &self.backend.class_buckets {
            for (chunk_index, bucket) in pool.chunks.iter().enumerate() {
                for (slot_index, resident) in bucket.slots.iter().enumerate() {
                    if let Some(resident) = resident {
                        let slot = u32::try_from(slot_index)
                            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
                        old_locations.push((
                            BrainClassId(class_raw),
                            chunk_index,
                            slot,
                            resident.brain_slot.record().slot_generation,
                            resident.ownership.organism_id,
                        ));
                    }
                }
            }
        }
        for (class_id, chunk_index, slot, generation, _) in &old_locations {
            let bucket = self
                .backend
                .class_buckets
                .get_mut(&class_id.raw())
                .and_then(|pool| pool.chunks.get_mut(*chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if bucket
                .pipelines
                .retire_slot_active_side(*slot, *generation)
                .is_err()
            {
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        }
        for (class_id, chunk_index, slot, generation, organism_id) in &old_locations {
            let bucket = self
                .backend
                .class_buckets
                .get_mut(&class_id.raw())
                .and_then(|pool| pool.chunks.get_mut(*chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            bucket.slots[*slot as usize] = None;
            if *generation == u32::MAX {
                bucket.retired.insert(*slot);
            } else {
                bucket.free_slots.push(*slot);
            }
            self.backend.organisms.remove(&organism_id.raw());
        }

        self.backend.organisms.clear();
        for (reservation, prepared) in reservations.iter().zip(staged.into_iter()) {
            {
                let bucket = self
                    .backend
                    .class_buckets
                    .get_mut(&prepared.class_id.raw())
                    .and_then(|pool| pool.chunks.get_mut(prepared.chunk_index))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                bucket.free_slots.retain(|slot| *slot != reservation.slot);
                bucket.generations[reservation.slot as usize] = reservation.reserved_generation;
                bucket.slots[reservation.slot as usize] = Some(prepared.resident);
            }
            self.backend.slot_generation_watermarks.insert(
                (prepared.class_id.raw(), reservation.slot),
                reservation.reserved_generation,
            );
            self.backend
                .organisms
                .insert(prepared.handle.organism_id().raw(), prepared.handle);
        }
        self.backend.curated_residency_generation = self
            .backend
            .curated_residency_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        self.backend.curated_residency_generation_fingerprint = cohort.new_generation_fingerprint;
        let first_handle = receipt
            .ordered_residents
            .first()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .handle;
        self.backend.commit_admission_event(
            GpuAllocationEventKind::AdmitFromRetainedSlot,
            first_handle,
            self.backend.admission.physical_allocated_bytes,
        )?;
        Ok(())
    }

    fn mark_unknown(&mut self) {
        self.backend.mark_device_lost();
    }
}

#[allow(clippy::infallible_destructuring_match)]
fn acquire_required_gpu(
    factory: &impl GpuDeviceFactory,
) -> Result<RequiredGpuDevice, ScaffoldContractError> {
    let required_features = required_device_features();
    let required_limits = required_device_limits();
    let mut found_base_compatible_without_timestamps = false;
    for candidate in factory.request_adapters()? {
        let adapter = match candidate {
            GpuAdapterCandidate::Hardware(adapter) => adapter,
            #[cfg(test)]
            GpuAdapterCandidate::Software => continue,
        };
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu
            || info.backend == wgpu::Backend::Noop
            || backend_slug(info.backend).is_err()
            || !required_limits.check_limits(&adapter.limits())
        {
            continue;
        }
        if validate_required_device_features(adapter.features()).is_err() {
            found_base_compatible_without_timestamps = true;
            continue;
        }
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("alife-required-closed-loop-device"),
            required_features,
            required_limits: required_limits.clone(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        };
        let Ok((device, queue)) = factory.request_device(&adapter, &descriptor) else {
            continue;
        };
        let lost = Arc::new(AtomicBool::new(false));
        let callback_lost = Arc::clone(&lost);
        device.set_device_lost_callback(move |_reason, _message| {
            callback_lost.store(true, Ordering::Release);
        });
        let hardware = build_hardware_receipt(
            &info,
            required_features,
            device.features(),
            &device.limits(),
        )?;
        return Ok(RequiredGpuDevice {
            adapter,
            device,
            queue,
            hardware,
            lost,
        });
    }
    Err(if found_base_compatible_without_timestamps {
        ScaffoldContractError::GpuTimestampQueryUnavailable
    } else {
        ScaffoldContractError::NeuralBackendUnavailable
    })
}

fn required_device_features() -> wgpu::Features {
    wgpu::Features::TIMESTAMP_QUERY
}

fn validate_required_device_features(
    available: wgpu::Features,
) -> Result<(), ScaffoldContractError> {
    let required = required_device_features();
    if REQUIRED_GPU_FEATURE_MASK != 1 {
        return Err(ScaffoldContractError::GpuLayoutMismatch);
    }
    if available.contains(required) {
        Ok(())
    } else {
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable)
    }
}

fn required_device_limits() -> wgpu::Limits {
    let mut required = wgpu::Limits::downlevel_defaults();
    required.max_storage_buffers_per_shader_stage =
        required.max_storage_buffers_per_shader_stage.max(10);
    required.max_buffer_size = required.max_buffer_size.max(268_435_456);
    required.max_storage_buffer_binding_size =
        required.max_storage_buffer_binding_size.max(134_217_728);
    required
}

#[cfg(test)]
struct UnavailableGpuFactory;

#[cfg(test)]
impl GpuDeviceFactory for UnavailableGpuFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    }

    fn request_device(
        &self,
        _adapter: &wgpu::Adapter,
        _descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
        unreachable!("unavailable adapter must stop before device request")
    }
}

#[cfg(test)]
#[derive(Default)]
struct SoftwareAdapterGpuFactory {
    device_requests: std::cell::Cell<u32>,
}

#[cfg(test)]
impl SoftwareAdapterGpuFactory {
    fn device_request_count(&self) -> u32 {
        self.device_requests.get()
    }
}

#[cfg(test)]
impl GpuDeviceFactory for SoftwareAdapterGpuFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
        Ok(vec![GpuAdapterCandidate::Software])
    }

    fn request_device(
        &self,
        _adapter: &wgpu::Adapter,
        _descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
        self.device_requests.set(self.device_requests.get() + 1);
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    }
}

#[cfg(test)]
#[derive(Clone)]
struct RuntimeArenaFixtureSlot {
    generation: u32,
    owner: Option<(OrganismId, PhenotypeHash)>,
    retired: bool,
    ranges: Vec<Vec<u32>>,
}

#[cfg(test)]
struct RuntimeArenaTestHarness {
    backend_instance_id: NonZeroU64,
    class_id: BrainClassId,
    slots: Vec<RuntimeArenaFixtureSlot>,
    state: GpuBackendState,
    fail_next_scrub: bool,
}

#[cfg(test)]
impl RuntimeArenaTestHarness {
    fn n512(slot_count: usize) -> Self {
        Self {
            backend_instance_id: NonZeroU64::new(1).unwrap(),
            class_id: BrainClassId(1),
            slots: vec![
                RuntimeArenaFixtureSlot {
                    generation: 0,
                    owner: None,
                    retired: false,
                    ranges: (0..9).map(|_| vec![0; 8]).collect(),
                };
                slot_count
            ],
            state: GpuBackendState::Ready,
            fail_next_scrub: false,
        }
    }

    fn insert_fixture(
        &mut self,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        let index = self
            .slots
            .iter()
            .position(|slot| slot.owner.is_none() && !slot.retired)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next = self.slots[index]
            .generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        self.insert_at(index, organism_id, phenotype_hash, next)
    }

    fn insert_fixture_with_generation(
        &mut self,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
        generation: u32,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        let index = self
            .slots
            .iter()
            .position(|slot| slot.owner.is_none() && !slot.retired)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        self.insert_at(index, organism_id, phenotype_hash, generation)
    }

    fn rebind_fixture_for_restore(
        &mut self,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        if self
            .slots
            .iter()
            .any(|slot| slot.owner.is_some_and(|owner| owner.0 == organism_id))
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        self.insert_fixture(organism_id, phenotype_hash)
    }

    fn insert_at(
        &mut self,
        index: usize,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
        generation: u32,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        if !matches!(self.state, GpuBackendState::Ready) || generation == 0 {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let slot = &mut self.slots[index];
        if slot.owner.is_some() || slot.retired {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        slot.generation = generation;
        slot.owner = Some((organism_id, phenotype_hash));
        Ok(GpuBrainHandle {
            backend_instance_id: self.backend_instance_id,
            class_id: self.class_id,
            slot: index as u32,
            generation,
            organism_id,
            phenotype_hash,
        })
    }

    fn fill_every_reserved_range(&mut self, handle: GpuBrainHandle, value: u32) {
        assert!(self.owns(handle));
        for range in &mut self.slots[handle.slot as usize].ranges {
            range.fill(value);
        }
    }

    fn remove_fixture(&mut self, handle: GpuBrainHandle) -> Result<(), ScaffoldContractError> {
        if !matches!(self.state, GpuBackendState::Ready) || !self.owns(handle) {
            return Err(if matches!(self.state, GpuBackendState::Ready) {
                ScaffoldContractError::BrainOwnershipMismatch
            } else {
                ScaffoldContractError::NeuralBackendUnavailable
            });
        }
        if self.fail_next_scrub {
            self.fail_next_scrub = false;
            self.state = GpuBackendState::DeviceLost {
                last_checkpoint_digest: None,
            };
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let slot = &mut self.slots[handle.slot as usize];
        for range in &mut slot.ranges {
            range.fill(0);
        }
        slot.owner = None;
        if slot.generation == u32::MAX {
            slot.retired = true;
        }
        Ok(())
    }

    fn every_reserved_range_is_zero(&self, slot: u32) -> bool {
        self.slots[slot as usize]
            .ranges
            .iter()
            .flatten()
            .all(|word| *word == 0)
    }

    fn slot_is_permanently_retired(&self, slot: u32) -> bool {
        self.slots[slot as usize].retired
    }

    fn free_slot_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.owner.is_none() && !slot.retired)
            .count()
    }

    fn fail_next_scrub_after_submit(&mut self) {
        self.fail_next_scrub = true;
    }

    fn state(&self) -> &GpuBackendState {
        &self.state
    }

    fn owns(&self, handle: GpuBrainHandle) -> bool {
        handle.backend_instance_id == self.backend_instance_id
            && handle.class_id == self.class_id
            && self.slots.get(handle.slot as usize).is_some_and(|slot| {
                slot.generation == handle.generation
                    && slot.owner == Some((handle.organism_id, handle.phenotype_hash))
            })
    }

    fn validate_frame_organism(
        &self,
        handle: GpuBrainHandle,
        organism_id: OrganismId,
    ) -> Result<(), ScaffoldContractError> {
        if self.owns(handle) && handle.organism_id == organism_id {
            Ok(())
        } else {
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct RuntimePreflightTestHarness {
    allocated_arenas: usize,
    counters: (u64, u64, u64),
}

#[cfg(test)]
impl RuntimePreflightTestHarness {
    fn validate_class(&mut self, class_id: BrainClassId) -> Result<(), ScaffoldContractError> {
        match class_id.raw() {
            1..=3 => Ok(()),
            _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass),
        }
    }

    fn allocated_arena_count(&self) -> usize {
        self.allocated_arenas
    }

    fn runtime_counters(&self) -> (u64, u64, u64) {
        self.counters
    }

    fn perception_upload_count(&self) -> u64 {
        self.counters.1
    }

    fn validate_frame_digest(
        &mut self,
        expected: PerceptionFrameDigest,
        actual: PerceptionFrameDigest,
    ) -> Result<(), ScaffoldContractError> {
        if expected == actual {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidPerceptionFrame)
        }
    }
}

#[cfg(test)]
#[path = "../tests/support/closed_loop_runtime_private.rs"]
mod task7_private_tests;

#[cfg(test)]
mod staging_backend_tests {
    use super::*;

    #[test]
    fn staging_state_plan_separates_identity_and_resets_accounting() {
        with_runtime_allocation_state_for_test(41, 77, || {
            let live = new_ephemeral_backend_state_plan().expect("live plan is valid");
            let staging = new_ephemeral_backend_state_plan().expect("staging plan is valid");

            assert_ne!(live.backend_instance_id, staging.backend_instance_id);
            assert_eq!(staging.state, GpuBackendState::Ready);
            assert_eq!(staging.next_dispatch_generation, 1);
            assert_eq!(staging.next_sleep_job_id, 1);
            assert_eq!(staging.curated_residency_generation, 0);
            assert!(staging.recorded_pressure_replay_empty);
        });
    }
}

#[cfg(test)]
mod selector_diagnostic_binding_tests {
    use super::*;

    #[test]
    fn selector_binding_uses_global_family_start_and_local_weight_index() {
        let binding = selector_diagnostic_family_binding_offsets(24_576, 108_370, 25_344)
            .expect("measured N2048 binding is valid");

        assert_eq!(binding, (25_344, 111_442));
    }
}

#[cfg(test)]
mod curated_founder_gpu_cutover_tests {
    use super::*;
    use alife_core::{
        BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler,
        Tick,
    };

    #[test]
    fn curated_founder_gpu_cutover_is_atomic_and_consumes_bound_projection() {
        let mut backend = CuratedResidencyTestBackend::seeded();
        let cohort = backend.ordered_cohort();
        let old_snapshot = backend.snapshot();

        backend.fail_preparation_at(1);
        let first_attempt = backend.replace_curated_cohort(&cohort);
        assert!(matches!(
            first_attempt,
            GpuCuratedResidencyOutcome::PreSubmitFailure {
                retryable: true,
                ..
            }
        ));
        assert_eq!(backend.snapshot(), old_snapshot);
        assert!(backend.snapshot().has_one_generation_only());
        assert_eq!(backend.port().submit_count(), 0);
        assert_eq!(
            backend.port().trace,
            vec![
                TestCuratedResidencyTrace::Prepare(0),
                TestCuratedResidencyTrace::Prepare(1),
            ]
        );

        backend.clear_preparation_failure();
        backend.clear_trace();
        let committed = backend.replace_curated_cohort(&cohort);
        let receipt = match committed {
            GpuCuratedResidencyOutcome::Committed(receipt) => receipt,
            other => panic!("expected committed curated cohort, got {other:?}"),
        };

        assert_eq!(
            receipt.generation_fingerprint,
            cohort.new_generation_fingerprint
        );
        assert!(receipt.submission_completed);
        assert_eq!(receipt.backend_hardware_generation, 77);
        assert_eq!(receipt.ordered_residents.len(), 2);
        assert_eq!(backend.port().submit_count(), 1);
        assert_eq!(backend.port().poll_count(), 1);
        assert_eq!(
            backend.port().trace,
            vec![
                TestCuratedResidencyTrace::Prepare(0),
                TestCuratedResidencyTrace::Prepare(1),
                TestCuratedResidencyTrace::Scrub(0),
                TestCuratedResidencyTrace::Scrub(1),
                TestCuratedResidencyTrace::Initialize(0),
                TestCuratedResidencyTrace::Initialize(1),
                TestCuratedResidencyTrace::Submit,
                TestCuratedResidencyTrace::Poll,
                TestCuratedResidencyTrace::Commit,
            ]
        );

        for (index, (entry, resident)) in cohort
            .ordered_entries
            .iter()
            .zip(receipt.ordered_residents.iter())
            .enumerate()
        {
            assert_eq!(resident.organism_id, entry.organism_id);
            assert_eq!(
                resident.opaque_target_identity,
                entry.opaque_target_identity
            );
            assert_eq!(resident.exact_phenotype_hash, entry.exact_phenotype_hash);
            assert_eq!(resident.exact_foundation_hash, entry.exact_foundation_hash);
            assert_eq!(resident.handle.organism_id(), entry.organism_id);
            assert_eq!(resident.handle.phenotype_hash(), entry.exact_phenotype_hash);
            assert_eq!(resident.handle.slot(), 2 + index as u32);
            assert_eq!(resident.handle.generation(), 1);
        }
        assert!(backend.snapshot().has_one_generation_only());
        assert_eq!(backend.snapshot(), old_snapshot);
    }

    fn test_phenotype(_seed: u64) -> BrainPhenotype {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(0x4E35_3132_5F00_0001, capacity.id());
        let development = DevelopmentState::new(
            genome.id,
            Tick::ZERO,
            NormalizedScalar::new(1.0).expect("fixture maturation is valid"),
        );
        let foundation =
            FoundationWeightAsset::builtin_nano512_v1(SensorProfile::PrivilegedAffordanceV1)
                .expect("fixture foundation is valid");
        PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &capacity,
            &development,
            SensorProfile::PrivilegedAffordanceV1,
            &foundation,
        )
        .expect("fixture phenotype is valid")
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestSlot {
        generation_watermark: u32,
        owner: Option<OrganismId>,
        retired: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestSnapshot {
        generation_fingerprint: [u64; 4],
        admission_generation: u64,
        residents: Vec<GpuCuratedResidentReceipt>,
        slots: Vec<TestSlot>,
        fail_stop: bool,
    }

    impl TestSnapshot {
        fn has_one_generation_only(&self) -> bool {
            self.residents
                .iter()
                .map(|resident| resident.handle.generation())
                .collect::<BTreeSet<_>>()
                .len()
                <= 1
        }
    }

    #[derive(Debug, Clone)]
    struct TestCuratedResidencyPort {
        backend_instance_id: NonZeroU64,
        class_id: BrainClassId,
        generation: u64,
        generation_fingerprint: [u64; 4],
        admission_generation: u64,
        residents: Vec<GpuCuratedResidentReceipt>,
        slots: Vec<TestSlot>,
        fail_preparation_at: Option<usize>,
        trace: Vec<TestCuratedResidencyTrace>,
        fail_stop: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestCuratedResidencyTrace {
        Prepare(usize),
        Scrub(u32),
        Initialize(usize),
        Submit,
        Poll,
        Commit,
    }

    struct CuratedResidencyTestBackend {
        port: TestCuratedResidencyPort,
    }

    impl CuratedResidencyTestBackend {
        fn seeded() -> Self {
            let class_id = BrainCapacityClass::N512_ID;
            let phenotype_one = test_phenotype(1);
            let phenotype_two = test_phenotype(2);
            let foundation_one = phenotype_one
                .foundation_abi()
                .foundation_payload_digest()
                .expect("fixture foundation digest");
            let foundation_two = phenotype_two
                .foundation_abi()
                .foundation_payload_digest()
                .expect("fixture foundation digest");
            assert_eq!(foundation_one, foundation_two);
            let first_handle = GpuBrainHandle {
                backend_instance_id: NonZeroU64::new(41).unwrap(),
                class_id,
                slot: 0,
                generation: 1,
                organism_id: OrganismId(9001),
                phenotype_hash: phenotype_one.phenotype_hash(),
            };
            let second_handle = GpuBrainHandle {
                backend_instance_id: NonZeroU64::new(41).unwrap(),
                class_id,
                slot: 1,
                generation: 1,
                organism_id: OrganismId(9002),
                phenotype_hash: phenotype_two.phenotype_hash(),
            };
            let residents = vec![
                GpuCuratedResidentReceipt {
                    organism_id: OrganismId(9001),
                    opaque_target_identity: GpuCuratedResidencyTargetIdentity::new(7001),
                    exact_phenotype_hash: phenotype_one.phenotype_hash(),
                    exact_foundation_hash: foundation_one,
                    handle: first_handle,
                },
                GpuCuratedResidentReceipt {
                    organism_id: OrganismId(9002),
                    opaque_target_identity: GpuCuratedResidencyTargetIdentity::new(7002),
                    exact_phenotype_hash: phenotype_two.phenotype_hash(),
                    exact_foundation_hash: foundation_two,
                    handle: second_handle,
                },
            ];
            Self {
                port: TestCuratedResidencyPort {
                    backend_instance_id: NonZeroU64::new(41).unwrap(),
                    class_id,
                    generation: 1,
                    generation_fingerprint: [1, 2, 3, 4],
                    admission_generation: 1,
                    residents,
                    slots: vec![
                        TestSlot {
                            generation_watermark: 1,
                            owner: Some(OrganismId(9001)),
                            retired: false,
                        },
                        TestSlot {
                            generation_watermark: 1,
                            owner: Some(OrganismId(9002)),
                            retired: false,
                        },
                        TestSlot {
                            generation_watermark: 0,
                            owner: None,
                            retired: false,
                        },
                        TestSlot {
                            generation_watermark: 0,
                            owner: None,
                            retired: false,
                        },
                    ],
                    fail_preparation_at: None,
                    trace: Vec::new(),
                    fail_stop: false,
                },
            }
        }

        fn ordered_cohort(&self) -> GpuCuratedResidencyCohort {
            let first = test_phenotype(11);
            let second = test_phenotype(12);
            let foundation = first
                .foundation_abi()
                .foundation_payload_digest()
                .expect("fixture foundation digest");
            assert_eq!(
                Some(foundation),
                second.foundation_abi().foundation_payload_digest()
            );
            GpuCuratedResidencyCohort {
                expected_old_generation: self.port.generation,
                new_generation_fingerprint: [5, 6, 7, 8],
                ordered_entries: vec![
                    GpuCuratedResidencyEntry {
                        organism_id: OrganismId(101),
                        opaque_target_identity: GpuCuratedResidencyTargetIdentity::new(201),
                        exact_phenotype_hash: first.phenotype_hash(),
                        exact_foundation_hash: foundation,
                        phenotype: first,
                    },
                    GpuCuratedResidencyEntry {
                        organism_id: OrganismId(102),
                        opaque_target_identity: GpuCuratedResidencyTargetIdentity::new(202),
                        exact_phenotype_hash: second.phenotype_hash(),
                        exact_foundation_hash: foundation,
                        phenotype: second,
                    },
                ],
            }
        }

        fn fail_preparation_at(&mut self, entry_index: usize) {
            self.port.fail_preparation_at = Some(entry_index);
        }

        fn clear_preparation_failure(&mut self) {
            self.port.fail_preparation_at = None;
        }

        fn clear_trace(&mut self) {
            self.port.trace.clear();
        }

        fn replace_curated_cohort(
            &mut self,
            cohort: &GpuCuratedResidencyCohort,
        ) -> GpuCuratedResidencyOutcome {
            run_curated_residency_transaction(&mut self.port, cohort)
        }

        fn snapshot(&self) -> TestSnapshot {
            TestSnapshot {
                generation_fingerprint: self.port.generation_fingerprint,
                admission_generation: self.port.admission_generation,
                residents: self.port.residents.clone(),
                slots: self.port.slots.clone(),
                fail_stop: self.port.fail_stop,
            }
        }

        fn port(&self) -> &TestCuratedResidencyPort {
            &self.port
        }
    }

    impl TestCuratedResidencyPort {
        fn snapshot(&self) -> Result<CuratedResidencyPortSnapshot, ScaffoldContractError> {
            Ok(CuratedResidencyPortSnapshot {
                backend_instance_id: self.backend_instance_id,
                generation: self.generation,
                admission_generation: self.admission_generation,
                live_brains: self.residents.len() as u32,
                max_hot_brains: self.slots.len() as u32,
                logical_committed_bytes: self.residents.len() as u64 * 10,
                logical_budget_bytes: self.slots.len() as u64 * 10,
                logical_slot_commit_bytes: 10,
                physical_allocated_bytes: 100,
                transient_new_physical_bytes: 0,
                physical_ceiling_bytes: 100,
                slots: self
                    .slots
                    .iter()
                    .enumerate()
                    .map(|(slot, state)| CuratedResidencySlotState {
                        class_id: self.class_id,
                        chunk_index: 0,
                        slot: slot as u32,
                        generation_watermark: state.generation_watermark,
                        reserved_generation: 0,
                        occupied: state.owner.is_some(),
                        retired: state.retired,
                    })
                    .collect(),
                old_residents: self.residents.clone(),
                backend_hardware_generation: 77,
            })
        }

        fn submit_count(&self) -> usize {
            self.trace
                .iter()
                .filter(|event| matches!(event, TestCuratedResidencyTrace::Submit))
                .count()
        }

        fn poll_count(&self) -> usize {
            self.trace
                .iter()
                .filter(|event| matches!(event, TestCuratedResidencyTrace::Poll))
                .count()
        }
    }

    impl CuratedResidencyTransactionPort for TestCuratedResidencyPort {
        type StagedEntry = usize;

        fn snapshot(&mut self) -> Result<CuratedResidencyPortSnapshot, ScaffoldContractError> {
            TestCuratedResidencyPort::snapshot(self)
        }

        fn prepare_entry(
            &mut self,
            entry_index: usize,
            _entry: &GpuCuratedResidencyEntry,
            _reservation: CuratedResidencySlotState,
        ) -> Result<Self::StagedEntry, ScaffoldContractError> {
            self.trace
                .push(TestCuratedResidencyTrace::Prepare(entry_index));
            if self.fail_preparation_at == Some(entry_index) {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            Ok(entry_index)
        }

        fn record_old_slot_scrub(
            &mut self,
            resident: &GpuCuratedResidentReceipt,
        ) -> Result<(), ScaffoldContractError> {
            self.trace
                .push(TestCuratedResidencyTrace::Scrub(resident.handle.slot()));
            Ok(())
        }

        fn record_new_slot_initialization(
            &mut self,
            staged: &Self::StagedEntry,
        ) -> Result<(), ScaffoldContractError> {
            self.trace
                .push(TestCuratedResidencyTrace::Initialize(*staged));
            Ok(())
        }

        fn submit_once(&mut self) -> Result<(), ScaffoldContractError> {
            self.trace.push(TestCuratedResidencyTrace::Submit);
            Ok(())
        }

        fn poll_completion(&mut self) -> Result<(), ScaffoldContractError> {
            self.trace.push(TestCuratedResidencyTrace::Poll);
            Ok(())
        }

        fn commit(
            &mut self,
            _cohort: &GpuCuratedResidencyCohort,
            _reservations: &[CuratedResidencySlotState],
            _staged: Vec<Self::StagedEntry>,
            _receipt: &GpuCuratedResidencyReceipt,
        ) -> Result<(), ScaffoldContractError> {
            self.trace.push(TestCuratedResidencyTrace::Commit);
            Ok(())
        }

        fn mark_unknown(&mut self) {
            self.fail_stop = true;
        }
    }
}
