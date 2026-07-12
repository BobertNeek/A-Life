//! Bounded display-only animation for the shared water material.

use bevy::{
    math::{Affine2, Vec2},
    prelude::{App, Assets, Color, Handle, Res, ResMut, Resource, StandardMaterial, Time, Update},
};

#[derive(Debug, Clone, Resource)]
pub(crate) struct Fvr11AnimatedWaterMaterial {
    pub handle: Handle<StandardMaterial>,
    pub phase: f32,
}

#[derive(Debug, Clone, Copy)]
struct WaterMotion {
    uv_translation: Vec2,
    tint_scale: f32,
}

pub(crate) fn install_animated_water_material(app: &mut App, handle: Handle<StandardMaterial>) {
    app.insert_resource(Fvr11AnimatedWaterMaterial { handle, phase: 0.0 });
    app.add_systems(Update, animate_water_material);
}

fn animate_water_material(
    time: Res<Time>,
    mut water: ResMut<Fvr11AnimatedWaterMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    water.phase = (water.phase + time.delta_secs() * 0.32) % std::f32::consts::TAU;
    let motion = water_motion_at_phase(water.phase);
    if let Some(material) = materials.get_mut(&water.handle) {
        material.uv_transform = Affine2::from_translation(motion.uv_translation);
        material.base_color = Color::srgba(
            (0.94 * motion.tint_scale).clamp(0.0, 1.0),
            (0.99 * motion.tint_scale).clamp(0.0, 1.0),
            (1.00 * motion.tint_scale).clamp(0.0, 1.0),
            0.78,
        );
    }
}

fn water_motion_at_phase(phase: f32) -> WaterMotion {
    WaterMotion {
        uv_translation: Vec2::new(phase.sin() * 0.002, (phase * 2.0 + 0.7).sin() * 0.002),
        tint_scale: 1.0 + (phase + 1.3).sin() * 0.02,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_motion_and_tint_pulse_are_small_and_periodic() {
        let start = water_motion_at_phase(0.0);
        let cycle = water_motion_at_phase(std::f32::consts::TAU);
        assert!((start.uv_translation.x - cycle.uv_translation.x).abs() < 0.000_01);
        assert!((start.uv_translation.y - cycle.uv_translation.y).abs() < 0.000_01);
        assert!((start.tint_scale - cycle.tint_scale).abs() < 0.000_01);
        for index in 0..64 {
            let sample = water_motion_at_phase(index as f32 * 0.1);
            assert!(sample.uv_translation.x.abs() <= 0.002_1);
            assert!(sample.uv_translation.y.abs() <= 0.002_1);
            assert!((0.98..=1.02).contains(&sample.tint_scale));
        }
    }
}
