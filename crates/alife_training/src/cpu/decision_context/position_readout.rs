//! Separate four-state positional context readout. Never remaps legacy R sources.
use super::*;
use serde::{Deserialize, Serialize};
pub const ID2018: &str = "positional-context-isotropic-two-gain-2018-v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionHead2018 {
    pub schema: u16,
    pub identity: String,
    pub gains: [f32; 2],
    pub first_moment: [f32; 2],
    pub second_moment: [f32; 2],
    pub update_ages: [u32; 2],
    pub optimizer_step: u32,
    pub config: AdamWConfig,
}
impl PositionHead2018 {
    pub fn fresh(gains: [f32; 2], config: AdamWConfig) -> Result<Self, TrainingError> {
        let h = Self {
            schema: 1,
            identity: ID2018.into(),
            gains,
            first_moment: [0.; 2],
            second_moment: [0.; 2],
            update_ages: [0; 2],
            optimizer_step: 0,
            config,
        };
        h.validate()?;
        Ok(h)
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.config.validate()?;
        finite(&self.gains)?;
        finite(&self.first_moment)?;
        finite(&self.second_moment)?;
        if self.schema != 1 || self.identity != ID2018 || self.second_moment.iter().any(|x| *x < 0.)
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn apply_gradient(&mut self, g: [f32; 2]) -> Result<(), TrainingError> {
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
            (n.gains[i], n.first_moment[i], n.second_moment[i]) = super::adam_coordinate(
                n.gains[i],
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionEffect2018 {
    pub translation: [f32; 2],
    pub angular: [f32; 2],
}
impl PositionEffect2018 {
    pub fn validate(&self) -> Result<(), TrainingError> {
        finite(&self.translation)?;
        finite(&self.angular)?;
        let norm = |v: [f32; 2]| f64::from(v[0]).powi(2) + f64::from(v[1]).powi(2);
        if norm(self.translation) > 1.000002
            || norm(self.angular) > 1.000002
            || self.translation.iter().any(|x| *x != 0.) && self.angular.iter().any(|x| *x != 0.)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionScore2018 {
    pub state: [f32; 4],
    pub effect: PositionEffect2018,
    pub gains: [f32; 2],
    pub before_score: f32,
    pub raw: f32,
    pub component: f32,
    pub final_score: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionAdjoint2018 {
    pub gains: [f32; 2],
    pub state: [f32; 4],
    pub translation: [f32; 2],
    pub angular: [f32; 2],
    pub before_score: f32,
}
fn prepare(state: [f32; 4]) -> Result<([f32; 2], f32, [f32; 2], f32), TrainingError> {
    let s = super::PositionState2013 {
        x: state[0],
        z: state[1],
        occupancy: state[2],
        confidence: state[3],
    };
    s.validate()?;
    let v = [4. * state[0], 4. * state[1]];
    let denom = (1. + (v[0] * v[0] + v[1] * v[1])).sqrt();
    let u = [v[0] / denom, v[1] / denom];
    let oc = state[2] * state[3];
    finite(&[v[0], v[1], denom, u[0], u[1], oc])?;
    Ok((v, denom, u, oc))
}
pub fn score2018(
    h: &PositionHead2018,
    state: [f32; 4],
    effect: PositionEffect2018,
    base: f32,
) -> Result<PositionScore2018, TrainingError> {
    h.validate()?;
    let (_, _, u, oc) = prepare(state)?;
    score_prepared2018(h, state, effect, base, u, oc)
}
fn score_prepared2018(
    h: &PositionHead2018,
    state: [f32; 4],
    effect: PositionEffect2018,
    base: f32,
    u: [f32; 2],
    oc: f32,
) -> Result<PositionScore2018, TrainingError> {
    effect.validate()?;
    finite(&[base])?;
    let t = u[0] * effect.translation[0] + u[1] * effect.translation[1];
    let a = u[0] * effect.angular[0] + u[1] * effect.angular[1];
    let raw = oc * (h.gains[0] * t + h.gains[1] * a);
    finite(&[t, a, raw])?;
    let component = raw.tanh();
    let final_score = if component == 0. {
        base
    } else {
        base + component
    };
    finite(&[component, final_score])?;
    Ok(PositionScore2018 {
        state,
        effect,
        gains: h.gains,
        before_score: base,
        raw,
        component,
        final_score,
    })
}
pub fn backward2018(
    s: &PositionScore2018,
    adjoint: f32,
) -> Result<PositionAdjoint2018, TrainingError> {
    let rebuilt = score2018(
        &PositionHead2018::fresh(s.gains, AdamWConfig::default())?,
        s.state,
        s.effect,
        s.before_score,
    )?;
    if serde_json::to_vec(&rebuilt).map_err(|_| invalid())?
        != serde_json::to_vec(s).map_err(|_| invalid())?
    {
        return Err(invalid());
    }
    finite(&[adjoint])?;
    let (v, d, u, oc) = prepare(s.state)?;
    s.effect.validate()?;
    finite(&s.gains)?;
    let b = adjoint * (1. - s.component * s.component);
    if b == 0. {
        return Ok(PositionAdjoint2018 {
            gains: [0.; 2],
            state: [0.; 4],
            translation: [0.; 2],
            angular: [0.; 2],
            before_score: adjoint,
        });
    }
    let e = [
        s.gains[0] * s.effect.translation[0] + s.gains[1] * s.effect.angular[0],
        s.gains[0] * s.effect.translation[1] + s.gains[1] * s.effect.angular[1],
    ];
    let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
    let ue = dot(u, e);
    finite(&[ue])?;
    if oc == 0. {
        let out = PositionAdjoint2018 {
            gains: [0.; 2],
            state: [0., 0., b * s.state[3] * ue, b * s.state[2] * ue],
            translation: [0.; 2],
            angular: [0.; 2],
            before_score: adjoint,
        };
        finite(&out.state)?;
        return Ok(out);
    }
    let ve = dot(v, e);
    let d3 = d * d * d;
    let ordinary = [e[0] / d - v[0] * ve / d3, e[1] / d - v[1] * ve / d3];
    let projected = if ordinary.iter().all(|x| x.is_finite()) {
        ordinary
    } else {
        [(e[0] - u[0] * ue) / d, (e[1] - u[1] * ue) / d]
    };
    let out = PositionAdjoint2018 {
        gains: [
            b * oc * dot(u, s.effect.translation),
            b * oc * dot(u, s.effect.angular),
        ],
        state: [
            b * oc * 4. * projected[0],
            b * oc * 4. * projected[1],
            b * s.state[3] * ue,
            b * s.state[2] * ue,
        ],
        translation: [b * oc * s.gains[0] * u[0], b * oc * s.gains[0] * u[1]],
        angular: [b * oc * s.gains[1] * u[0], b * oc * s.gains[1] * u[1]],
        before_score: adjoint,
    };
    finite(&out.gains)?;
    finite(&out.state)?;
    finite(&out.translation)?;
    finite(&out.angular)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_gain_exact_bits_but_trainable() {
        let h = PositionHead2018::fresh([0.; 2], AdamWConfig::default()).unwrap();
        let s = score2018(
            &h,
            [0.2, -0.1, 1., 0.8],
            PositionEffect2018 {
                translation: [1., 0.],
                angular: [0.; 2],
            },
            -0.,
        )
        .unwrap();
        assert_eq!(s.final_score.to_bits(), (-0f32).to_bits());
        let g = backward2018(&s, 1.).unwrap();
        assert!(g.gains[0] > 0.);
        assert_eq!(g.state, [0.; 4]);
    }
    #[test]
    fn all_source_and_parameter_finite_differences() {
        let h = PositionHead2018::fresh([1., -0.7], AdamWConfig::default()).unwrap();
        let state = [0.2, -0.1, 0.9, 0.8];
        for effect in [
            PositionEffect2018 {
                translation: [0.6, 0.8],
                angular: [0.; 2],
            },
            PositionEffect2018 {
                translation: [0.; 2],
                angular: [-0.3, 0.4],
            },
        ] {
            let s = score2018(&h, state, effect, 3.).unwrap();
            let g = backward2018(&s, 0.7).unwrap();
            let loss =
                |h: &PositionHead2018, x| score2018(h, x, effect, 0.).unwrap().component * 0.7;
            for i in 0..4 {
                let mut a = state;
                let mut b = state;
                a[i] += 0.001;
                b[i] -= 0.001;
                assert!(((loss(&h, a) - loss(&h, b)) / 0.002 - g.state[i]).abs() < 1e-4);
            }
            for i in 0..2 {
                let mut a = h.clone();
                let mut b = h.clone();
                a.gains[i] += 0.001;
                b.gains[i] -= 0.001;
                assert!(((loss(&a, state) - loss(&b, state)) / 0.002 - g.gains[i]).abs() < 1e-4);
            }
        }
    }
    #[test]
    fn independent_adam_signed_and_atomic() {
        let mut h = PositionHead2018::fresh([-1., 1.], AdamWConfig::default()).unwrap();
        h.apply_gradient([0.3, -0.7]).unwrap();
        assert_eq!(h.update_ages, [1, 1]);
        let b = serde_json::to_vec(&h).unwrap();
        let restored: PositionHead2018 = serde_json::from_slice(&b).unwrap();
        restored.validate().unwrap();
        assert_eq!(h, restored);
        let before = h.clone();
        assert!(h.apply_gradient([f32::NAN, 0.]).is_err());
        assert_eq!(h, before);
    }
    #[test]
    fn invalid_effect_and_state_rejected() {
        let h = PositionHead2018::fresh([f32::MAX; 2], AdamWConfig::default()).unwrap();
        let effect = PositionEffect2018 {
            translation: [1., 0.],
            angular: [0., 1.],
        };
        assert!(score2018(&h, [0.; 4], effect, 0.).is_err());
        let h = PositionHead2018::fresh([1.; 2], AdamWConfig::default()).unwrap();
        assert!(score2018(
            &h,
            [2., 0., 1., 1.],
            PositionEffect2018 {
                translation: [0.; 2],
                angular: [0.; 2]
            },
            0.
        )
        .is_err());
    }
}
fn digest2018<T: Serialize>(x: &T) -> Result<[u64; 4], TrainingError> {
    let mut d = alife_core::CanonicalDigestBuilder::new(b"ALIFE-POSITION-READOUT2018");
    d.write_bytes(&serde_json::to_vec(x).map_err(|_| invalid())?);
    Ok(d.finish256())
}
/// Current self-effect/provenance only. State is read from the native P tape, not supplied here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionReadoutFrame2018 {
    pub schema: u16,
    pub identity: String,
    pub owner: u64,
    pub current_sequence: u64,
    pub current_tick: u64,
    pub processed_position_reference_tick: u64,
    pub source_generation: Option<u64>,
    pub tick_digest: [u64; 4],
    pub effects: Vec<PositionEffect2018>,
}
impl PositionReadoutFrame2018 {
    pub fn bind_replay(
        t: &crate::TrainingReplayTick,
        owner: u64,
        sequence: u64,
        tick: u64,
        reference_tick: u64,
        generation: Option<u64>,
        effects: Vec<PositionEffect2018>,
    ) -> Result<Self, TrainingError> {
        let f = Self {
            schema: 1,
            identity: ID2018.into(),
            owner,
            current_sequence: sequence,
            current_tick: tick,
            processed_position_reference_tick: reference_tick,
            source_generation: generation,
            tick_digest: digest2018(t)?,
            effects,
        };
        f.validate(t)?;
        Ok(f)
    }
    pub fn validate(&self, t: &crate::TrainingReplayTick) -> Result<(), TrainingError> {
        if self.schema != 1
            || self.identity != ID2018
            || self.owner == 0
            || self.processed_position_reference_tick > self.current_tick
            || self.tick_digest != digest2018(t)?
            || self.effects.len() != t.candidates.len()
        {
            return Err(invalid());
        }
        for e in &self.effects {
            e.validate()?
        }
        Ok(())
    }
    /// Kinds and nominal effects must come from the same actual candidate collector.
    pub fn bind_candidates(
        t: &crate::TrainingReplayTick,
        owner: u64,
        reference_tick: u64,
        generation: Option<u64>,
        retained: &super::RetainedFrame1992,
        candidates: &[alife_core::ActionCandidate],
    ) -> Result<Self, TrainingError> {
        retained.validate(t)?;
        if candidates.len() != t.candidates.len() {
            return Err(invalid());
        }
        let mut effects = Vec::new();
        for ((c, raw), e) in candidates
            .iter()
            .zip(&t.candidates)
            .zip(&retained.candidate_own_effects)
        {
            if c.family != raw.family || c.features.0 != raw.decoder_inputs[..24] {
                return Err(invalid());
            }
            let translation = if c.kind == alife_core::ActionKind::Move {
                *e
            } else {
                [0.; 2]
            };
            let angular = if c.kind == alife_core::ActionKind::Look {
                [e[0] * 0.5, e[1] * 0.5]
            } else {
                [0.; 2]
            };
            effects.push(PositionEffect2018 {
                translation,
                angular,
            });
        }
        Self::bind_replay(
            t,
            owner,
            retained.current_sequence,
            retained.current_tick,
            reference_tick,
            generation,
            effects,
        )
    }
}

pub fn score_batch2018(
    h: &PositionHead2018,
    state: [f32; 4],
    effects: &[PositionEffect2018],
    bases: &[f32],
) -> Result<Vec<PositionScore2018>, TrainingError> {
    h.validate()?;
    if effects.len() != bases.len() {
        return Err(invalid());
    }
    let (_, _, u, oc) = prepare(state)?;
    effects
        .iter()
        .zip(bases)
        .map(|(e, b)| score_prepared2018(h, state, *e, *b, u, oc))
        .collect()
}
