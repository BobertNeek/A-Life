// Test-only frozen2013-v2 reference arithmetic, not runtime code.
// Versioned numerical CPU prototype only. No World or legacy runtime admission.

use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct State {
    pub(super) h: [f32; 2],
    pub(super) occupied: f32,
    pub(super) confidence: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            h: [0.; 2],
            occupied: 0.,
            confidence: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) struct Input {
    pub(super) pulse: f32,
    pub(super) capture: f32,
    pub(super) range: f32,
    pub(super) left: f32,
    pub(super) forward: f32,
    pub(super) sin: f32,
    pub(super) cos: f32,
    pub(super) dx: f32,
    pub(super) dz: f32,
    pub(super) confidence: f32,
    pub(super) fade: f32,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            pulse: 0.,
            capture: 0.,
            range: 0.,
            left: 0.,
            forward: 0.,
            sin: 0.,
            cos: 1.,
            dx: 0.,
            dz: 0.,
            confidence: 1.,
            fade: 1.,
        }
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) struct Model {
    pub(super) weights: [f32; 6],
}
impl Default for Model {
    fn default() -> Self {
        Self {
            weights: [0.25, -0.25, 0.25, 0.25, 1. / 32., 1. / 32.],
        }
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) struct Tape {
    pub(super) input: Input,
    pub(super) occupied: f32,
    pub(super) previous_confidence: f32,
    pub(super) derivative: [f32; 2],
}
#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Work {
    pub(super) transitions: u64,
    pub(super) bilinear_terms: u64,
    pub(super) bounded_activations: u64,
}
pub(super) fn step(m: &Model, s: &mut State, i: Input, w: &mut Work) -> Result<Tape, &'static str> {
    let values = [
        i.pulse,
        i.capture,
        i.range,
        i.left,
        i.forward,
        i.sin,
        i.cos,
        i.dx,
        i.dz,
        i.confidence,
        i.fade,
    ];
    if values
        .iter()
        .chain(m.weights.iter())
        .any(|v| !v.is_finite())
        || ![0., 1.].contains(&i.pulse)
        || ![0., 1.].contains(&i.capture)
        || !(0. ..=1.).contains(&i.range)
        || !(0. ..=1.).contains(&i.confidence)
        || !(0. ..=1.).contains(&i.fade)
    {
        return Err("invalid typed numerical input");
    }
    if s.h.iter().any(|x| !x.is_finite() || x.abs() > 1.)
        || [s.occupied, s.confidence]
            .iter()
            .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
        || [i.left, i.forward, i.sin, i.cos]
            .iter()
            .any(|x| x.abs() > 1.)
    {
        return Err("invalid neural state or raw directional input");
    }
    let transitions = w.transitions.checked_add(1).ok_or("work overflow")?;
    if i.pulse == 0. {
        w.transitions = transitions;
        return Ok(Tape {
            input: i,
            occupied: s.occupied,
            previous_confidence: s.confidence,
            derivative: [1., 1.],
        });
    }
    let terms = w.bilinear_terms.checked_add(6).ok_or("work overflow")?;
    let bounds = w
        .bounded_activations
        .checked_add(2)
        .ok_or("work overflow")?;
    let previous_confidence = s.confidence;
    let pc = i.pulse * i.capture;
    let o = s.occupied + pc * (1. - s.occupied);
    let q = [
        m.weights[0] * i.range * i.forward * i.cos + m.weights[1] * i.range * i.left * i.sin,
        m.weights[2] * i.range * i.forward * i.sin + m.weights[3] * i.range * i.left * i.cos,
    ];
    let pre = [
        (1. - pc) * s.h[0] + pc * q[0] - i.pulse * o * m.weights[4] * i.dx,
        (1. - pc) * s.h[1] + pc * q[1] - i.pulse * o * m.weights[5] * i.dz,
    ];
    if q.iter().chain(pre.iter()).any(|x| !x.is_finite()) {
        return Err("nonfinite neural intermediate");
    }
    let derivative = pre.map(|v| if v.abs() < 1. { 1. } else { 0. });
    s.h = pre.map(|v| v.clamp(-1., 1.));
    s.occupied = o;
    s.confidence += i.pulse
        * (i.capture * (i.confidence - s.confidence)
            + (1. - i.capture) * (i.fade - 1.) * s.confidence);
    w.transitions = transitions;
    w.bilinear_terms = terms;
    w.bounded_activations = bounds;
    Ok(Tape {
        input: i,
        occupied: o,
        previous_confidence,
        derivative,
    })
}
pub(super) fn backward(tapes: &[Tape], mut adj: [f32; 2]) -> ([f32; 6], [f32; 2]) {
    let mut g = [0.; 6];
    for t in tapes.iter().rev() {
        let i = t.input;
        let pc = i.pulse * i.capture;
        let a = [adj[0] * t.derivative[0], adj[1] * t.derivative[1]];
        g[0] += a[0] * pc * i.range * i.forward * i.cos;
        g[1] += a[0] * pc * i.range * i.left * i.sin;
        g[2] += a[1] * pc * i.range * i.forward * i.sin;
        g[3] += a[1] * pc * i.range * i.left * i.cos;
        g[4] -= a[0] * i.pulse * t.occupied * i.dx;
        g[5] -= a[1] * i.pulse * t.occupied * i.dz;
        adj = [a[0] * (1. - pc), a[1] * (1. - pc)];
    }
    (g, adj)
}
pub(super) fn input_backward(m: &Model, tapes: &[Tape], mut a: [f32; 2]) -> Vec<[f32; 7]> {
    let mut out = vec![[0.; 7]; tapes.len()];
    for (k, t) in tapes.iter().enumerate().rev() {
        let i = t.input;
        let pc = i.pulse * i.capture;
        let ax = a[0] * t.derivative[0];
        let az = a[1] * t.derivative[1];
        let w = m.weights;
        // range,left,forward,sin,cos,dx,dz
        out[k] = [
            pc * (ax * (w[0] * i.forward * i.cos + w[1] * i.left * i.sin)
                + az * (w[2] * i.forward * i.sin + w[3] * i.left * i.cos)),
            pc * i.range * (ax * w[1] * i.sin + az * w[3] * i.cos),
            pc * i.range * (ax * w[0] * i.cos + az * w[2] * i.sin),
            pc * i.range * (ax * w[1] * i.left + az * w[2] * i.forward),
            pc * i.range * (ax * w[0] * i.forward + az * w[3] * i.left),
            -ax * i.pulse * t.occupied * w[4],
            -az * i.pulse * t.occupied * w[5],
        ];
        a = [ax * (1. - pc), az * (1. - pc)];
    }
    out
}
pub(super) fn full_state_backward(
    m: &Model,
    tapes: &[Tape],
    mut a: [f32; 4],
) -> ([f32; 4], Vec<[f32; 2]>) {
    let mut c = vec![[0.; 2]; tapes.len()];
    for (k, t) in tapes.iter().enumerate().rev() {
        let i = t.input;
        let pc = i.pulse * i.capture;
        let ax = a[0] * t.derivative[0];
        let az = a[1] * t.derivative[1];
        c[k] = [
            a[3] * pc,
            a[3] * i.pulse * (1. - i.capture) * t.previous_confidence,
        ];
        a = [
            ax * (1. - pc),
            az * (1. - pc),
            (a[2] - i.pulse * (ax * m.weights[4] * i.dx + az * m.weights[5] * i.dz)) * (1. - pc),
            a[3] * (1. - pc + i.pulse * (1. - i.capture) * (i.fade - 1.)),
        ];
    }
    (a, c)
}
