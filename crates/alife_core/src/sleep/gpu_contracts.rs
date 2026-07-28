//! Engine-neutral GPU sleep transaction identities and durable progress state.

use core::num::NonZeroU64;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    BrainClassId, CanonicalDigestBuilder, OrganismId, PhenotypeHash, ScaffoldContractError,
    Validate,
};

pub const GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION: u16 = 1;
const REQUEST_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-REQUEST-V1";
const STAGING_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-STAGING-V1";
const INPUT_WEIGHT_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-INPUT-V1";
const OUTPUT_WEIGHT_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-OUTPUT-V1";
const MUTABLE_STATE_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-MUTABLE-STATE-V1";
const COMMIT_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-COMMIT-V1";

/// Canonical digest of the complete resolved active lifetime/fast weight banks.
///
/// This helper performs validation and serialization only. It deliberately has
/// no weight-update formula and never accepts allocation-local GPU offsets.
pub fn compute_gpu_sleep_input_weight_digest(
    phenotype_hash: PhenotypeHash,
    class_id: BrainClassId,
    active_generation: u64,
    active_weight_bank: u8,
    lifetime: &[f32],
    fast: &[f32],
) -> Result<[u64; 4], ScaffoldContractError> {
    compute_weight_digest(
        INPUT_WEIGHT_DIGEST_DOMAIN,
        phenotype_hash,
        class_id,
        active_generation,
        active_weight_bank,
        lifetime,
        fast,
    )
}

/// Canonical digest of the complete resolved inactive banks produced by one
/// staged GPU consolidation transaction.
pub fn compute_gpu_sleep_output_weight_digest(
    phenotype_hash: PhenotypeHash,
    class_id: BrainClassId,
    output_generation: u64,
    output_weight_bank: u8,
    lifetime: &[f32],
    fast: &[f32],
) -> Result<[u64; 4], ScaffoldContractError> {
    compute_weight_digest(
        OUTPUT_WEIGHT_DIGEST_DOMAIN,
        phenotype_hash,
        class_id,
        output_generation,
        output_weight_bank,
        lifetime,
        fast,
    )
}

/// Canonical digest of the complete post-commit mutable GPU slot image.
pub fn compute_gpu_sleep_mutable_state_digest(words: &[u32]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(MUTABLE_STATE_DIGEST_DOMAIN);
    digest.write_sequence_len(words.len());
    for word in words {
        digest.write_u32(*word);
    }
    digest.finish256()
}

/// Canonical final receipt digest for an exactly-once sleep commit.
#[allow(clippy::too_many_arguments)]
pub fn compute_gpu_sleep_commit_digest(
    staging_digest: [u64; 4],
    phenotype_hash: PhenotypeHash,
    organism_id: OrganismId,
    cycle_id: u64,
    active_weight_generation: u64,
    active_weight_bank: u8,
    active_eligibility_generation: u64,
    active_eligibility_bank: u8,
    replay_journal_generation: u64,
    replay_journal_cursor: u32,
    replay_journal_event_count: u32,
    post_commit_mutable_state_digest: [u64; 4],
) -> Result<[u64; 4], ScaffoldContractError> {
    organism_id.validate()?;
    if staging_digest == [0; 4]
        || phenotype_hash == PhenotypeHash([0; 4])
        || cycle_id == 0
        || active_weight_generation == 0
        || active_weight_bank > 1
        || active_eligibility_generation == 0
        || active_eligibility_bank > 1
        || replay_journal_generation == 0
        || post_commit_mutable_state_digest == [0; 4]
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let mut digest = CanonicalDigestBuilder::new(COMMIT_DIGEST_DOMAIN);
    write_digest(&mut digest, staging_digest);
    write_digest(&mut digest, phenotype_hash.0);
    digest.write_u64(organism_id.raw());
    digest.write_u64(cycle_id);
    digest.write_u64(active_weight_generation);
    digest.write_u8(active_weight_bank);
    digest.write_u64(active_eligibility_generation);
    digest.write_u8(active_eligibility_bank);
    digest.write_u64(replay_journal_generation);
    digest.write_u32(replay_journal_cursor);
    digest.write_u32(replay_journal_event_count);
    write_digest(&mut digest, post_commit_mutable_state_digest);
    Ok(digest.finish256())
}

fn compute_weight_digest(
    domain: &[u8],
    phenotype_hash: PhenotypeHash,
    class_id: BrainClassId,
    generation: u64,
    weight_bank: u8,
    lifetime: &[f32],
    fast: &[f32],
) -> Result<[u64; 4], ScaffoldContractError> {
    if phenotype_hash == PhenotypeHash([0; 4])
        || generation == 0
        || weight_bank > 1
        || lifetime.is_empty()
        || lifetime.len() != fast.len()
        || u32::try_from(lifetime.len()).is_err()
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let mut digest = CanonicalDigestBuilder::new(domain);
    write_digest(&mut digest, phenotype_hash.0);
    digest.write_u16(class_id.raw());
    digest.write_u64(generation);
    digest.write_u8(weight_bank);
    digest.write_u32(lifetime.len() as u32);
    digest.write_sequence_len(lifetime.len());
    for weight in lifetime {
        digest.write_f32(*weight)?;
    }
    digest.write_sequence_len(fast.len());
    for weight in fast {
        digest.write_f32(*weight)?;
    }
    Ok(digest.finish256())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsolidationIntent {
    pub cycle_id: u64,
}

impl Validate for ConsolidationIntent {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.cycle_id == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(())
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConsolidationJobId(NonZeroU64);

impl ConsolidationJobId {
    pub fn try_from_raw(value: u64) -> Result<Self, ScaffoldContractError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ScaffoldContractError::InvalidId)
    }

    pub const fn raw(self) -> u64 {
        self.0.get()
    }
}

impl Serialize for ConsolidationJobId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.raw())
    }
}

impl<'de> Deserialize<'de> for ConsolidationJobId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = u64::deserialize(deserializer)?;
        Self::try_from_raw(raw).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuConsolidationRequest {
    pub schema_version: u16,
    pub request_flags: u16,
    pub cycle_id: u64,
    pub phenotype_hash: PhenotypeHash,
    pub input_generation: u64,
    pub expected_output_generation: u64,
    pub input_digest: [u64; 4],
    pub replay_digest: [u64; 4],
    pub max_replay_events: u32,
    pub max_replay_eligibility_samples: u32,
    pub request_digest: [u64; 4],
}

impl GpuConsolidationRequest {
    pub fn recompute_request_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        if self.schema_version != GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION
            || self.request_flags != 0
            || self.cycle_id == 0
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.input_generation == 0
            || self.expected_output_generation
                != self
                    .input_generation
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
            || self.input_digest == [0; 4]
            || self.replay_digest == [0; 4]
            || self.max_replay_events == 0
            || self.max_replay_events > 65_536
            || self.max_replay_eligibility_samples == 0
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let mut digest = CanonicalDigestBuilder::new(REQUEST_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.request_flags);
        digest.write_u64(self.cycle_id);
        write_digest(&mut digest, self.phenotype_hash.0);
        digest.write_u64(self.input_generation);
        digest.write_u64(self.expected_output_generation);
        write_digest(&mut digest, self.input_digest);
        write_digest(&mut digest, self.replay_digest);
        digest.write_u32(self.max_replay_events);
        digest.write_u32(self.max_replay_eligibility_samples);
        Ok(digest.finish256())
    }
}

impl Validate for GpuConsolidationRequest {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.request_digest != self.recompute_request_digest()? {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsolidationStagedOutput {
    pub job_id: ConsolidationJobId,
    pub output_generation: u64,
    pub output_weight_bank: u8,
    pub output_digest: [u64; 4],
    pub eligibility_reset_generation: u64,
    pub output_eligibility_bank: u8,
    pub eligibility_output_digest: [u64; 4],
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub replay_journal_output_digest: [u64; 4],
    pub staging_digest: [u64; 4],
    pub promoted_fast_l1_bits: u32,
    pub replay_induced_fast_l1_bits: u32,
}

impl ConsolidationStagedOutput {
    pub fn recompute_staging_digest(
        &self,
        request: &GpuConsolidationRequest,
        eligibility_reset_policy_raw: u16,
        replay_consume_policy_raw: u16,
    ) -> Result<[u64; 4], ScaffoldContractError> {
        request.validate_contract()?;
        self.validate_fields(request)?;
        if eligibility_reset_policy_raw != 1 || replay_consume_policy_raw != 1 {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let mut digest = CanonicalDigestBuilder::new(STAGING_DIGEST_DOMAIN);
        write_digest(&mut digest, request.request_digest);
        digest.write_u64(self.job_id.raw());
        digest.write_u64(request.cycle_id);
        write_digest(&mut digest, request.phenotype_hash.0);
        digest.write_u64(self.output_generation);
        digest.write_u8(self.output_weight_bank);
        write_digest(&mut digest, self.output_digest);
        digest.write_u64(self.eligibility_reset_generation);
        digest.write_u8(self.output_eligibility_bank);
        write_digest(&mut digest, self.eligibility_output_digest);
        digest.write_u64(self.replay_journal_generation);
        digest.write_u32(self.replay_journal_cursor);
        digest.write_u32(self.replay_journal_event_count);
        write_digest(&mut digest, self.replay_journal_output_digest);
        digest.write_u16(eligibility_reset_policy_raw);
        digest.write_u16(replay_consume_policy_raw);
        digest.write_u32(self.promoted_fast_l1_bits);
        digest.write_u32(self.replay_induced_fast_l1_bits);
        Ok(digest.finish256())
    }

    pub fn validate_against(
        &self,
        request: &GpuConsolidationRequest,
        eligibility_reset_policy_raw: u16,
        replay_consume_policy_raw: u16,
    ) -> Result<(), ScaffoldContractError> {
        if self.staging_digest
            != self.recompute_staging_digest(
                request,
                eligibility_reset_policy_raw,
                replay_consume_policy_raw,
            )?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }

    pub fn promoted_fast_l1(self) -> f32 {
        f32::from_bits(self.promoted_fast_l1_bits)
    }

    pub fn replay_induced_fast_l1(self) -> f32 {
        f32::from_bits(self.replay_induced_fast_l1_bits)
    }

    fn validate_fields(
        &self,
        request: &GpuConsolidationRequest,
    ) -> Result<(), ScaffoldContractError> {
        let promoted = self.promoted_fast_l1();
        let replay = self.replay_induced_fast_l1();
        if self.output_generation != request.expected_output_generation
            || self.output_weight_bank > 1
            || self.output_digest == [0; 4]
            || self.eligibility_reset_generation == 0
            || self.output_eligibility_bank > 1
            || self.eligibility_output_digest == [0; 4]
            || self.replay_journal_generation == 0
            || self.replay_journal_cursor > request.max_replay_events
            || self.replay_journal_event_count > request.max_replay_events
            || self.replay_journal_output_digest == [0; 4]
            || !promoted.is_finite()
            || promoted.is_sign_negative()
            || !replay.is_finite()
            || replay.is_sign_negative()
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsolidationState {
    #[default]
    None,
    Pending {
        intent: ConsolidationIntent,
        replay_digest: [u64; 4],
        replay_event_count: u32,
        replay_eligibility_sample_count: u32,
    },
    Prepared {
        request: GpuConsolidationRequest,
    },
    Submitted {
        request: GpuConsolidationRequest,
        job_id: ConsolidationJobId,
    },
    Completed {
        request: GpuConsolidationRequest,
        staged: ConsolidationStagedOutput,
    },
    Committed {
        cycle_id: u64,
        output_generation: u64,
        output_digest: [u64; 4],
    },
}

impl ConsolidationState {
    pub const fn kind_raw(&self) -> u16 {
        match self {
            Self::None => 0,
            Self::Pending { .. } => 1,
            Self::Prepared { .. } => 2,
            Self::Submitted { .. } => 3,
            Self::Completed { .. } => 4,
            Self::Committed { .. } => 5,
        }
    }

    pub fn validate_for_cycle(&self, active_cycle_id: u64) -> Result<(), ScaffoldContractError> {
        match *self {
            Self::None => Ok(()),
            Self::Pending {
                intent,
                replay_digest,
                replay_event_count,
                replay_eligibility_sample_count,
            } => {
                intent.validate_contract()?;
                if intent.cycle_id != active_cycle_id
                    || replay_digest == [0; 4]
                    || (replay_event_count == 0 && replay_eligibility_sample_count != 0)
                {
                    Err(ScaffoldContractError::ConsolidationGenerationMismatch)
                } else {
                    Ok(())
                }
            }
            Self::Prepared { request } | Self::Submitted { request, .. } => {
                request.validate_contract()?;
                if request.cycle_id == active_cycle_id {
                    Ok(())
                } else {
                    Err(ScaffoldContractError::ConsolidationGenerationMismatch)
                }
            }
            Self::Completed { request, staged } => {
                request.validate_contract()?;
                staged.validate_against(&request, 1, 1)?;
                if request.cycle_id == active_cycle_id {
                    Ok(())
                } else {
                    Err(ScaffoldContractError::ConsolidationGenerationMismatch)
                }
            }
            Self::Committed {
                cycle_id,
                output_generation,
                output_digest,
            } if cycle_id == active_cycle_id
                && output_generation != 0
                && output_digest != [0; 4] =>
            {
                Ok(())
            }
            Self::Committed { .. } => Err(ScaffoldContractError::ConsolidationGenerationMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsolidationDriverEvent {
    ReplayAssetPersisted {
        intent: ConsolidationIntent,
        replay_digest: [u64; 4],
        replay_event_count: u32,
        replay_eligibility_sample_count: u32,
    },
    Prepared {
        request: GpuConsolidationRequest,
    },
    Submitted {
        request: GpuConsolidationRequest,
        job_id: ConsolidationJobId,
    },
    /// Rebinds a durable Submitted transaction to a new process-local job.
    /// The request and every durable input remain identical; only the lost
    /// capability-like job identifier may change after restart.
    RecoveredSubmitted {
        request: GpuConsolidationRequest,
        lost_job_id: ConsolidationJobId,
        recovered_job_id: ConsolidationJobId,
    },
    Completed {
        request: GpuConsolidationRequest,
        staged: ConsolidationStagedOutput,
    },
    Committed {
        cycle_id: u64,
        output_generation: u64,
        output_digest: [u64; 4],
    },
}

fn write_digest(builder: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        builder.write_u64(word);
    }
}
