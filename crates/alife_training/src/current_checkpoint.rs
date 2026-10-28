//! Self-contained accepted fresh-cohort state. Episode/replay history is temporary.
use crate::FoundationTrainingHandoffV1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path};

const MAGIC: &[u8; 16] = b"ALIFE-CURRENT-1\n";
const HEADER: usize = 16 + 8 + 32;
pub const MAX_CURRENT_CHECKPOINT_BYTES: u64 = 256 * 1024 * 1024;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
mod same_individual;
pub use same_individual::*;
mod terminal;
pub use terminal::*;

/// Exact union of consumed, reserved, interrupted and assessment seeds. Ranges
/// are coalesced, not a receipt log. Unknown historical freshness stays unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointSeedsV1 {
    pub historical_freshness: String,
    pub excluded_ranges: Vec<[u64; 2]>,
    pub pending_range: Option<[u64; 2]>,
}
impl CheckpointSeedsV1 {
    pub fn validate(&self) -> Result<()> {
        if !matches!(
            self.historical_freshness.as_str(),
            "operator-attested-complete" | "unknown"
        ) || self.excluded_ranges.len() > 1_000_000
        {
            return Err("invalid checkpoint seed metadata".into());
        }
        let mut previous = None;
        for [first, last] in &self.excluded_ranges {
            if *first == 0
                || first > last
                || previous.is_some_and(|end: u64| end.saturating_add(1) >= *first)
            {
                return Err("checkpoint exclusions must be sorted, disjoint and coalesced".into());
            }
            previous = Some(*last);
        }
        if let Some([a, b]) = self.pending_range {
            if a == 0 || a > b || !self.excluded_ranges.iter().any(|[x, y]| *x <= a && b <= *y) {
                return Err("pending allocation must be fully excluded and nonempty".into());
            }
        }
        Ok(())
    }
    pub fn contains(&self, seed: u64) -> bool {
        self.excluded_ranges
            .iter()
            .any(|[a, b]| *a <= seed && seed <= *b)
    }
    pub fn reserve(&mut self, first: u64, last: u64) -> Result<()> {
        self.validate()?;
        if self.pending_range == Some([first, last]) {
            return Ok(());
        }
        if self.pending_range.is_some() {
            return Err("resume the current unused seed allocation first".into());
        }
        if first == 0
            || first > last
            || self
                .excluded_ranges
                .iter()
                .any(|[a, b]| first <= *b && *a <= last)
        {
            return Err("allocation overlaps an excluded seed or is invalid".into());
        }
        self.excluded_ranges.push([first, last]);
        self.excluded_ranges.sort_unstable();
        let mut ranges: Vec<[u64; 2]> = Vec::new();
        for [a, b] in self.excluded_ranges.drain(..) {
            if let Some(last) = ranges
                .last_mut()
                .filter(|last| last[1].saturating_add(1) >= a)
            {
                last[1] = last[1].max(b);
            } else {
                ranges.push([a, b]);
            }
        }
        self.excluded_ranges = ranges;
        self.pending_range = Some([first, last]);
        self.validate()
    }
    pub fn consume(&mut self, seed: u64) -> Result<()> {
        self.validate()?;
        let [first, last] = self.pending_range.ok_or("no owned seed allocation")?;
        if seed != first {
            return Err("dispatch must consume the next owned seed".into());
        }
        self.pending_range = if first == last {
            None
        } else {
            Some([first + 1, last])
        };
        Ok(())
    }
}

/// Latest private world/organisms and their exact declared asset closure. The
/// native layer admits the save and sleep journal through the existing loader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentLifeCheckpointV1 {
    pub world_json: String,
    pub sleep_journal_json: String,
    pub assets_base64: std::collections::BTreeMap<String, String>,
}
impl CurrentLifeCheckpointV1 {
    pub fn asset_bytes(&self, name: &str) -> Result<Vec<u8>> {
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.decode(
            self.assets_base64
                .get(name)
                .ok_or("missing private checkpoint asset")?,
        )?)
    }
    pub fn insert_asset(&mut self, name: String, bytes: &[u8]) {
        use base64::Engine;
        self.assets_base64.insert(
            name,
            base64::engine::general_purpose::STANDARD.encode(bytes),
        );
    }
}

/// One accepted founder and both optimizer heads, not an individual-life save.
/// No path, receipt, ancestry object or external compiler asset is referenced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentTrainingCheckpointV1 {
    pub schema_version: u16,
    pub seed: u64,
    pub founder_seed_base: u64,
    pub seeds: CheckpointSeedsV1,
    pub handoff: FoundationTrainingHandoffV1,
    pub life: Option<CurrentLifeCheckpointV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_individual: Option<SameIndividualCurrentV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_disposition: Option<CurrentTerminalDispositionV1>,
}
impl CurrentTrainingCheckpointV1 {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.seed == 0 || self.founder_seed_base == 0 {
            return Err("invalid current checkpoint identity".into());
        }
        self.seeds.validate()?;
        if !self.seeds.contains(self.seed) {
            return Err("accepted source seed must remain excluded".into());
        }
        if self
            .seeds
            .pending_range
            .is_some_and(|[a, b]| a <= self.seed && self.seed <= b)
        {
            return Err("accepted source seed cannot be unused".into());
        }
        if let Some(life) = &self.life {
            let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
            if save.deterministic_seed != self.seed {
                return Err("private life belongs to a different accepted episode seed".into());
            }
        }
        self.handoff.validate()?;
        match &self.same_individual {
            Some(tag) => tag.validate(self)?,
            None if self.handoff.continuation
                == crate::FoundationContinuationKindV1::FreshCohort => {}
            None => {
                return Err(
                    "same-individual handoff requires its exact private/progress tag".into(),
                )
            }
        }
        if let Some(disposition) = &self.terminal_disposition {
            disposition.validate(self)?;
        }
        self.handoff
            .policy_version
            .checked_add(1)
            .ok_or("policy overflow")?;
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let payload = serde_json::to_vec(self)?;
        if payload.len() as u64 + HEADER as u64 > MAX_CURRENT_CHECKPOINT_BYTES {
            return Err("current checkpoint byte limit exceeded".into());
        }
        let mut bytes = Vec::with_capacity(HEADER + payload.len());
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&Sha256::digest(&payload));
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER
            || bytes.len() as u64 > MAX_CURRENT_CHECKPOINT_BYTES
            || &bytes[..16] != MAGIC
            || u64::from_le_bytes(bytes[16..24].try_into()?) != (bytes.len() - HEADER) as u64
            || Sha256::digest(&bytes[HEADER..]).as_slice() != &bytes[24..HEADER]
        {
            return Err("current checkpoint length, format or SHA-256 mismatch".into());
        }
        let checkpoint: Self = serde_json::from_slice(&bytes[HEADER..])?;
        checkpoint.validate()?;
        Ok(checkpoint)
    }
    pub fn read(path: &Path) -> Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file() || metadata.len() > MAX_CURRENT_CHECKPOINT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "checkpoint must be a bounded regular file",
            )
            .into());
        }
        Self::decode(&fs::read(path)?)
    }
}
