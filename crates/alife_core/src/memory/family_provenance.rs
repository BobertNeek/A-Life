//! Opt-in provenance copied during the existing family aggregate pass.
//! No query, record selection, reward/phase filter or extra bank export.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryFamilySourceV1 {
    pub memory_id: MemoryId,
    pub organism_id: OrganismId,
    pub profile: crate::SensorProfileIdentity,
    pub family: CandidateActionFamily,
    /// The prior episode's identity. Category recall can legitimately differ
    /// from the current query target; it is never another current candidate row.
    pub prior_tracked_target: Option<crate::TrackedObjectId>,
    pub sequence_id: ExperienceSequenceId,
    pub first_tick: Tick,
    pub last_tick: Tick,
    pub observation_count: u32,
    pub episode_confidence: f32,
    pub similarity: f32,
    pub aggregate_weight: f32,
    pub outcome_contact: Option<PhysicalContactKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryFamilySourcesV1 {
    pub sources: [Option<MemoryFamilySourceV1>; MEMORY_RECALL_TOP_K],
    pub category_generalized: bool,
}

impl MemoryFamilySourcesV1 {
    pub(super) fn empty() -> Self {
        Self {
            sources: [None; MEMORY_RECALL_TOP_K],
            category_generalized: false,
        }
    }
}
