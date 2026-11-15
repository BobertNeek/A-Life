//! Opaque same-lookup basis. Tariffs are fixed logical events, not machine instructions.
use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestOutcomeWork2286 {
    pub selection: u64,
    pub provenance_capture: u64,
    pub provenance_cache_copy: u64,
    pub basis_construction: u64,
    pub support_validation: u64,
    pub cpu_transform: u64,
}
impl IngestOutcomeWork2286 {
    pub fn total(&self) -> Result<u64, ScaffoldContractError> {
        [
            self.selection,
            self.provenance_capture,
            self.provenance_cache_copy,
            self.basis_construction,
            self.support_validation,
            self.cpu_transform,
        ]
        .into_iter()
        .try_fold(0_u64, |a, b| {
            a.checked_add(b)
                .ok_or(ScaffoldContractError::ScalarOutOfRange)
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryIngestOutcomeRow2286 {
    pub candidate_index: u16,
    pub query_digest: [u64; 4],
    /// Ordinary raw valence and energy, before the one confidence multiplication.
    pub values: [f32; 2],
    pub confidence: f32,
    pub source_count: u16,
    pub best_source: Option<MemoryId>,
    pub sources: MemoryFamilySourcesV1,
    pub provenance_valid: bool,
    pub support_eligible: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryIngestOutcomeEvidence2286 {
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub profile: crate::SensorProfileIdentity,
    pub base_frame_digest: PerceptionBaseDigest,
    pub bank_digest: [u64; 4],
    pub input_generation: u64,
    pub candidate_count: u16,
    pub rows: Vec<OrdinaryIngestOutcomeRow2286>,
    pub work: IngestOutcomeWork2286,
}
/// Not deserializable and no public constructor: diagnostic wire evidence is
/// never accepted as an operational override. Constructed only by Core lookup.
#[derive(Debug, PartialEq)]
pub struct OrdinaryIngestOutcomeBasis2286 {
    pub(super) evidence: OrdinaryIngestOutcomeEvidence2286,
}
impl OrdinaryIngestOutcomeBasis2286 {
    pub fn evidence(&self) -> &OrdinaryIngestOutcomeEvidence2286 {
        &self.evidence
    }
    pub fn into_evidence(self) -> OrdinaryIngestOutcomeEvidence2286 {
        self.evidence
    }
    /// The standard encoder already validates the actual joint recall/frame.
    /// Check primitive cross-binding without another context encoding or hash.
    /// Return the actual deterministic binding event count, not a tariff guess.
    pub fn validate_for_recall(
        &self,
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
    ) -> Result<u64, ScaffoldContractError> {
        let e = &self.evidence;
        let r = recall.receipt();
        if e.organism_id != frame.organism_id()
            || e.tick != frame.tick()
            || e.profile != frame.profile_provenance().identity()
            || e.base_frame_digest != frame.base_digest()
            || r.base_frame_digest != frame.base_digest()
            || e.bank_digest != r.bank_digest
            || e.input_generation != r.input_generation
            || e.organism_id.raw() != r.organism_id_raw
            || usize::from(e.candidate_count) != frame.candidates().len()
            || recall.candidate_keys().len() != frame.candidates().len()
            || recall.final_frame_digest() != frame.frame_digest()
            || e.rows.len() > frame.candidates().len()
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let mut actual = 16; // Fixed anchor validation event tariff.
        let mut rows = e.rows.iter();
        for (i, c) in frame.candidates().iter().enumerate() {
            actual += 1; // Executed family-selection event for every candidate.
            if c.family != CandidateActionFamily::Ingest {
                continue;
            }
            let row = rows
                .next()
                .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
            actual += 1;
            if usize::from(row.candidate_index) != i {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
            let digest = recall.candidate_keys()[i].query().canonical_digest();
            for (a, b) in row.query_digest.iter().zip(digest) {
                actual += 1;
                if *a != b {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
            }
        }
        if rows.next().is_some() {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(actual)
    }
}
/// Four bounded slots plus sixteen logical source-field events per occupied slot.
pub(super) fn provenance_tariff(s: &MemoryFamilySourcesV1) -> u64 {
    4 + 16 * s.sources.iter().flatten().count() as u64
}

/// Validate bounded provenance while constructing the opaque same-lookup basis.
/// This is not an object-kind/reward filter and does not require each contributing
/// source's confidence to be positive: the aggregate confidence supplies support.
pub(super) fn validate_support(
    sources: &MemoryFamilySourcesV1,
    count: u16,
    best: Option<MemoryId>,
    confidence: f32,
    organism: OrganismId,
    profile: crate::SensorProfileIdentity,
) -> Result<(bool, u64), ScaffoldContractError> {
    let mut actual = 4;
    let mut seen = 0;
    let mut first = None;
    for slot in &sources.sources {
        actual += 1;
        if let Some(s) = slot {
            actual += 16;
            seen += 1;
            if first.is_none() {
                first = Some(s.memory_id);
            }
            if s.organism_id != organism
                || s.profile != profile
                || s.family != CandidateActionFamily::Ingest
                || s.memory_id.raw() == 0
                || !s.episode_confidence.is_finite()
                || !(0.0..=1.0).contains(&s.episode_confidence)
                || !s.similarity.is_finite()
                || !s.aggregate_weight.is_finite()
                || s.similarity < 0.0
                || s.aggregate_weight < 0.0
            {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
        }
    }
    if seen != usize::from(count)
        || first != best
        || !confidence.is_finite()
        || !(0.0..=1.0).contains(&confidence)
    {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    Ok((count > 0 && confidence > 0.0, actual))
}
