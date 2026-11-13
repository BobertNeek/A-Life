//! Added positional contextual neural state, experimental and CPU-only.
//! No old profile or collector admission is enabled by this numerical module.
#[path = "position_kernel.rs"]
mod kernel;
use super::*;
use serde::{Deserialize, Serialize};
pub const POSITION_ID2013: &str = "retained-positional-neural-context-2013-v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionModel2013 {
    pub schema: u16,
    pub identity: String,
    pub weights: [f32; 6],
    pub first_moment: [f32; 6],
    pub second_moment: [f32; 6],
    pub update_ages: [u32; 6],
    pub optimizer_step: u32,
    pub optimizer: AdamWConfig,
}
impl PositionModel2013 {
    pub fn fresh(optimizer: AdamWConfig) -> Result<Self, TrainingError> {
        let m = Self {
            schema: 1,
            identity: POSITION_ID2013.into(),
            weights: [0.25, -0.25, 0.25, 0.25, 1. / 32., 1. / 32.],
            first_moment: [0.; 6],
            second_moment: [0.; 6],
            update_ages: [0; 6],
            optimizer_step: 0,
            optimizer,
        };
        m.validate()?;
        Ok(m)
    }
    pub fn validate(&self) -> Result<(), TrainingError> {
        self.optimizer.validate()?;
        if self.schema != 1
            || self.identity != POSITION_ID2013
            || self.second_moment.iter().any(|x| *x < 0.)
        {
            return Err(invalid());
        }
        finite(&self.weights)?;
        finite(&self.first_moment)?;
        finite(&self.second_moment)?;
        Ok(())
    }
    pub fn apply_gradients(&mut self, g: &[f32; 6]) -> Result<(), TrainingError> {
        self.validate()?;
        finite(g)?;
        let mut n = self.clone();
        let norm = g.iter().map(|g| g * g).sum::<f32>().sqrt();
        if !norm.is_finite() {
            return Err(invalid());
        }
        let scale = if norm > self.optimizer.gradient_clip {
            self.optimizer.gradient_clip / norm
        } else {
            1.
        };
        n.optimizer_step = n.optimizer_step.checked_add(1).ok_or_else(invalid)?;
        for (j, gradient) in g.iter().enumerate() {
            n.update_ages[j] = n.update_ages[j].checked_add(1).ok_or_else(invalid)?;
            let (w, m, v) = super::adam_coordinate(
                n.weights[j],
                n.first_moment[j],
                n.second_moment[j],
                gradient * scale,
                n.update_ages[j],
                n.optimizer,
            )?;
            n.weights[j] = w;
            n.first_moment[j] = m;
            n.second_moment[j] = v;
        }
        n.validate()?;
        *self = n;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PositionState2013 {
    pub x: f32,
    pub z: f32,
    pub occupancy: f32,
    pub confidence: f32,
}
impl PositionState2013 {
    pub fn validate(&self) -> Result<(), TrainingError> {
        finite(&[self.x, self.z, self.occupancy, self.confidence])?;
        if self.x.abs() > 1.
            || self.z.abs() > 1.
            || !(0. ..=1.).contains(&self.occupancy)
            || !(0. ..=1.).contains(&self.confidence)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
/// Detached numerical frame. Real host admission must bind the raw collector witness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionInput2013 {
    pub boundary: bool,
    pub capture: bool,
    pub range01: f32,
    pub bearing: [f32; 2],
    pub source_yaw_sc: [f32; 2],
    pub own_delta_world: [f32; 2],
    pub observation_confidence: f32,
    pub confidence_decay: f32,
}
impl PositionInput2013 {
    fn kernel(&self) -> kernel::Input {
        kernel::Input {
            pulse: if self.boundary { 1. } else { 0. },
            capture: if self.capture { 1. } else { 0. },
            range: self.range01,
            left: self.bearing[0],
            forward: self.bearing[1],
            sin: self.source_yaw_sc[0],
            cos: self.source_yaw_sc[1],
            dx: self.own_delta_world[0],
            dz: self.own_delta_world[1],
            confidence: self.observation_confidence,
            fade: self.confidence_decay,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PositionWork2013 {
    pub transitions: u64,
    pub active_updates: u64,
}
/// Complete arithmetic tariff will additionally include frame binding/source-history work.
/// These counts describe only the neural transition kernel, not a host debit.
pub struct PositionTape2013 {
    model_weights: [f32; 6],
    steps: Vec<kernel::Tape>,
    pub states: Vec<PositionState2013>,
    pub work: PositionWork2013,
}
pub fn forward2013(
    model: &PositionModel2013,
    initial: PositionState2013,
    inputs: &[PositionInput2013],
) -> Result<PositionTape2013, TrainingError> {
    model.validate()?;
    initial.validate()?;
    let m = kernel::Model {
        weights: model.weights,
    };
    let mut s = kernel::State {
        h: [initial.x, initial.z],
        occupied: initial.occupancy,
        confidence: initial.confidence,
    };
    let mut w = kernel::Work::default();
    let mut steps = vec![];
    let mut states = vec![initial];
    let mut active_updates = 0u64;
    for i in inputs {
        let t = kernel::step(&m, &mut s, i.kernel(), &mut w).map_err(|_| invalid())?;
        steps.push(t);
        states.push(PositionState2013 {
            x: s.h[0],
            z: s.h[1],
            occupancy: s.occupied,
            confidence: s.confidence,
        });
        active_updates = active_updates
            .checked_add(u64::from(i.boundary))
            .ok_or_else(invalid)?;
    }
    Ok(PositionTape2013 {
        model_weights: model.weights,
        steps,
        states,
        work: PositionWork2013 {
            transitions: w.transitions,
            active_updates,
        },
    })
}
pub struct PositionGradient2013 {
    pub parameters: [f32; 6],
    pub initial_state: [f32; 4],
    pub source_inputs: Vec<[f32; 7]>,
    pub confidence_inputs: Vec<[f32; 2]>,
}
pub fn backward2013(
    model: &PositionModel2013,
    tape: &PositionTape2013,
    final_adjoint: [f32; 4],
) -> Result<PositionGradient2013, TrainingError> {
    model.validate()?;
    finite(&final_adjoint)?;
    if model.weights.map(f32::to_bits) != tape.model_weights.map(f32::to_bits) {
        return Err(invalid());
    }
    let m = kernel::Model {
        weights: model.weights,
    };
    let (parameters, _) = kernel::backward(&tape.steps, [final_adjoint[0], final_adjoint[1]]);
    let (initial_state, confidence_inputs) =
        kernel::full_state_backward(&m, &tape.steps, final_adjoint);
    let source_inputs =
        kernel::input_backward(&m, &tape.steps, [final_adjoint[0], final_adjoint[1]]);
    finite(&parameters)?;
    finite(&initial_state)?;
    for g in &source_inputs {
        finite(g)?
    }
    for g in &confidence_inputs {
        finite(g)?
    }
    Ok(PositionGradient2013 {
        parameters,
        initial_state,
        source_inputs,
        confidence_inputs,
    })
}

/// Neural-kernel work only; no collector history, quaternion preparation, decay
/// preparation, hashing, memory allocation, ATP debit or measured hardware FLOPs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionKernelCost2013 {
    pub dispatches: u64,
    pub state_reads: u64,
    pub input_parameter_reads: u64,
    pub scalar_arithmetic: u64,
    pub neuron_updates: u64,
}
impl PositionWork2013 {
    pub fn logical_work(&self) -> Result<PositionKernelCost2013, TrainingError> {
        if self.active_updates > self.transitions {
            return Err(invalid());
        }
        let mul = |v: u64, n: u64| v.checked_mul(n).ok_or_else(invalid);
        Ok(PositionKernelCost2013 {
            dispatches: self.transitions,
            state_reads: mul(self.transitions, 4)?,
            input_parameter_reads: mul(self.active_updates, 17)?,
            scalar_arithmetic: mul(self.active_updates, 43)?,
            neuron_updates: mul(self.active_updates, 4)?,
        })
    }
}
#[cfg(test)]
#[path = "position_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "position_reference.rs"]
mod reference;
