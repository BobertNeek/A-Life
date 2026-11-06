//! Short-lived broad phase for repeated queries from one mating participant.
//!
//! This only removes impossible candidates. The existing sphere/box predicates
//! still decide contact, including tangency and zero-length segments.

use super::*;
use std::cell::OnceCell;

pub(super) struct MatingReachCandidates<'a> {
    center: Vec3f,
    objects: Vec<&'a WorldObject>,
    boxes: Vec<&'a [f32; 6]>,
}

#[cfg(test)]
impl MatingReachCandidates<'_> {
    pub(super) fn candidate_counts(&self) -> (usize, usize) {
        (self.objects.len(), self.boxes.len())
    }
}

impl HeadlessWorld {
    pub(super) fn mating_segment_clear<'a>(
        &'a self,
        center: Vec3f,
        start: Vec3f,
        end: Vec3f,
        ignored: Option<WorldEntityId>,
        candidates: &OnceCell<Option<MatingReachCandidates<'a>>>,
    ) -> bool {
        let Some(candidates) = candidates.get_or_init(|| self.mating_reach_candidates(center))
        else {
            return self.solid_segment_clear_from(start, end, ignored, self.objects.values());
        };
        // The normal caller has already checked a one-unit mating distance.
        // Verify the broader cube too, so this optimization cannot silently
        // become authoritative for a later caller with a different range.
        let local = [start, end].into_iter().all(|point| {
            [
                (point.x, candidates.center.x),
                (point.y, candidates.center.y),
                (point.z, candidates.center.z),
            ]
            .into_iter()
            .all(|(value, center)| {
                value.is_finite() && (f64::from(value) - f64::from(center)).abs() <= 1.01
            })
        });
        if !local {
            return self.solid_segment_clear_from(start, end, ignored, self.objects.values());
        }
        solid_segment_clear_among(
            start,
            end,
            ignored,
            candidates.objects.iter().copied(),
            candidates.boxes.iter().copied(),
        )
    }

    fn mating_reach_candidates(&self, center: Vec3f) -> Option<MatingReachCandidates<'_>> {
        if [center.x, center.y, center.z]
            .into_iter()
            .any(|coordinate| !coordinate.is_finite() || coordinate.abs() > 4096.0)
        {
            return None;
        }
        let objects = self
            .objects
            .values()
            .filter(|object| object.kind == WorldObjectKind::Obstacle && !object.consumed)
            .filter(|object| {
                let radius = object.radius.max(HEADLESS_CONTACT_RADIUS);
                let coordinates = [object.position.x, object.position.y, object.position.z];
                // Unbounded/imported values keep the exact full predicate.
                if !radius.is_finite()
                    || radius > 65536.0
                    || coordinates
                        .iter()
                        .any(|value| !value.is_finite() || value.abs() > 65536.0)
                {
                    return true;
                }
                // Endpoints are inside center +/- 1.01. A clamped projection
                // stays in their cube; with coordinates <= 4097.01 its f32
                // rounding is < 0.002. For these bounded spheres, subtraction,
                // three squares, two sums and sqrt add < 0.1 of distance error
                // (differences < 70000, no overflow). Radius + 2 therefore has
                // > 0.88 spare clearance. f64 broad-phase arithmetic cannot
                // erase that margin. This is candidate padding, not reach.
                coordinates
                    .into_iter()
                    .zip([center.x, center.y, center.z])
                    .all(|(value, center)| {
                        (f64::from(value) - f64::from(center)).abs() <= f64::from(radius) + 2.0
                    })
            })
            .collect();
        let mut boxes = Vec::new();
        if let Some(terrain) = &self.terrain {
            let surface = terrain.surface();
            // The immutable index covers each box's complete X/Z footprint
            // and is rebuilt on terrain restore. Under the local endpoint
            // bounds above, the normalized-ray/slab roundoff is < 0.01. The
            // +/- 2 cube plus a full 10-unit cell halo also covers f32 cell
            // division rounding, including a box face on a cell boundary.
            let cell = f64::from(crate::terrain_surface::CELL);
            let min_x = ((f64::from(center.x) - 2.0) / cell).floor() as i32 - 1;
            let max_x = ((f64::from(center.x) + 2.0) / cell).floor() as i32 + 1;
            let min_z = ((f64::from(center.z) - 2.0) / cell).floor() as i32 - 1;
            let max_z = ((f64::from(center.z) + 2.0) / cell).floor() as i32 + 1;
            let mut indices = Vec::new();
            let mut full_scan = false;
            'cells: for z in min_z..=max_z {
                for x in min_x..=max_x {
                    if let Some(rows) = surface.cells.get(&(x, z)) {
                        // Dense or duplicated rows can make gathering costlier
                        // than retaining all boxes. This is only a fallback.
                        if rows.len() > surface.obstacles.len().saturating_sub(indices.len()) {
                            full_scan = true;
                            break 'cells;
                        }
                        indices.extend_from_slice(rows);
                    }
                }
            }
            if full_scan {
                boxes.extend(surface.obstacles.iter());
            } else {
                indices.sort_unstable();
                indices.dedup();
                boxes.extend(indices.into_iter().map(|index| &surface.obstacles[index]));
            }
        }
        Some(MatingReachCandidates {
            center,
            objects,
            boxes,
        })
    }
}
