//! Detached own-event numerical kernel 2076, schema 1. No host or World API.
//! Float64 state/parameters; no migration of any pre-existing checkpoint.
//! Input order: nominal translation/body/gaze, displacement, measured body/gaze.
//! Vector derivatives are ambient derivatives; unit inputs remain constrained at admission.
pub const IDENTITY: &str = "own-event2076-f64-v1";
pub type Error = &'static str;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub stride: f64,
    pub occupied: bool,
    pub last_sequence: i64,
    pub reference_tick: i64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            stride: 0.,
            occupied: false,
            last_sequence: -1,
            reference_tick: -1,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub sequence: i64,
    pub start_tick: i64,
    pub end_tick: i64,
    pub admitted: bool,
    pub issued: bool,
    pub moving: bool,
    pub vectors: [[f64; 2]; 6],
}
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub state: State,
    pub context: [f64; 7],
    pub calibrated: bool,
    pub processed: bool,
    pub translation_prediction_available: bool,
    pub prior_stride: f64,
    pub raw_shortfall: f64,
}
#[derive(Clone, Debug)]
pub struct Tape {
    state: State,
    event: Event,
    parameters: [f64; 5],
    output: Output,
    scale: f64,
    translation: [f64; 2],
    body: [f64; 2],
    gaze: [f64; 2],
    alpha: f64,
    magnitude: f64,
}
impl Tape {
    pub fn output(&self) -> &Output {
        &self.output
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Gradients {
    pub state: f64,
    pub parameters: [f64; 5],
    pub inputs: [[f64; 2]; 6],
}
fn finite(x: f64) -> Result<f64, Error> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err("nonfinite numeric input or intermediate")
    }
}
fn validate_state(s: &State) -> Result<(), Error> {
    finite(s.stride)?;
    if (s.occupied && s.stride <= 0.)
        || (!s.occupied && s.stride != 0.)
        || s.last_sequence < -1
        || s.reference_tick < -1
    {
        return Err("state occupancy/reference");
    }
    Ok(())
}
fn unit(v: [f64; 2]) -> Result<(), Error> {
    finite(v[0])?;
    finite(v[1])?;
    if (v[0].hypot(v[1]) - 1.).abs() > 1e-6 {
        return Err("unit geometry");
    }
    Ok(())
}
fn sigmoid(x: f64) -> f64 {
    if x >= 0. {
        1. / (1. + (-x).exp())
    } else {
        let t = x.exp();
        t / (1. + t)
    }
}
pub fn forward(state: State, event: Event, parameters: [f64; 5]) -> Result<Tape, Error> {
    validate_state(&state)?;
    for x in parameters {
        finite(x)?;
    }
    let mut t = Tape {
        state,
        event,
        parameters,
        output: Output {
            state,
            context: [0.; 7],
            calibrated: false,
            processed: false,
            translation_prediction_available: false,
            prior_stride: state.stride,
            raw_shortfall: 0.,
        },
        scale: 1.,
        translation: [0.; 2],
        body: [0.; 2],
        gaze: [0.; 2],
        alpha: 0.,
        magnitude: 0.,
    };
    // No numeric or reference validation of unused event data, and no reference advance.
    if !event.admitted || !event.issued {
        return Ok(t);
    }
    if event.sequence < 0
        || event.sequence <= state.last_sequence
        || event.start_tick < 0
        || event.end_tick <= event.start_tick
        || event.start_tick < state.reference_tick
    {
        return Err("duplicate/out-of-order event");
    }
    for i in [1, 2, 4, 5] {
        unit(event.vectors[i])?;
    }
    for i in [0, 3] {
        for x in event.vectors[i] {
            finite(x)?;
        }
    }
    if event.moving {
        unit(event.vectors[0])?;
    } else if event.vectors[0] != [0.; 2] {
        return Err("Stop requires zero translation");
    }
    t.magnitude = finite(event.vectors[3][0].hypot(event.vectors[3][1]))?;
    t.scale = state.stride.hypot(1.);
    let calibrated = state.occupied && event.moving;
    for j in 0..2 {
        let expected = if calibrated {
            state.stride * event.vectors[0][j]
        } else {
            event.vectors[0][j] * 0.
        };
        t.translation[j] = finite(event.vectors[3][j] - expected)? / t.scale;
        t.body[j] = event.vectors[4][j] - event.vectors[1][j];
        t.gaze[j] = event.vectors[5][j] - event.vectors[2][j];
        t.output.context[j] = finite(parameters[1] * t.translation[j])?.tanh();
        t.output.context[j + 2] = finite(parameters[2] * t.body[j])?.tanh();
        t.output.context[j + 4] = finite(parameters[3] * t.gaze[j])?.tanh();
    }
    if calibrated {
        let dot = finite(
            event.vectors[3][0] * event.vectors[0][0] + event.vectors[3][1] * event.vectors[0][1],
        )?;
        t.output.raw_shortfall = finite(state.stride - dot)? / t.scale;
    }
    t.output.context[6] = finite(parameters[4] * t.output.raw_shortfall)?.tanh();
    let mut stride = state.stride;
    if event.moving && t.magnitude > 0. {
        t.alpha = sigmoid(parameters[0]);
        stride = if state.occupied {
            finite((1. - t.alpha) * state.stride + t.alpha * t.magnitude)?
        } else {
            t.magnitude
        };
    }
    t.output.state = State {
        stride,
        occupied: state.occupied || (event.moving && t.magnitude > 0.),
        last_sequence: event.sequence,
        reference_tick: event.end_tick,
    };
    validate_state(&t.output.state)?;
    t.output.calibrated = calibrated;
    t.output.processed = true;
    t.output.translation_prediction_available = state.occupied || !event.moving;
    Ok(t)
}
fn product(a: f64, b: f64) -> Result<f64, Error> {
    if a == 0. || b == 0. {
        Ok(0.)
    } else {
        finite(a * b)
    }
}
// Nonfinite-only fallback: combine three finite factors without an unused
// overflowing prefix. Ordinary finite arithmetic remains unchanged.
fn product3(a: f64, b: f64, c: f64) -> Result<f64, Error> {
    if a == 0. || b == 0. || c == 0. {
        return Ok(0.);
    }
    let normal = a * b * c;
    if normal.is_finite() {
        return Ok(normal);
    }
    let mut factors = [a, b, c];
    factors.sort_by(|x, y| x.abs().total_cmp(&y.abs()));
    // Smallest times largest first keeps a compensating small factor available.
    finite((factors[0] * factors[2]) * factors[1])
}
fn add(dst: &mut f64, x: f64) -> Result<(), Error> {
    *dst = finite(*dst + x)?;
    Ok(())
}
/// Exact tape parameter binding, including signed zero. No mutable input/tape fields.
/// Cotangents seed the seven context outputs and the returned stride state.
pub fn backward(
    t: &Tape,
    parameters: [f64; 5],
    context: [f64; 7],
    next_stride: f64,
) -> Result<Gradients, Error> {
    if parameters.map(f64::to_bits) != t.parameters.map(f64::to_bits) {
        return Err("tape/model mismatch");
    }
    for x in context {
        finite(x)?;
    }
    finite(next_stride)?;
    let mut g = Gradients {
        state: 0.,
        parameters: [0.; 5],
        inputs: [[0.; 2]; 6],
    };
    if !t.output.processed {
        g.state = next_stride;
        return Ok(g);
    }
    let s = t.state.stride;
    let inv = 1. / t.scale;
    let normalized = s / t.scale;
    for j in 0..2 {
        let d = product(context[j], 1. - t.output.context[j] * t.output.context[j])?;
        if d != 0. {
            add(&mut g.parameters[1], product(d, t.translation[j])?)?;
            add(&mut g.inputs[3][j], product3(d, parameters[1], inv)?)?;
            let mut ds = -t.translation[j] * normalized * inv;
            if t.output.calibrated {
                ds -= t.event.vectors[0][j] * inv;
                add(
                    &mut g.inputs[0][j],
                    product3(d, parameters[1], -normalized)?,
                )?;
            }
            add(&mut g.state, product3(d, parameters[1], ds)?)?;
        }
        for (offset, pi, ni, mi, res) in [(2, 2, 1, 4, t.body[j]), (4, 3, 2, 5, t.gaze[j])] {
            let d = product(
                context[offset + j],
                1. - t.output.context[offset + j] * t.output.context[offset + j],
            )?;
            if d != 0. {
                add(&mut g.parameters[pi], product(d, res)?)?;
                let a = product(d, parameters[pi])?;
                add(&mut g.inputs[mi][j], a)?;
                add(&mut g.inputs[ni][j], -a)?;
            }
        }
    }
    if t.output.calibrated {
        let d = product(context[6], 1. - t.output.context[6] * t.output.context[6])?;
        if d != 0. {
            add(&mut g.parameters[4], product(d, t.output.raw_shortfall)?)?;
            add(
                &mut g.state,
                product3(
                    d,
                    parameters[4],
                    inv - t.output.raw_shortfall * normalized * inv,
                )?,
            )?;
            for j in 0..2 {
                add(
                    &mut g.inputs[3][j],
                    product3(d, parameters[4], -t.event.vectors[0][j] * inv)?,
                )?;
                add(
                    &mut g.inputs[0][j],
                    product3(d, parameters[4], -t.event.vectors[3][j] * inv)?,
                )?;
            }
        }
    }
    if t.event.moving && t.magnitude > 0. {
        let dm = if t.state.occupied {
            add(&mut g.state, product(next_stride, 1. - t.alpha)?)?;
            // Difference of nonnegative finite strides is representable; saturated sigmoid has zero derivative.
            add(
                &mut g.parameters[0],
                product(
                    next_stride,
                    product(t.alpha * (1. - t.alpha), t.magnitude - s)?,
                )?,
            )?;
            product(next_stride, t.alpha)?
        } else {
            next_stride
        };
        for j in 0..2 {
            add(
                &mut g.inputs[3][j],
                product(dm, t.event.vectors[3][j] / t.magnitude)?,
            )?;
        }
    } else {
        add(&mut g.state, next_stride)?;
    }
    Ok(g)
}
/// Independent five-coordinate Adam, versioned bit-safe cold payload. Ages can differ.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub parameters: [f64; 5],
    pub first: [f64; 5],
    pub second: [f64; 5],
    pub ages: [u64; 5],
}
impl Default for Model {
    fn default() -> Self {
        Self {
            parameters: [0., 1., 1., 1., 1.],
            first: [0.; 5],
            second: [0.; 5],
            ages: [0; 5],
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Adam {
    pub learning_rate: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub epsilon: f64,
    pub weight_decay: f64,
}
impl Default for Adam {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.,
        }
    }
}
impl Model {
    pub fn validate(&self) -> Result<(), Error> {
        for x in self
            .parameters
            .into_iter()
            .chain(self.first)
            .chain(self.second)
        {
            finite(x)?;
        }
        if self.second.iter().any(|x| *x < 0.) {
            return Err("negative second moment");
        }
        Ok(())
    }
    pub fn update(
        &mut self,
        gradient: [f64; 5],
        selected: [bool; 5],
        a: Adam,
    ) -> Result<(), Error> {
        self.validate()?;
        for x in gradient {
            finite(x)?;
        }
        for x in [a.learning_rate, a.beta1, a.beta2, a.epsilon, a.weight_decay] {
            finite(x)?;
        }
        if a.learning_rate < 0.
            || !(0. ..1.).contains(&a.beta1)
            || !(0. ..1.).contains(&a.beta2)
            || a.epsilon <= 0.
            || a.weight_decay < 0.
        {
            return Err("invalid Adam");
        }
        let mut n = self.clone();
        for i in 0..5 {
            if !selected[i] {
                continue;
            }
            n.ages[i] = n.ages[i].checked_add(1).ok_or("age overflow")?;
            n.first[i] = finite(a.beta1 * n.first[i] + (1. - a.beta1) * gradient[i])?;
            n.second[i] =
                finite(a.beta2 * n.second[i] + (1. - a.beta2) * gradient[i] * gradient[i])?;
            let m = n.first[i] / (1. - a.beta1.powf(n.ages[i] as f64));
            let v = n.second[i] / (1. - a.beta2.powf(n.ages[i] as f64));
            n.parameters[i] = finite(
                n.parameters[i]
                    - a.learning_rate
                        * (m / (v.sqrt() + a.epsilon) + a.weight_decay * n.parameters[i]),
            )?;
        }
        n.validate()?;
        *self = n;
        Ok(())
    }
    pub fn cold_bits(&self) -> Result<Vec<u64>, Error> {
        self.validate()?;
        let mut b = vec![2076, 1];
        b.extend(self.parameters.map(f64::to_bits));
        b.extend(self.first.map(f64::to_bits));
        b.extend(self.second.map(f64::to_bits));
        b.extend(self.ages);
        Ok(b)
    }
    pub fn restore_bits(b: &[u64]) -> Result<Self, Error> {
        if b.len() != 22 || b[..2] != [2076, 1] {
            return Err("cold schema/length");
        }
        let mut m = Self::default();
        for i in 0..5 {
            m.parameters[i] = f64::from_bits(b[2 + i]);
            m.first[i] = f64::from_bits(b[7 + i]);
            m.second[i] = f64::from_bits(b[12 + i]);
            m.ages[i] = b[17 + i];
        }
        m.validate()?;
        Ok(m)
    }
}
impl State {
    pub fn cold_bits(&self) -> Result<[u64; 5], Error> {
        validate_state(self)?;
        Ok([
            self.stride.to_bits(),
            self.occupied as u64,
            self.last_sequence as u64,
            self.reference_tick as u64,
            2076,
        ])
    }
    pub fn restore_bits(b: [u64; 5]) -> Result<Self, Error> {
        if b[4] != 2076 || b[1] > 1 {
            return Err("state cold schema");
        };
        let s = Self {
            stride: f64::from_bits(b[0]),
            occupied: b[1] == 1,
            last_sequence: b[2] as i64,
            reference_tick: b[3] as i64,
        };
        validate_state(&s)?;
        Ok(s)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> Event {
        Event {
            sequence: 0,
            start_tick: 0,
            end_tick: 1,
            admitted: true,
            issued: true,
            moving: true,
            vectors: [[1., 0.], [1., 0.], [1., 0.], [0.2, 0.], [1., 0.], [1., 0.]],
        }
    }
    fn prior() -> State {
        State {
            stride: 0.2,
            occupied: true,
            ..State::default()
        }
    }
    fn p() -> [f64; 5] {
        [0., 1., 1., 1., 1.]
    }
    fn close(a: f64, b: f64) {
        assert!(
            (a - b).abs() < 2e-8 * (1. + a.abs() + b.abs()),
            "{a} != {b}"
        )
    }
    #[test]
    fn first_calibration_and_straight() {
        let t = forward(State::default(), event(), p()).unwrap();
        assert!(!t.output.calibrated);
        assert_eq!(t.output.raw_shortfall, 0.);
        assert_eq!(t.output.state.stride, 0.2);
        assert_eq!(
            forward(prior(), event(), p()).unwrap().output.context,
            [0.; 7]
        );
    }
    #[test]
    fn detour_shortened_block_stop() {
        let mut e = event();
        e.vectors[3] = [0., 0.2];
        e.vectors[4] = [0., 1.];
        let t = forward(prior(), e, p()).unwrap();
        assert!(t.output.context[0] < 0. && t.output.context[1] > 0. && t.output.context[6] > 0.);
        e.vectors[3] = [0., 0.];
        assert_eq!(forward(prior(), e, p()).unwrap().output.state.stride, 0.2);
        e.vectors[3] = [0.1, 0.];
        close(forward(prior(), e, p()).unwrap().output.state.stride, 0.15);
        e.moving = false;
        e.vectors[0] = [0., 0.];
        let t = forward(prior(), e, p()).unwrap();
        assert_eq!(t.output.state.stride, 0.2);
        assert_eq!(t.output.context[6], 0.);
        assert!(t.output.context[0] > 0.);
    }
    #[test]
    fn uncalibrated_block_and_stop() {
        let mut e = event();
        e.vectors[3] = [0.; 2];
        let t = forward(State::default(), e, p()).unwrap();
        assert!(
            !t.output.state.occupied
                && !t.output.calibrated
                && !t.output.translation_prediction_available
        );
        e.moving = false;
        e.vectors[0] = [0.; 2];
        assert!(
            forward(State::default(), e, p())
                .unwrap()
                .output
                .translation_prediction_available
        );
    }
    #[test]
    fn off_unused_numeric_exact_identity() {
        for (admitted, issued) in [(false, true), (true, false), (false, false)] {
            let mut e = event();
            e.admitted = admitted;
            e.issued = issued;
            e.sequence = -99;
            e.vectors = [[f64::NAN, f64::INFINITY]; 6];
            let s = State {
                stride: -0.,
                ..State::default()
            };
            let t = forward(s, e, p()).unwrap();
            assert_eq!(t.output.state.cold_bits().unwrap(), s.cold_bits().unwrap());
            assert_eq!(t.output.context, [0.; 7]);
            assert!(!t.output.processed);
            let g = backward(&t, p(), [1.; 7], 2.).unwrap();
            assert_eq!(g.state, 2.);
            assert_eq!(g.parameters, [0.; 5]);
            assert_eq!(g.inputs, [[0.; 2]; 6]);
        }
    }
    #[test]
    fn replay_out_of_order_invalid_atomic() {
        let s = forward(State::default(), event(), p())
            .unwrap()
            .output
            .state;
        let bits = s.cold_bits().unwrap();
        assert!(forward(s, event(), p()).is_err());
        let mut e = event();
        e.sequence = 1;
        e.end_tick = 2;
        assert!(forward(s, e, p()).is_err());
        e.start_tick = 1;
        e.vectors[3][0] = f64::NAN;
        assert!(forward(s, e, p()).is_err());
        assert_eq!(s.cold_bits().unwrap(), bits);
        e.vectors[3][0] = 0.1;
        e.vectors[1] = [2., 0.];
        assert!(forward(s, e, p()).is_err());
    }
    #[test]
    fn finite_differences_all_18_coordinates() {
        let s = prior();
        let mut e = event();
        e.vectors = [
            [0.8, 0.6],
            [0.6, 0.8],
            [0.8, -0.6],
            [0.12, 0.08],
            [0., 1.],
            [1., 0.],
        ];
        let params = [0.3, 0.7, -0.6, 0.8, 0.9];
        let weights = [0.2, -0.3, 0.4, 0.5, -0.6, 0.7, 0.8];
        let t = forward(s, e, params).unwrap();
        let g = backward(&t, params, weights, 0.9).unwrap();
        let objective = |s, e, p| {
            let t = forward(s, e, p).unwrap();
            0.9 * t.output.state.stride
                + t.output
                    .context
                    .iter()
                    .zip(weights)
                    .map(|(x, w)| x * w)
                    .sum::<f64>()
        };
        let eps = 1e-7;
        for i in 0..18 {
            let (mut sh, mut sl) = (s, s);
            let (mut eh, mut el) = (e, e);
            let (mut ph, mut pl) = (params, params);
            let expected = if i == 0 {
                sh.stride += eps;
                sl.stride -= eps;
                g.state
            } else if i <= 5 {
                ph[i - 1] += eps;
                pl[i - 1] -= eps;
                g.parameters[i - 1]
            } else {
                let k = i - 6;
                eh.vectors[k / 2][k % 2] += eps;
                el.vectors[k / 2][k % 2] -= eps;
                g.inputs[k / 2][k % 2]
            };
            close(
                (objective(sh, eh, ph) - objective(sl, el, pl)) / (2. * eps),
                expected,
            );
        }
    }
    #[test]
    fn zero_gain_live_and_signed() {
        let mut e = event();
        e.vectors[3] = [0.1, 0.1];
        let t = forward(prior(), e, [0.; 5]).unwrap();
        assert_eq!(t.output.context, [0.; 7]);
        assert_ne!(
            backward(&t, [0.; 5], [1.; 7], 0.).unwrap().parameters[4],
            0.
        );
        let pos = forward(prior(), e, p()).unwrap();
        let neg = forward(prior(), e, [0., -1., -1., -1., -1.]).unwrap();
        for i in 0..7 {
            assert_eq!(pos.output.context[i], -neg.output.context[i]);
        }
    }
    #[test]
    fn saturation_unused_and_true_overflow() {
        let mut e = event();
        e.vectors[3] = [0.1, 0.];
        let ps = [f64::MAX, f64::MAX, 1., 1., f64::MAX];
        let t = forward(prior(), e, ps).unwrap();
        let g = backward(&t, ps, [1.; 7], 0.).unwrap();
        assert!(g.parameters.iter().all(|x| x.is_finite()));
        assert_eq!(g.parameters[1], 0.);
        let mut bad = event();
        bad.vectors[3] = [-1e308, 0.];
        let s = State {
            stride: 1e308,
            ..prior()
        };
        assert!(forward(s, bad, p()).is_err());
        let t = forward(prior(), event(), p()).unwrap();
        assert!(backward(&t, [1.; 5], [0.; 7], 0.).is_err());
    }
    #[test]
    fn first_ema_unused_eta_and_block_state_gradient() {
        let t = forward(State::default(), event(), p()).unwrap();
        let g = backward(&t, p(), [0.; 7], 1.).unwrap();
        assert_eq!(g.parameters[0], 0.);
        assert_eq!(g.inputs[3], [1., 0.]);
        let mut e = event();
        e.vectors[3] = [0.; 2];
        let t = forward(prior(), e, p()).unwrap();
        let g = backward(&t, p(), [0.; 7], 1.).unwrap();
        assert_eq!(g.state, 1.);
        assert_eq!(g.parameters, [0.; 5]);
    }
    #[test]
    fn finite_scaled_adjoint_despite_overflowing_prefix() {
        let s = State {
            stride: 10.,
            ..prior()
        };
        let mut e = event();
        e.vectors[3] = [10., 0.];
        e.moving = false;
        e.vectors[0] = [0., 0.];
        e.vectors[3] = [0., 0.];
        let ps = [0., f64::MAX, 1., 1., 0.];
        let t = forward(s, e, ps).unwrap();
        let g = backward(&t, ps, [2., 0., 0., 0., 0., 0., 0.], 0.).unwrap();
        close(g.inputs[3][0] / f64::MAX, 2. / 101f64.sqrt());
        assert!(g.state.is_finite());
        // Angular unscaled derivative genuinely exceeds f64 and must reject.
        let ps = [0., 1., f64::MAX, 1., 1.];
        let t = forward(prior(), event(), ps).unwrap();
        assert!(backward(&t, ps, [0., 0., 2., 0., 0., 0., 0.], 0.).is_err());
    }
    #[test]
    fn adam_independent_ages_cold_and_failure() {
        let mut m = Model::default();
        m.parameters[0] = -0.;
        m.ages = [4, 3, 2, 1, 0];
        let bits = m.cold_bits().unwrap();
        assert_eq!(
            Model::restore_bits(&bits).unwrap().cold_bits().unwrap(),
            bits
        );
        m.update([0.2; 5], [true, false, true, false, true], Adam::default())
            .unwrap();
        assert_eq!(m.ages, [5, 3, 3, 1, 1]);
        let before = m.cold_bits().unwrap();
        assert!(m.update([f64::MAX; 5], [true; 5], Adam::default()).is_err());
        assert_eq!(m.cold_bits().unwrap(), before);
        let mut bad = before.clone();
        bad[0] = 2018;
        assert!(Model::restore_bits(&bad).is_err());
        assert_eq!(
            State::restore_bits(prior().cold_bits().unwrap()).unwrap(),
            prior()
        );
    }
}
