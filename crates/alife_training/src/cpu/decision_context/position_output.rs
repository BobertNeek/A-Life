//! Detached opt-in output amplitudes; not a host or checkpoint migration.
use super::*;

use serde::{Deserialize, Serialize};
pub const POSITION_OUTPUT_ID2029: &str = "positional-output-amplitudes-2029-v1";
type R<T> = Result<T, TrainingError>;
fn finite(x: &[f32]) -> R<()> {
    if x.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(invalid())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionEffectKind2029 {
    Translation,
    Angular,
    Other,
}
impl PositionEffectKind2029 {
    pub fn from_action(k: alife_core::ActionKind) -> Self {
        match k {
            alife_core::ActionKind::Move => Self::Translation,
            alife_core::ActionKind::Look => Self::Angular,
            _ => Self::Other,
        }
    }
    fn index(self) -> Option<usize> {
        match self {
            Self::Translation => Some(0),
            Self::Angular => Some(1),
            Self::Other => None,
        }
    }
    fn validate(self, e: PositionEffect2018) -> R<()> {
        e.validate().map_err(|_| invalid())?;
        let zero = |v: [f32; 2]| v.iter().all(|x| *x == 0.);
        if match self {
            Self::Translation => zero(e.angular),
            Self::Angular => zero(e.translation),
            Self::Other => zero(e.translation) && zero(e.angular),
        } {
            Ok(())
        } else {
            Err(invalid())
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionOutputAdam2029 {
    pub amplitudes: [f32; 2],
    pub first_moment: [f32; 2],
    pub second_moment: [f32; 2],
    pub update_ages: [u32; 2],
    pub optimizer_step: u32,
    pub config: AdamWConfig,
}
impl PositionOutputAdam2029 {
    pub fn fresh(amplitudes: [f32; 2], config: AdamWConfig) -> R<Self> {
        let h = Self {
            amplitudes,
            first_moment: [0.; 2],
            second_moment: [0.; 2],
            update_ages: [0; 2],
            optimizer_step: 0,
            config,
        };
        h.validate()?;
        Ok(h)
    }
    pub fn validate(&self) -> R<()> {
        self.config.validate()?;
        finite(&self.amplitudes)?;
        finite(&self.first_moment)?;
        finite(&self.second_moment)?;
        if self.second_moment.iter().any(|v| *v < 0.) {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn update(&mut self, g: [f32; 2]) -> R<()> {
        self.validate()?;
        finite(&g)?;
        let mut n = self.clone();
        let norm = (g[0] * g[0] + g[1] * g[1]).sqrt();
        finite(&[norm])?;
        let scale = if norm > n.config.gradient_clip {
            n.config.gradient_clip / norm
        } else {
            1.
        };
        for (i, gradient) in g.iter().enumerate() {
            n.update_ages[i] = n.update_ages[i].checked_add(1).ok_or_else(invalid)?;
            (n.amplitudes[i], n.first_moment[i], n.second_moment[i]) = adam_coordinate(
                n.amplitudes[i],
                n.first_moment[i],
                n.second_moment[i],
                gradient * scale,
                n.update_ages[i],
                n.config,
            )?;
        }
        n.optimizer_step = n.optimizer_step.checked_add(1).ok_or_else(invalid)?;
        n.validate()?;
        *self = n;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionOutputHead2029 {
    pub schema: u16,
    pub identity: String,
    pub inner: PositionHead2018,
    pub output: PositionOutputAdam2029,
}
impl PositionOutputHead2029 {
    pub fn fresh(inner: PositionHead2018, amplitudes: [f32; 2], config: AdamWConfig) -> R<Self> {
        let h = Self {
            schema: 1,
            identity: POSITION_OUTPUT_ID2029.into(),
            inner,
            output: PositionOutputAdam2029::fresh(amplitudes, config)?,
        };
        h.validate()?;
        Ok(h)
    }
    pub fn validate(&self) -> R<()> {
        if self.schema != 1 || self.identity != POSITION_OUTPUT_ID2029 {
            return Err(invalid());
        }
        self.inner.validate().map_err(|_| invalid())?;
        self.output.validate()
    }
    pub fn update(&mut self, inner: [f32; 2], output: [f32; 2]) -> R<()> {
        self.validate()?;
        let mut n = self.clone();
        n.inner.apply_gradient(inner).map_err(|_| invalid())?;
        n.output.update(output)?;
        n.validate()?;
        *self = n;
        Ok(())
    }
    pub fn cold(&self) -> R<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| invalid())
    }
    pub fn restore(bytes: &[u8]) -> R<Self> {
        let h: Self = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        h.validate()?;
        Ok(h)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionOutputScore2029 {
    pub kind: PositionEffectKind2029,
    pub hidden: PositionScore2018,
    pub amplitudes: [f32; 2],
    pub base: f32,
    pub component: f32,
    pub final_score: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionOutputAdjoint2029 {
    pub inner: PositionAdjoint2018,
    pub amplitudes: [f32; 2],
    pub base: f32,
}
fn merge_output2029(
    h: &PositionOutputHead2029,
    kind: PositionEffectKind2029,
    hidden: PositionScore2018,
    base: f32,
) -> R<PositionOutputScore2029> {
    finite(&[base])?;
    kind.validate(hidden.effect)?;
    let component = kind.index().map_or(hidden.component, |i| {
        h.output.amplitudes[i] * hidden.component
    });
    let final_score = if component == 0. {
        base
    } else {
        base + component
    };
    finite(&[component, final_score])?;
    Ok(PositionOutputScore2029 {
        kind,
        hidden,
        amplitudes: h.output.amplitudes,
        base,
        component,
        final_score,
    })
}
pub fn score_output2029(
    h: &PositionOutputHead2029,
    state: [f32; 4],
    kind: PositionEffectKind2029,
    effect: PositionEffect2018,
    base: f32,
) -> R<PositionOutputScore2029> {
    h.validate()?;
    kind.validate(effect)?;
    let hidden = score2018(&h.inner, state, effect, 0.)?;
    merge_output2029(h, kind, hidden, base)
}
/// Shared source normalization once per batch; logical work uses derive(n) plus for_kinds.
pub fn score_output_batch2029(
    h: &PositionOutputHead2029,
    state: [f32; 4],
    kinds: &[PositionEffectKind2029],
    effects: &[PositionEffect2018],
    bases: &[f32],
) -> R<Vec<PositionOutputScore2029>> {
    h.validate()?;
    if kinds.len() != effects.len() || bases.len() != effects.len() {
        return Err(invalid());
    }
    let hidden = score_batch2018(&h.inner, state, effects, &vec![0.; effects.len()])?;
    hidden
        .into_iter()
        .zip(kinds)
        .zip(bases)
        .map(|((s, k), b)| merge_output2029(h, *k, s, *b))
        .collect()
}

fn backward_output_score2029(
    s: &PositionOutputScore2029,
    adj: f32,
) -> R<PositionOutputAdjoint2029> {
    finite(&[adj, s.base, s.component, s.final_score])?;
    finite(&s.amplitudes)?;
    s.kind.validate(s.hidden.effect)?;
    let h = PositionOutputHead2029::fresh(
        PositionHead2018::fresh(s.hidden.gains, AdamWConfig::default()).map_err(|_| invalid())?,
        s.amplitudes,
        AdamWConfig::default(),
    )?;
    let rebuilt = score_output2029(&h, s.hidden.state, s.kind, s.hidden.effect, s.base)?;
    if serde_json::to_vec(&rebuilt).map_err(|_| invalid())?
        != serde_json::to_vec(s).map_err(|_| invalid())?
    {
        return Err(invalid());
    }
    let mut amplitudes = [0.; 2];
    let chain = if let Some(i) = s.kind.index() {
        amplitudes[i] = adj * s.hidden.component;
        if adj == 0. || s.amplitudes[i] == 0. || s.hidden.component.abs() == 1. {
            0.
        } else {
            adj * s.amplitudes[i]
        }
    } else {
        0.
    };
    finite(&amplitudes)?;
    let mut inner = if chain.is_finite() {
        backward2018(&s.hidden, chain)?
    } else {
        wide_inner_adjoint2029(
            &s.hidden,
            f64::from(adj) * f64::from(s.amplitudes[s.kind.index().ok_or_else(invalid)?]),
        )?
    };
    inner.before_score = 0.;
    Ok(PositionOutputAdjoint2029 {
        inner,
        amplitudes,
        base: adj,
    })
}
/// Versioned detached frame wraps the unchanged provenance and adds explicit kinds.
/// It does not execute a collector, host, or transform current perception.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionOutputFrame2029 {
    pub schema: u16,
    pub identity: String,
    pub inherited: super::PositionReadoutFrame2018,
    pub kinds: Vec<PositionEffectKind2029>,
}
impl PositionOutputFrame2029 {
    pub fn bind(
        t: &crate::TrainingReplayTick,
        inherited: super::PositionReadoutFrame2018,
        candidates: &[alife_core::ActionCandidate],
    ) -> R<Self> {
        if candidates.len() != t.candidates.len() {
            return Err(invalid());
        }
        for (c, r) in candidates.iter().zip(&t.candidates) {
            if c.family != r.family || c.features.0 != r.decoder_inputs[..24] {
                return Err(invalid());
            }
        }
        let f = Self {
            schema: 1,
            identity: POSITION_OUTPUT_ID2029.into(),
            inherited,
            kinds: candidates
                .iter()
                .map(|c| PositionEffectKind2029::from_action(c.kind))
                .collect(),
        };
        f.validate_candidates(t, candidates)?;
        Ok(f)
    }
    pub fn validate_candidates(
        &self,
        t: &crate::TrainingReplayTick,
        candidates: &[alife_core::ActionCandidate],
    ) -> R<()> {
        self.validate(t)?;
        if candidates.len() != self.kinds.len() {
            return Err(invalid());
        }
        for ((c, r), k) in candidates.iter().zip(&t.candidates).zip(&self.kinds) {
            if *k != PositionEffectKind2029::from_action(c.kind)
                || c.family != r.family
                || c.features.0 != r.decoder_inputs[..24]
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    pub fn validate(&self, t: &crate::TrainingReplayTick) -> R<()> {
        if self.schema != 1
            || self.identity != POSITION_OUTPUT_ID2029
            || self.kinds.len() != self.inherited.effects.len()
        {
            return Err(invalid());
        }
        self.inherited.validate(t).map_err(|_| invalid())?;
        for (k, e) in self.kinds.iter().zip(&self.inherited.effects) {
            k.validate(*e)?
        }
        Ok(())
    }
}

/// Immutable forward tape. A different current inner/output weight vector is rejected.
pub struct PositionOutputTape2029 {
    score: PositionOutputScore2029,
}
impl PositionOutputTape2029 {
    pub fn score(&self) -> &PositionOutputScore2029 {
        &self.score
    }
}
impl PositionOutputHead2029 {
    pub fn forward_detached(
        &self,
        state: [f32; 4],
        kind: PositionEffectKind2029,
        effect: PositionEffect2018,
        base: f32,
    ) -> R<PositionOutputTape2029> {
        Ok(PositionOutputTape2029 {
            score: score_output2029(self, state, kind, effect, base)?,
        })
    }
    pub fn backward_detached(
        &self,
        tape: &PositionOutputTape2029,
        adjoint: f32,
    ) -> R<PositionOutputAdjoint2029> {
        self.validate()?;
        if self.inner.gains.map(f32::to_bits) != tape.score.hidden.gains.map(f32::to_bits)
            || self.output.amplitudes.map(f32::to_bits) != tape.score.amplitudes.map(f32::to_bits)
        {
            return Err(invalid());
        }
        backward_output_score2029(&tape.score, adjoint)
    }
}

#[cfg(test)]
#[path = "position_output_tests.rs"]
mod tests;

// Only used when the f32 outer chain overflows. Do not round tiny local
// adjoints to f32 before amplification: they can become finite live gradients.
fn wide_inner_adjoint2029(s: &PositionScore2018, chain: f64) -> R<PositionAdjoint2018> {
    let v = [4. * f64::from(s.state[0]), 4. * f64::from(s.state[1])];
    let d = (1. + v[0] * v[0] + v[1] * v[1]).sqrt();
    let u = [v[0] / d, v[1] / d];
    let oc = f64::from(s.state[2]) * f64::from(s.state[3]);
    let b = chain * (1. - f64::from(s.component).powi(2));
    let t = s.effect.translation.map(f64::from);
    let a = s.effect.angular.map(f64::from);
    let gains = s.gains.map(f64::from);
    let e = [
        gains[0] * t[0] + gains[1] * a[0],
        gains[0] * t[1] + gains[1] * a[1],
    ];
    let dot = |x: [f64; 2], y: [f64; 2]| x[0] * y[0] + x[1] * y[1];
    let ue = dot(u, e);
    let out = PositionAdjoint2018 {
        gains: [(b * oc * dot(u, t)) as f32, (b * oc * dot(u, a)) as f32],
        state: [
            (b * oc * 4. * (e[0] - u[0] * ue) / d) as f32,
            (b * oc * 4. * (e[1] - u[1] * ue) / d) as f32,
            (b * f64::from(s.state[3]) * ue) as f32,
            (b * f64::from(s.state[2]) * ue) as f32,
        ],
        translation: [
            (b * oc * gains[0] * u[0]) as f32,
            (b * oc * gains[0] * u[1]) as f32,
        ],
        angular: [
            (b * oc * gains[1] * u[0]) as f32,
            (b * oc * gains[1] * u[1]) as f32,
        ],
        before_score: 0.,
    };
    finite(&out.gains)?;
    finite(&out.state)?;
    finite(&out.translation)?;
    finite(&out.angular)?;
    Ok(out)
}
