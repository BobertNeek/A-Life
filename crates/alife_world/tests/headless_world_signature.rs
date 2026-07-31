use alife_core::{OrganismId, Vec3f};
use alife_world::HeadlessScenarioBuilder;

#[test]
fn canonical_signature_distinguishes_same_seed_wrong_and_later_worlds() {
    let original = HeadlessScenarioBuilder::new(44_001)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .food("resource", Vec3f::new(2.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let exact_clone = original.clone();
    let wrong_world = HeadlessScenarioBuilder::new(44_001)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .food("resource", Vec3f::new(3.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let mut later_world = original.clone();
    later_world.advance_tick();

    assert_eq!(
        original.canonical_signature_digest().unwrap(),
        exact_clone.canonical_signature_digest().unwrap()
    );
    assert_ne!(
        original.canonical_signature_digest().unwrap(),
        wrong_world.canonical_signature_digest().unwrap()
    );
    assert_ne!(
        original.canonical_signature_digest().unwrap(),
        later_world.canonical_signature_digest().unwrap()
    );
}

#[test]
fn canonical_signature_distinguishes_one_bit_radius_change_at_contact_threshold() {
    let contact_radius = 0.75_f32;
    let one_bit_larger = f32::from_bits(contact_radius.to_bits() + 1);
    let at_threshold = HeadlessScenarioBuilder::new(44_002)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .obstacle(
            "contact-boundary",
            Vec3f::new(0.75, 0.0, 0.0),
            contact_radius,
        )
        .build()
        .unwrap();
    let outside_threshold = HeadlessScenarioBuilder::new(44_002)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .obstacle(
            "contact-boundary",
            Vec3f::new(0.75, 0.0, 0.0),
            one_bit_larger,
        )
        .build()
        .unwrap();

    assert_eq!(at_threshold.seed(), outside_threshold.seed());
    assert_eq!(at_threshold.tick(), outside_threshold.tick());
    assert_ne!(
        at_threshold.canonical_signature_digest().unwrap(),
        outside_threshold.canonical_signature_digest().unwrap()
    );
}
