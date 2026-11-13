//! New detached joint potential. No candidate selection, sensory binding, or old-state migration.
//! Effects: nominal world translation, body-axis delta/2, gaze delta/2.
//! Position is relative-world metres/32; occupancy and confidence are separate evidence.
use super::*;
use serde::{Deserialize, Serialize};
pub const JOINT_HEAD_ID2056: &str = "command-conditional-joint-head-2056-v1";
fn checked2056(x: f64) -> Result<f32, TrainingError> {
    let y = x as f32;
    if !x.is_finite() || !y.is_finite() {
        Err(invalid())
    } else {
        Ok(y)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointHead2056 {
    pub schema: u16,
    pub identity: String,
    pub coefficients: [f32; 3],
    pub amplitude: f32,
    pub first_moment: [f32; 4],
    pub second_moment: [f32; 4],
    pub update_ages: [u32; 4],
    pub optimizer_step: u32,
    pub config: AdamWConfig,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointScore2056 {
    pub state: [f32; 4],
    pub effects: [[f32; 2]; 3],
    pub parameters: [f32; 4],
    /// Exact f64 intermediates, avoiding lossy JSON numeric roundtrips.
    pub raw_bits: u64,
    pub hidden_bits: u64,
    pub value: f32,
}
#[derive(Debug, Clone)]
pub struct JointTape2056 {
    score: JointScore2056,
}
impl JointTape2056 {
    pub fn score(&self) -> &JointScore2056 {
        &self.score
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointAdjoint2056 {
    pub parameters: [f32; 4],
    pub state: [f32; 4],
    pub effects: [[f32; 2]; 3],
}
impl JointHead2056 {
    pub fn fresh(
        coefficients: [f32; 3],
        amplitude: f32,
        config: AdamWConfig,
    ) -> Result<Self, TrainingError> {
        let s = Self {
            schema: 1,
            identity: JOINT_HEAD_ID2056.into(),
            coefficients,
            amplitude,
            first_moment: [0.; 4],
            second_moment: [0.; 4],
            update_ages: [0; 4],
            optimizer_step: 0,
            config,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn parameters(&self) -> [f32; 4] {
        [
            self.coefficients[0],
            self.coefficients[1],
            self.coefficients[2],
            self.amplitude,
        ]
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.schema != 1 || self.identity != JOINT_HEAD_ID2056 {
            return Err(invalid());
        }
        self.config.validate()?;
        finite(&self.parameters())?;
        finite(&self.first_moment)?;
        finite(&self.second_moment)?;
        if self.second_moment.iter().any(|v| *v < 0.) {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn cold(&self) -> Result<Vec<u8>, TrainingError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| invalid())
    }
    pub fn restore(b: &[u8]) -> Result<Self, TrainingError> {
        let s: Self = serde_json::from_slice(b).map_err(|_| invalid())?;
        s.validate()?;
        Ok(s)
    }
    /// Selected coordinates age independently; None preserves that coordinate exactly.
    /// Shared norm clipping covers only selected gradients. All changes commit atomically.
    pub fn update_selected(&mut self, g: [Option<f32>; 4]) -> Result<(), TrainingError> {
        self.validate()?;
        let mut norm = 0f64;
        for v in g.iter().flatten() {
            finite(&[*v])?;
            norm += f64::from(*v).powi(2)
        }
        if g.iter().all(Option::is_none) {
            return Ok(());
        }
        let norm = norm.sqrt();
        let scale = if norm > f64::from(self.config.gradient_clip) {
            f64::from(self.config.gradient_clip) / norm
        } else {
            1.
        };
        let mut n = self.clone();
        let mut p = n.parameters();
        for (i, v) in g.iter().enumerate() {
            if let Some(v) = v {
                n.update_ages[i] = n.update_ages[i].checked_add(1).ok_or_else(invalid)?;
                let gradient = checked2056(f64::from(*v) * scale)?;
                (p[i], n.first_moment[i], n.second_moment[i]) = adam_coordinate(
                    p[i],
                    n.first_moment[i],
                    n.second_moment[i],
                    gradient,
                    n.update_ages[i],
                    n.config,
                )?;
            }
        }
        n.coefficients.copy_from_slice(&p[..3]);
        n.amplitude = p[3];
        n.optimizer_step = n.optimizer_step.checked_add(1).ok_or_else(invalid)?;
        n.validate()?;
        *self = n;
        Ok(())
    }
    pub fn update(&mut self, g: [f32; 4]) -> Result<(), TrainingError> {
        self.update_selected(g.map(Some))
    }
    pub fn forward(
        &self,
        state: [f32; 4],
        effects: [[f32; 2]; 3],
    ) -> Result<JointTape2056, TrainingError> {
        self.validate()?;
        finite(&state)?;
        if state[..2].iter().any(|v| v.abs() > 1.)
            || state[2..].iter().any(|v| !(0. ..=1.).contains(v))
        {
            return Err(invalid());
        }
        for e in effects {
            finite(&e)?;
            if e.iter().any(|v| v.abs() > 1.) {
                return Err(invalid());
            }
        }
        let v = [4. * f64::from(state[0]), 4. * f64::from(state[1])];
        let d = (1. + v[0] * v[0] + v[1] * v[1]).sqrt();
        let u = [v[0] / d, v[1] / d];
        let mut sum = 0.;
        for (k, e) in effects.iter().enumerate() {
            sum +=
                f64::from(self.coefficients[k]) * (u[0] * f64::from(e[0]) + u[1] * f64::from(e[1]));
        }
        let raw = f64::from(state[2]) * f64::from(state[3]) * sum;
        let hidden = raw.tanh();
        let value = checked2056(f64::from(self.amplitude) * hidden)?;
        Ok(JointTape2056 {
            score: JointScore2056 {
                state,
                effects,
                parameters: self.parameters(),
                raw_bits: raw.to_bits(),
                hidden_bits: hidden.to_bits(),
                value,
            },
        })
    }
    pub fn backward(
        &self,
        t: &JointTape2056,
        adjoint: f32,
    ) -> Result<JointAdjoint2056, TrainingError> {
        self.validate()?;
        finite(&[adjoint])?;
        let s = &t.score;
        if self
            .parameters()
            .iter()
            .zip(s.parameters)
            .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err(invalid());
        }
        let v = [4. * f64::from(s.state[0]), 4. * f64::from(s.state[1])];
        let d = (1. + v[0] * v[0] + v[1] * v[1]).sqrt();
        let u = [v[0] / d, v[1] / d];
        let o = f64::from(s.state[2]);
        let c = f64::from(s.state[3]);
        let hidden = f64::from_bits(s.hidden_bits);
        let b = f64::from(adjoint) * f64::from(self.amplitude) * (1. - hidden * hidden);
        let mut e = [0.; 2];
        let mut dots = [0.; 3];
        for (k, eff) in s.effects.iter().enumerate() {
            dots[k] = u[0] * f64::from(eff[0]) + u[1] * f64::from(eff[1]);
            for j in 0..2 {
                e[j] += f64::from(self.coefficients[k]) * f64::from(eff[j]);
            }
        }
        let sum = self
            .coefficients
            .iter()
            .zip(dots)
            .map(|(w, q)| f64::from(*w) * q)
            .sum::<f64>();
        let ue = u[0] * e[0] + u[1] * e[1];
        let mut a = JointAdjoint2056 {
            parameters: [0.; 4],
            state: [0.; 4],
            effects: [[0.; 2]; 3],
        };
        for k in 0..3 {
            a.parameters[k] = checked2056(b * o * c * dots[k])?;
            for j in 0..2 {
                a.effects[k][j] = checked2056(b * o * c * f64::from(self.coefficients[k]) * u[j])?;
            }
        }
        a.parameters[3] = checked2056(f64::from(adjoint) * hidden)?;
        for j in 0..2 {
            a.state[j] = checked2056(b * o * c * 4. * (e[j] - u[j] * ue) / d)?;
        }
        a.state[2] = checked2056(b * c * sum)?;
        a.state[3] = checked2056(b * o * sum)?;
        Ok(a)
    }
}
#[cfg(test)]
mod tests2056 {
    use super::*;
    fn h() -> JointHead2056 {
        JointHead2056::fresh([0.7, -0.2, 0.4], 1.3, AdamWConfig::default()).unwrap()
    }
    const S: [f32; 4] = [0.2, -0.1, 0.8, 0.7];
    const E: [[f32; 2]; 3] = [[0.6, 0.2], [-0.2, 0.4], [0.1, -0.3]];
    #[test]
    fn all_fourteen_adjoints_central_difference() {
        let m = h();
        let t = m.forward(S, E).unwrap();
        let a = m.backward(&t, 1.).unwrap();
        let eps = 0.001;
        let near = |fd: f32, g: f32| assert!((fd - g).abs() < 0.00015, "{fd} vs {g}");
        for i in 0..4 {
            let mut hi = m.clone();
            let mut lo = m.clone();
            if i < 3 {
                hi.coefficients[i] += eps;
                lo.coefficients[i] -= eps
            } else {
                hi.amplitude += eps;
                lo.amplitude -= eps
            }
            near(
                (hi.forward(S, E).unwrap().score.value - lo.forward(S, E).unwrap().score.value)
                    / (2. * eps),
                a.parameters[i],
            );
            let mut hi = S;
            let mut lo = S;
            hi[i] += eps;
            lo[i] -= eps;
            near(
                (m.forward(hi, E).unwrap().score.value - m.forward(lo, E).unwrap().score.value)
                    / (2. * eps),
                a.state[i],
            );
        }
        for i in 0..3 {
            for j in 0..2 {
                let mut hi = E;
                let mut lo = E;
                hi[i][j] += eps;
                lo[i][j] -= eps;
                near(
                    (m.forward(S, hi).unwrap().score.value - m.forward(S, lo).unwrap().score.value)
                        / (2. * eps),
                    a.effects[i][j],
                );
            }
        }
    }
    #[test]
    fn zero_output_is_live_and_model_bound() {
        let mut m = h();
        m.amplitude = 0.;
        let t = m.forward(S, E).unwrap();
        assert_eq!(t.score.value, 0.);
        let a = m.backward(&t, 1.).unwrap();
        assert_ne!(a.parameters[3], 0.);
        assert_eq!(a.parameters[..3], [0.; 3]);
        assert_eq!(a.state, [0.; 4]);
        m.coefficients[0] += 1.;
        assert!(m.backward(&t, 1.).is_err());
    }
    #[test]
    fn cold_unequal_ages_and_transactional_failure() {
        let mut m = h();
        m.update_selected([Some(0.1), Some(-0.2), Some(0.3), None])
            .unwrap();
        m.update_selected([None, None, None, Some(0.2)]).unwrap();
        m.update_selected([None, None, None, Some(-0.1)]).unwrap();
        assert_eq!(m.update_ages, [1, 1, 1, 2]);
        let b = m.cold().unwrap();
        assert_eq!(b, JointHead2056::restore(&b).unwrap().cold().unwrap());
        assert!(m.update([0., 0., 0., f32::NAN]).is_err());
        assert_eq!(b, m.cold().unwrap());
        m.update_ages[3] = u32::MAX;
        let b = m.cold().unwrap();
        assert!(m.update([0.; 4]).is_err());
        assert_eq!(b, m.cold().unwrap());
    }
    #[test]
    fn finite_extremes_zero_tiny_and_true_overflow() {
        let mut m = JointHead2056::fresh([0.; 3], f32::MAX, AdamWConfig::default()).unwrap();
        let t = m.forward([0., 0., 1., 1.], [[0.; 2]; 3]).unwrap();
        let a = m.backward(&t, f32::MAX).unwrap();
        assert_eq!(a.state, [0.; 4]);
        assert_eq!(a.effects, [[0.; 2]; 3]);
        let t = m
            .forward([2.5e-21, 0., 1., 1e-30], [[1., 0.], [0.; 2], [0.; 2]])
            .unwrap();
        let a = m.backward(&t, f32::MAX).unwrap();
        assert!(a.parameters[0] > 1e26);
        m.coefficients[0] = 2.;
        let t = m.forward([2.5e-41, 0., 1., 1.], [[0.; 2]; 3]).unwrap();
        assert!(m.backward(&t, 1.).unwrap().effects[0][0] > 0.);
        let t = m
            .forward([0., 0., 1., 1.], [[1., 0.], [0.; 2], [0.; 2]])
            .unwrap();
        assert!(m.backward(&t, 1.).is_err());
    }
    #[test]
    fn invalid_inputs_and_saturated_gradients() {
        let mut m = h();
        assert!(m.forward([2., 0., 1., 1.], E).is_err());
        assert!(m.forward(S, [[f32::NAN, 0.]; 3]).is_err());
        m.coefficients = [f32::MAX; 3];
        m.amplitude = f32::MAX;
        let t = m.forward([1., 0., 1., 1.], [[1., 0.]; 3]).unwrap();
        let a = m.backward(&t, 1.).unwrap();
        assert_eq!(a.parameters[..3], [0.; 3]);
        assert_eq!(a.parameters[3], 1.);
        assert_eq!(a.state, [0.; 4]);
    }
}
