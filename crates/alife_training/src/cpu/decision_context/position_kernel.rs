// Versioned numerical CPU prototype only. No World or legacy runtime admission.
#[cfg(test)]
const VERSION: u32 = 201301;
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
        if i.pulse == 0. {
            continue;
        }
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
#[cfg(test)]
fn interval(m: &Model, s: &mut State, mut i: Input, n: usize, w: &mut Work) -> Vec<Tape> {
    assert!(n > 0);
    i.pulse = 1.;
    let mut ts = vec![step(m, s, i, w).unwrap()];
    for _ in 1..n {
        ts.push(step(m, s, Input::default(), w).unwrap());
    }
    ts
}
#[cfg(test)]
fn checkpoint(m: &Model, s: State) -> Vec<u8> {
    let mut b = VERSION.to_le_bytes().to_vec();
    for v in m
        .weights
        .into_iter()
        .chain(s.h)
        .chain([s.occupied, s.confidence])
    {
        b.extend(v.to_le_bytes())
    }
    b
}
#[cfg(test)]
fn restore(b: &[u8]) -> Result<(Model, State), &'static str> {
    if b.len() != 44 || u32::from_le_bytes(b[..4].try_into().unwrap()) != VERSION {
        return Err("version/shape");
    };
    let mut v = [0.; 10];
    for (j, c) in b[4..].as_chunks::<4>().0.iter().enumerate() {
        v[j] = f32::from_le_bytes(*c)
    }
    if v.iter().any(|x| !x.is_finite())
        || v[6..8].iter().any(|x| x.abs() > 1.)
        || v[8..].iter().any(|x| !(0. ..=1.).contains(x))
    {
        return Err("state");
    };
    Ok((
        Model {
            weights: v[..6].try_into().unwrap(),
        },
        State {
            h: [v[6], v[7]],
            occupied: v[8],
            confidence: v[9],
        },
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn capture() -> Input {
        Input {
            capture: 1.,
            range: 0.3,
            left: 0.6,
            forward: 0.8,
            ..Input::default()
        }
    }
    #[test]
    fn stationary_cadence() {
        for n in [1, 3, 4, 7] {
            let mut s = State::default();
            let m = Model::default();
            let mut w = Work::default();
            interval(&m, &mut s, capture(), n, &mut w);
            let start = s;
            for _ in 1..96 {
                interval(&m, &mut s, Input::default(), n, &mut w);
            }
            assert_eq!(s, start);
            assert!((s.h[0] * 32. - 1.92).abs() < 1e-5);
            assert!((s.h[1] * 32. - 1.44).abs() < 1e-5);
            assert_eq!(w.transitions, 96 * n as u64);
        }
    }
    #[test]
    fn no_cue_moving() {
        let mut s = State::default();
        let mut w = Work::default();
        for j in 0..96 {
            interval(
                &Model::default(),
                &mut s,
                Input {
                    dx: 0.2,
                    dz: -0.3,
                    ..Input::default()
                },
                [1, 3, 4, 7][j % 4],
                &mut w,
            );
        }
        assert_eq!(s, State::default());
    }
    #[test]
    fn transverse_mirror() {
        let m = Model::default();
        let mut a = State::default();
        let mut b = a;
        let mut w = Work::default();
        interval(&m, &mut a, capture(), 3, &mut w);
        interval(
            &m,
            &mut b,
            Input {
                left: -0.6,
                ..capture()
            },
            3,
            &mut w,
        );
        for j in 0..96 {
            interval(
                &m,
                &mut a,
                Input {
                    dz: 0.01,
                    ..Input::default()
                },
                [1, 3, 4, 7][j % 4],
                &mut w,
            );
            interval(
                &m,
                &mut b,
                Input {
                    dz: -0.01,
                    ..Input::default()
                },
                [1, 3, 4, 7][j % 4],
                &mut w,
            );
        }
        assert_eq!(a.h[0], b.h[0]);
        assert_eq!(a.h[1], -b.h[1]);
        assert!((a.h[1] * 32. - 0.48).abs() < 1e-5);
    }
    #[test]
    fn rotations() {
        for yaw in [
            0.,
            std::f32::consts::FRAC_PI_2,
            -std::f32::consts::FRAC_PI_2,
            3.0,
        ] {
            let mut s = State::default();
            let mut w = Work::default();
            interval(
                &Model::default(),
                &mut s,
                Input {
                    sin: yaw.sin(),
                    cos: yaw.cos(),
                    ..capture()
                },
                4,
                &mut w,
            );
            assert!((s.h[0] * 32. - (1.92 * yaw.cos() - 1.44 * yaw.sin())).abs() < 1e-5);
            assert!((s.h[1] * 32. - (1.92 * yaw.sin() + 1.44 * yaw.cos())).abs() < 1e-5);
        }
    }
    #[test]
    fn replacement_first_motion() {
        let m = Model::default();
        let mut a = State::default();
        let mut b = a;
        let mut w = Work::default();
        interval(&m, &mut a, capture(), 7, &mut w);
        let new = Input {
            left: -0.8,
            forward: 0.6,
            dx: 0.05,
            dz: -0.02,
            ..capture()
        };
        interval(&m, &mut a, new, 1, &mut w);
        interval(&m, &mut b, new, 7, &mut w);
        assert_eq!(a, b);
        assert!((a.h[0] * 32. - (1.44 - 0.05)).abs() < 1e-5);
    }
    #[test]
    fn confidence_not_distance() {
        let mut s = State::default();
        let mut w = Work::default();
        interval(&Model::default(), &mut s, capture(), 3, &mut w);
        let h = s.h;
        for _ in 0..96 {
            interval(
                &Model::default(),
                &mut s,
                Input {
                    fade: 0.95,
                    ..Input::default()
                },
                7,
                &mut w,
            );
        }
        assert_eq!(s.h, h);
        assert!((s.confidence - 0.95f32.powi(96)).abs() < 1e-5);
        assert_eq!(s.occupied, 1.);
    }
    #[test]
    fn saturation() {
        let mut s = State {
            h: [0., 0.],
            occupied: 1.,
            confidence: 1.,
        };
        let mut w = Work::default();
        interval(
            &Model::default(),
            &mut s,
            Input {
                dx: -64.,
                dz: 64.,
                ..Input::default()
            },
            7,
            &mut w,
        );
        assert_eq!(s.h, [1., -1.]);
        interval(
            &Model::default(),
            &mut s,
            Input {
                dx: 1.,
                dz: -1.,
                ..Input::default()
            },
            4,
            &mut w,
        );
        assert_eq!(s.h, [31. / 32., -31. / 32.]);
    }
    #[test]
    fn cold_and_rejection() {
        let m = Model::default();
        let mut s = State::default();
        interval(&m, &mut s, capture(), 3, &mut Work::default());
        let b = checkpoint(&m, s);
        let (m2, s2) = restore(&b).unwrap();
        assert_eq!(checkpoint(&m2, s2), b);
        let mut bad = b.clone();
        bad[0] ^= 1;
        assert!(restore(&bad).is_err());
        assert!(restore(&b[..40]).is_err());
    }
    #[test]
    fn gradients() {
        let run = |m: &Model| {
            let mut s = State::default();
            let mut w = Work::default();
            let mut ts = interval(
                m,
                &mut s,
                Input {
                    sin: 0.6,
                    cos: 0.8,
                    ..capture()
                },
                4,
                &mut w,
            );
            for _ in 0..5 {
                ts.extend(interval(
                    m,
                    &mut s,
                    Input {
                        dx: 0.02,
                        dz: -0.03,
                        ..Input::default()
                    },
                    7,
                    &mut w,
                ));
            }
            (s.h[0] * 0.7 + s.h[1] * (-0.4), ts)
        };
        let m = Model::default();
        let (_, ts) = run(&m);
        let (g, initial) = backward(&ts, [0.7, -0.4]);
        assert_eq!(initial, [0., 0.]);
        for (j, expected) in g.iter().enumerate() {
            let mut hi = m;
            let mut lo = m;
            hi.weights[j] += 1e-3;
            lo.weights[j] -= 1e-3;
            let fd = (run(&hi).0 - run(&lo).0) / 2e-3;
            assert!((fd - expected).abs() < 1e-4, "{j}: {fd} != {expected}");
        }
    }
    #[test]
    fn work_zero_and_active() {
        let mut wa = Work::default();
        let mut wb = Work::default();
        interval(
            &Model::default(),
            &mut State::default(),
            capture(),
            7,
            &mut wa,
        );
        interval(
            &Model::default(),
            &mut State::default(),
            Input::default(),
            7,
            &mut wb,
        );
        assert_eq!(wa, wb);
        assert_eq!(wa.bilinear_terms, 6);
        assert_eq!(wa.bounded_activations, 2);
    }
}
/// Position-loss adjoints to ordinary sensed inputs; capture/pulse are provenance
/// events, not differentiable fitted controls. No output-confidence loss here.
pub(super) fn input_backward(m: &Model, tapes: &[Tape], mut a: [f32; 2]) -> Vec<[f32; 7]> {
    let mut out = vec![[0.; 7]; tapes.len()];
    for (k, t) in tapes.iter().enumerate().rev() {
        let i = t.input;
        if i.pulse == 0. {
            continue;
        }
        let pc = i.pulse * i.capture;
        let ax = a[0] * t.derivative[0];
        let az = a[1] * t.derivative[1];
        let w = m.weights;
        // range,left,forward,sin,cos,dx,dz
        if pc != 0. {
            let range_x = if ax == 0. {
                0.
            } else {
                ax * (w[0] * i.forward * i.cos + w[1] * i.left * i.sin)
            };
            let range_z = if az == 0. {
                0.
            } else {
                az * (w[2] * i.forward * i.sin + w[3] * i.left * i.cos)
            };
            out[k][..5].copy_from_slice(&[
                pc * (range_x + range_z),
                pc * i.range * (ax * w[1] * i.sin + az * w[3] * i.cos),
                pc * i.range * (ax * w[0] * i.cos + az * w[2] * i.sin),
                pc * i.range * (ax * w[1] * i.left + az * w[2] * i.forward),
                pc * i.range * (ax * w[0] * i.forward + az * w[3] * i.left),
            ]);
        }
        out[k][5] = -ax * i.pulse * t.occupied * w[4];
        out[k][6] = -az * i.pulse * t.occupied * w[5];
        a = [ax * (1. - pc), az * (1. - pc)];
    }
    out
}
#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
struct Adam {
    first: [f32; 6],
    second: [f32; 6],
    age: u32,
}
#[cfg(test)]
impl Default for Adam {
    fn default() -> Self {
        Self {
            first: [0.; 6],
            second: [0.; 6],
            age: 0,
        }
    }
}
#[cfg(test)]
impl Adam {
    fn update(&mut self, m: &mut Model, g: [f32; 6]) -> Result<(), &'static str> {
        if g.iter().any(|x| !x.is_finite()) {
            return Err("gradient");
        };
        let age = self.age.checked_add(1).ok_or("age overflow")?;
        let mut n = self.clone();
        let mut w = m.weights;
        for j in 0..6 {
            n.first[j] = 0.9 * n.first[j] + 0.1 * g[j];
            n.second[j] = 0.999 * n.second[j] + 0.001 * g[j] * g[j];
            let mh = n.first[j] / (1. - 0.9f32.powf(age as f32));
            let vh = n.second[j] / (1. - 0.999f32.powf(age as f32));
            w[j] -= 0.001 * mh / (vh.sqrt() + 1e-8);
        }
        if w.iter()
            .chain(n.first.iter())
            .chain(n.second.iter())
            .any(|v| !v.is_finite())
        {
            return Err("nonfinite update");
        };
        n.age = age;
        *self = n;
        m.weights = w;
        Ok(())
    }
}
#[cfg(test)]
mod moretests {
    use super::*;
    #[test]
    fn source_input_gradients() {
        let m = Model::default();
        let base = Input {
            pulse: 1.,
            capture: 1.,
            range: 0.31,
            left: 0.6,
            forward: 0.8,
            sin: 0.6,
            cos: 0.8,
            dx: 0.03,
            dz: -0.02,
            ..Input::default()
        };
        let run = |i| {
            let mut s = State::default();
            let t = step(&m, &mut s, i, &mut Work::default()).unwrap();
            (0.7 * s.h[0] - 0.4 * s.h[1], t)
        };
        let (_, t) = run(base);
        let a = input_backward(&m, &[t], [0.7, -0.4])[0];
        for (j, expected) in a.iter().enumerate() {
            let mut hi = base;
            let mut lo = base;
            let perturb = |i: &mut Input, d: f32| {
                let v = match j {
                    0 => &mut i.range,
                    1 => &mut i.left,
                    2 => &mut i.forward,
                    3 => &mut i.sin,
                    4 => &mut i.cos,
                    5 => &mut i.dx,
                    _ => &mut i.dz,
                };
                *v += d;
            };
            perturb(&mut hi, 0.001);
            perturb(&mut lo, -0.001);
            let fd = (run(hi).0 - run(lo).0) / 0.002;
            assert!((fd - expected).abs() < 1e-4, "{j}: {fd} {expected}");
        }
    }
    #[test]
    fn independent_optimizer_age_and_atomicity() {
        let mut old_model = Model::default();
        let mut old = Adam::default();
        for _ in 0..3 {
            old.update(&mut old_model, [0.2; 6]).unwrap()
        }
        let before = old.clone();
        let mut fresh_model = Model::default();
        let mut fresh = Adam::default();
        fresh.update(&mut fresh_model, [0.3; 6]).unwrap();
        assert_eq!(old, before);
        assert_eq!(old.age, 3);
        assert_eq!(fresh.age, 1);
        let w = fresh_model.weights;
        let a = fresh.clone();
        assert!(fresh.update(&mut fresh_model, [f32::NAN; 6]).is_err());
        assert_eq!(fresh, a);
        assert_eq!(fresh_model.weights, w);
    }
}

pub(super) fn full_state_backward(
    m: &Model,
    tapes: &[Tape],
    mut a: [f32; 4],
) -> ([f32; 4], Vec<[f32; 2]>) {
    let mut c = vec![[0.; 2]; tapes.len()];
    for (k, t) in tapes.iter().enumerate().rev() {
        let i = t.input;
        if i.pulse == 0. {
            continue;
        }
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
#[cfg(test)]
mod state_gradient_tests {
    use super::*;
    #[test]
    fn all_initial_state_gradients() {
        let m = Model::default();
        let run = |mut s: State| {
            let mut w = Work::default();
            let mut t = vec![];
            for _ in 0..4 {
                t.extend(interval(
                    &m,
                    &mut s,
                    Input {
                        dx: 0.03,
                        dz: -0.04,
                        fade: 0.97,
                        ..Input::default()
                    },
                    4,
                    &mut w,
                ));
            }
            (
                0.7 * s.h[0] - 0.4 * s.h[1] + 0.2 * s.occupied + 0.3 * s.confidence,
                t,
            )
        };
        let base = State {
            h: [0.05, -0.04],
            occupied: 0.8,
            confidence: 0.7,
        };
        let (_, t) = run(base);
        let (g, _) = full_state_backward(&m, &t, [0.7, -0.4, 0.2, 0.3]);
        for (j, expected) in g.iter().enumerate() {
            let mut hi = base;
            let mut lo = base;
            let perturb = |s: &mut State, d: f32| match j {
                0 => s.h[0] += d,
                1 => s.h[1] += d,
                2 => s.occupied += d,
                _ => s.confidence += d,
            };
            perturb(&mut hi, 0.001);
            perturb(&mut lo, -0.001);
            let fd = (run(hi).0 - run(lo).0) / 0.002;
            assert!((fd - expected).abs() < 1e-4);
        }
    }
    #[test]
    fn saturated_derivative_zero() {
        let m = Model::default();
        let mut s = State {
            h: [0.9, -0.9],
            occupied: 1.,
            confidence: 0.7,
        };
        let t = step(
            &m,
            &mut s,
            Input {
                pulse: 1.,
                dx: -10.,
                dz: 10.,
                ..Input::default()
            },
            &mut Work::default(),
        )
        .unwrap();
        let (g, _) = full_state_backward(&m, &[t], [1., 1., 0., 0.]);
        assert_eq!(g, [0.; 4]);
    }
}

#[cfg(test)]
mod identity_validation_tests {
    use super::*;
    #[test]
    fn no_boundary_bit_and_gradient_identity() {
        let m = Model::default();
        let mut s = State {
            h: [-0., 1.],
            occupied: 1.,
            confidence: -0.,
        };
        let before = s;
        let t = step(&m, &mut s, Input::default(), &mut Work::default()).unwrap();
        assert_eq!(s.h.map(f32::to_bits), before.h.map(f32::to_bits));
        assert_eq!(s.confidence.to_bits(), before.confidence.to_bits());
        assert_eq!(full_state_backward(&m, &[t], [1., 1., 1., 1.]).0, [1.; 4]);
    }
    #[test]
    fn overflow_or_invalid_state_does_not_commit() {
        let mut m = Model::default();
        m.weights[0] = f32::MAX;
        m.weights[1] = f32::MAX;
        let mut s = State::default();
        let mut w = Work::default();
        let before = s;
        assert!(step(
            &m,
            &mut s,
            Input {
                pulse: 1.,
                capture: 1.,
                range: 1.,
                left: 1.,
                forward: 1.,
                sin: 1.,
                cos: 1.,
                ..Input::default()
            },
            &mut w
        )
        .is_err());
        assert_eq!(s, before);
        assert_eq!(w, Work::default());
        s.h[0] = 2.;
        assert!(step(&Model::default(), &mut s, Input::default(), &mut w).is_err());
        assert_eq!(s.h[0], 2.);
    }
}
