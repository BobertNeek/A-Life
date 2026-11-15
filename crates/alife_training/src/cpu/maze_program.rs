//! Explicit finite research circuit. These are policy edits, not acquired learning.
use super::*;
use alife_core::*;

/// Exact graph coordinates: sensory write, two recurrent phase loops, cross-phase,
/// and four bridges to existing Approach/Inspect decoder source populations.
pub const MAZE_RECURRENT_EDGES2346: [(usize, u32, u32); 14] = [
    (698, 49, 909),
    (699, 49, 910),
    (14288, 909, 1600),
    (15629, 1600, 909),
    (14305, 910, 1601),
    (15646, 1601, 910),
    (14289, 909, 1601),
    (15645, 1601, 909),
    (14304, 910, 1600),
    (15630, 1600, 910),
    (10366, 909, 1776),
    (10362, 909, 1760),
    (10423, 910, 1777),
    (10419, 910, 1761),
];
pub(super) fn recurrent_coordinate(p: &BrainPhenotype, index: usize) -> bool {
    MAZE_RECURRENT_EDGES2346.iter().any(|&(i, s, t)| {
        i == index
            && p.synapses().get(i).is_some_and(|edge| {
                edge.source() == s
                    && edge.target() == t
                    && matches!(edge.kind(), CompiledSynapseKind::Recurrent)
            })
    })
}
pub(super) fn program_coordinate(p: &BrainPhenotype, index: usize) -> bool {
    recurrent_coordinate(p, index)
        || matches!(p.synapses().get(index).map(|s|s.kind()),Some(CompiledSynapseKind::Decoder(c)) if c.head()==DecoderHeadKind::ActionCandidate && matches!(c.family(),CandidateActionFamily::Approach|CandidateActionFamily::Inspect))
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MazeBlockedHistory2346 {
    pub organism_id: OrganismId,
    pub source_tick: Tick,
    pub source_sequence: ExperienceSequenceId,
    pub source_frame: PerceptionFrameDigest,
    pub primitive: ActionId,
    pub blocked: bool,
}
impl MazeBlockedHistory2346 {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.source_sequence.validate()?;
        if self.source_frame.0 == [0;4] || !matches!(self.primitive.raw(), 101 | 102 | 105) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
    pub(super) fn from_outcome(
        frame: &PerceptionFrame,
        sequence: ExperienceSequenceId,
        joint: &JointPhysicalOutcome,
    ) -> Result<(Option<Self>, u64), ScaffoldContractError> {
        let mut work = 1;
        let mut found = None;
        for channel in &joint.channel_outcomes {
            work += 2;
            if channel.channel != MotorChannel::Locomotion {
                continue;
            }
            work += 2;
            let Some(e) = channel.execution_evidence else {
                continue;
            };
            if !matches!(e.primitive.raw(), 101 | 102 | 105) {
                continue;
            }
            if found.is_some() {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            work += 7;
            found = Some(Self {
                organism_id: frame.organism_id(),
                source_tick: frame.tick(),
                source_sequence: sequence,
                source_frame: frame.frame_digest(),
                primitive: e.primitive,
                blocked: channel.physical.contact == PhysicalContactKind::Blocked,
            });
        }
        Ok((found, work))
    }
}
impl CpuFoundationTrainer {
    /// Only this named bounded module may calibrate these fourteen recurrent edges.
    /// Every private trace and optimizer field remains unchanged.
    pub fn calibrated_maze_program_copy2346(
        &self,
        edits: &[(u32, f32)],
    ) -> Result<Self, crate::TrainingError> {
        if edits.is_empty() {
            return Err(super::invalid());
        }
        let mut cp = self.checkpoint();
        let mut seen = std::collections::BTreeSet::new();
        let mut changed = false;
        for &(i, w) in edits {
            let i = i as usize;
            if !w.is_finite()
                || w.abs() > 16.
                || !seen.insert(i)
                || !program_coordinate(self.phenotype(), i)
            {
                return Err(super::invalid());
            }
            changed |= cp.weights[i].to_bits() != w.to_bits();
            cp.weights[i] = w;
        }
        if !changed {
            return Err(super::invalid());
        }
        let mut next =
            Self::from_checkpoint(self.phenotype.clone(), self.source_foundation.clone(), cp)?;
        next.set_replay_execution(self.replay_execution());
        Ok(next)
    }
}
