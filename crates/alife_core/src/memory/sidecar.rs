use super::candidate_index::target_bins;
use super::*;

pub const PORTABLE_MEMORY_BANK_ASSET_SCHEMA_VERSION: u16 = 2;

const PORTABLE_MEMORY_RECORD_DOMAIN: &[u8] = b"ALIFE-PORTABLE-MEMORY-RECORD-V2";
const PORTABLE_MEMORY_BANK_DOMAIN: &[u8] = b"ALIFE-PORTABLE-MEMORY-BANK-V2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableMemoryRecordV2 {
    pub schema_version: u16,
    pub memory_id_raw: u64,
    pub organism_id_raw: u64,
    pub sealed_sequence_id_raw: u64,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub query_version_raw: u16,
    pub action_id_raw: u32,
    pub action_kind_raw: u8,
    pub family_raw: u16,
    pub tracked_object_id_raw: u64,
    pub target_bins: [i8; CANDIDATE_FEATURE_COUNT],
    pub query_feature_bits: Vec<u32>,
    pub target_latent_bits: Vec<u32>,
    pub family_value_bits: Vec<u32>,
    pub confidence_bits: u32,
    pub salience_q16: u16,
    pub observation_count: u32,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableMemoryBankAssetV2 {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile: crate::SensorProfileIdentity,
    pub capacity: u32,
    pub max_feature_len: u32,
    pub max_match_count: u32,
    pub min_match_score_bits: u32,
    pub empty_confidence_bits: u32,
    pub generation: u64,
    pub next_memory_id_raw: u64,
    pub last_observed_sequence_id_raw: u64,
    pub merge_count: u64,
    pub eviction_count: u64,
    pub records: Vec<PortableMemoryRecordV2>,
    pub active_bank_digest: [u64; 4],
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryUpdateKind {
    Inserted {
        inserted: MemoryId,
    },
    Merged {
        into: MemoryId,
    },
    Evicted {
        removed: MemoryId,
        inserted: MemoryId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUpdateReceipt {
    pub sealed_sequence_id: ExperienceSequenceId,
    pub organism_id_raw: u64,
    pub bucket: MemoryBucketReceiptKey,
    pub target_bucket: TargetMemoryBucketReceiptKey,
    pub input_generation: u64,
    pub output_generation: u64,
    pub kind: MemoryUpdateKind,
    pub record_count: u32,
    pub capacity: u32,
    pub merge_count: u64,
    pub eviction_count: u64,
    pub before_digest: [u64; 4],
    pub after_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionIdentity {
    pub organism_id_raw: u64,
    pub cycle_id: u64,
    pub policy_version: u16,
    pub max_records_after: u32,
    pub input_generation: u64,
    pub input_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionReceipt {
    pub identity: MemoryCompactionIdentity,
    pub output_generation: u64,
    pub output_digest: [u64; 4],
    pub merged: u32,
    pub evicted: u32,
    pub record_count: u32,
    pub capacity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryCompactionPhase {
    Idle,
    Pending {
        cycle_id: u64,
        input_generation: u64,
        input_digest: [u64; 4],
        max_records_after: u32,
        policy_version: u16,
    },
    Staged {
        cycle_id: u64,
        input_generation: u64,
        output_generation: u64,
        input_digest: [u64; 4],
        output_digest: [u64; 4],
        receipt: MemoryCompactionReceipt,
    },
    Committed {
        cycle_id: u64,
        output_generation: u64,
        output_digest: [u64; 4],
        receipt: MemoryCompactionReceipt,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionCheckpoint {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub active_generation: u64,
    pub active_digest: [u64; 4],
    pub last_committed_cycle_id: Option<u64>,
    pub next_cycle_id: u64,
    pub phase: MemoryCompactionPhase,
}

impl MemoryCompactionCheckpoint {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != MEMORY_RECALL_SCHEMA_VERSION
            || self.organism_id_raw == 0
            || self.next_cycle_id == 0
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        match self.phase {
            MemoryCompactionPhase::Idle => Ok(()),
            MemoryCompactionPhase::Pending {
                cycle_id,
                input_generation,
                input_digest,
                max_records_after,
                policy_version,
            } => {
                if cycle_id == 0
                    || self.next_cycle_id != cycle_id.saturating_add(1)
                    || input_generation != self.active_generation
                    || input_digest != self.active_digest
                    || max_records_after == 0
                    || policy_version == 0
                {
                    Err(ScaffoldContractError::InvalidMemoryQuery)
                } else {
                    Ok(())
                }
            }
            MemoryCompactionPhase::Staged {
                cycle_id,
                input_generation,
                output_generation,
                input_digest,
                output_digest,
                receipt,
            } => {
                if self.next_cycle_id != cycle_id.saturating_add(1)
                    || input_generation != self.active_generation
                    || input_digest != self.active_digest
                    || output_generation != input_generation.saturating_add(1)
                    || receipt.identity.cycle_id != cycle_id
                    || receipt.identity.input_generation != input_generation
                    || receipt.identity.input_digest != input_digest
                    || receipt.output_generation != output_generation
                    || receipt.output_digest != output_digest
                {
                    Err(ScaffoldContractError::InvalidMemoryQuery)
                } else {
                    Ok(())
                }
            }
            MemoryCompactionPhase::Committed {
                cycle_id,
                output_generation,
                output_digest,
                receipt,
            } => {
                if self.last_committed_cycle_id != Some(cycle_id)
                    || self.next_cycle_id != cycle_id.saturating_add(1)
                    || self.active_generation != output_generation
                    || self.active_digest != output_digest
                    || receipt.identity.cycle_id != cycle_id
                    || receipt.output_generation != output_generation
                    || receipt.output_digest != output_digest
                {
                    Err(ScaffoldContractError::InvalidMemoryQuery)
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedMemoryCompaction {
    checkpoint: MemoryCompactionCheckpoint,
    receipt: MemoryCompactionReceipt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySidecarState {
    organism_id: OrganismId,
    profile: crate::SensorProfileIdentity,
    bank: MemoryBank,
    compaction: MemoryCompactionCheckpoint,
    staged_bank: Option<MemoryBank>,
}

impl MemorySidecarState {
    pub fn new(
        organism_id: OrganismId,
        config: MemoryBankConfig,
    ) -> Result<Self, ScaffoldContractError> {
        Self::new_profiled(
            organism_id,
            crate::SensorProfileIdentity {
                profile_id: crate::SensorProfile::PrivilegedAffordanceV1.into(),
                profile_schema_version: 1,
                sensory_abi_version: crate::SensoryAbiVersion::CURRENT.raw(),
            },
            config,
        )
    }

    pub fn new_profiled(
        organism_id: OrganismId,
        profile: crate::SensorProfileIdentity,
        config: MemoryBankConfig,
    ) -> Result<Self, ScaffoldContractError> {
        organism_id.validate()?;
        profile.validate_contract()?;
        let bank = MemoryBank::new(config)?;
        let active_digest = bank.candidate_store.digest(bank.config.capacity)?;
        let compaction = MemoryCompactionCheckpoint {
            schema_version: MEMORY_RECALL_SCHEMA_VERSION,
            organism_id_raw: organism_id.raw(),
            active_generation: bank.candidate_store.generation,
            active_digest,
            last_committed_cycle_id: None,
            next_cycle_id: 1,
            phase: MemoryCompactionPhase::Idle,
        };
        compaction.validate_contract()?;
        Ok(Self {
            organism_id,
            profile,
            bank,
            compaction,
            staged_bank: None,
        })
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn profile(&self) -> crate::SensorProfileIdentity {
        self.profile
    }

    pub const fn bank(&self) -> &MemoryBank {
        &self.bank
    }

    pub const fn compaction_checkpoint(&self) -> &MemoryCompactionCheckpoint {
        &self.compaction
    }

    pub fn recall_frame(
        &self,
        draft: &PerceptionFrameDraft,
    ) -> Result<PreparedMemoryRecall, ScaffoldContractError> {
        if draft.organism_id() != self.organism_id {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if draft.profile_provenance().identity() != self.profile {
            return Err(ScaffoldContractError::SensorProfileMismatch);
        }
        self.bank.recall_frame(draft)
    }

    pub fn observe_sealed_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<MemoryUpdateReceipt, ScaffoldContractError> {
        if matches!(
            self.compaction.phase,
            MemoryCompactionPhase::Pending { .. } | MemoryCompactionPhase::Staged { .. }
        ) {
            return Err(ScaffoldContractError::MemoryCompactionConflict);
        }
        if patch.pre_action().organism_id != self.organism_id {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if patch.header().sensor_profile.identity() != self.profile {
            return Err(ScaffoldContractError::SensorProfileMismatch);
        }
        let receipt = self.bank.observe_sealed_patch(patch)?;
        self.compaction.active_generation = receipt.output_generation;
        self.compaction.active_digest = receipt.after_digest;
        self.compaction.phase = MemoryCompactionPhase::Idle;
        self.staged_bank = None;
        self.compaction.validate_contract()?;
        Ok(receipt)
    }

    pub fn prepare_compaction(
        &mut self,
        cycle_id: u64,
        max_records_after: u32,
        policy_version: u16,
    ) -> Result<PreparedMemoryCompaction, ScaffoldContractError> {
        self.compaction.validate_contract()?;
        if cycle_id == 0 || max_records_after == 0 || policy_version == 0 {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if usize::try_from(max_records_after).unwrap_or(usize::MAX) > self.bank.config.capacity {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }

        let previous_checkpoint = self.compaction;
        match self.compaction.phase {
            MemoryCompactionPhase::Committed {
                cycle_id: committed_cycle,
                receipt,
                ..
            } => {
                if cycle_id == committed_cycle
                    && receipt.identity.max_records_after == max_records_after
                    && receipt.identity.policy_version == policy_version
                {
                    return Ok(PreparedMemoryCompaction {
                        checkpoint: self.compaction,
                        receipt,
                    });
                }
                if cycle_id != self.compaction.next_cycle_id {
                    return Err(ScaffoldContractError::MemoryCompactionConflict);
                }
                // A second sleep cycle can legitimately begin before another
                // waking patch mutates memory (for example after restoring at
                // a sealed sleep boundary). Preserve the committed identity
                // for rollback while allowing the unchanged active bank to be
                // compacted under the next cycle ID.
                self.compaction.phase = MemoryCompactionPhase::Idle;
                self.staged_bank = None;
            }
            MemoryCompactionPhase::Staged {
                cycle_id: staged_cycle,
                receipt,
                ..
            } => {
                if cycle_id == staged_cycle
                    && receipt.identity.max_records_after == max_records_after
                    && receipt.identity.policy_version == policy_version
                    && self.staged_bank.is_some()
                {
                    return Ok(PreparedMemoryCompaction {
                        checkpoint: self.compaction,
                        receipt,
                    });
                }
                return Err(ScaffoldContractError::MemoryCompactionConflict);
            }
            MemoryCompactionPhase::Pending {
                cycle_id: pending_cycle,
                max_records_after: pending_max,
                policy_version: pending_policy,
                ..
            } if cycle_id != pending_cycle
                || max_records_after != pending_max
                || policy_version != pending_policy =>
            {
                return Err(ScaffoldContractError::MemoryCompactionConflict);
            }
            MemoryCompactionPhase::Pending { .. } | MemoryCompactionPhase::Idle => {}
        }

        let active_digest = self
            .bank
            .candidate_store
            .digest(self.bank.config.capacity)?;
        if self.compaction.active_generation != self.bank.candidate_store.generation
            || self.compaction.active_digest != active_digest
        {
            return Err(ScaffoldContractError::MemoryCompactionConflict);
        }

        if matches!(self.compaction.phase, MemoryCompactionPhase::Idle) {
            if cycle_id != self.compaction.next_cycle_id {
                return Err(ScaffoldContractError::MemoryCompactionConflict);
            }
            let next_cycle_id = cycle_id
                .checked_add(1)
                .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
            self.compaction.next_cycle_id = next_cycle_id;
            self.compaction.phase = MemoryCompactionPhase::Pending {
                cycle_id,
                input_generation: self.compaction.active_generation,
                input_digest: self.compaction.active_digest,
                max_records_after,
                policy_version,
            };
            self.compaction.validate_contract()?;
        }

        let mut staged_bank = self.bank.clone();
        let receipt = match staged_bank.compact_candidate_records(
            self.organism_id,
            cycle_id,
            max_records_after,
            policy_version,
        ) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.compaction = previous_checkpoint;
                self.staged_bank = None;
                return Err(error);
            }
        };
        let checkpoint = MemoryCompactionCheckpoint {
            phase: MemoryCompactionPhase::Staged {
                cycle_id,
                input_generation: receipt.identity.input_generation,
                output_generation: receipt.output_generation,
                input_digest: receipt.identity.input_digest,
                output_digest: receipt.output_digest,
                receipt,
            },
            ..self.compaction
        };
        if let Err(error) = checkpoint.validate_contract() {
            self.compaction = previous_checkpoint;
            self.staged_bank = None;
            return Err(error);
        }
        self.compaction = checkpoint;
        self.staged_bank = Some(staged_bank);
        Ok(PreparedMemoryCompaction {
            checkpoint,
            receipt,
        })
    }

    pub fn commit_compaction(
        &mut self,
        prepared: PreparedMemoryCompaction,
    ) -> Result<MemoryCompactionReceipt, ScaffoldContractError> {
        if let MemoryCompactionPhase::Committed { receipt, .. } = self.compaction.phase {
            if receipt == prepared.receipt {
                return Ok(receipt);
            }
            return Err(ScaffoldContractError::MemoryCompactionConflict);
        }
        prepared.checkpoint.validate_contract()?;
        if prepared.checkpoint != self.compaction
            || !matches!(
                prepared.checkpoint.phase,
                MemoryCompactionPhase::Staged { .. }
            )
        {
            return Err(ScaffoldContractError::MemoryCompactionConflict);
        }
        let staged_bank = self
            .staged_bank
            .as_ref()
            .ok_or(ScaffoldContractError::MemoryCompactionConflict)?;
        if prepared.receipt.identity.organism_id_raw != self.organism_id.raw()
            || prepared.receipt.identity.input_generation != self.compaction.active_generation
            || prepared.receipt.identity.input_digest != self.compaction.active_digest
            || self
                .bank
                .candidate_store
                .digest(self.bank.config.capacity)?
                != prepared.receipt.identity.input_digest
            || staged_bank
                .candidate_store
                .digest(staged_bank.config.capacity)?
                != prepared.receipt.output_digest
        {
            return Err(ScaffoldContractError::MemoryCompactionConflict);
        }

        let committed = MemoryCompactionCheckpoint {
            schema_version: MEMORY_RECALL_SCHEMA_VERSION,
            organism_id_raw: self.organism_id.raw(),
            active_generation: prepared.receipt.output_generation,
            active_digest: prepared.receipt.output_digest,
            last_committed_cycle_id: Some(prepared.receipt.identity.cycle_id),
            next_cycle_id: self.compaction.next_cycle_id,
            phase: MemoryCompactionPhase::Committed {
                cycle_id: prepared.receipt.identity.cycle_id,
                output_generation: prepared.receipt.output_generation,
                output_digest: prepared.receipt.output_digest,
                receipt: prepared.receipt,
            },
        };
        committed.validate_contract()?;

        let staged_bank = self
            .staged_bank
            .take()
            .ok_or(ScaffoldContractError::MemoryCompactionConflict)?;
        self.bank = staged_bank;
        self.compaction = committed;
        Ok(prepared.receipt)
    }

    pub fn export_active_bank(&self) -> Result<PortableMemoryBankAssetV2, ScaffoldContractError> {
        portable_bank_from_memory(self.organism_id, self.profile, &self.bank)
    }

    pub fn export_staged_bank(
        &self,
    ) -> Result<Option<PortableMemoryBankAssetV2>, ScaffoldContractError> {
        self.staged_bank
            .as_ref()
            .map(|bank| portable_bank_from_memory(self.organism_id, self.profile, bank))
            .transpose()
    }

    pub fn restore_portable(
        profile: crate::SensorProfileIdentity,
        checkpoint: MemoryCompactionCheckpoint,
        active_asset: PortableMemoryBankAssetV2,
        staged_asset: Option<PortableMemoryBankAssetV2>,
    ) -> Result<Self, ScaffoldContractError> {
        profile
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        checkpoint
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        if active_asset.profile != profile
            || active_asset.organism_id_raw != checkpoint.organism_id_raw
            || staged_asset.as_ref().is_some_and(|asset| {
                asset.profile != profile || asset.organism_id_raw != checkpoint.organism_id_raw
            })
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }

        let active_generation = active_asset.generation;
        let active_digest = active_asset.active_bank_digest;
        let active_bank = memory_bank_from_portable(active_asset)?;
        let staged_meta = staged_asset
            .as_ref()
            .map(|asset| (asset.generation, asset.active_bank_digest));
        let staged_bank = staged_asset.map(memory_bank_from_portable).transpose()?;
        if staged_bank
            .as_ref()
            .is_some_and(|bank| bank.config != active_bank.config)
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }

        let organism_id = OrganismId(checkpoint.organism_id_raw);
        organism_id.validate()?;
        match checkpoint.phase {
            MemoryCompactionPhase::Idle => {
                if staged_bank.is_some()
                    || active_generation != checkpoint.active_generation
                    || active_digest != checkpoint.active_digest
                {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
                Ok(Self {
                    organism_id,
                    profile,
                    bank: active_bank,
                    compaction: checkpoint,
                    staged_bank: None,
                })
            }
            MemoryCompactionPhase::Pending {
                cycle_id,
                max_records_after,
                policy_version,
                ..
            } => {
                if staged_bank.is_some()
                    || active_generation != checkpoint.active_generation
                    || active_digest != checkpoint.active_digest
                {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
                let mut state = Self {
                    organism_id,
                    profile,
                    bank: active_bank,
                    compaction: checkpoint,
                    staged_bank: None,
                };
                let prepared =
                    state.prepare_compaction(cycle_id, max_records_after, policy_version)?;
                state.commit_compaction(prepared)?;
                Ok(state)
            }
            MemoryCompactionPhase::Staged {
                cycle_id,
                output_generation,
                output_digest,
                receipt,
                ..
            } => {
                let Some((staged_generation, staged_digest)) = staged_meta else {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                };
                if staged_generation != output_generation || staged_digest != output_digest {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
                if active_generation == checkpoint.active_generation
                    && active_digest == checkpoint.active_digest
                {
                    let mut state = Self {
                        organism_id,
                        profile,
                        bank: active_bank,
                        compaction: checkpoint,
                        staged_bank,
                    };
                    state.commit_compaction(PreparedMemoryCompaction {
                        checkpoint,
                        receipt,
                    })?;
                    Ok(state)
                } else if active_generation == output_generation && active_digest == output_digest {
                    let committed = MemoryCompactionCheckpoint {
                        active_generation: output_generation,
                        active_digest: output_digest,
                        last_committed_cycle_id: Some(cycle_id),
                        phase: MemoryCompactionPhase::Committed {
                            cycle_id,
                            output_generation,
                            output_digest,
                            receipt,
                        },
                        ..checkpoint
                    };
                    committed.validate_contract()?;
                    Ok(Self {
                        organism_id,
                        profile,
                        bank: active_bank,
                        compaction: committed,
                        staged_bank: None,
                    })
                } else {
                    Err(ScaffoldContractError::InvalidMemoryQuery)
                }
            }
            MemoryCompactionPhase::Committed { .. } => {
                if staged_bank.is_some()
                    || active_generation != checkpoint.active_generation
                    || active_digest != checkpoint.active_digest
                {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
                Ok(Self {
                    organism_id,
                    profile,
                    bank: active_bank,
                    compaction: checkpoint,
                    staged_bank: None,
                })
            }
        }
    }
}

impl PortableMemoryRecordV2 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(PORTABLE_MEMORY_RECORD_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u64(self.memory_id_raw);
        digest.write_u64(self.organism_id_raw);
        digest.write_u64(self.sealed_sequence_id_raw);
        digest.write_u64(self.first_tick_raw);
        digest.write_u64(self.last_tick_raw);
        digest.write_u16(self.profile_id_raw);
        digest.write_u16(self.profile_schema_version);
        digest.write_u16(self.sensory_abi_version_raw);
        digest.write_u16(self.query_version_raw);
        digest.write_u32(self.action_id_raw);
        digest.write_u8(self.action_kind_raw);
        digest.write_u16(self.family_raw);
        digest.write_u64(self.tracked_object_id_raw);
        for bin in self.target_bins {
            digest.write_i8(bin);
        }
        write_portable_float_bits(&mut digest, &self.query_feature_bits)?;
        write_portable_float_bits(&mut digest, &self.target_latent_bits)?;
        write_portable_float_bits(&mut digest, &self.family_value_bits)?;
        digest.write_f32(portable_float(self.confidence_bits)?)?;
        digest.write_u16(self.salience_q16);
        digest.write_u32(self.observation_count);
        Ok(digest.finish256())
    }

    fn validate_for(
        &self,
        organism_id_raw: u64,
        profile: crate::SensorProfileIdentity,
    ) -> Result<(), ScaffoldContractError> {
        let record_profile = crate::SensorProfileIdentity {
            profile_id: crate::SensorProfileId(self.profile_id_raw),
            profile_schema_version: self.profile_schema_version,
            sensory_abi_version: self.sensory_abi_version_raw,
        };
        record_profile
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        MemoryId(self.memory_id_raw).validate()?;
        ExperienceSequenceId(self.sealed_sequence_id_raw).validate()?;
        ActionId(self.action_id_raw).validate()?;
        let kind = ActionKind::try_from_raw(self.action_kind_raw)?;
        let family = CandidateActionFamily::try_from_raw(
            u8::try_from(self.family_raw).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        )?;
        let query_features = portable_floats(&self.query_feature_bits)?;
        let target_latent = portable_floats(&self.target_latent_bits)?;
        let family_value = portable_floats(&self.family_value_bits)?;
        let confidence = portable_float(self.confidence_bits)?;
        if self.schema_version != PORTABLE_MEMORY_BANK_ASSET_SCHEMA_VERSION
            || self.organism_id_raw != organism_id_raw
            || record_profile != profile
            || self.first_tick_raw > self.last_tick_raw
            || self.observation_count == 0
            || query_features.len() != crate::MEMORY_QUERY_V2_FEATURE_COUNT
            || target_latent.len() != MEMORY_LATENT_V1_COUNT
            || family_value.len() != MEMORY_VALUE_V1_COUNT
            || target_bins(&query_features) != self.target_bins
            || !family.is_compatible_with(kind)
            || !(0.0..=1.0).contains(&confidence)
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }
}

impl PortableMemoryBankAssetV2 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(PORTABLE_MEMORY_BANK_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u64(self.organism_id_raw);
        digest.write_u16(self.profile.profile_id.raw());
        digest.write_u16(self.profile.profile_schema_version);
        digest.write_u16(self.profile.sensory_abi_version);
        digest.write_u32(self.capacity);
        digest.write_u32(self.max_feature_len);
        digest.write_u32(self.max_match_count);
        digest.write_f32(portable_float(self.min_match_score_bits)?)?;
        digest.write_f32(portable_float(self.empty_confidence_bits)?)?;
        digest.write_u64(self.generation);
        digest.write_u64(self.next_memory_id_raw);
        digest.write_u64(self.last_observed_sequence_id_raw);
        digest.write_u64(self.merge_count);
        digest.write_u64(self.eviction_count);
        digest.write_sequence_len(self.records.len());
        for record in &self.records {
            for word in record.canonical_digest {
                digest.write_u64(word);
            }
        }
        for word in self.active_bank_digest {
            digest.write_u64(word);
        }
        Ok(digest.finish256())
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.profile
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        OrganismId(self.organism_id_raw).validate()?;
        let config = portable_memory_config(self)?;
        if self.schema_version != PORTABLE_MEMORY_BANK_ASSET_SCHEMA_VERSION
            || self.records.len() > config.capacity
            || self.next_memory_id_raw == 0
            || (self.records.is_empty() && self.last_observed_sequence_id_raw != 0)
            || (!self.records.is_empty() && self.last_observed_sequence_id_raw == 0)
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let mut previous_id = 0;
        for record in &self.records {
            record.validate_for(self.organism_id_raw, self.profile)?;
            if record.memory_id_raw <= previous_id
                || record.memory_id_raw >= self.next_memory_id_raw
                || record.sealed_sequence_id_raw > self.last_observed_sequence_id_raw
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
            previous_id = record.memory_id_raw;
        }
        if self.canonical_digest != self.recompute_canonical_digest()? {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }
}

fn portable_bank_from_memory(
    organism_id: OrganismId,
    profile: crate::SensorProfileIdentity,
    bank: &MemoryBank,
) -> Result<PortableMemoryBankAssetV2, ScaffoldContractError> {
    organism_id.validate()?;
    profile.validate_contract()?;
    bank.config.validate_contract()?;
    bank.candidate_store
        .validate_for_capacity(bank.config.capacity)?;
    if bank.len != 0 || bank.records.iter().any(Option::is_some) {
        return Err(ScaffoldContractError::MemoryModeConflict);
    }
    let mut records = Vec::with_capacity(bank.candidate_store.records.len());
    for record in bank.candidate_store.records.values() {
        let mut portable = PortableMemoryRecordV2 {
            schema_version: PORTABLE_MEMORY_BANK_ASSET_SCHEMA_VERSION,
            memory_id_raw: record.memory_id.raw(),
            organism_id_raw: record.organism_id_raw,
            sealed_sequence_id_raw: record.source_sequence_id.raw(),
            first_tick_raw: record.first_tick.raw(),
            last_tick_raw: record.last_tick.raw(),
            profile_id_raw: record.profile_id_raw,
            profile_schema_version: record.profile_schema_version,
            sensory_abi_version_raw: record.sensory_abi_version_raw,
            query_version_raw: record.query_version_raw,
            action_id_raw: record.action_id_raw,
            action_kind_raw: record.action_kind_raw,
            family_raw: record.family_raw,
            tracked_object_id_raw: record.tracked_object_id_raw,
            target_bins: target_bins(&record.query_features),
            query_feature_bits: record
                .query_features
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            target_latent_bits: record
                .target_latent
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            family_value_bits: record
                .family_value
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            confidence_bits: record.confidence.to_bits(),
            salience_q16: record.salience_q16,
            observation_count: record.observation_count,
            canonical_digest: [0; 4],
        };
        portable.canonical_digest = portable.recompute_canonical_digest()?;
        portable.validate_for(organism_id.raw(), profile)?;
        records.push(portable);
    }
    let last_observed_sequence_id_raw = bank
        .candidate_store
        .last_sequence_by_organism
        .get(&organism_id.raw())
        .copied()
        .unwrap_or(0);
    if bank.candidate_store.last_sequence_by_organism.len()
        > usize::from(last_observed_sequence_id_raw != 0)
    {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    let mut asset = PortableMemoryBankAssetV2 {
        schema_version: PORTABLE_MEMORY_BANK_ASSET_SCHEMA_VERSION,
        organism_id_raw: organism_id.raw(),
        profile,
        capacity: u32::try_from(bank.config.capacity)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        max_feature_len: u32::try_from(bank.config.max_feature_len)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        max_match_count: u32::try_from(bank.config.max_match_count)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        min_match_score_bits: bank.config.min_match_score.to_bits(),
        empty_confidence_bits: bank.config.empty_confidence.raw().to_bits(),
        generation: bank.candidate_store.generation,
        next_memory_id_raw: bank.candidate_store.next_memory_id,
        last_observed_sequence_id_raw,
        merge_count: bank.candidate_store.merge_count,
        eviction_count: bank.candidate_store.eviction_count,
        records,
        active_bank_digest: bank.candidate_store.digest(bank.config.capacity)?,
        canonical_digest: [0; 4],
    };
    asset.canonical_digest = asset.recompute_canonical_digest()?;
    asset.validate_contract()?;
    Ok(asset)
}

fn memory_bank_from_portable(
    asset: PortableMemoryBankAssetV2,
) -> Result<MemoryBank, ScaffoldContractError> {
    asset
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    let config = portable_memory_config(&asset)?;
    let mut records = std::collections::BTreeMap::new();
    for portable in &asset.records {
        let query_features = portable_floats(&portable.query_feature_bits)?;
        let target_latent: [f32; MEMORY_LATENT_V1_COUNT] =
            portable_floats(&portable.target_latent_bits)?
                .try_into()
                .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let family_value: [f32; MEMORY_VALUE_V1_COUNT] =
            portable_floats(&portable.family_value_bits)?
                .try_into()
                .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let record = CandidateMemoryRecordV2 {
            schema_version: MEMORY_RECALL_SCHEMA_VERSION,
            memory_id: MemoryId(portable.memory_id_raw),
            organism_id_raw: portable.organism_id_raw,
            source_sequence_id: ExperienceSequenceId(portable.sealed_sequence_id_raw),
            first_tick: Tick::new(portable.first_tick_raw),
            last_tick: Tick::new(portable.last_tick_raw),
            profile_id_raw: portable.profile_id_raw,
            profile_schema_version: portable.profile_schema_version,
            sensory_abi_version_raw: portable.sensory_abi_version_raw,
            query_version_raw: portable.query_version_raw,
            action_id_raw: portable.action_id_raw,
            action_kind_raw: portable.action_kind_raw,
            family_raw: portable.family_raw,
            tracked_object_id_raw: portable.tracked_object_id_raw,
            query_features,
            target_latent,
            family_value,
            confidence: portable_float(portable.confidence_bits)?,
            salience_q16: portable.salience_q16,
            observation_count: portable.observation_count,
        };
        record.validate_contract()?;
        records.insert(record.memory_id.raw(), record);
    }
    let mut last_sequence_by_organism = std::collections::BTreeMap::new();
    if asset.last_observed_sequence_id_raw != 0 {
        last_sequence_by_organism
            .insert(asset.organism_id_raw, asset.last_observed_sequence_id_raw);
    }
    let mut candidate_store = CandidateMemoryStoreV2 {
        generation: asset.generation,
        next_memory_id: asset.next_memory_id_raw,
        merge_count: asset.merge_count,
        eviction_count: asset.eviction_count,
        records,
        last_sequence_by_organism,
        family_index: std::collections::BTreeMap::new(),
        target_index: std::collections::BTreeMap::new(),
    };
    candidate_store.validate_for_capacity(config.capacity)?;
    if candidate_store.digest(config.capacity)? != asset.active_bank_digest {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    candidate_store.rebuild_indices();
    Ok(MemoryBank {
        config,
        candidate_store,
        records: vec![None; config.capacity],
        next_write_index: 0,
        len: 0,
        next_memory_id: 1,
        last_inserted_ticks: Vec::new(),
    })
}

fn portable_memory_config(
    asset: &PortableMemoryBankAssetV2,
) -> Result<MemoryBankConfig, ScaffoldContractError> {
    MemoryBankConfig::new(
        usize::try_from(asset.capacity).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        usize::try_from(asset.max_feature_len)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        usize::try_from(asset.max_match_count)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        portable_float(asset.min_match_score_bits)?,
        Confidence::new(portable_float(asset.empty_confidence_bits)?)?,
    )
    .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)
}

fn portable_float(bits: u32) -> Result<f32, ScaffoldContractError> {
    let value = f32::from_bits(bits);
    if value.is_finite() && bits != (-0.0_f32).to_bits() {
        Ok(value)
    } else {
        Err(ScaffoldContractError::InvalidMemoryQuery)
    }
}

fn portable_floats(bits: &[u32]) -> Result<Vec<f32>, ScaffoldContractError> {
    bits.iter().copied().map(portable_float).collect()
}

fn write_portable_float_bits(
    digest: &mut CanonicalDigestBuilder,
    bits: &[u32],
) -> Result<(), ScaffoldContractError> {
    digest.write_sequence_len(bits.len());
    for value in bits {
        digest.write_f32(portable_float(*value)?)?;
    }
    Ok(())
}
