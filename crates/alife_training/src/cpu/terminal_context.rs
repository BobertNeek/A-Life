//! Explicit experimental CPU terminal-memory context, separate from the 36-lane ABI.
//! Fixed-in-life head weights; existing waking biological learner remains unchanged.
#[path = "terminal_head.rs"]
pub mod head;
#[path = "native_terminal.rs"]
pub mod reader;
use alife_core::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const TERMINAL_PARENT_SHA256: &str =
    "ec710f2176ae4cfccdd342953c7778385f1880efc7a0216ca5412aad68d6628c";
pub const TERMINAL_GENOME_SHA256: &str =
    "5c63a675c89be9195de6b2bdedfc51a0f6f95fa11917ef8db6b75492bba025f7";
const PARENT_PHENOTYPE: [u64; 4] = [
    8227526484390048264,
    5321359027925582130,
    15675904855675944572,
    6368561322487548543,
];
const PARENT_COMPILER_INPUTS: [u64; 4] = [
    14503636050429498377,
    16782976114342650278,
    15437838106123747822,
    12386633400297544944,
];

/// Host wire version, not the independent terminal recipe version.
/// Default-off stays byte-compatible; enabled heads must not be discarded by old readers.
pub(crate) fn terminal_host_checkpoint_schema2135(has_head: bool) -> u16 {
    if has_head { 3 } else { 2 }
}

pub(crate) fn validate_terminal_host_checkpoint_schema2135(
    version: u16,
    has_head: bool,
) -> Result<(), &'static str> {
    match (version, has_head) {
        (2, false) | (3, true) => Ok(()),
        (2, true) => Err(
            "prototype version2 terminal checkpoint is ambiguous; automatic migration is unsupported",
        ),
        (3, false) => Err("version3 CPU checkpoint is missing its terminal head"),
        _ => Err("unsupported CPU checkpoint schema"),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalContextConfig2135 {
    pub schema_version: u16,
    pub parent_sha256: String,
    pub construction_genome_sha256: String,
    pub motor_head_count: u8,
    pub compiled_phenotype_hash: [u64; 4],
    pub compiler_inputs_digest: [u64; 4],
    pub head_recipe: [u8; 11],
    pub mask_new_context: bool,
}
impl TerminalContextConfig2135 {
    pub fn new(gain: f32) -> Result<Self, String> {
        let h = head::Head::new(true, gain).map_err(|_| "invalid terminal gain")?;
        Ok(Self {
            schema_version: 2,
            parent_sha256: TERMINAL_PARENT_SHA256.into(),
            construction_genome_sha256: TERMINAL_GENOME_SHA256.into(),
            motor_head_count: 2,
            compiled_phenotype_hash: PARENT_PHENOTYPE,
            compiler_inputs_digest: PARENT_COMPILER_INPUTS,
            head_recipe: h.to_bytes(),
            mask_new_context: false,
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2
            || self.parent_sha256 != TERMINAL_PARENT_SHA256
            || self.construction_genome_sha256 != TERMINAL_GENOME_SHA256
            || self.motor_head_count != 2
            || self.compiled_phenotype_hash != PARENT_PHENOTYPE
            || self.compiler_inputs_digest != PARENT_COMPILER_INPUTS
        {
            return Err("terminal context recipe identity mismatch".into());
        }
        let h = head::Head::from_bytes(&self.head_recipe)
            .map_err(|_| "invalid terminal head recipe")?;
        if !h.enabled() {
            return Err("off is None, not an enabled research recipe".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalCandidateReceipt2135 {
    pub candidate_index: u16,
    pub query_digest: [u64; 4],
    pub source_memory_ids: Vec<u64>,
    pub source_sequence_ids: Vec<u64>,
    pub source_action_ids: Vec<u32>,
    pub consumed_source_count: usize,
    pub hunger_now: f32,
    pub desirability: Option<f32>,
    pub delta: f32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalAdmission2135 {
    Admitted,
    InsufficientAtp {
        required_atp_q16: u32,
        available_atp_q16: u32,
    },
    BankCapacityExceeded {
        records: usize,
        limit: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalAdmissionPlan2135 {
    pub admission: TerminalAdmission2135,
    pub requested_work_units: u64,
    pub requested_atp_q16: u32,
    pub charged_work_units: u64,
    pub charged_atp_q16: u32,
    /// Mandatory bounded admission bookkeeping is recorded even when no residual
    /// dispatch ATP remains. Native cognitive-work accounting bills all its work;
    /// the dispatch-reservation ledger debits only available residual ATP.
    pub admission_check_unfunded_atp_q16: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalContextReceipt2135 {
    pub schema_version: u16,
    pub config: TerminalContextConfig2135,
    pub bank_digest: [u64; 4],
    pub bank_generation: u64,
    pub records: usize,
    pub candidates: Vec<TerminalCandidateReceipt2135>,
    /// Conservative version1 research tariff, not hardware FLOPs. Includes
    /// bounded bank export/validation/hash/copy and per-query validation/read/head.
    pub reserved_work_units: u64,
    pub reserved_atp_q16: u32,
    pub capacity_rejected: bool,
    pub admission: TerminalAdmission2135,
    pub requested_work_units: u64,
    pub requested_atp_q16: u32,
    pub admission_check_unfunded_atp_q16: u32,
}
/// Cheap ongoing check of the actual compiled construction, not weights alone.
/// These digests were obtained by native compilation of the exact exported pair.
pub fn verify_terminal_parent2135(
    asset: &FoundationWeightAsset,
    p: &BrainPhenotype,
) -> Result<(), String> {
    let expected = Blake3Digest::from_bytes([
        224, 252, 172, 202, 37, 120, 184, 145, 35, 70, 23, 108, 246, 112, 221, 152, 69, 131, 49,
        17, 45, 92, 183, 43, 52, 64, 61, 151, 61, 176, 43, 91,
    ]);
    if asset.digest() != expected
        || p.phenotype_hash().0 != PARENT_PHENOTYPE
        || p.compiler_inputs_digest() != PARENT_COMPILER_INPUTS
        || p.cognitive_architecture().motor_head_count() != 2
        || p.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
    {
        return Err(
            "terminal context requires exact two-head bearing4 asset and compiled construction"
                .into(),
        );
    }
    let channels = p
        .candidate_decoder()
        .factorized_motor_channels(p)
        .map_err(|e| e.to_string())?;
    if channels != [MotorChannel::Locomotion, MotorChannel::Manipulation] {
        return Err("terminal parent motor layout differs".into());
    }
    Ok(())
}
/// Full exported construction-genome verification at enable/cold admission, not
/// an additional uncharged memory operation on each decision's hot path.
pub fn verify_terminal_genome2135(genome: &CreatureGenome) -> Result<(), String> {
    let bytes = serde_json::to_vec(genome).map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&bytes)) != TERMINAL_GENOME_SHA256
        || genome
            .brain
            .construction
            .maternal_architecture
            .motor_head_count()
            != 2
        || genome
            .brain
            .construction
            .paternal_architecture
            .motor_head_count()
            != 2
    {
        return Err("terminal parent construction genome differs".into());
    }
    Ok(())
}
pub fn reserve_terminal_work2135(records: usize, candidates: usize) -> Result<(u64, u32), String> {
    if candidates > MAX_ACTION_CANDIDATES {
        return Err("too many terminal candidates".into());
    }
    let units = if records > 64 {
        32
    } else {
        128 + (records as u64) * 4096 + (candidates as u64) * (4096 + (records as u64) * 4096)
    };
    let debit = u32::try_from((units * 65535).div_ceil(1_000_000))
        .map_err(|_| "terminal debit overflow")?;
    Ok((units, debit))
}
/// Same unchanged read/head tariff for Active, ZeroHead and post-read Mask.
/// Only metadata and existing query identities are inspected before admission.
pub fn admit_terminal_work2135(
    records: usize,
    query_count: usize,
    approach_count: usize,
    available: u32,
) -> Result<TerminalAdmissionPlan2135, String> {
    if query_count > MAX_ACTION_CANDIDATES || approach_count > query_count {
        return Err("invalid terminal admission counts".into());
    }
    let (read_work_units, _) = reserve_terminal_work2135(records, approach_count)?;
    let check_work_units = 128 + query_count as u64;
    let requested_work_units = read_work_units.max(check_work_units);
    let requested_atp_q16 = u32::try_from((requested_work_units * 65535).div_ceil(1_000_000))
        .map_err(|_| "terminal reservation overflow")?;
    let admission = if records > 64 {
        TerminalAdmission2135::BankCapacityExceeded { records, limit: 64 }
    } else if requested_atp_q16 > available {
        TerminalAdmission2135::InsufficientAtp {
            required_atp_q16: requested_atp_q16,
            available_atp_q16: available,
        }
    } else {
        TerminalAdmission2135::Admitted
    };
    let charged_work_units = if admission == TerminalAdmission2135::Admitted {
        requested_work_units
    } else {
        check_work_units
    };
    let check_debit = u32::try_from((charged_work_units * 65535).div_ceil(1_000_000))
        .map_err(|_| "terminal check debit overflow")?;
    let charged_atp_q16 = check_debit.min(available);
    Ok(TerminalAdmissionPlan2135 {
        admission,
        requested_work_units,
        requested_atp_q16,
        charged_work_units,
        charged_atp_q16,
        admission_check_unfunded_atp_q16: check_debit - charged_atp_q16,
    })
}
pub fn prepare_terminal2135(
    config: &TerminalContextConfig2135,
    memory: &MemorySidecarState,
    recall: &FinalizedMemoryRecall,
    available_atp_q16: u32,
) -> Result<TerminalContextReceipt2135, String> {
    config.validate()?;
    let records = memory.bank().len();
    let keys = recall.candidate_keys();
    let approach_count = keys
        .iter()
        .filter(|k| {
            k.query().action_family() == CandidateActionFamily::Approach
                && k.query().tracked_object_id().is_some()
        })
        .count();
    let plan = admit_terminal_work2135(records, keys.len(), approach_count, available_atp_q16)?;
    let mut result = TerminalContextReceipt2135 {
        schema_version: 2,
        config: config.clone(),
        bank_digest: [0; 4],
        bank_generation: 0,
        records,
        candidates: Vec::new(),
        reserved_work_units: plan.charged_work_units,
        reserved_atp_q16: plan.charged_atp_q16,
        capacity_rejected: records > 64,
        admission: plan.admission,
        requested_work_units: plan.requested_work_units,
        requested_atp_q16: plan.requested_atp_q16,
        admission_check_unfunded_atp_q16: plan.admission_check_unfunded_atp_q16,
    };
    if result.admission != TerminalAdmission2135::Admitted {
        // No bank export, record validation, appearance search or head execution.
        // Empty candidates ensure the ordinary scores and learner stay intact.
        return Ok(result);
    }
    let bank = memory.export_active_bank().map_err(|e| e.to_string())?;
    result.bank_digest = bank.canonical_digest;
    result.bank_generation = bank.generation;
    let h = head::Head::from_bytes(&config.head_recipe).map_err(|_| "invalid terminal recipe")?;
    for key in keys {
        let query = key.query();
        if query.action_family() != CandidateActionFamily::Approach
            || query.tracked_object_id().is_none()
        {
            continue;
        }
        let read = reader::read(reader::Policy::AppearanceOnlyConsumedV1, &bank, query)?
            .ok_or("missing terminal read")?;
        let desirability = read.context.as_ref().map(|c| c.desirability);
        let work = head::Work::reader(
            read.records_examined as u32,
            read.appearance_comparisons as u32,
        )
        .map_err(|_| "invalid terminal work")?;
        let score = h
            .evaluate(
                true,
                if config.mask_new_context {
                    desirability.map(|_| 0.0)
                } else {
                    desirability
                },
                work,
            )
            .map_err(|_| "invalid terminal score")?;
        let sources = read
            .context
            .as_ref()
            .map(|c| c.sources.as_slice())
            .unwrap_or(&[]);
        result.candidates.push(TerminalCandidateReceipt2135 {
            candidate_index: query.candidate_index(),
            query_digest: query.canonical_digest(),
            source_memory_ids: sources.iter().map(|s| s.memory_id).collect(),
            source_sequence_ids: sources.iter().map(|s| s.sequence_id).collect(),
            source_action_ids: sources.iter().map(|s| s.action_id).collect(),
            consumed_source_count: sources.len(),
            hunger_now: query.features()[12],
            desirability,
            delta: score.delta,
        });
    }
    Ok(result)
}
pub fn apply_terminal2135(
    receipt: &TerminalContextReceipt2135,
    scores: &mut [f32],
) -> Result<(), String> {
    for c in &receipt.candidates {
        let score = scores
            .get_mut(usize::from(c.candidate_index))
            .ok_or("terminal candidate absent")?;
        if c.delta != 0.0 {
            *score += c.delta;
        }
        if !score.is_finite() {
            return Err("nonfinite terminal score".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipe_roundtrip_and_strict_identity() {
        let c = TerminalContextConfig2135::new(16.0).unwrap();
        c.validate().unwrap();
        let bytes = serde_json::to_vec(&c).unwrap();
        let back: TerminalContextConfig2135 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, c);
        let mut bad = c.clone();
        bad.parent_sha256.clear();
        assert!(bad.validate().is_err());
        let mut bad = c.clone();
        bad.motor_head_count = 1;
        assert!(bad.validate().is_err());
        let mut bad = c.clone();
        bad.construction_genome_sha256.clear();
        assert!(bad.validate().is_err());
        let mut bad = c.clone();
        bad.compiled_phenotype_hash[0] ^= 1;
        assert!(bad.validate().is_err());
        let mut bad = c;
        bad.head_recipe[0] = 0;
        assert!(bad.validate().is_err());
    }
    #[test]
    fn reserve_is_bounded_and_capacity_failure_is_not_free() {
        let a = reserve_terminal_work2135(9, 3).unwrap();
        assert!(a.0 > 0 && a.1 > 0);
        let b = reserve_terminal_work2135(64, 3).unwrap();
        assert!(b.0 > a.0 && b.1 > a.1);
        let c = reserve_terminal_work2135(65, 3).unwrap();
        assert_eq!(c.0, 32);
        assert!(c.1 > 0);
        assert!(reserve_terminal_work2135(1, MAX_ACTION_CANDIDATES + 1).is_err());
    }
    #[test]
    fn low_atp_abstains_without_blocking_ordinary_step() {
        let required = reserve_terminal_work2135(9, 2).unwrap().1;
        for available in [0, 1, required - 1] {
            let p = admit_terminal_work2135(9, 5, 2, available).unwrap();
            assert!(matches!(
                p.admission,
                TerminalAdmission2135::InsufficientAtp { .. }
            ));
            assert_eq!(p.charged_work_units, 133);
            assert!(p.charged_atp_q16 <= available);
            if available == 0 {
                assert_eq!(p.charged_atp_q16, 0);
                assert!(p.admission_check_unfunded_atp_q16 > 0);
            }
        }
        assert_eq!(
            admit_terminal_work2135(9, 5, 2, required)
                .unwrap()
                .admission,
            TerminalAdmission2135::Admitted
        );
    }
    #[test]
    fn capacity_and_active_zero_mask_have_identical_admission() {
        for available in [0, 1, u32::MAX] {
            let p = admit_terminal_work2135(65, 5, 2, available).unwrap();
            assert_eq!(
                p.admission,
                TerminalAdmission2135::BankCapacityExceeded {
                    records: 65,
                    limit: 64
                }
            );
            assert_eq!(p.charged_work_units, 133);
            assert!(p.charged_atp_q16 <= available);
            let reference = admit_terminal_work2135(9, 5, 2, available).unwrap();
            for (gain, mask) in [(16.0, false), (0.0, false), (16.0, true)] {
                let mut c = TerminalContextConfig2135::new(gain).unwrap();
                c.mask_new_context = mask;
                c.validate().unwrap();
                assert_eq!(
                    admit_terminal_work2135(9, 5, 2, available).unwrap(),
                    reference
                );
            }
        }
    }
}

#[cfg(test)]
mod host_schema_tests {
    use super::*;

    // Minimal wire projection: same serde omission rule and old permissive reader.
    // Full host/world cold restoration remains a remote qualification gate.
    #[derive(Serialize, Deserialize)]
    struct Envelope {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        terminal_context2135: Option<TerminalContextConfig2135>,
        schema_version: u16,
    }
    #[derive(Serialize, Deserialize)]
    struct OldEnvelope {
        schema_version: u16,
    }
    #[test]
    fn off_wire_bytes_remain_version2() {
        let new = Envelope {
            terminal_context2135: None,
            schema_version: terminal_host_checkpoint_schema2135(false),
        };
        let old = OldEnvelope { schema_version: 2 };
        assert_eq!(
            serde_json::to_vec(&new).unwrap(),
            serde_json::to_vec(&old).unwrap()
        );
    }
    #[test]
    fn strict_head_version_consistency() {
        for version in [0, 1, 2, 3, 4, u16::MAX] {
            for has_head in [false, true] {
                assert_eq!(
                    validate_terminal_host_checkpoint_schema2135(version, has_head).is_ok(),
                    matches!((version, has_head), (2, false) | (3, true))
                );
            }
        }
    }
    #[test]
    fn enabled_wire_roundtrip_and_old_reader_rejection() {
        let config = TerminalContextConfig2135::new(f32::MAX).unwrap();
        let new = Envelope {
            terminal_context2135: Some(config.clone()),
            schema_version: terminal_host_checkpoint_schema2135(true),
        };
        let bytes = serde_json::to_vec(&new).unwrap();
        let restored: Envelope = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.terminal_context2135, Some(config));
        assert!(
            validate_terminal_host_checkpoint_schema2135(
                restored.schema_version,
                restored.terminal_context2135.is_some()
            )
            .is_ok()
        );
        let old: OldEnvelope = serde_json::from_slice(&bytes).unwrap();
        // Original host admission rejects every version other than 2 (version1 separately).
        assert_ne!(old.schema_version, 2);
        assert_eq!(old.schema_version, 3);
    }
}
