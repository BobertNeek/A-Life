# True 2.5D Neurochemical Visual Feedback - Endocrine Asset Addendum

Classification: CA44A-ext-05 addendum

Branch: `codex/true25d-endocrine-asset-feedback-phase6`

This is a continuation addendum to CA44A-ext-05, not a new roadmap item and not
a CA roadmap advancement. CA44 remains blocked until independent external
tester evidence exists. CA45 was not started. The next roadmap item remains
CA44 after evidence, not CA45.

## Objective

Move neurochemical feedback from detached world cues toward direct creature
presentation feedback in the default True 2.5D Player View, while keeping the
feedback display-only and non-authoritative.

## Implementation Summary

- Added `GraphicalTrue25dCreatureEndocrinePresentation` to selected creature
  roots in the True 2.5D layer.
- Added `GraphicalTrue25dEndocrineAssetFeedbackResource` as a bounded runtime
  receipt for:
  - fixed flat endocrine tensor channel count/source;
  - pain/adrenaline posture;
  - cortisol/stress desaturation;
  - dopamine and hunger-satisfaction biolume;
  - H_shadow learning biolume;
  - compact particle-trail count;
  - CPU-shadow/no-bulk-readback boundary.
- The selected creature root receives a bounded transform/posture pulse derived
  from the existing `CreatureVisualSnapshot`, its core
  `EndocrineSnapshot::to_array()` channel data, bounded drive companions, and
  GPU telemetry.
- Existing low-poly neurochemical cue meshes remain supplemental display-only
  material shells around the selected creature.

## Boundaries

- Display-only.
- No action authority.
- No weight authority.
- No semantic, teacher, topology, memory, UI, or GPU bypass.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU claim.
- No active bulk neural readback.
- `alife_core` is unchanged.

## Focused Evidence

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_creature_asset_feedback -- --nocapture
```

Result: PASS, 1 passed.

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
```

Result: PASS, 10 passed.

## Known Limitations

- The feedback uses bounded root transform/material-shell presentation. It does
  not yet edit Blender-authored animation clips or glTF material internals.
- Dopamine is now carried through the app visual snapshot from the existing
  core endocrine snapshot and used as a display-only biolume input. Low hunger
  and H_shadow learning remain bounded presentation companions.
- This is still not independent external CA44 tester evidence.
- This does not start CA45 and does not request external tester evidence.
- This addendum does not replace CA44. CA44 remains the next roadmap item after
  independent evidence is available.

## Invariant Checks

- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.
- No Bevy/wgpu/app dependency leaked into `alife_core`.
- No action authority changed.
- No CPU fallback or CPU shadow parity weakening.

## Next

Continue only if explicitly instructed. The CA roadmap remains stopped at CA44
until independent external tester evidence is available.
