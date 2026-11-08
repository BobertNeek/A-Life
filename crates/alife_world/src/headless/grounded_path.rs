//! Ground-anchored hazard trajectory shared by ordinary motion and the opt-in actuator.
use super::*;

impl HeadlessWorld {
    pub(super) fn grounded_hazard_contacts(
        &self,
        organism: OrganismId,
        start: Vec3f,
        end: Vec3f,
    ) -> Result<Option<Vec<WorldEntityId>>, ScaffoldContractError> {
        let Some(terrain) = &self.terrain else {
            return Ok(None);
        };
        if terrain.surface().height(start.x, start.z) != Some(start.y) {
            return Ok(None); // Exact legacy chord for non-grounded old/editor poses.
        }
        let segments = terrain
            .grounded_move_segments(start, end)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if segments.iter().all(|segment| {
            segment.start[1] == f64::from(start.y) && segment.end[1] == f64::from(start.y)
        }) {
            return Ok(None); // Flat route is the exact legacy chord.
        }
        let contacts = self
            .objects
            .values()
            .filter(|object| {
                if object.consumed || object.kind != WorldObjectKind::Hazard {
                    return false;
                }
                if object.carried_by == Some(organism) {
                    // A self-carried source translates with the body, not along an
                    // invented static-world sweep. It is never a reflex escape source.
                    return distance(object.position, start) <= object.radius;
                }
                distance(object.position, end) <= object.radius
                    || segments.iter().any(|segment| {
                        grounded_segment_sphere_entry(
                            segment.start,
                            segment.end,
                            object.position,
                            object.radius,
                        )
                        .is_some()
                    })
            })
            .map(|object| object.id)
            .collect();
        Ok(Some(contacts))
    }
}

/// Closed sphere entry along one true affine ground triangle segment.
pub(super) fn grounded_segment_sphere_entry(
    start: [f64; 3],
    end: [f64; 3],
    center: Vec3f,
    radius: f32,
) -> Option<f64> {
    let center = [
        f64::from(center.x),
        f64::from(center.y),
        f64::from(center.z),
    ];
    let delta = std::array::from_fn::<_, 3, _>(|i| end[i] - start[i]);
    let offset = std::array::from_fn::<_, 3, _>(|i| center[i] - start[i]);
    let radius_squared = f64::from(radius).powi(2);
    if offset.iter().map(|v| v * v).sum::<f64>() <= radius_squared {
        return Some(0.0);
    }
    let length_squared = delta.iter().map(|v| v * v).sum::<f64>();
    if length_squared == 0.0 {
        return None;
    }
    let middle = offset.iter().zip(delta).map(|(o, d)| o * d).sum::<f64>() / length_squared;
    let squared = |t: f64| {
        offset
            .iter()
            .zip(delta)
            .map(|(o, d)| (o - t * d).powi(2))
            .sum::<f64>()
    };
    if squared(middle.clamp(0.0, 1.0)) > radius_squared {
        return None;
    }
    let half = ((radius_squared - squared(middle)).max(0.0) / length_squared).sqrt();
    Some((middle - half).clamp(0.0, 1.0))
}
