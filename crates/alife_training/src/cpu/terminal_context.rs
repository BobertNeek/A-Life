//! Explicit experimental CPU terminal-memory context, separate from the 36-lane ABI.
//! Fixed-in-life head weights; existing waking biological learner remains unchanged.
#[path = "terminal_head.rs"]
pub mod head;
#[path = "native_terminal.rs"]
pub mod reader;
use alife_core::*;
use serde::{Deserialize, Serialize};
const PARENT_SHA256: &str = "8f84afd6ac2ec575a8df61d18a0a18ac17296615a9ef7af7d22df416b3dd0ad7";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalContextConfig2135 {
    pub schema_version: u16,
    pub parent_sha256: String,
    pub head_recipe: [u8; 11],
    pub mask_new_context: bool,
}
impl TerminalContextConfig2135 {
    pub fn new(gain: f32) -> Result<Self, String> {
        let h = head::Head::new(true, gain).map_err(|_| "invalid terminal gain")?;
        Ok(Self {
            schema_version: 1,
            parent_sha256: PARENT_SHA256.into(),
            head_recipe: h.to_bytes(),
            mask_new_context: false,
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.parent_sha256 != PARENT_SHA256 {
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
}
/// Worst-case reservation before export or query. Bank >64 is explicit no-read
/// degradation, never record-ID truncation. Tariff v1: 1 ATP/1,000,000 units.
pub fn verify_terminal_parent2135(asset: &FoundationWeightAsset) -> Result<(), String> {
    let expected = Blake3Digest::from_bytes([
        122, 192, 140, 116, 78, 57, 82, 32, 128, 23, 16, 252, 126, 217, 107, 130, 52, 169, 88, 187,
        173, 215, 72, 197, 218, 124, 186, 46, 179, 243, 9, 240,
    ]);
    if asset.digest() != expected {
        return Err("terminal context requires exact pursuit-memory16 parent".into());
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
pub fn prepare_terminal2135(
    config: &TerminalContextConfig2135,
    memory: &MemorySidecarState,
    recall: &FinalizedMemoryRecall,
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
    let (units, debit) = reserve_terminal_work2135(records, approach_count)?;
    let mut result = TerminalContextReceipt2135 {
        schema_version: 1,
        config: config.clone(),
        bank_digest: [0; 4],
        bank_generation: 0,
        records,
        candidates: Vec::new(),
        reserved_work_units: units,
        reserved_atp_q16: debit,
        capacity_rejected: records > 64,
    };
    if records > 64 {
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
}
