//! 2083 detached direct own-feedback adapter. No controller or host admission.
//! Frozen2076 remains the seven-context/stride kernel. This adds its missing
//! direct-e pullback, never an inverse of a saturated tanh.
//! Numerical forward support is finite-intermediate: (stride-dot)/stride must
//! itself be finite before gain multiplication. A compensating tiny/zero gain
//! does not rescue an overflowing ratio. This is not a signed-weight permission
//! restriction; all five finite signed parameters remain mutable.
use super::own_event_kernel as core;
pub const IDENTITY: &str = "direct-own-feedback2083-f64-v1";
pub type Error = &'static str;
#[derive(Clone, Debug)]
pub struct Tape {
    inner: core::Tape,
    state: core::State,
    event: core::Event,
    parameters: [f64; 5],
    dot: f64,
    ratio: f64,
    signed_error: f64,
    error: f64,
}
impl Tape {
    pub fn own_output(&self) -> &core::Output {
        self.inner.output()
    }
    pub fn error(&self) -> f64 {
        self.error
    }
    pub fn signed_error(&self) -> f64 {
        self.signed_error
    }
    pub fn calibrated_moving(&self) -> bool {
        self.inner.output().calibrated
    }
    pub fn historical_parameters(&self) -> [f64; 5] {
        self.parameters
    }
    pub fn historical_state(&self) -> core::State {
        self.state
    }
    pub fn historical_event(&self) -> core::Event {
        self.event
    }
}
fn finite(x: f64) -> Result<f64, Error> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err("nonfinite direct-feedback arithmetic")
    }
}
pub fn forward(
    state: core::State,
    event: core::Event,
    parameters: [f64; 5],
) -> Result<Tape, Error> {
    let inner = core::forward(state, event, parameters)?;
    let mut t = Tape {
        inner,
        state,
        event,
        parameters,
        dot: 0.,
        ratio: 0.,
        signed_error: 0.,
        error: 0.,
    };
    // Off, Stop, first observation, or an uncalibrated stationary event must not
    // divide by stride or inspect unused event numerics.
    if t.inner.output().calibrated {
        t.dot = finite(
            event.vectors[3][0] * event.vectors[0][0] + event.vectors[3][1] * event.vectors[0][1],
        )?;
        t.ratio = finite(finite(state.stride - t.dot)? / state.stride)?;
        t.signed_error = finite(parameters[4] * t.ratio)?;
        t.error = if t.signed_error > 0. {
            t.signed_error
        } else {
            0.
        };
    }
    Ok(t)
}
// Positive finite input into [1,2)*2^exponent. Handles subnormals exactly.
fn frexp_positive(x: f64) -> (f64, i32) {
    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        let (m, e) = frexp_positive(x * 18014398509481984.);
        return (m, e - 54);
    }
    (
        f64::from_bits((bits & ((1u64 << 52) - 1)) | (1023u64 << 52)),
        exponent - 1023,
    )
}
/// Stable factor product/quotient for active pullbacks: zero adjoints are handled
/// before this call. No overflowing s*s or gain*dot prefix is evaluated.
fn scaled_ratio(numerators: &[f64], denominators: &[f64]) -> Result<f64, Error> {
    for x in numerators.iter().chain(denominators) {
        finite(*x)?;
    }
    if denominators.contains(&0.) {
        return Err("zero pullback denominator");
    }
    if numerators.contains(&0.) {
        return Ok(0.);
    }
    let mut sign = 1.;
    let mut mantissa = 1.;
    let mut exponent = 0i32;
    for x in numerators {
        if x.is_sign_negative() {
            sign = -sign;
        }
        let (m, e) = frexp_positive(x.abs());
        mantissa *= m;
        exponent += e;
        if mantissa >= 2. {
            mantissa *= 0.5;
            exponent += 1;
        }
    }
    for x in denominators {
        if x.is_sign_negative() {
            sign = -sign;
        }
        let (m, e) = frexp_positive(x.abs());
        mantissa /= m;
        exponent -= e;
        if mantissa < 1. {
            mantissa *= 2.;
            exponent -= 1;
        }
    }
    if exponent > 1023 {
        return Err("direct-feedback adjoint overflow");
    }
    if exponent < -1075 {
        return Ok(sign * 0.);
    }
    let result = if exponent >= -1022 {
        mantissa * 2f64.powi(exponent)
    } else {
        (mantissa * 2f64.powi(exponent + 1022)) * f64::MIN_POSITIVE
    };
    finite(sign * result)
}
fn add(dst: &mut f64, x: f64) -> Result<(), Error> {
    *dst = finite(*dst + x)?;
    Ok(())
}
/// Seeds ALL original outputs plus direct e. Caller must use the historical
/// parameter values; current updated model values do not retag old feedback.
/// Positive-part derivative is exactly zero on inactive branch and the kink.
/// All validation is transactional: no state/model/tape mutation occurs.
pub fn backward(
    t: &Tape,
    historical_parameters: [f64; 5],
    context: [f64; 7],
    next_stride: f64,
    error_adjoint: f64,
) -> Result<core::Gradients, Error> {
    finite(error_adjoint)?;
    let mut g = core::backward(&t.inner, historical_parameters, context, next_stride)?;
    if error_adjoint == 0. || !t.inner.output().calibrated || t.signed_error <= 0. {
        return Ok(g);
    }
    let s = t.state.stride;
    let gain = t.parameters[4];
    add(
        &mut g.state,
        scaled_ratio(&[error_adjoint, gain, t.dot], &[s, s])?,
    )?;
    add(
        &mut g.parameters[4],
        scaled_ratio(&[error_adjoint, t.ratio], &[])?,
    )?;
    for j in 0..2 {
        add(
            &mut g.inputs[3][j],
            scaled_ratio(&[-error_adjoint, gain, t.event.vectors[0][j]], &[s])?,
        )?;
        add(
            &mut g.inputs[0][j],
            scaled_ratio(&[-error_adjoint, gain, t.event.vectors[3][j]], &[s])?,
        )?;
    }
    Ok(g)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> core::State {
        core::State {
            stride: 0.2,
            occupied: true,
            ..core::State::default()
        }
    }
    fn event() -> core::Event {
        core::Event {
            sequence: 0,
            start_tick: 0,
            end_tick: 1,
            admitted: true,
            issued: true,
            moving: true,
            vectors: [
                [0.8, 0.6],
                [1., 0.],
                [1., 0.],
                [0.08, 0.02],
                [1., 0.],
                [1., 0.],
            ],
        }
    }
    fn params() -> [f64; 5] {
        [0.3, 0.7, -0.6, 0.8, 0.9]
    }
    fn close(x: f64, y: f64) {
        assert!(
            (x - y).abs() < 3e-8 * (1. + x.abs() + y.abs()),
            "{x} != {y}"
        )
    }
    #[test]
    fn ordinary_inverse_agreement_and_saturated_direct() {
        for s in [0.001, 0.1, 1., 10.] {
            for ratio in [-1., 0., 0.5, 1., 2.] {
                for gain in [-2., 0., 0.5, 1.] {
                    let mut st = state();
                    st.stride = s;
                    let mut ev = event();
                    ev.vectors[0] = [1., 0.];
                    ev.vectors[3] = [s * ratio, 0.];
                    let mut p = params();
                    p[4] = gain;
                    let t = forward(st, ev, p).unwrap();
                    let inverse = (t.own_output().context[6].atanh() * s.hypot(1.) / s).max(0.);
                    close(t.error(), inverse);
                }
            }
        }
        let st = core::State {
            stride: 1.,
            ..state()
        };
        let mut ev = event();
        ev.vectors[0] = [1., 0.];
        ev.vectors[3] = [-1., 0.];
        let mut p = params();
        p[4] = 100.;
        let t = forward(st, ev, p).unwrap();
        assert_eq!(t.error(), 200.);
        assert_eq!(t.own_output().context[6], 1.);
        let g = backward(&t, p, [0.; 7], 0., 1.).unwrap();
        assert_eq!(g.parameters[4], 2.);
        assert_eq!(g.inputs[3][0], -100.);
    }
    #[test]
    fn all_eighteen_combined_adjoints() {
        let s = state();
        let e = event();
        let p = params();
        let weights = [0.2, -0.3, 0.4, 0.5, -0.6, 0.7, 0.8];
        let t = forward(s, e, p).unwrap();
        let g = backward(&t, p, weights, 0.9, 0.6).unwrap();
        let loss = |s, e, p| {
            let t = forward(s, e, p).unwrap();
            t.error() * 0.6
                + t.own_output().state.stride * 0.9
                + t.own_output()
                    .context
                    .iter()
                    .zip(weights)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
        };
        let eps = 1e-7;
        for i in 0..18 {
            let (mut sh, mut sl) = (s, s);
            let (mut eh, mut el) = (e, e);
            let (mut ph, mut pl) = (p, p);
            let expected = if i == 0 {
                sh.stride += eps;
                sl.stride -= eps;
                g.state
            } else if i < 6 {
                ph[i - 1] += eps;
                pl[i - 1] -= eps;
                g.parameters[i - 1]
            } else {
                let k = i - 6;
                eh.vectors[k / 2][k % 2] += eps;
                el.vectors[k / 2][k % 2] -= eps;
                g.inputs[k / 2][k % 2]
            };
            close((loss(sh, eh, ph) - loss(sl, el, pl)) / (2. * eps), expected);
        }
    }
    #[test]
    fn kink_inactive_and_negative_gain() {
        let mut p = params();
        p[4] = 0.;
        let t = forward(state(), event(), p).unwrap();
        let g = backward(&t, p, [0.; 7], 0., 1.).unwrap();
        assert_eq!(t.error(), 0.);
        assert_eq!(g.parameters, [0.; 5]);
        // Original context gain remains trainable at zero: composing e cannot drop it.
        let g = backward(&t, p, [0., 0., 0., 0., 0., 0., 1.], 0., 1.).unwrap();
        assert_ne!(g.parameters[4], 0.);
        p[4] = -1.;
        assert_eq!(forward(state(), event(), p).unwrap().error(), 0.);
        let mut e = event();
        e.vectors[0] = [1., 0.];
        e.vectors[3] = [0.4, 0.];
        let t = forward(state(), e, p).unwrap();
        assert_eq!(t.error(), 1.);
        assert!(backward(&t, p, [0.; 7], 0., 1.).unwrap().inputs[3][0] > 0.);
        e.vectors[3] = [0.2, 0.];
        p[4] = 1.;
        let t = forward(state(), e, p).unwrap();
        assert_eq!(t.error(), 0.);
        assert_eq!(backward(&t, p, [0.; 7], 0., 1.).unwrap().state, 0.);
    }
    #[test]
    fn uncalibrated_stop_off_and_unused_numeric() {
        let p = params();
        let t = forward(core::State::default(), event(), p).unwrap();
        assert_eq!(t.error(), 0.);
        assert!(!t.calibrated_moving());
        for (admitted, issued) in [(false, true), (true, false)] {
            let mut e = event();
            e.admitted = admitted;
            e.issued = issued;
            e.vectors = [[f64::NAN, f64::INFINITY]; 6];
            let t = forward(state(), e, p).unwrap();
            assert_eq!(t.error(), 0.);
            let g = backward(&t, p, [1.; 7], 2., 3.).unwrap();
            assert_eq!(g.state, 2.);
            assert_eq!(g.parameters, [0.; 5]);
        }
        let mut e = event();
        e.moving = false;
        e.vectors[0] = [0.; 2];
        let t = forward(state(), e, p).unwrap();
        assert_eq!(t.error(), 0.);
        assert_eq!(t.own_output().state.stride, 0.2);
    }
    #[test]
    fn historical_binding_and_transaction_failure() {
        let s = state();
        let e = event();
        let p = params();
        let t = forward(s, e, p).unwrap();
        let mut changed = p;
        changed[4] = 2.;
        assert!(backward(&t, changed, [0.; 7], 0., 1.).is_err());
        assert_eq!(t.historical_parameters(), p);
        assert_eq!(
            t.historical_state().cold_bits().unwrap(),
            s.cold_bits().unwrap()
        );
        let mut tiny = s;
        tiny.stride = f64::from_bits(1);
        let mut e = event();
        e.vectors[3] = [-1., 0.];
        assert!(forward(tiny, e, p).is_err());
        assert_eq!(s.stride, 0.2);
        assert!(backward(&t, p, [0.; 7], 0., f64::INFINITY).is_err());
        let mut duplicate = e;
        duplicate.sequence = t.own_output().state.last_sequence;
        assert!(forward(t.own_output().state, duplicate, p).is_err());
    }
    #[test]
    fn stable_extreme_ratios_and_true_overflow() {
        assert_eq!(
            scaled_ratio(&[f64::MAX, 2., 0.25], &[]).unwrap(),
            f64::MAX * 0.5
        );
        assert_eq!(scaled_ratio(&[1e-200, 1e-200], &[1e-200]).unwrap(), 1e-200);
        assert_eq!(scaled_ratio(&[1., 0.], &[f64::from_bits(1)]).unwrap(), 0.);
        assert!(scaled_ratio(&[f64::MAX, 2.], &[]).is_err());
    }
    #[test]
    fn two_event_state_chain_preserves_original_bptt() {
        let p = params();
        let s = state();
        let e1 = event();
        let t1 = forward(s, e1, p).unwrap();
        let mut e2 = event();
        e2.sequence = 1;
        e2.start_tick = 1;
        e2.end_tick = 2;
        e2.vectors[3] = [0.03, 0.01];
        let t2 = forward(t1.own_output().state, e2, p).unwrap();
        let g2 = backward(&t2, p, [0.2; 7], 0.4, 0.7).unwrap();
        let g1 = backward(&t1, p, [0.1; 7], g2.state, 0.3).unwrap();
        let loss = |s, p| {
            let a = forward(s, e1, p).unwrap();
            let b = forward(a.own_output().state, e2, p).unwrap();
            0.3 * a.error()
                + 0.7 * b.error()
                + 0.1 * a.own_output().context.iter().sum::<f64>()
                + 0.2 * b.own_output().context.iter().sum::<f64>()
                + 0.4 * b.own_output().state.stride
        };
        let eps = 1e-7;
        let mut hi = s;
        let mut lo = s;
        hi.stride += eps;
        lo.stride -= eps;
        close((loss(hi, p) - loss(lo, p)) / (2. * eps), g1.state);
        for i in 0..5 {
            let mut hi = p;
            let mut lo = p;
            hi[i] += eps;
            lo[i] -= eps;
            close(
                (loss(s, hi) - loss(s, lo)) / (2. * eps),
                g1.parameters[i] + g2.parameters[i],
            );
        }
    }
    #[test]
    fn composed_angular_zero_and_live_coupling_pullback() {
        // Detached f64 angular formula only, not the native f32 head or a host.
        let angular = |error: f64, k: [f64; 2]| {
            let v = [0.4f64, 0.8];
            let den = (1. + v[0] * v[0] + v[1] * v[1]).sqrt();
            let u = [v[0] / den, v[1] / den];
            let effects = [[0.3, 0.2], [0.1, -0.1], [-0.2, 0.4]];
            let gains = [0.7, -0.6, 0.8];
            let dots = effects.map(|e| u[0] * e[0] + u[1] * e[1]);
            let raw = 0.8
                * 0.9
                * (gains[0] * dots[0]
                    + gains[1] * (1. - k[0] * error) * dots[1]
                    + gains[2] * (1. - k[1] * error) * dots[2]);
            let hidden = raw.tanh();
            let scale = 2. * (1. - hidden * hidden) * 0.8 * 0.9;
            (
                2. * hidden,
                scale * (-gains[1] * k[0] * dots[1] - gains[2] * k[1] * dots[2]),
                [
                    -scale * gains[1] * error * dots[1],
                    -scale * gains[2] * error * dots[2],
                ],
            )
        };
        let s = state();
        let e = event();
        let p = params();
        let t = forward(s, e, p).unwrap();
        let (zero_value, zero_de, zero_dk) = angular(t.error(), [0.; 2]);
        assert_eq!(zero_value, angular(0., [0.; 2]).0);
        assert_eq!(zero_de, 0.);
        assert!(zero_dk.iter().all(|v| *v != 0.));
        let eps = 1e-7;
        for i in 0..2 {
            let mut hi = [0.; 2];
            let mut lo = [0.; 2];
            hi[i] = eps;
            lo[i] = -eps;
            close(
                (angular(t.error(), hi).0 - angular(t.error(), lo).0) / (2. * eps),
                zero_dk[i],
            );
        }
        for k in [[0., 0.], [0.5, -0.4]] {
            let (_, de, dk) = angular(t.error(), k);
            let g = backward(&t, p, [0.1; 7], 0.2, de).unwrap();
            let loss = |s, e, p| {
                let t = forward(s, e, p).unwrap();
                angular(t.error(), k).0
                    + 0.1 * t.own_output().context.iter().sum::<f64>()
                    + 0.2 * t.own_output().state.stride
            };
            for i in 0..18 {
                let (mut sh, mut sl) = (s, s);
                let (mut eh, mut el) = (e, e);
                let (mut ph, mut pl) = (p, p);
                let expected = if i == 0 {
                    sh.stride += eps;
                    sl.stride -= eps;
                    g.state
                } else if i < 6 {
                    ph[i - 1] += eps;
                    pl[i - 1] -= eps;
                    g.parameters[i - 1]
                } else {
                    let j = i - 6;
                    eh.vectors[j / 2][j % 2] += eps;
                    el.vectors[j / 2][j % 2] -= eps;
                    g.inputs[j / 2][j % 2]
                };
                close((loss(sh, eh, ph) - loss(sl, el, pl)) / (2. * eps), expected);
            }
            for i in 0..2 {
                let mut hi = k;
                let mut lo = k;
                hi[i] += eps;
                lo[i] -= eps;
                close(
                    (angular(t.error(), hi).0 - angular(t.error(), lo).0) / (2. * eps),
                    dk[i],
                );
            }
        }
    }
}
