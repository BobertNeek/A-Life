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
