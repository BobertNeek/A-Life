//! Detached native M0 microcircuit component, NOT an admitted A-Life brain.
//! No world, sampler, optimizer, or automatic host call. Fixed private parameters
//! cannot be mutated by this component; that is not proof of future integrated
//! plasticity isolation. Uses the existing CPU leaky-tanh/dendrite update order.

#[derive(Clone, Copy)]
struct Branch {
    target: usize,
    inputs: &'static [(usize, f32)],
    threshold: f32,
    gain: f32,
}
#[derive(Clone, Copy)]
struct Readout {
    family: u8,
    source: usize,
    lane: usize,
    weight: f32,
}
mod plan;
pub use plan::RECIPE_SHA256;

pub const SETPOINTS: [f32; 11] = [
    0.35, 0.35, 0.25, 0.10, 0.35, 0.35, 0.50, 0.10, 0.50, 0.35, 0.50,
];
const N: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Full,
    SourceCut,
    ControllerRecurrenceCut,
    MotorMediationCut,
    ReadoutCut,
    EqualWorkSham,
    /// Deliberately invalid common-reference match, diagnostic negative control.
    ReferenceMismatch,
}
#[derive(Clone, Copy, Debug)]
pub struct Modulation {
    pub interoceptive_gain: f32,
    pub regional_gain: f32,
    pub afferent_offset: f32,
    pub projection_gain: f32,
    pub local_threshold: f32,
}
impl Default for Modulation {
    fn default() -> Self {
        Self {
            interoceptive_gain: 1.,
            regional_gain: 1.,
            afferent_offset: 0.,
            projection_gain: 0.5,
            local_threshold: 0.25,
        }
    }
}
impl Modulation {
    fn validate(self) -> bool {
        [
            self.interoceptive_gain,
            self.regional_gain,
            self.afferent_offset,
            self.projection_gain,
            self.local_threshold,
        ]
        .iter()
        .all(|v| v.is_finite())
            && (0. ..=4.).contains(&self.interoceptive_gain)
            && (0. ..=4.).contains(&self.regional_gain)
            && (0. ..=4.).contains(&self.projection_gain)
            && self.afferent_offset.abs() <= 4.
            && self.local_threshold.abs() <= 4.
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub microsteps: u64,
    pub neuron_updates: u64,
    pub encoder_assignments: u64,
    pub branches: u64,
    pub branch_inputs: u64,
    pub readout_products: u64,
}
/// All state belongs to this detached component. No creature checkpoint format
/// or old ABI is extended. The copy exposes state only for mechanism tests.
#[derive(Clone, Debug, PartialEq)]
pub struct Kernel {
    control: Control,
    activations: [f32; N],
    ema: [f32; N],
    metabolic: [f32; N],
    work: Work,
}
impl Kernel {
    pub fn new(control: Control) -> Self {
        Self {
            control,
            activations: [0.; N],
            ema: [0.; N],
            metabolic: [0.; N],
            work: Work::default(),
        }
    }
    /// Diagnostic equal common initial operating point; never edits a creature.
    pub fn with_common_activation(control: Control, value: f32) -> Result<Self, &'static str> {
        if !value.is_finite() || value.abs() > 1. {
            return Err("invalid_initial_state");
        }
        let mut k = Self::new(control);
        k.activations.fill(value);
        Ok(k)
    }
    pub fn work(&self) -> Work {
        self.work
    }
    pub fn activations(&self) -> &[f32; N] {
        &self.activations
    }
    pub fn pressure(&self, drive: usize) -> Option<f32> {
        (drive < 11).then(|| self.activations[22 + drive * 2] - self.activations[23 + drive * 2])
    }
    pub fn low_atp(&self) -> f32 {
        self.activations[44] - self.activations[45]
    }
    pub fn controller(&self) -> [f32; 3] {
        [
            self.activations[46] - self.activations[47],
            self.activations[48] - self.activations[49],
            self.activations[50] - self.activations[51],
        ]
    }
    /// One synchronous previous-state microstep. No hidden settling pass.
    pub fn step(&mut self, drives: [f32; 11], m: Modulation) -> Result<(), &'static str> {
        if !m.validate()
            || !drives
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        {
            return Err("invalid_input");
        }
        let old = self.activations;
        let mut encoded = [0.; N];
        for d in 0..11 {
            let signal = if matches!(self.control, Control::SourceCut | Control::EqualWorkSham) {
                SETPOINTS[d]
            } else {
                drives[d]
            };
            encoded[d * 2] = signal * m.interoceptive_gain * m.regional_gain + m.afferent_offset;
            let mismatch = if self.control == Control::ReferenceMismatch {
                1.5
            } else {
                1.
            };
            encoded[d * 2 + 1] = SETPOINTS[d] * m.interoceptive_gain * m.regional_gain * mismatch
                + m.afferent_offset;
        }
        let mut dendritic = [0.; N];
        for b in plan::BRANCHES {
            let mut sum = 0.;
            for &(source, weight) in b.inputs {
                let mut weight = weight;
                if self.control == Control::ControllerRecurrenceCut
                    && (46..52).contains(&b.target)
                    && source >= 46
                {
                    weight = 0.;
                }
                if self.control == Control::MotorMediationCut && b.target >= 52 {
                    weight = 0.;
                }
                sum += old[source] * weight;
            }
            dendritic[b.target] += (sum - b.threshold).max(0.).tanh() * b.gain;
        }
        for target in 0..N {
            let activated = (encoded[target] + m.projection_gain * dendritic[target]
                - m.local_threshold)
                .tanh();
            self.activations[target] = 0.5 * old[target] + 0.5 * activated;
            self.metabolic[target] = (0.99 * self.metabolic[target]
                + 0.01 * self.activations[target] * self.activations[target])
                .clamp(0., 1.);
            self.ema[target] =
                (0.98 * self.ema[target] + 0.02 * self.activations[target].abs()).clamp(0., 1.);
        }
        self.work.microsteps += 1;
        self.work.neuron_updates += N as u64;
        self.work.encoder_assignments += 22;
        self.work.branches += plan::BRANCHES.len() as u64;
        self.work.branch_inputs += plan::BRANCHES
            .iter()
            .map(|b| b.inputs.len() as u64)
            .sum::<u64>();
        Ok(())
    }
    pub fn candidate_delta(
        &mut self,
        family: u8,
        features: &[f32; 54],
    ) -> Result<f32, &'static str> {
        if family >= 8 || !features.iter().all(|v| v.is_finite() && v.abs() <= 1.) {
            return Err("invalid_candidate");
        }
        let mut value = 0.;
        for r in plan::READOUTS.iter().filter(|r| r.family == family) {
            let weight = if matches!(self.control, Control::ReadoutCut | Control::EqualWorkSham) {
                0.
            } else {
                r.weight
            };
            value += self.activations[r.source] * features[r.lane] * weight;
            self.work.readout_products += 1;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests;
