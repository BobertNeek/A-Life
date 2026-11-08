# World-only rest versus realized motion correction

Base:2da9dd323ad6f074556a851174a2edbc9e34e216. World-only local commit:d89f05149f70a31dd22f4af16c85a207b268954b. Distinct factor from waking-recovery-candidate-v1 and from the optional contact-withdrawal reflex. This source copy does not change chemistry genes/defaults, neural wiring, loaded saves, weights, or optimizer state.

## Defect and correction boundary

A factorized motor bundle can request both locomotion and Rest. The old world transaction executed both channels, summed sleep_recovery=1 from Rest, and advanced biology once even when real displacement occurred. BodyState::apply_event maps positive recovery to sleeping. Thus an organism physically moving during the interval could simultaneously receive full fatigue recovery and sleeping metabolic regulation.

The minimum world rule is physical, not cognitive: actual organism displacement in any part of the completed interval excludes physiological sleep/recovery in that interval. Merely requesting motion is insufficient. A completely blocked attempt or NO_LOCOMOTION with successful stationary Rest can still recover. Explicit movement effort remains charged exactly as before.

The helper normalize_sleep_recovery_for_motion receives the completed joint body event and the individual actual displacement vectors. If any vector is nonzero, only sleep_recovery becomes0. Every other body-event field is preserved. It is called after all channel/ambient/care events are combined and before the one biology advance. It does not choose or rewrite neural actions and does not add costs, doses, advances, or schema fields.

Individual vectors are essential: outward voluntary motion followed by an opposite withdrawal still constitutes real work even when the final net displacement iszero. The reflex worker will chain its actual ContactWithdrawalReceipt.displacement with the voluntary vectors at the same post-reflex/prebiology boundary. This clean-base factor contains no reflex implementation.

## Validation status

Prerepair native run:3 focused tests,2 passed and1 failed exactly at actual movement+Rest sleep_recovery=1 versus expected0 (carry=false). Log: ../rest-motion-prerepair-tests.log. Separate target motor-reflex-target-v1 is reserved with the reflex worker; shared training target is not used. Patched focused run:4/4 passed (../rest-motion-repaired-tests.log). Full alife_world suite PASSED:585 passed,0 failed,3 ignored. Log: ../rest-motion-full-world-tests.log. Existing persistence/save-load suites are included; no ignored test is claimed passed. rustfmt --check, git diff --check, and patch application to clean base passed.

Passed world tests cover movement+Rest with and without carried food, movement-only energy comparison, standalone and blocked-motion Rest, one biology advance, and rollback. Passed pure-helper tests cover no-motion, nonzero motion, net cancellation, and preservation of all other event fields.

## Scope limits

The rule does not attempt a complete model of exertion. Stationary manipulation, isometric effort, and non-displacing failed attempts retain current recovery semantics. The requested minimum is to stop actual locomotor/withdrawal motion from simultaneously being physiological sleep. Blocked attempts still pay their existing cost. This change is not a hardcoded planner or a guarantee of action diversity/emotion.

Any combined reflex+world-rule artifact requires separately labeled validation by the reflex owner. No new autonomous lifetimes or policy cohorts are part of this world regression.

Patch: /workspace/shared/rest-motion-world-candidate-v1.patch; SHA256 c394e2fbfadcf2fea7e7ecba15ad44ba7f757e0d366a06b7f8e50871010169dc. The helper+tests-only patch /workspace/shared/rest-motion-helper-tests-v1.patch was clean-apply checked against reflex source; its call is deliberately installed after reflex energy/damage composition with voluntary and withdrawal vectors chained. Full base patch call context conflicts with reflex composition as expected, so do not apply both patches.


## Reproduction and qualification commands

Prerepair and repaired focus: cargo test -p alife_world --lib rest_motion_ -- --nocapture --test-threads=1

Full suite: cargo test -p alife_world -- --test-threads=1

Rust1.99.1, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, RUST_MIN_STACK=33554432. Separate target was released after full suite. No extra actual-policy seeds were used or reserved. The tests reuse deterministic prepared-world fixture36001.

Headless source SHA256: ddb351afd1c5a12e8013f6e9c90076ba616dfed54659ea92de99ee8300a79ff9.
Prerepair log SHA256: c58949749856f17f90cdcff9188b0b5622f49cd263b0bc3ae363a3be9939b1be.
Repaired focused log SHA256: 1ee69aa5dc7579ba6c681323b02a415aeef15e397a730dd44eda779bbc3c17f9.

This qualification covers the clean-base world-only motion rule. The optional reflex is absent here; the separate reflex owner must qualify its chained integration and any later pain-triggered sleep interruption. Stationary pain interrupting sleep is not silently included in this factor.

Durable prerepair reproducer patch: /workspace/shared/rest-motion-prerepair-tests-only.patch. Apply to the exact clean base then run the focused command above to reproduce2 passes/1 intentional failure. Do not stack this tests-only patch with the final patch, which already includes the tests.

The three ignored tests were existing manual CPU contact-cost timing, manual CPU mating-query timing, and extended headless soak. They were not run or silently marked passed.
