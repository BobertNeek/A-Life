# True 2.5D Neurochemical Visual Feedback Handoff

Created: 2026-07-01

Current status: archival/current-state handoff

## Current State

- Classification: CA44A-ext-05 and CA44A-ext-05 addendums.
- Original Phase 5 work is committed, merged to `main`, post-merge validated,
  and pushed.
- Later endocrine addendums are committed, merged to `main`, post-merge
  validated, and pushed.
- No dirty Phase 5 work is expected.
- Stale Phase 5 branches should not be recovered, merged, rebased, or treated
  as active handoff work.
- This handoff is historical/archival. Current `main` and
  `CA44A_TRUE25D_EXTENSION_ROLLUP.md` are the active status sources.
- CA roadmap continuation remains stopped.
- This work does not advance the CA roadmap.
- CA44 remains blocked until independent external tester evidence exists.
- CA45 was not started and is not authorized.
- The next roadmap item remains CA44 after evidence, not CA45.

## Implemented Phase 5 Slice

The original CA44A-ext-05 slice added display-only selected-creature
neurochemical world cues in the default True 2.5D Player View:

- hunger glow;
- pain spike;
- stress/desaturation aura;
- energy trail;
- sleep bloom;
- H_shadow learning biolume.

The cues derive from existing `CreatureVisualSnapshot` fields and bounded
graphical GPU telemetry. They are presentation entities only; they do not emit
actions, modify weights, alter drives, bypass arbitration, or change
ExperiencePatch sealing.

## Merged Addendums

- `TRUE25D_ENDOCRINE_ASSET_FEEDBACK.md` records selected-creature
  posture/material-shell feedback.
- `TRUE25D_ENDOCRINE_GLTF_FEEDBACK_CONTRACT.md` records the validated GLB
  metadata contract for endocrine-capable creature assets.
- `TRUE25D_ENDOCRINE_ANIMATION_PARTICLE_FEEDBACK.md` records bounded
  selected-creature animation-speed feedback and three display-only
  bioluminescent particle lanes.

The consolidated extension sequence is summarized in:

```text
docs/creatures_agi_roadmap_pack/status/CA44A_TRUE25D_EXTENSION_ROLLUP.md
```

## Historical Evidence

Historical branch receipts recorded focused Bevy tests, graphical smoke,
forced CPU fallback smoke, and full validation. Current rollup validation is
docs/status-only because no runtime code is changed by the rollup.

## Boundaries And Invariants

- `alife_core` was not changed by the handoff cleanup.
- No Bevy, wgpu, renderer, app, model-runtime, or tool dependency is added to
  `alife_core`.
- Visual feedback is display-only.
- No action authority is added.
- No weight authority is added.
- No semantic, teacher, topology, memory, UI, or GPU bypass is added.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU runtime claim is added.
- No active bulk neural readback is added.
- No S12, G25, or P37 is created.
- No release tag is created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media should be committed.
- The active True 2.5D status preserves the existing reviewed scheduler
  boundary: CA13 records 20Hz authoritative simulation plus 60Hz presentation.
  Any 60Hz authoritative simulation change requires a separate reviewed
  scheduler plan.

## Resume Guidance

Do not resume from a dirty Phase 5 branch. If more visual work is desired,
create a fresh explicitly authorized branch from current `main`, keep it under
CA44A or a later reviewed roadmap item as instructed, and do not start CA45
without explicit approval.
