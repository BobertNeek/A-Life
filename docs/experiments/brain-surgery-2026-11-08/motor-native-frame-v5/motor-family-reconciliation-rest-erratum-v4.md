# Additive erratum: rest was not tested with a lawful neutral locomotion choice

This corrects the rest-related interpretation of the frozen motor-family reconciliation v3 report and earlier summaries. It does not change source commit `d40c96cd847dcd869ea480744b6e0d6c0f3fe0ce`, tree `0c17cd5d244ec482b9a21c282ad2962ac3f38473`, the v3 archive, scalar results or tests. No new run was performed and the shared native target was not used.

## Correction

**Zero wholly resting bundles in the full-opportunity fixture is structurally forced by the fixture, not evidence that the candidate cannot choose legitimate rest.** The fixture offers three non-neutral locomotion choices and no native NO_LOCOMOTION candidate. Idle/Rest representatives occupy the posture slot; the unchanged sampler must independently fill the nonempty locomotion mask with one of those three choices.

The v3 statements that quiet-rest behavior failed or remained unsatisfactory, and any earlier “rest failure” summary based on this bundle count, are withdrawn. The correct status is: **rest with an available lawful neutral locomotion option was not evaluated**. The sparse Idle/Rest-only control merely removes all locomotion opportunities; it cannot substitute for that test.

Pain-conflict selection changes, signed-logit contributions, measured birth-dendrite saturation and the sampled A/B target mismatch remain valid within the documented one-head synthetic fixture. No-dendrite results remain an altered neural-state diagnostic, not a normal-birth claim. None of these results qualifies production behavior.

## Exact frozen-source evidence

All paths below are relative to `/workspace/scratch/ee5e08a3ca9b/motor-family-reconciliation-v1`, at the frozen commit above.

- `crates/alife_training/src/cpu/replay/motor_reconciliation_assay.rs:148–221`: full candidate definitions are Idle, Rest, Approach A, Approach B, Avoid H, Eat A, Eat B and Inspect A. Only indices 2, 3 and 4 have `ActionKind::Move`; all have synthetic non-neutral targets. There is no locomotion-neutral entry. Lines 225–226 remove every target-bearing entry only in the sparse control. Lines 239–247 assign synthetic action IDs and targets, not native no-op primitives.
- `crates/alife_training/src/cpu/input.rs:134–140`: Move maps to locomotion slot 0; Rest maps to posture slot 4. Lines 215–226 construct masks from candidates and force an Idle representative into posture slot 4. Lines 235–247 fill every other nonempty enabled slot. Thus the full fixture's locomotion mask is exactly bits 2, 3 and 4 (`0x1c`), and it is sampled even when Idle or Rest is representative.
- `crates/alife_core/src/genome.rs:164–173` gives the scaffold one motor head. `crates/alife_core/src/phenotype/decoder.rs:34–38,218–232` makes that head locomotion. `motor_reconciliation_assay.rs:15–33` compiles the scaffold unchanged across arms.
- `motor_reconciliation_assay.rs:406–415` defines `is_rest` from the representative's family and `active` as `motor_candidates[..4].iter().any(Option::is_some)`. Lines 523–529 assert a zero “valid_rest_bundle” count for every nonsparse frame. This is a fixture/mask property, not an independently demonstrated behavioral failure.

## Verified native neutral primitives and the second metric limitation

Constants in `crates/alife_world/src/headless.rs:1084–1092` are:

| Primitive | Native action ID | Enumerator kind/family | Target |
|---|---:|---|---|
| NO_LOCOMOTION | 103 | Move / Approach | None |
| NO_MANIPULATION | 212 | Interact / Contact | None |
| NO_POSTURE | 104 | Hold / Other | None |

These are verified values; NO_POSTURE is **104**, not 315. `crates/alife_world/src/candidate_enumerator.rs:406–449` supplies each with zero features plus its intrinsic action basis, confidence 1, zero required effort and one-tick duration. Its comment explicitly states that a visible object must not force movement, rest or manipulation.

`headless.rs:7372–7377` classifies these native IDs as their respective no-op actions. Lines 5283–5304 execute them with zero displacement/no physical contact. Lines 7510–7526, 7529–7551 and 7574–7577 preserve the neutral primitive and targetless semantics when converting motor channels.

The current presence-only metric would also misclassify `Some(NO_LOCOMOTION)` as movement if the native neutral were added. `Some` means a command is present, not that it causes motion. Family and target presence alone are also insufficient: NO_LOCOMOTION belongs to Approach, and other targetless primitives can perform real actions.

## Explicit next-control plan — not run

1. Prefer fixed **production-enumerated frames**, retaining native NO_LOCOMOTION 103, NO_MANIPULATION 212 and NO_POSTURE 104 wherever their channels are enabled. Verify their exact IDs, kinds, targets, features and decoder-mask membership. Do not manufacture an abstention mask, choose actions externally or merely append another posture Idle/Rest.
2. Use the actual production founder's matched motor-head configuration across unchanged, candidate and zero-competition arms. Record the compiled enabled-channel set. Do not assume this one-head scaffold represents all production factors or guess a head count.
3. Replace slot-presence and representative-family rest classification with a semantic audit of **every selected primitive and target** using native action semantics. Separately report locomotion-neutral selection, active locomotion intent, quiescent bundles, and Rest posture accompanied by neutral other channels. A neutral representative can coexist with Rest in a different slot; representative family alone must not decide the result. NO_POSTURE is neutral, not evidence of a Rest recovery action.
4. Include satiated full-opportunity frames with genuine neutral choices, fatigue/rest frames and the same drive-conflict cases. Keep decoder, activation, drive gains and birth dendrites matched; keep any no-dendrite diagnostic separately labelled. Retain exact sampled bundles and signed contributions.
5. Require behaviorally discriminating assertions: a neutral command must not count as movement; a targeted Approach/Flee must count as active intent; neutral-plus-Rest must be distinguishable from neutral-only quiescence. Preserve the actual compiler and sampler and keep target-binding questions separate.
6. These are intended-action measurements. Do not claim stationary sleep, recovery or physical inactivity without separately authorized world execution and receipts. No new cohort, gain change, native test or target use is part of this erratum.

## Corrected short summary for future use

The isolated candidate changes some pain selections and signed logits in its one-head synthetic fixture. Saturation and a sampled target mismatch remain measured limitations. **Its ability to select lawful rest alongside other opportunities is untested because the fixture omitted native neutral locomotion and its metric equated slot occupancy with activity.**
