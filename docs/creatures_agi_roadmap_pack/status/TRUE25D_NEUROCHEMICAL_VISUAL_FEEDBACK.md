# True 2.5D Neurochemical Visual Feedback

Classification: CA44A-ext-05

Plan context: CA44A extension slice for True 2.5D runtime pipeline hardening

Branch: `codex/true25d-neurochemical-visual-feedback-phase5`

This is a CA44A extension slice. It does not advance the CA roadmap. CA44
remains blocked until independent external tester evidence exists. CA45 was
not started. The next roadmap item remains CA44 after evidence, not CA45.

## Objective

Make the selected creature's internal drive state readable in the default
True 2.5D Player View without bringing back the debug dashboard or changing
simulation semantics.

## Implementation Summary

- Added a True 2.5D neurochemical feedback contract in `alife_game_app`.
- Added six display-only in-world cue roles around the selected creature:
  - hunger glow;
  - pain spike;
  - stress/desaturation aura;
  - energy trail;
  - sleep bloom;
  - H_shadow learning biolume.
- Cues derive intensity from existing `CreatureVisualSnapshot` drive fields and
  bounded graphical GPU telemetry.
- Cues follow the selected creature through the runtime update path.
- Cues are native low-poly Bevy presentation meshes/materials, not UI text and
  not action/cognition inputs.

## Boundaries

- Display-only.
- No action authority.
- No weight authority.
- No hidden vector injection.
- No semantic, teacher, topology, memory, or GPU bypass.
- CPU shadow gate remains preserved.
- No active bulk neural readback is added.

## Evidence

Focused checks for this slice:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_player_view_has_display_only_neurochemical_world_cues -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
```

Graphical smoke and full validation are recorded in the final receipt for this
branch.

Roadmap evidence status:

- This branch is local CA44A visual/stability hardening only.
- It is not independent external CA44 tester evidence.
- It does not unblock CA44 by itself.

## Known Limitations

- This CA44A-ext-05 slice has an addendum at
  `docs/creatures_agi_roadmap_pack/status/TRUE25D_ENDOCRINE_ASSET_FEEDBACK.md`.
  The addendum adds selected-creature root posture/asset-presentation feedback
  while preserving the same display-only boundaries and without advancing the
  CA roadmap.
- This CA44A-ext-05 slice also has a GLB metadata-contract addendum at
  `docs/creatures_agi_roadmap_pack/status/TRUE25D_ENDOCRINE_GLTF_FEEDBACK_CONTRACT.md`.
  That addendum validates endocrine visual-feedback metadata inside the active
  committed creature GLB files and records the same no-action/no-weight
  authority boundary. It does not add authored keyframe animation clips.
- This CA44A-ext-05 slice also has an animation/particle addendum at
  `docs/creatures_agi_roadmap_pack/status/TRUE25D_ENDOCRINE_ANIMATION_PARTICLE_FEEDBACK.md`.
  That addendum wires the validated endocrine-feedback contract into a bounded
  selected-creature animation-speed layer and three display-only bioluminescent
  particle lanes.
- This phase does not add new Blender-authored assets. The fixed Blender path
  remains available for future mesh calibration, but was not required for this
  display-feedback slice.
- Learning biolume is visible only when runtime telemetry reports post-seal
  H_shadow applications.
- These cues are presentation mirrors only; they do not alter drives,
  arbitration, ExperiencePatch sealing, or GPU neural runtime behavior.
- The active True 2.5D goal still has a separate 60Hz authoritative headless
  simulation-cadence gap. Current evidence preserves CA13's 20Hz authoritative
  simulation cadence and records `claim_60hz_sim=false`; changing that cadence
  requires a separate reviewed scheduler plan.

## Invariant Checks

- `alife_core` unchanged.
- No Bevy/wgpu/app dependency leak into `alife_core`.
- No full action-authoritative GPU claim.
- CPU fallback preserved.
- CPU shadow parity preserved.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media are intended for tracking.
- No S12, G25, or P37 created.
- No release tag created.

## Next Plan

This completes only CA44A-ext-05. Roadmap continuation remains stopped. CA44
remains the next roadmap item after independent external tester evidence is
provided. CA45 was not started and must not be started from this status
document.
