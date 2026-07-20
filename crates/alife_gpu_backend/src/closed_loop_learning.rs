//! GPU eligibility transaction ABI and bounded public receipts.

use alife_core::{
    validate_outcome_credit_schema, ActionId, CandidateActionFamily, CandidateFeatureDigest,
    CanonicalDigestBuilder, ExperienceSequenceId, OrganismId, OutcomeCreditPacket,
    PerceptionFrameDigest, PhenotypeHash, ScaffoldContractError, SchemaVersions, Tick,
};
use bytemuck::{Pod, Zeroable};

pub const GPU_LEARNING_HEADER_WORDS: usize = 20;
pub const GPU_PENDING_ELIGIBILITY_WORDS: usize = 36;
pub const GPU_PENDING_ELIGIBILITY_BYTES: usize = GPU_PENDING_ELIGIBILITY_WORDS * 4;
pub const GPU_OUTCOME_CREDIT_WORDS: usize = 40;
pub const GPU_OUTCOME_CREDIT_BYTES: usize = GPU_OUTCOME_CREDIT_WORDS * 4;
pub const GPU_FAST_PLASTICITY_COMMIT_WORDS: usize = 16;
pub const GPU_FAST_PLASTICITY_COMMIT_BYTES: usize = GPU_FAST_PLASTICITY_COMMIT_WORDS * 4;
pub const GPU_CLOSED_LOOP_TICK_READBACK_BYTES: usize =
    std::mem::size_of::<crate::GpuSelectionRecord>();
pub const GPU_COMPACT_READBACK_CAPACITY_PER_ROW_BYTES: usize =
    if GPU_FAST_PLASTICITY_COMMIT_BYTES > GPU_CLOSED_LOOP_TICK_READBACK_BYTES {
        GPU_FAST_PLASTICITY_COMMIT_BYTES
    } else {
        GPU_CLOSED_LOOP_TICK_READBACK_BYTES
    };
pub const CLOSED_LOOP_ELIGIBILITY_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_eligibility.wgsl")
);

const PENDING_RECEIPT_DOMAIN: &[u8] = b"alife.gpu.pending-eligibility-receipt.v1";
const DISCARD_RECEIPT_DOMAIN: &[u8] = b"alife.gpu.pending-eligibility-discard.v1";

/// Exact post-outcome credit row consumed by the production plasticity WGSL.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct GpuOutcomeCreditRecord {
    pub schema_version: u32,
    pub selected_candidate_and_family: u32,
    pub organism_id: [u32; 2],
    pub phenotype_hash: [u32; 8],
    pub sequence_id: [u32; 2],
    pub originating_tick: [u32; 2],
    pub outcome_tick: [u32; 2],
    pub selected_action: u32,
    pub active_activation_side: u32,
    pub candidate_feature_digest: [u32; 4],
    pub frame_digest: [u32; 8],
    pub dispatch_generation: [u32; 2],
    pub reward_prediction_error: f32,
    pub pain: f32,
    pub homeostatic_improvement: f32,
    pub frustration: f32,
    pub novelty: f32,
    pub modulator_value: f32,
}

impl TryFrom<&OutcomeCreditPacket> for GpuOutcomeCreditRecord {
    type Error = ScaffoldContractError;

    fn try_from(packet: &OutcomeCreditPacket) -> Result<Self, Self::Error> {
        validate_outcome_credit_schema(packet)?;
        packet.organism_id().validate()?;
        packet.sequence_id().validate()?;
        packet.selected_action().validate()?;
        let family = CandidateActionFamily::try_from_raw(packet.selected_family().raw())?;
        let modulator = packet.modulator();
        let components = [
            modulator.reward_prediction_error(),
            modulator.pain(),
            modulator.homeostatic_improvement(),
            modulator.frustration(),
            modulator.novelty(),
            modulator.value(),
        ];
        if components.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if components.iter().any(|value| !(-1.0..=1.0).contains(value))
            || packet.active_activation_side() > 1
            || packet.dispatch_generation() == 0
            || packet.outcome_tick().raw() <= packet.originating_tick().raw()
            || packet.phenotype_hash() == PhenotypeHash([0; 4])
            || packet.frame_digest() == PerceptionFrameDigest([0; 4])
            || packet.candidate_feature_digest() == CandidateFeatureDigest([0; 2])
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        Ok(Self {
            schema_version: u32::from(packet.schema_version()),
            selected_candidate_and_family: pack_candidate_index_and_family(
                packet.selected_candidate(),
                family,
            ),
            organism_id: split_u64(packet.organism_id().raw()),
            phenotype_hash: split_u64x4(packet.phenotype_hash().0),
            sequence_id: split_u64(packet.sequence_id().raw()),
            originating_tick: split_u64(packet.originating_tick().raw()),
            outcome_tick: split_u64(packet.outcome_tick().raw()),
            selected_action: packet.selected_action().raw(),
            active_activation_side: u32::from(packet.active_activation_side()),
            candidate_feature_digest: split_u64x2(packet.candidate_feature_digest().0),
            frame_digest: split_u64x4(packet.frame_digest().0),
            dispatch_generation: split_u64(packet.dispatch_generation()),
            reward_prediction_error: components[0],
            pain: components[1],
            homeostatic_improvement: components[2],
            frustration: components[3],
            novelty: components[4],
            modulator_value: components[5],
        })
    }
}

impl GpuOutcomeCreditRecord {
    pub(crate) fn words(&self) -> &[u32] {
        bytemuck::cast_slice(std::slice::from_ref(self))
    }
}

/// Compact GPU-side commit proof. This record spans the slot diagnostic and
/// selection rows and contains no weight values.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub(crate) struct GpuFastPlasticityCommitRecord {
    pub schema_version: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub status: u32,
    pub input_fast_generation: [u32; 2],
    pub output_fast_generation: [u32; 2],
    pub output_eligibility_generation: [u32; 2],
    pub replay_generation: [u32; 2],
    pub transaction_generation: [u32; 2],
    pub fast_weights_changed: u32,
    pub max_abs_delta_bits: u32,
}

impl GpuFastPlasticityCommitRecord {
    pub(crate) fn from_words(words: &[u32]) -> Result<Self, crate::GpuClosedLoopError> {
        if words.len() != GPU_FAST_PLASTICITY_COMMIT_WORDS {
            return Err(crate::GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }

    pub(crate) const fn input_fast_generation(&self) -> u64 {
        join_u64(self.input_fast_generation[0], self.input_fast_generation[1])
    }

    pub(crate) const fn output_fast_generation(&self) -> u64 {
        join_u64(
            self.output_fast_generation[0],
            self.output_fast_generation[1],
        )
    }

    pub(crate) const fn output_eligibility_generation(&self) -> u64 {
        join_u64(
            self.output_eligibility_generation[0],
            self.output_eligibility_generation[1],
        )
    }

    pub(crate) const fn replay_generation(&self) -> u64 {
        join_u64(self.replay_generation[0], self.replay_generation[1])
    }

    pub(crate) const fn transaction_generation(&self) -> u64 {
        join_u64(
            self.transaction_generation[0],
            self.transaction_generation[1],
        )
    }

    pub(crate) const fn max_abs_delta(&self) -> f32 {
        f32::from_bits(self.max_abs_delta_bits)
    }
}

/// Host-visible proof of one committed GPU waking-learning transaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuLearningReceipt {
    pub handle: crate::GpuBrainHandle,
    pub sequence_id: ExperienceSequenceId,
    pub dispatch_generation: u64,
    pub active_activation_side: u8,
    pub input_fast_generation: u64,
    pub output_fast_generation: u64,
    pub output_eligibility_generation: u64,
    pub replay_journal_generation: u64,
    pub fast_weights_changed: u32,
    pub max_abs_delta: f32,
    pub hardware_receipt_generation: u64,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct GpuLearningHeader {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub brain_slot_index: u32,
    pub active_activation_side: u32,
    pub dispatch_generation_lo: u32,
    pub dispatch_generation_hi: u32,
    pub candidate_count: u32,
    pub candidate_offset: u32,
    pub decoder_learning_input_offset: u32,
    pub selection_offset: u32,
    pub outcome_offset: u32,
    pub recurrent_synapse_count: u32,
    pub decoder_synapse_count: u32,
    pub decoder_input_stride: u32,
    pub pending_eligibility_offset: u32,
    pub reserved: [u32; 3],
}

impl GpuLearningHeader {
    pub fn from_words(words: &[u32]) -> Result<Self, crate::GpuClosedLoopError> {
        if words.len() != GPU_LEARNING_HEADER_WORDS {
            return Err(crate::GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }

    pub fn words(&self) -> &[u32] {
        bytemuck::cast_slice(std::slice::from_ref(self))
    }

    pub const fn dispatch_generation(&self) -> u64 {
        join_u64(self.dispatch_generation_lo, self.dispatch_generation_hi)
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct GpuPendingEligibilityRecord {
    pub schema_version: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub active_activation_side: u32,
    pub phenotype_hash: [u32; 8],
    pub organism_id: [u32; 2],
    pub dispatch_generation: [u32; 2],
    pub originating_tick: [u32; 2],
    pub frame_digest: [u32; 8],
    pub candidate_index_and_family: u32,
    pub action_id: u32,
    pub candidate_feature_digest: [u32; 4],
    pub active_eligibility_generation: [u32; 2],
    pub staging_eligibility_generation: [u32; 2],
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct GpuEligibilityDiscardRecord {
    pub schema_version: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub status: u32,
    pub active_eligibility_bank: u32,
    pub reserved: u32,
    pub active_eligibility_generation: [u32; 2],
    pub discarded_staging_generation: [u32; 2],
    pub transaction_generation: [u32; 2],
}

impl GpuEligibilityDiscardRecord {
    pub fn from_words(words: &[u32]) -> Result<Self, crate::GpuClosedLoopError> {
        if words.len() != std::mem::size_of::<Self>() / 4 {
            return Err(crate::GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }

    pub const fn active_eligibility_generation(&self) -> u64 {
        join_u64(
            self.active_eligibility_generation[0],
            self.active_eligibility_generation[1],
        )
    }

    pub const fn discarded_staging_generation(&self) -> u64 {
        join_u64(
            self.discarded_staging_generation[0],
            self.discarded_staging_generation[1],
        )
    }

    pub const fn transaction_generation(&self) -> u64 {
        join_u64(
            self.transaction_generation[0],
            self.transaction_generation[1],
        )
    }
}

impl GpuPendingEligibilityRecord {
    pub fn from_words(words: &[u32]) -> Result<Self, crate::GpuClosedLoopError> {
        if words.len() != GPU_PENDING_ELIGIBILITY_WORDS {
            return Err(crate::GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }

    pub fn words(&self) -> &[u32] {
        bytemuck::cast_slice(std::slice::from_ref(self))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn template(
        slot: u32,
        slot_generation: u32,
        active_activation_side: u8,
        phenotype_hash: PhenotypeHash,
        organism_id: OrganismId,
        dispatch_generation: u64,
        originating_tick: Tick,
        frame_digest: PerceptionFrameDigest,
        active_eligibility_generation: u64,
        staging_eligibility_generation: u64,
    ) -> Result<Self, ScaffoldContractError> {
        organism_id.validate()?;
        let expected_staging_generation = active_eligibility_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if slot_generation == 0
            || active_activation_side > 1
            || dispatch_generation == 0
            || active_eligibility_generation == 0
            || staging_eligibility_generation == 0
            || staging_eligibility_generation != expected_staging_generation
            || phenotype_hash == PhenotypeHash([0; 4])
            || frame_digest == PerceptionFrameDigest([0; 4])
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(Self {
            schema_version: u32::from(SchemaVersions::CURRENT.learning.raw()),
            slot,
            slot_generation,
            active_activation_side: u32::from(active_activation_side),
            phenotype_hash: split_u64x4(phenotype_hash.0),
            organism_id: split_u64(organism_id.raw()),
            dispatch_generation: split_u64(dispatch_generation),
            originating_tick: split_u64(originating_tick.raw()),
            frame_digest: split_u64x4(frame_digest.0),
            candidate_index_and_family: u32::MAX,
            action_id: 0,
            candidate_feature_digest: [0; 4],
            active_eligibility_generation: split_u64(active_eligibility_generation),
            staging_eligibility_generation: split_u64(staging_eligibility_generation),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityIdentity {
    handle_generation: u32,
    phenotype_hash: PhenotypeHash,
    dispatch_generation: u64,
    originating_tick: Tick,
    frame_digest: PerceptionFrameDigest,
    active_activation_side: u8,
    candidate_index: u16,
    action_id: ActionId,
    action_family: CandidateActionFamily,
    candidate_feature_digest: CandidateFeatureDigest,
    active_eligibility_generation: u64,
    staging_eligibility_generation: u64,
}

impl PendingEligibilityIdentity {
    pub const fn handle_generation(&self) -> u32 {
        self.handle_generation
    }
    pub const fn phenotype_hash(&self) -> PhenotypeHash {
        self.phenotype_hash
    }
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }
    pub const fn originating_tick(&self) -> Tick {
        self.originating_tick
    }
    pub const fn frame_digest(&self) -> PerceptionFrameDigest {
        self.frame_digest
    }
    pub const fn active_activation_side(&self) -> u8 {
        self.active_activation_side
    }
    pub const fn candidate_index(&self) -> u16 {
        self.candidate_index
    }
    pub const fn action_id(&self) -> ActionId {
        self.action_id
    }
    pub const fn action_family(&self) -> CandidateActionFamily {
        self.action_family
    }
    pub const fn candidate_feature_digest(&self) -> CandidateFeatureDigest {
        self.candidate_feature_digest
    }
    pub const fn active_eligibility_generation(&self) -> u64 {
        self.active_eligibility_generation
    }
    pub const fn staging_eligibility_generation(&self) -> u64 {
        self.staging_eligibility_generation
    }

    fn write_canonical(&self, digest: &mut CanonicalDigestBuilder) {
        digest.write_u32(self.handle_generation);
        for word in self.phenotype_hash.0 {
            digest.write_u64(word);
        }
        digest.write_u64(self.dispatch_generation);
        digest.write_u64(self.originating_tick.raw());
        for word in self.frame_digest.0 {
            digest.write_u64(word);
        }
        digest.write_u8(self.active_activation_side);
        digest.write_u16(self.candidate_index);
        digest.write_u32(self.action_id.raw());
        digest.write_u8(self.action_family.raw());
        for word in self.candidate_feature_digest.0 {
            digest.write_u64(word);
        }
        digest.write_u64(self.active_eligibility_generation);
        digest.write_u64(self.staging_eligibility_generation);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityReceipt {
    identity: PendingEligibilityIdentity,
    receipt_digest: [u64; 4],
}

impl PendingEligibilityReceipt {
    pub const fn identity(&self) -> &PendingEligibilityIdentity {
        &self.identity
    }
    pub const fn receipt_digest(&self) -> [u64; 4] {
        self.receipt_digest
    }

    pub(crate) fn from_gpu_record(
        record: GpuPendingEligibilityRecord,
        expected_slot: u32,
        expected_organism: OrganismId,
        expected_phenotype: PhenotypeHash,
    ) -> Result<Self, ScaffoldContractError> {
        let schema = u32::from(SchemaVersions::CURRENT.learning.raw());
        let phenotype_hash = PhenotypeHash(join_u32x8(record.phenotype_hash));
        let organism_id = OrganismId(join_u64(record.organism_id[0], record.organism_id[1]));
        let dispatch_generation =
            join_u64(record.dispatch_generation[0], record.dispatch_generation[1]);
        let originating_tick = Tick::new(join_u64(
            record.originating_tick[0],
            record.originating_tick[1],
        ));
        let frame_digest = PerceptionFrameDigest(join_u32x8(record.frame_digest));
        let candidate_index = record.candidate_index_and_family & 0xffff;
        let family_raw = (record.candidate_index_and_family >> 16) & 0xff;
        let reserved = record.candidate_index_and_family >> 24;
        let candidate_index = u16::try_from(candidate_index)
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        let family_raw =
            u8::try_from(family_raw).map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        let action_family = CandidateActionFamily::try_from_raw(family_raw)?;
        let action_id = ActionId(record.action_id);
        action_id.validate()?;
        let candidate_feature_digest =
            CandidateFeatureDigest(join_u32x4(record.candidate_feature_digest));
        let active_eligibility_generation = join_u64(
            record.active_eligibility_generation[0],
            record.active_eligibility_generation[1],
        );
        let staging_eligibility_generation = join_u64(
            record.staging_eligibility_generation[0],
            record.staging_eligibility_generation[1],
        );
        let active_activation_side = u8::try_from(record.active_activation_side)
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        if record.schema_version != schema
            || record.slot != expected_slot
            || record.slot_generation == 0
            || active_activation_side > 1
            || organism_id != expected_organism
            || phenotype_hash != expected_phenotype
            || dispatch_generation == 0
            || frame_digest == PerceptionFrameDigest([0; 4])
            || reserved != 0
            || candidate_feature_digest == CandidateFeatureDigest([0; 2])
            || active_eligibility_generation == 0
            || staging_eligibility_generation
                != active_eligibility_generation
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let identity = PendingEligibilityIdentity {
            handle_generation: record.slot_generation,
            phenotype_hash,
            dispatch_generation,
            originating_tick,
            frame_digest,
            active_activation_side,
            candidate_index,
            action_id,
            action_family,
            candidate_feature_digest,
            active_eligibility_generation,
            staging_eligibility_generation,
        };
        let mut digest = CanonicalDigestBuilder::new(PENDING_RECEIPT_DOMAIN);
        identity.write_canonical(&mut digest);
        Ok(Self {
            identity,
            receipt_digest: digest.finish256(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityDiscardReceipt {
    pub identity: PendingEligibilityIdentity,
    pub discarded_staging_generation: u64,
    pub hardware_receipt_generation: u64,
    pub receipt_digest: [u64; 4],
}

impl PendingEligibilityDiscardReceipt {
    pub(crate) fn new(
        identity: PendingEligibilityIdentity,
        hardware_receipt_generation: u64,
    ) -> Self {
        let mut digest = CanonicalDigestBuilder::new(DISCARD_RECEIPT_DOMAIN);
        identity.write_canonical(&mut digest);
        digest.write_u64(identity.staging_eligibility_generation);
        digest.write_u64(hardware_receipt_generation);
        Self {
            identity,
            discarded_staging_generation: identity.staging_eligibility_generation,
            hardware_receipt_generation,
            receipt_digest: digest.finish256(),
        }
    }
}

pub const fn pack_candidate_index_and_family(
    candidate_index: u16,
    family: CandidateActionFamily,
) -> u32 {
    candidate_index as u32 | ((family.raw() as u32) << 16)
}

pub(crate) const fn split_u64(value: u64) -> [u32; 2] {
    [value as u32, (value >> 32) as u32]
}

pub(crate) fn split_u64x4(values: [u64; 4]) -> [u32; 8] {
    let mut result = [0; 8];
    for (index, value) in values.into_iter().enumerate() {
        result[index * 2] = value as u32;
        result[index * 2 + 1] = (value >> 32) as u32;
    }
    result
}

pub(crate) fn split_u64x2(values: [u64; 2]) -> [u32; 4] {
    let mut result = [0; 4];
    for (index, value) in values.into_iter().enumerate() {
        result[index * 2] = value as u32;
        result[index * 2 + 1] = (value >> 32) as u32;
    }
    result
}

const fn join_u64(lo: u32, hi: u32) -> u64 {
    lo as u64 | ((hi as u64) << 32)
}

fn join_u32x8(values: [u32; 8]) -> [u64; 4] {
    std::array::from_fn(|index| join_u64(values[index * 2], values[index * 2 + 1]))
}

pub(crate) fn phenotype_hash_from_gpu_words(values: [u32; 8]) -> PhenotypeHash {
    PhenotypeHash(join_u32x8(values))
}

fn join_u32x4(values: [u32; 4]) -> [u64; 2] {
    std::array::from_fn(|index| join_u64(values[index * 2], values[index * 2 + 1]))
}
