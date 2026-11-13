//! Explicit retained-working-memory decision head. No sensory/legacy-lane overwrite.
//! Detached native CPU replay substrate; body collector admission is separate.
use super::*;
use alife_core::CanonicalDigestBuilder;
use serde::{Deserialize, Serialize};
pub const RETAINED_PARAMETERS1992: usize = 48;
pub const RETAINED_SOURCES1992: [usize; 8] = [2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026];
pub const RETAINED_ID1992: &str = "retained-working-memory-decision-context-1992-v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedSource1992 {
    pub source_sequence: u64,
    pub source_tick: u64,
    pub delivered_sequence: u64,
    pub delivered_tick: u64,
    pub original_frame_digest: [u64; 4],
    pub original_tuple_digest: [u64; 4],
    pub confidence: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedFrame1992 {
    pub schema_version: u16,
    pub identity: String,
    pub current_sequence: u64,
    pub current_tick: u64,
    pub base_tick_digest: [u64; 4],
    pub source: Option<RetainedSource1992>,
    pub candidate_own_effects: Vec<[f32; 2]>,
}
fn digest<T: Serialize>(domain: &[u8], v: &T) -> Result<[u64; 4], TrainingError> {
    let mut d = CanonicalDigestBuilder::new(domain);
    d.write_bytes(&serde_json::to_vec(v).map_err(|_| invalid())?);
    Ok(d.finish256())
}
impl RetainedFrame1992 {
    /// Detached replay binding only, not authority to fabricate a World source.
    pub fn bind_replay(
        t: &crate::TrainingReplayTick,
        current_sequence: u64,
        current_tick: u64,
        source: Option<RetainedSource1992>,
        candidate_own_effects: Vec<[f32; 2]>,
    ) -> Result<Self, TrainingError> {
        let f = Self {
            schema_version: 1,
            identity: RETAINED_ID1992.into(),
            current_sequence,
            current_tick,
            base_tick_digest: digest(b"ALIFE-RETAINED1992-BASE-TICK", t)?,
            source,
            candidate_own_effects,
        };
        f.validate(t)?;
        Ok(f)
    }
    pub fn validate(&self, t: &crate::TrainingReplayTick) -> Result<(), TrainingError> {
        if self.schema_version != 1
            || self.identity != RETAINED_ID1992
            || self.base_tick_digest != digest(b"ALIFE-RETAINED1992-BASE-TICK", t)?
            || self.candidate_own_effects.len() != t.candidates.len()
            || self
                .candidate_own_effects
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || x.abs() > 2.0)
        {
            return Err(invalid());
        }
        if let Some(s) = &self.source {
            if s.source_sequence >= s.delivered_sequence
                || s.delivered_sequence > self.current_sequence
                || s.source_tick > s.delivered_tick
                || s.delivered_tick > self.current_tick
                || !s.confidence.is_finite()
                || !(0.0..=1.0).contains(&s.confidence)
                || s.original_frame_digest == [0; 4]
                || s.original_tuple_digest == [0; 4]
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn inputs(&self, c: usize) -> Option<[f32; 6]> {
        let s = self.source.as_ref()?;
        let [x, z] = self.candidate_own_effects[c];
        let fresh = 1.0 / (1.0 + (self.current_tick - s.source_tick) as f32);
        Some([
            x,
            z,
            x * s.confidence,
            z * s.confidence,
            x * fresh,
            z * fresh,
        ])
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedHead1992 {
    pub schema_version: u16,
    pub identity: String,
    pub source_neurons: Vec<usize>,
    pub score_bound: f32,
    pub weights: Vec<f32>,
    pub first_moment: Vec<f32>,
    pub second_moment: Vec<f32>,
    pub update_ages: Vec<u32>,
    pub optimizer_step: u32,
    pub config: AdamWConfig,
}
impl RetainedHead1992 {
    pub fn fresh_zero(score_bound: f32, config: AdamWConfig) -> Result<Self, TrainingError> {
        let s = Self {
            schema_version: 1,
            identity: RETAINED_ID1992.into(),
            source_neurons: RETAINED_SOURCES1992.to_vec(),
            score_bound,
            weights: vec![0.; 48],
            first_moment: vec![0.; 48],
            second_moment: vec![0.; 48],
            update_ages: vec![0; 48],
            optimizer_step: 0,
            config,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.config.validate()?;
        if self.schema_version != 1
            || self.identity != RETAINED_ID1992
            || self.source_neurons != RETAINED_SOURCES1992
            || !self.score_bound.is_finite()
            || self.score_bound <= 0.
            || self.weights.len() != 48
            || self.first_moment.len() != 48
            || self.second_moment.len() != 48
            || self.update_ages.len() != 48
            || self.second_moment.iter().any(|v| *v < 0.)
        {
            return Err(invalid());
        }
        finite(&self.weights)?;
        finite(&self.first_moment)?;
        finite(&self.second_moment)?;
        Ok(())
    }
    pub fn apply_gradients(&mut self, g: &[f32]) -> Result<(), TrainingError> {
        self.validate()?;
        if g.len() != 48 {
            return Err(invalid());
        }
        finite(g)?;
        let mut next = self.clone();
        let norm = g.iter().map(|x| x * x).sum::<f32>().sqrt();
        if !norm.is_finite() {
            return Err(invalid());
        }
        let scale = if norm > self.config.gradient_clip {
            self.config.gradient_clip / norm
        } else {
            1.
        };
        next.optimizer_step = next.optimizer_step.checked_add(1).ok_or_else(invalid)?;
        for (i, g) in g.iter().enumerate() {
            let age = next.update_ages[i].checked_add(1).ok_or_else(invalid)?;
            let (w, m, v) = super::adam_coordinate(
                next.weights[i],
                next.first_moment[i],
                next.second_moment[i],
                g * scale,
                age,
                next.config,
            )?;
            next.weights[i] = w;
            next.first_moment[i] = m;
            next.second_moment[i] = v;
            next.update_ages[i] = age;
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetainedScore1992 {
    pub decision: usize,
    pub candidate: usize,
    pub source: Option<RetainedSource1992>,
    pub current_tick: u64,
    pub base_score: f32,
    pub source_activity: [f32; 8],
    pub context_inputs: [f32; 6],
    pub raw_score: f32,
    pub bounded_score: f32,
    pub final_score: f32,
}
fn score(
    head: &RetainedHead1992,
    frame: &RetainedFrame1992,
    decision: usize,
    candidate: usize,
    a: &[f32],
    base: f32,
) -> Result<RetainedScore1992, TrainingError> {
    let activity = RETAINED_SOURCES1992.map(|i| a[i]);
    let inputs = frame.inputs(candidate).unwrap_or([0.; 6]);
    let mut raw = 0.;
    if frame.source.is_some() {
        for (i, x) in activity.iter().enumerate() {
            for (j, y) in inputs.iter().enumerate() {
                raw += x * y * head.weights[i * 6 + j];
            }
        }
    }
    let bounded = head.score_bound * (raw / head.score_bound).tanh();
    let final_score = if bounded == 0. { base } else { base + bounded };
    finite(&[raw, bounded, final_score])?;
    Ok(RetainedScore1992 {
        decision,
        candidate,
        source: frame.source.clone(),
        current_tick: frame.current_tick,
        base_score: base,
        source_activity: activity,
        context_inputs: inputs,
        raw_score: raw,
        bounded_score: bounded,
        final_score,
    })
}

impl RetainedHead1992 {
    /// Detached CPU score only. Caller must bind the actual neural endpoint; this is not body admission.
    pub fn score_detached(
        &self,
        frame: &RetainedFrame1992,
        tick: &crate::TrainingReplayTick,
        candidate: usize,
        activity: &[f32],
        base: f32,
    ) -> Result<RetainedScore1992, TrainingError> {
        self.validate()?;
        frame.validate(tick)?;
        if activity.len() != 2048 || candidate >= tick.candidates.len() {
            return Err(invalid());
        }
        finite(activity)?;
        finite(&[base])?;
        score(self, frame, 0, candidate, activity, base)
    }
    /// Local adjoints only, not recurrent BPTT or a whole-model training step.
    pub fn local_gradients(
        &self,
        frame: &RetainedFrame1992,
        tick: &crate::TrainingReplayTick,
        candidate: usize,
        activity: &[f32],
        upstream: f32,
    ) -> Result<(Vec<f32>, [f32; 8]), TrainingError> {
        finite(&[upstream])?;
        let s = self.score_detached(frame, tick, candidate, activity, 0.)?;
        let mut weights = vec![0.; 48];
        let mut sources = [0.; 8];
        if frame.source.is_some() {
            let y = (s.raw_score / self.score_bound).tanh();
            let g = upstream * (1. - y * y);
            for (i, source) in sources.iter_mut().enumerate() {
                for (j, input) in s.context_inputs.iter().enumerate() {
                    weights[i * 6 + j] = g * s.source_activity[i] * input;
                    *source += g * self.weights[i * 6 + j] * input;
                }
            }
        }
        finite(&weights)?;
        finite(&sources)?;
        Ok((weights, sources))
    }
}
