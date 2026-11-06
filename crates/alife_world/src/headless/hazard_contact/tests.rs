use super::*;
use crate::HeadlessScenarioBuilder;
use alife_core::OrganismId;

fn sample(seed: &mut u64) -> f32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*seed >> 32) as u32) as f32 / u32::MAX as f32
}
fn point(seed: &mut u64) -> Vec3f {
    Vec3f::new(
        sample(seed) * 8.0 - 4.0,
        sample(seed),
        sample(seed) * 8.0 - 4.0,
    )
}

#[test]
fn interval_certificates_enclose_existing_f32_point_predicates() {
    let world = HeadlessScenarioBuilder::new(4260)
        .obstacle("solid", Vec3f::ZERO, 0.75)
        .build()
        .unwrap();
    let template = world.entity(world.entity_id("solid").unwrap()).unwrap();
    let mut seed = 716052;
    for index in 0..1600 {
        let start = point(&mut seed);
        let end = Vec3f::new(
            start.x + (sample(&mut seed) - 0.5) * 0.2,
            start.y,
            start.z + (sample(&mut seed) - 0.5) * 0.2,
        );
        let source = point(&mut seed);
        let center = point(&mut seed);
        let mut object = template.clone();
        object.position = center;
        object.radius = 0.75 + sample(&mut seed);
        let bounds = [
            center.x - 0.5,
            center.z - 0.5,
            center.x + 0.5,
            center.z + 0.5,
            center.y - 0.5,
            center.y + 0.5,
        ];
        let solid = if index % 2 == 0 {
            Solid::Sphere(&object)
        } else {
            Solid::Box(&bounds)
        };
        let curve = Curve {
            start,
            delta: subtract(end, start),
        };
        let range = distance_interval(curve.bounds(0.0, 1.0), point_intervals(coordinates(source)));
        let relation =
            RayIntervals::new(curve.bounds(0.0, 1.0), coordinates(source)).relation(solid);
        for step in 0..=256 {
            let p = curve.at(step as f32 / 256.0);
            let clear = clear_from_solids(p, source, &[solid]);
            let measured = distance(p, source);
            assert!(range.ok && measured >= range.lo && measured <= range.hi);
            match relation {
                Occlusion::Blocked => assert!(!clear, "false blocked certificate {index}/{step}"),
                Occlusion::Clear => assert!(clear, "false clear certificate {index}/{step}"),
                Occlusion::Unresolved => {}
            }
            let exact = RayIntervals::new(point_intervals(coordinates(p)), coordinates(source))
                .relation(solid);
            assert_eq!(
                exact,
                if clear {
                    Occlusion::Clear
                } else {
                    Occlusion::Blocked
                }
            );
        }
    }
}

#[test]
fn rounded_radius_admission_and_uncertain_blocker_keep_legacy_damage() {
    let world = HeadlessScenarioBuilder::new(4260)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .hazard("thorn", Vec3f::new(0.05, 0.45, 0.6), 0.6)
        .build()
        .unwrap();
    let hazard = world.entity(world.entity_id("thorn").unwrap()).unwrap();
    let perpendicular = f64::from(hazard.position.y).powi(2) + f64::from(hazard.position.z).powi(2);
    assert!(perpendicular > f64::from(hazard.radius).powi(2));
    assert_eq!(
        HazardContacts::new(&world).swept(Vec3f::ZERO, Vec3f::new(0.1, 0.0, 0.0), hazard),
        HazardExposure::Contact
    );

    let world = HeadlessScenarioBuilder::new(4260)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .hazard("thorn", Vec3f::new(0.75, 0.0, 0.0), 0.6)
        .obstacle("grazing", Vec3f::new(0.375, 0.45, 0.6), 0.75)
        .build()
        .unwrap();
    let hazard = world.entity(world.entity_id("thorn").unwrap()).unwrap();
    let contacts = HazardContacts::new(&world);
    assert!(!contacts.point_exposed(Vec3f::ZERO, hazard));
    let exposure = contacts.swept(Vec3f::ZERO, Vec3f::new(0.1, 0.0, 0.0), hazard);
    assert_eq!(exposure, HazardExposure::Unresolved);
    assert!(exposure.retains_damage());
    assert_eq!(
        contacts.swept(Vec3f::ZERO, Vec3f::ZERO, hazard),
        HazardExposure::None
    );
}

#[test]
fn parallel_thresholds_zero_rays_and_unrepresentable_paths_are_conservative() {
    // Corner products alone can miss an interior infinity-times-zero NaN.
    // Such an overflowed projection must never turn into a finite certificate.
    let projection = FloatInterval::at(f32::MAX)
        .mul(FloatInterval::new(-2.0, 2.0))
        .mul(FloatInterval::new(-1.0, 1.0))
        .clamp01();
    assert!(!projection.ok);
    let epsilon = f32::EPSILON;
    let bounds = [0.6, epsilon * 0.5, 0.65, epsilon * 1.5, -1.0, 1.0];
    for z in [epsilon, f32::from_bits(epsilon.to_bits() + 1)] {
        let source = Vec3f::new(1.0, 0.0, z);
        let solid = Solid::Box(&bounds);
        let clear = clear_from_solids(Vec3f::ZERO, source, &[solid]);
        assert_eq!(clear, z == epsilon);
        assert_eq!(
            RayIntervals::new(point_intervals([0.0; 3]), coordinates(source)).relation(solid),
            if clear {
                Occlusion::Clear
            } else {
                Occlusion::Blocked
            }
        );
    }
    let bounds = [-0.1, -0.1, 0.1, 0.1, -0.1, 0.1];
    assert_eq!(
        RayIntervals::new(point_intervals([0.0; 3]), [0.0; 3]).relation(Solid::Box(&bounds)),
        Occlusion::Blocked
    );
    let world = HeadlessScenarioBuilder::new(4260)
        .hazard("thorn", Vec3f::new(1.0, 0.0, 0.0), 0.6)
        .obstacle("solid", Vec3f::ZERO, 0.75)
        .build()
        .unwrap();
    let hazard = world.entity(world.entity_id("thorn").unwrap()).unwrap();
    assert!(HazardContacts::new(&world)
        .swept(Vec3f::new(f32::MAX, 0.0, 0.0), hazard.position, hazard)
        .retains_damage());
}
