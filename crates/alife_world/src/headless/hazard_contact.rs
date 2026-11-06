//! Shared object-hazard contact under the world's existing f32 reach predicates.
//!
//! Damage is removed only when the full executed path is certified outside the
//! stored radius or obstructed. Uncertain arithmetic and exhausted subdivision
//! retain legacy damage. This is center-based contact, not material-volume CSG;
//! height-field reach and ecological area pressure remain outside this helper.
use super::{
    closest_point_on_segment, distance, scale, sight_box_hit, subtract, HeadlessWorld, Vec3f,
    WorldObject, WorldObjectKind, HEADLESS_CONTACT_RADIUS,
};

const MAX_EXPOSURE_SAMPLES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HazardExposure {
    None,
    Contact,
    Unresolved,
}
impl HazardExposure {
    pub(super) fn retains_damage(self) -> bool {
        self != Self::None
    }
}

#[derive(Clone, Copy)]
enum Solid<'a> {
    Sphere(&'a WorldObject),
    Box(&'a [f32; 6]),
}

/// These ranges enclose rounded f32 operation results, not real arithmetic.
/// Each operator evaluates the extrema of its monotone/sign-separated domains
/// using the same f32 operations as the point predicate. Invalid/ambiguous
/// branches are never used to certify absence of exposure.
#[derive(Clone, Copy, Debug)]
struct FloatInterval {
    lo: f32,
    hi: f32,
    ok: bool,
}
impl FloatInterval {
    fn new(lo: f32, hi: f32) -> Self {
        Self {
            lo,
            hi,
            ok: lo.is_finite() && hi.is_finite() && lo <= hi,
        }
    }
    fn at(x: f32) -> Self {
        Self::new(x, x)
    }
    fn bad() -> Self {
        Self {
            lo: 0.0,
            hi: 0.0,
            ok: false,
        }
    }
    fn add(self, b: Self) -> Self {
        if !self.ok || !b.ok {
            return Self::bad();
        }
        Self::new(self.lo + b.lo, self.hi + b.hi)
    }
    fn sub(self, b: Self) -> Self {
        if !self.ok || !b.ok {
            return Self::bad();
        }
        Self::new(self.lo - b.hi, self.hi - b.lo)
    }
    fn mul(self, b: Self) -> Self {
        if !self.ok || !b.ok {
            return Self::bad();
        }
        Self::corners([
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ])
    }
    fn div(self, b: Self) -> Self {
        if !self.ok || !b.ok || b.lo <= 0.0 && b.hi >= 0.0 {
            return Self::bad();
        }
        Self::corners([
            self.lo / b.lo,
            self.lo / b.hi,
            self.hi / b.lo,
            self.hi / b.hi,
        ])
    }
    fn corners(q: [f32; 4]) -> Self {
        if q.iter().any(|v| v.is_nan()) {
            return Self::bad();
        }
        Self::new(
            q.into_iter().fold(f32::INFINITY, f32::min),
            q.into_iter().fold(f32::NEG_INFINITY, f32::max),
        )
    }
    fn sq(self) -> Self {
        if !self.ok {
            return Self::bad();
        }
        let hi = (self.lo * self.lo).max(self.hi * self.hi);
        let lo = if self.lo <= 0.0 && self.hi >= 0.0 {
            0.0
        } else {
            (self.lo * self.lo).min(self.hi * self.hi)
        };
        Self::new(lo, hi)
    }
    fn sqrt(self) -> Self {
        if !self.ok || self.lo < 0.0 {
            return Self::bad();
        }
        Self::new(self.lo.sqrt(), self.hi.sqrt())
    }
    fn min(self, b: Self) -> Self {
        if !self.ok || !b.ok {
            return Self::bad();
        }
        Self::new(self.lo.min(b.lo), self.hi.min(b.hi))
    }
    fn max(self, b: Self) -> Self {
        if !self.ok || !b.ok {
            return Self::bad();
        }
        Self::new(self.lo.max(b.lo), self.hi.max(b.hi))
    }
    fn clamp01(self) -> Self {
        if !self.ok {
            return Self::bad();
        }
        Self::new(self.lo.clamp(0.0, 1.0), self.hi.clamp(0.0, 1.0))
    }
}
type CoordinateIntervals = [FloatInterval; 3];
fn coordinates(p: Vec3f) -> [f32; 3] {
    [p.x, p.y, p.z]
}
fn point_intervals(p: [f32; 3]) -> CoordinateIntervals {
    p.map(FloatInterval::at)
}
fn distance_interval(a: CoordinateIntervals, b: CoordinateIntervals) -> FloatInterval {
    let d: [FloatInterval; 3] = std::array::from_fn(|i| a[i].sub(b[i]));
    d[0].sq().add(d[1].sq()).add(d[2].sq()).sqrt()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Occlusion {
    Blocked,
    Clear,
    Unresolved,
}
struct RayIntervals {
    p: CoordinateIntervals,
    d: CoordinateIntervals,
    n: FloatInterval,
    len: FloatInterval,
    dir: CoordinateIntervals,
}
impl RayIntervals {
    fn new(p: CoordinateIntervals, h: [f32; 3]) -> Self {
        let d: CoordinateIntervals = std::array::from_fn(|i| FloatInterval::at(h[i]).sub(p[i]));
        let n = d[0].sq().add(d[1].sq()).add(d[2].sq());
        let len = n.sqrt();
        let inv = FloatInterval::at(1.0).div(len);
        let dir = d.map(|x| x.mul(inv));
        Self { p, d, n, len, dir }
    }
    fn relation(&self, s: Solid<'_>) -> Occlusion {
        match s {
            Solid::Sphere(object) => {
                let c = point_intervals(coordinates(object.position));
                let closest = if self.n.ok && self.n.hi == 0.0 {
                    self.p
                } else if self.n.ok && self.n.lo > 0.0 {
                    let q: CoordinateIntervals = std::array::from_fn(|i| c[i].sub(self.p[i]));
                    let dot = q[0]
                        .mul(self.d[0])
                        .add(q[1].mul(self.d[1]))
                        .add(q[2].mul(self.d[2]));
                    let t = dot.div(self.n).clamp01();
                    std::array::from_fn(|i| self.p[i].add(self.d[i].mul(t)))
                } else {
                    return Occlusion::Unresolved;
                };
                let distance = distance_interval(c, closest);
                let r = object.radius.max(HEADLESS_CONTACT_RADIUS);
                if !distance.ok {
                    Occlusion::Unresolved
                } else if distance.hi <= r {
                    Occlusion::Blocked
                } else if distance.lo > r {
                    Occlusion::Clear
                } else {
                    Occlusion::Unresolved
                }
            }
            Solid::Box(bounds) => {
                let lo = [bounds[0], bounds[4], bounds[1]];
                let hi = [bounds[2], bounds[5], bounds[3]];
                if !self.len.ok {
                    return Occlusion::Unresolved;
                }
                if self.len.hi <= f32::EPSILON {
                    if (0..3)
                        .any(|i| self.p[i].ok && (self.p[i].hi < lo[i] || self.p[i].lo > hi[i]))
                    {
                        return Occlusion::Clear;
                    }
                    return if (0..3)
                        .all(|i| self.p[i].ok && self.p[i].lo >= lo[i] && self.p[i].hi <= hi[i])
                    {
                        Occlusion::Blocked
                    } else {
                        Occlusion::Unresolved
                    };
                }
                if self.len.lo <= f32::EPSILON {
                    return Occlusion::Unresolved;
                }
                let mut slabs = [None; 3];
                let mut uncertain = false;
                for i in 0..3 {
                    let p = self.p[i];
                    let velocity = self.dir[i];
                    if !p.ok || !velocity.ok {
                        uncertain = true;
                        continue;
                    }
                    let speed = velocity.lo.abs().max(velocity.hi.abs());
                    if speed <= f32::EPSILON {
                        if p.hi < lo[i] || p.lo > hi[i] {
                            return Occlusion::Clear;
                        }
                        if p.lo < lo[i] || p.hi > hi[i] {
                            uncertain = true;
                        }
                        continue;
                    }
                    // This axis is neutral even if direction changes sign or takes
                    // the parallel branch: near<=0 and far>=the maximum ray range.
                    let margin = p
                        .sub(FloatInterval::at(lo[i]))
                        .min(FloatInterval::at(hi[i]).sub(p));
                    if margin.ok && margin.lo >= 0.0 && margin.lo / speed >= self.len.hi {
                        continue;
                    }
                    if !(velocity.lo > f32::EPSILON || velocity.hi < -f32::EPSILON) {
                        uncertain = true;
                        continue;
                    }
                    let x = FloatInterval::at(lo[i]).sub(p).div(velocity);
                    let y = FloatInterval::at(hi[i]).sub(p).div(velocity);
                    let near = x.min(y);
                    let far = x.max(y);
                    if !near.ok || !far.ok {
                        uncertain = true;
                        continue;
                    }
                    if near.lo > self.len.hi || far.hi < 0.0 {
                        return Occlusion::Clear;
                    }
                    slabs[i] = Some((near, far));
                }
                for (i, a) in slabs.iter().enumerate() {
                    if let Some((near, _)) = a {
                        for (j, b) in slabs.iter().enumerate() {
                            if i != j {
                                if let Some((_, far)) = b {
                                    if near.lo > far.hi {
                                        return Occlusion::Clear;
                                    }
                                }
                            }
                        }
                    }
                }
                if uncertain {
                    return Occlusion::Unresolved;
                }
                for (i, a) in slabs.iter().enumerate() {
                    if let Some((near, far)) = a {
                        if near.hi > self.len.lo || far.lo < 0.0 {
                            return Occlusion::Unresolved;
                        }
                        // min(near,far)<=max(near,far) on the SAME axis by construction;
                        // comparing unrelated interval extrema there loses correlation.
                        for (j, b) in slabs.iter().enumerate() {
                            if i != j {
                                if let Some((_, other_far)) = b {
                                    if near.hi > other_far.lo {
                                        return Occlusion::Unresolved;
                                    }
                                }
                            }
                        }
                    }
                }
                Occlusion::Blocked
            }
        }
    }
}

pub(super) struct HazardContacts<'a> {
    world: &'a HeadlessWorld,
    objects: Vec<&'a WorldObject>,
}
impl<'a> HazardContacts<'a> {
    pub(super) fn new(world: &'a HeadlessWorld) -> Self {
        Self {
            world,
            objects: world
                .objects
                .values()
                .filter(|o| o.kind == WorldObjectKind::Obstacle && !o.consumed)
                .collect(),
        }
    }
    fn solids(&self) -> impl Iterator<Item = Solid<'_>> {
        self.objects.iter().copied().map(Solid::Sphere).chain(
            self.world
                .terrain
                .iter()
                .flat_map(|t| t.surface().obstacles.iter())
                .map(Solid::Box),
        )
    }
    fn point_clear(&self, point: Vec3f, source: Vec3f) -> bool {
        self.world
            .solid_segment_clear_from(point, source, None, self.objects.iter().copied())
    }
    pub(super) fn point_exposed(&self, point: Vec3f, hazard: &WorldObject) -> bool {
        distance(point, hazard.position) <= hazard.radius
            && self.point_clear(point, hazard.position)
    }
    pub(super) fn swept(&self, start: Vec3f, end: Vec3f, hazard: &WorldObject) -> HazardExposure {
        let source = hazard.position;
        let closest = closest_point_on_segment(source, start, end);
        // The caller has already admitted contact using these same predicates.
        // Keep an explicit positive gate so NaNs cannot become new admissions.
        if !(distance(end, source) <= hazard.radius || distance(closest, source) <= hazard.radius) {
            return HazardExposure::None;
        }
        if self.solids().next().is_none() {
            return HazardExposure::Contact;
        }
        if start == end {
            return if self.point_exposed(start, hazard) {
                HazardExposure::Contact
            } else {
                HazardExposure::None
            };
        }
        if distance(closest, source) <= hazard.radius && self.point_clear(closest, source) {
            return HazardExposure::Contact;
        }
        let curve = Curve {
            start,
            delta: subtract(end, start),
        };
        let n = curve.delta.x * curve.delta.x
            + curve.delta.y * curve.delta.y
            + curve.delta.z * curve.delta.z;
        let from_start = subtract(source, start);
        let parameter = if n == 0.0 {
            0.0
        } else {
            ((from_start.x * curve.delta.x
                + from_start.y * curve.delta.y
                + from_start.z * curve.delta.z)
                / n)
                .clamp(0.0, 1.0)
        };
        if !parameter.is_finite()
            || !coordinates(curve.at(1.0)).into_iter().all(f32::is_finite)
            || curve.at(1.0) != end
            || curve.at(parameter) != closest
        {
            return HazardExposure::Unresolved;
        }
        let full = RayIntervals::new(curve.bounds(0.0, 1.0), coordinates(source));
        let mut active = Vec::new();
        // Complete blockers include objects outside perception queries. Cull
        // only a certified f32 miss; collect the remainder once per hazard.
        for solid in self.solids() {
            match full.relation(solid) {
                Occlusion::Blocked => return HazardExposure::None,
                Occlusion::Clear => {}
                Occlusion::Unresolved => active.push(solid),
            }
        }
        let mut samples = 1;
        for parameter in [0.0, 1.0] {
            samples += 1;
            let point = curve.at(parameter);
            if distance(point, source) <= hazard.radius && clear_from_solids(point, source, &active)
            {
                return HazardExposure::Contact;
            }
        }
        let mut stack = vec![(0.0_f32, 1.0_f32)];
        while let Some((lo, hi)) = stack.pop() {
            let points = curve.bounds(lo, hi);
            let range = distance_interval(points, point_intervals(coordinates(source)));
            if range.ok && range.lo > hazard.radius {
                continue;
            }
            let ray = RayIntervals::new(points, coordinates(source));
            if active
                .iter()
                .any(|s| ray.relation(*s) == Occlusion::Blocked)
            {
                continue;
            }
            let middle = lo + (hi - lo) * 0.5;
            if samples >= MAX_EXPOSURE_SAMPLES || middle == lo || middle == hi {
                return HazardExposure::Unresolved;
            }
            samples += 1;
            let point = curve.at(middle);
            if distance(point, source) <= hazard.radius && clear_from_solids(point, source, &active)
            {
                return HazardExposure::Contact;
            }
            stack.push((middle, hi));
            stack.push((lo, middle));
        }
        HazardExposure::None
    }
}
struct Curve {
    start: Vec3f,
    delta: Vec3f,
}
impl Curve {
    fn at(&self, t: f32) -> Vec3f {
        Vec3f::new(
            self.start.x + self.delta.x * t,
            self.start.y + self.delta.y * t,
            self.start.z + self.delta.z * t,
        )
    }
    fn bounds(&self, lo: f32, hi: f32) -> CoordinateIntervals {
        let a = coordinates(self.at(lo));
        let b = coordinates(self.at(hi));
        std::array::from_fn(|i| FloatInterval::new(a[i].min(b[i]), a[i].max(b[i])))
    }
}
fn clear_from_solids(point: Vec3f, source: Vec3f, solids: &[Solid<'_>]) -> bool {
    let length = distance(point, source);
    let (direction, range) = if length <= f32::EPSILON {
        (Vec3f::ZERO, 0.0)
    } else {
        (scale(subtract(source, point), 1.0 / length), length)
    };
    !solids.iter().any(|s| match s {
        Solid::Sphere(object) => object.blocks_segment(point, source),
        Solid::Box(bounds) => sight_box_hit(point, direction, **bounds, range).is_some(),
    })
}

#[cfg(test)]
mod tests;
