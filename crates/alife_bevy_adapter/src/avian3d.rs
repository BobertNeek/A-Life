//! v0 scaffold: optional Avian 3D bridge for adapter movement plans.

use ::avian3d::prelude::LinearVelocity;

use crate::BevyActionPlan;

pub fn plan_to_linear_velocity(plan: &BevyActionPlan, ticks_per_second: f32) -> LinearVelocity {
    let scale = if ticks_per_second.is_finite() && ticks_per_second >= 0.0 {
        ticks_per_second
    } else {
        0.0
    };
    LinearVelocity(plan.displacement * scale)
}
