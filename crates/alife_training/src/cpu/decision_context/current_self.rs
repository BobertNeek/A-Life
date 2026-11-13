//! Explicit current-self command readout. Never aliases perception or retained memory.
use super::*;
use alife_core::{
    ActionCandidate, ActionId, ActionKind, CandidateActionFamily, CandidateObservationRef,
    CanonicalDigestBuilder,
};
use serde::{Deserialize, Serialize};
pub const CURRENT_SELF_ID2000: &str = "current-self-hold-drive-2000-v1";
pub const CURRENT_SELF_SOURCE2000: usize = 1791;
fn hash<T: Serialize>(x: &T) -> Result<[u64; 4], TrainingError> {
    let mut d = CanonicalDigestBuilder::new(b"ALIFE-CURRENT-SELF2000-TICK");
    d.write_bytes(&serde_json::to_vec(x).map_err(|_| invalid())?);
    Ok(d.finish256())
}
/// Only the registered self HoldGaze primitive qualifies. Physical contact lane18 alone does not.
pub fn is_hold_intention2000(c: &ActionCandidate) -> bool {
    c.action_id == ActionId(606)
        && c.kind == ActionKind::Look
        && c.family == CandidateActionFamily::Inspect
        && c.observation == CandidateObservationRef::None
        && c.target.entity.is_none()
        && c.target.position.is_none()
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentSelfFrame2000 {
    pub schema_version: u16,
    pub identity: String,
    pub base_tick_digest: [u64; 4],
    pub hold_intentions: Vec<bool>,
}
impl CurrentSelfFrame2000 {
    /// Detached replay fixture binding, not authority to invent collector self-command intent.
    pub fn bind_replay(
        t: &crate::TrainingReplayTick,
        hold_intentions: Vec<bool>,
    ) -> Result<Self, TrainingError> {
        let f = Self {
            schema_version: 1,
            identity: CURRENT_SELF_ID2000.into(),
            base_tick_digest: hash(t)?,
            hold_intentions,
        };
        f.validate(t)?;
        Ok(f)
    }
    pub fn bind_candidates(
        t: &crate::TrainingReplayTick,
        candidates: &[ActionCandidate],
    ) -> Result<Self, TrainingError> {
        if candidates.len() != t.candidates.len() {
            return Err(invalid());
        }
        for (c, d) in candidates.iter().zip(&t.candidates) {
            if c.family != d.family || c.features.0 != d.decoder_inputs[..24] {
                return Err(invalid());
            }
        }
        Self::bind_replay(t, candidates.iter().map(is_hold_intention2000).collect())
    }
    pub fn validate(&self, t: &crate::TrainingReplayTick) -> Result<(), TrainingError> {
        if self.schema_version != 1
            || self.identity != CURRENT_SELF_ID2000
            || self.base_tick_digest != hash(t)?
            || self.hold_intentions.len() != t.candidates.len()
        {
            return Err(invalid());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentSelfHead2000 {
    pub schema_version: u16,
    pub identity: String,
    pub source_neuron: usize,
    pub coefficient: f32,
    pub first_moment: f32,
    pub second_moment: f32,
    pub update_age: u32,
    pub optimizer_step: u32,
    pub config: AdamWConfig,
}
impl CurrentSelfHead2000 {
    /// Zero is deliberate: copying a Forward coefficient produced tonic, harmful Hold in the pilot.
    pub fn fresh_zero(config: AdamWConfig) -> Result<Self, TrainingError> {
        let h = Self {
            schema_version: 1,
            identity: CURRENT_SELF_ID2000.into(),
            source_neuron: 1791,
            coefficient: 0.,
            first_moment: 0.,
            second_moment: 0.,
            update_age: 0,
            optimizer_step: 0,
            config,
        };
        h.validate()?;
        Ok(h)
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.config.validate()?;
        finite(&[self.coefficient, self.first_moment, self.second_moment])?;
        if self.schema_version != 1
            || self.identity != CURRENT_SELF_ID2000
            || self.source_neuron != 1791
            || self.second_moment < 0.
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn apply_gradient(&mut self, g: f32) -> Result<(), TrainingError> {
        self.validate()?;
        finite(&[g])?;
        let mut n = self.clone();
        n.update_age = n.update_age.checked_add(1).ok_or_else(invalid)?;
        n.optimizer_step = n.optimizer_step.checked_add(1).ok_or_else(invalid)?;
        let g = g.clamp(-n.config.gradient_clip, n.config.gradient_clip);
        (n.coefficient, n.first_moment, n.second_moment) = super::adam_coordinate(
            n.coefficient,
            n.first_moment,
            n.second_moment,
            g,
            n.update_age,
            n.config,
        )?;
        n.validate()?;
        *self = n;
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentSelfScore2000 {
    pub decision: usize,
    pub candidate: usize,
    pub source_activity: f32,
    pub hold_intention: bool,
    pub coefficient: f32,
    pub before_score: f32,
    pub current_self_score: f32,
    pub final_score: f32,
}
fn score(
    h: &CurrentSelfHead2000,
    f: &CurrentSelfFrame2000,
    d: usize,
    c: usize,
    a: &[f32],
    base: f32,
) -> Result<CurrentSelfScore2000, TrainingError> {
    let source = a[1791];
    let intent = f.hold_intentions[c];
    let value = source * (if intent { 1. } else { 0. }) * h.coefficient;
    let final_score = if value == 0. { base } else { base + value };
    finite(&[source, value, final_score])?;
    Ok(CurrentSelfScore2000 {
        decision: d,
        candidate: c,
        source_activity: source,
        hold_intention: intent,
        coefficient: h.coefficient,
        before_score: base,
        current_self_score: value,
        final_score,
    })
}

impl CurrentSelfHead2000 {
    /// Detached CPU score only; no collector, replay, ATP debit or body action is performed.
    pub fn score_detached(
        &self,
        frame: &CurrentSelfFrame2000,
        tick: &crate::TrainingReplayTick,
        candidate: usize,
        activity: &[f32],
        base: f32,
    ) -> Result<CurrentSelfScore2000, TrainingError> {
        self.validate()?;
        frame.validate(tick)?;
        if activity.len() != 2048 || candidate >= tick.candidates.len() {
            return Err(invalid());
        }
        finite(activity)?;
        finite(&[base])?;
        score(self, frame, 0, candidate, activity, base)
    }
    /// Gradient of the underlying addition, even when the exact-zero merge preserves base bits.
    pub fn local_gradients(
        &self,
        frame: &CurrentSelfFrame2000,
        tick: &crate::TrainingReplayTick,
        candidate: usize,
        activity: &[f32],
        upstream: f32,
    ) -> Result<(f32, f32), TrainingError> {
        finite(&[upstream])?;
        let s = self.score_detached(frame, tick, candidate, activity, 0.)?;
        let g = if s.hold_intention {
            (upstream * s.source_activity, upstream * self.coefficient)
        } else {
            (0., 0.)
        };
        finite(&[g.0, g.1])?;
        Ok(g)
    }
}
