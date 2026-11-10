# Actual-contact local controlled test

On 2026-11-10 I reconstructed the Phase B source from accepted main c3aec16, the recorded reconstruction patches, the Phase A overlay and the Phase B source checkpoint 414585ab765bee2d51fd33d5b5d602d3ed781dd9. All 830 source entries matched the Phase B inventory SHA-256 c42372ed74383cb14a8ac23d3cfcf9e4967fc37cddd88a16640b84e3ed4c0f72 before test-only changes.

The accompanying test-permit.patch registers the existing four-interval fixture and permits dispatch only for its private Controlled test strategy. The production, non-test build remains fail-closed. No trained checkpoint or learned state was used or changed.

Rust 1.99.1: offline cargo check passed for alife_training with cpu-backend. The first test build failed because I registered the fixture at the wrong module level; I corrected the registration and the test build passed. One exact CPU host test then passed: prefix Play, warm repeat Play, cold repeat Play and matched cold NoManip. First contact registered the toy; the repeated Play used reduced novelty and produced stimulation, reducing the small accumulated boredom to zero. The matched no-manipulation continuation had no stimulation and boredom rose. Warm and cold repeat outputs matched. The fixture also exercised copied-receipt rejection and atomicity, with unobserved fields explicitly recorded as such.

This was a controlled, scripted action sequence, not an autonomous choice, learned competence, GPU hardware test or long-life result. It combines contact familiarity with the existing channel-native/recovery host but does not establish all-skill non-interference. Full compute and biological pricing remain unqualified. The test-only permit has not been merged into production.

Source reference: ../contact-phase-b-source-v2/README.md. Test command: cargo test --offline --locked -p alife_training --features cpu-backend --lib actual_contact_fixed_four_intervals_one_prefix_play_warm_cold_nomani_v1 -- --exact --nocapture, using the full module-qualified name shown in the test list. One test passed; 254 filtered out.
