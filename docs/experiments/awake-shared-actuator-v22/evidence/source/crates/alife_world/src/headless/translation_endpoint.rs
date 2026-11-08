//! Common bounded representation correction, not reflex intervention.
use super::*;

impl HeadlessWorld {
    /// Preserve the planned duration and original speed cap. Only an endpoint
    /// exceeding the existing numeric guard is contracted; no route is invented.
    pub(super) fn bounded_translation_endpoint(
        &self,
        start: Vec3f,
        end: Vec3f,
        speed: f32,
        duration: f32,
    ) -> Option<Vec3f> {
        if !speed.is_finite()
            || speed < 0.0
            || !duration.is_finite()
            || !(0.0..=1.0).contains(&duration)
        {
            return None;
        }
        let length = |p| {
            let d = subtract(p, start);
            if self.terrain.is_some() {
                d.x.hypot(d.z)
            } else {
                distance(Vec3f::ZERO, d)
            }
        };
        let allowance = speed * duration;
        // MOVE_STEP is <=1: this is exactly the existing budget tolerance.
        if length(end) <= allowance + 1e-6 {
            return Some(end);
        }
        let mut candidate = end;
        for _ in 0..8 {
            fn toward(value: f32, origin: f32) -> f32 {
                if value > origin {
                    value.next_down().max(origin)
                } else if value < origin {
                    value.next_up().min(origin)
                } else {
                    value
                }
            }
            candidate.x = toward(candidate.x, start.x);
            candidate.z = toward(candidate.z, start.z);
            if self.terrain.is_none() {
                candidate.y = toward(candidate.y, start.y);
            }
            if let Some(terrain) = &self.terrain {
                candidate = terrain.resolve_move(start, candidate)?;
            }
            if length(candidate) <= allowance {
                return Some(candidate);
            }
        }
        None
    }
}
