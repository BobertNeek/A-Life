# True 2.5D Endocrine Animation And Particle Feedback

Classification: CA44A-ext-05 addendum

Branch: `codex/true25d-endocrine-animation-particle-feedback`

This is a CA44A extension slice. It does not advance the CA roadmap. CA44
remains blocked until independent external tester evidence exists. CA45 was not
started. The next roadmap item remains CA44 after evidence, not CA45.

## Objective

Close the remaining CA44A-ext-05 runtime presentation gap between the validated
GLB endocrine metadata contract and the live selected-creature display path.
The selected creature now exposes bounded animation-speed and bioluminescent
particle-lane presentation derived from the same flat endocrine tensor and
bounded drive companions as the existing posture/material shell.

## Implementation Summary

- Added `animation_speed_multiplier`, `animation_phase_index`, and
  `animation_speed_layer_applied` to the True 2.5D endocrine asset feedback
  receipt.
- Added `GraphicalTrue25dEndocrineParticleLane` presentation entities.
- Spawned exactly three low-overhead particle lanes around the selected
  creature.
- Updated the Player View neurochemical system so particle lanes follow the
  selected creature, phase with the endocrine animation layer, and show/hide
  according to bounded biolume intensity.
- Propagated animation speed and biolume particle initialization onto the
  selected creature root presentation component.

## Runtime Contract

Inputs:

- `alife_core::EndocrineSnapshot::to_array()`;
- bounded pain and low-hunger drive companions;
- bounded H_shadow learning telemetry for display-only biolume.

Outputs:

- high pain/adrenaline raises a bounded selected-creature animation-speed
  multiplier and posture phase;
- cortisol remains mirrored through the existing stress/desaturation material
  shell cue;
- low hunger, dopamine, or H_shadow learning initializes visible emissive
  particle lanes.

All outputs are presentation-only.

## Boundaries

- No action authority.
- No weight authority.
- No semantic, teacher, topology, memory, UI, or GPU bypass.
- No active bulk neural readback.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU claim.
- `alife_core` is unchanged.

## Focused Evidence

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_creature_asset_feedback -- --nocapture
```

Result: PASS. The focused test now asserts:

- selected creature root receives the bounded animation-speed layer;
- exactly three endocrine particle-lane entities exist;
- visible particle lanes match the endocrine receipt;
- particle lanes are initialized from endocrine tensor evidence;
- particle lanes are display-only and have no action or weight authority.

## Remaining Goal Gap

The active True 2.5D goal still contains a 60Hz authoritative headless
simulation-cadence requirement. Current A-Life CA13 scheduler authority remains
20Hz fixed simulation plus 60Hz presentation. Existing
`TRUE25D_HEADLESS_CHUNK_CONTINUITY.md` records `claim_60hz_sim=false`.

Changing authoritative simulation cadence requires a separate reviewed
scheduler plan; this addendum does not change scheduler authority.

## Invariant Checks

- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.
- No Bevy/wgpu/app dependency leaked into `alife_core`.
- No action authority changed.
- No CPU fallback or CPU shadow parity weakening.

## Next

Do not resume CA roadmap execution from this document. CA44 remains the next
roadmap item after independent external tester evidence exists.
