# CA44A True 2.5D Extension Rollup

Status: complete through CA44A-ext-05 endocrine animation/particle addendum

Branch: `codex/CA44A-true25d-extension-rollup`

Roadmap status: CA roadmap remains stopped at CA44

## Purpose

This rollup consolidates the CA44A True 2.5D extension work completed after
the CA44A real-art/tick-stability unblocker. The extension sequence is
presentation and visual-hardening work for CA44 readiness only. It does not
advance the CA roadmap, does not start CA45, and does not replace the need for
independent external CA44 tester evidence.

## Current Roadmap State

- CA44 remains blocked until independent external tester evidence exists.
- CA45 is not authorized and was not started by these extension slices.
- The next roadmap item remains CA44 after evidence is provided.
- Stale dirty Phase 5 handoff branches should not be recovered.
- Historical True 2.5D branches may remain in git, but current `main` is the
  authoritative source for active status.

## Completed Sequence

| Slice | Status | Branch / merge commit | Purpose | Status docs |
| --- | --- | --- | --- | --- |
| CA44A base: real art assets and tick stability | Complete, merged | `codex/CA44A-real-art-assets-and-tick-stability`; merge `73c7aa41` | Fixed the tick-7 `TerminalInvalidState` path, added versioned art assets, established True 2.5D player-view direction, and preserved strict invalid-state reporting. | `CA44A_REAL_ART_ASSETS_AND_TICK_STABILITY.md` |
| CA44A-ext-01: True 2.5D runtime pipeline baseline | Complete, merged | `codex/true25d-runtime-pipeline-baseline`; merge `91db7e70` | Locked the orthographic 2.5D camera, replaced synchronous default ground generation with a repeatable ground substrate, and preserved procedural chunks as CPU/data ledger context. | `TRUE25D_RUNTIME_PIPELINE_BASELINE.md` |
| CA44A-ext-02: Blender asset calibration / normalized GLB lane | Complete, merged | `codex/true25d-blender-pipeline-calibration`; merge `3b51da8b` | Added Blender normalization tooling, normalized active `.glb` assets, and validated manifest-level size, origin, transform, and triangle-count contracts. | `TRUE25D_BLENDER_PIPELINE_CALIBRATION.md` |
| CA44A-ext-03: viewport render-bypass | Complete, merged | `codex/true25d-render-bypass-phase3`; merge `d5b8d512`; related proof merge `0465f86e` | Proved offscreen True 2.5D presentation entities can be hidden/bypassed while chunk data remains available as headless CPU/data context. | `TRUE25D_VIEWPORT_RENDER_BYPASS.md`, `TRUE25D_RENDER_BYPASS_PROOF.md`, `TRUE25D_HEADLESS_CHUNK_CONTINUITY.md` |
| CA44A-ext-04: stylization render pass | Complete, merged | `codex/true25d-stylization-render-pass-phase4`; merge `118318f1` | Added the GPU-backed postprocess stylization path with pixel-step sampling, toon quantization, and depth/luminance Sobel outline support. | `TRUE25D_STYLIZATION_RENDER_PASS.md` |
| CA44A-ext-05: neurochemical visual feedback | Complete, merged | `codex/true25d-neurochemical-visual-feedback-phase5`; merge `8ad8c962` | Added display-only in-world selected-creature cues for hunger, pain, stress, energy, sleep, and H_shadow learning. | `TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK.md`, `TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK_HANDOFF.md` |
| CA44A-ext-05 addendum: endocrine asset/posture feedback | Complete, merged | `codex/true25d-endocrine-asset-feedback-phase6`; merge `2886f9ef` | Moved feedback closer to creature presentation through bounded selected-creature posture/material-shell receipts. | `TRUE25D_ENDOCRINE_ASSET_FEEDBACK.md` |
| CA44A-ext-05 addendum: endocrine GLB metadata contract | Complete, merged | `codex/true25d-endocrine-gltf-feedback-contract`; merge `1e40f690` | Required active creature GLB files and manifest metadata to agree on a display-only endocrine feedback contract. | `TRUE25D_ENDOCRINE_GLTF_FEEDBACK_CONTRACT.md` |
| CA44A-ext-05 addendum: endocrine animation/particle feedback | Complete, merged | `codex/true25d-endocrine-animation-particle-feedback`; merge `b1d6983e` | Wired the validated endocrine contract into bounded selected-creature animation-speed and three display-only bioluminescent particle lanes. | `TRUE25D_ENDOCRINE_ANIMATION_PARTICLE_FEEDBACK.md` |

## Evidence Summary

- CA44A base status records 600-tick `gpu_alpha` stability evidence with no
  `TerminalInvalidState`, plus default graphical Player View smoke, forced CPU
  fallback smoke, package dry-run, and full branch validation.
- Runtime pipeline baseline evidence records focused True 2.5D Bevy tests,
  default graphical smoke selecting local `GpuPlastic`, forced CPU fallback,
  and full validation.
- Blender calibration evidence records local Blender discovery, normalized
  active `.glb` assets, asset validation tests, graphical smoke, forced
  fallback smoke, and full validation.
- Viewport render-bypass evidence records render-bypass focused tests,
  procedural travel tests, graphical smoke, forced fallback smoke, and full
  validation with the MSVC all-features linker flake handled by a single-job
  rerun.
- Stylization evidence records shader discovery, True 2.5D asset validation,
  feature-gated Bevy postprocess tests, graphical smoke, forced fallback smoke,
  and full validation.
- Neurochemical feedback evidence records focused Bevy app-shell tests for
  display-only cues plus graphical and validation evidence in the branch
  receipt.
- Endocrine addendum evidence records focused Bevy tests proving posture,
  GLB metadata, animation-speed, and particle-lane receipts are display-only
  and do not carry action or weight authority.

## Validation Status

Each merged slice records passing focused checks and branch validation in its
status document. Current rollup validation is docs/status-only and should use:

```powershell
cargo fmt --all -- --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
cargo tree -p alife_core
```

No runtime code change is required for this rollup.

## Known Limitations

- The True 2.5D work is presentation/visual hardening.
- It does not change simulation authority.
- It does not make Bevy visuals authoritative over world, sensory,
  navigation, action, cognition, semantic, teacher, topology, memory, neural,
  GPU, or ExperiencePatch systems.
- The authoritative simulation cadence remains the existing reviewed cadence:
  CA13 records 20Hz fixed simulation with 60Hz presentation.
- Any change to authoritative 60Hz simulation cadence requires a separate
  reviewed scheduler plan.
- Do not claim 60Hz authoritative headless simulation unless a reviewed
  scheduler plan proves it.
- Independent human external tester evidence is still missing for CA44.

## Invariant Status

- `alife_core` remains engine-independent.
- No Bevy, wgpu, renderer, app, Blender, asset-pipeline, or model-runtime
  dependency is introduced into `alife_core`.
- Stable IDs remain the portable/player-facing identifier boundary.
- CPU fallback remains available.
- CPU shadow parity remains the correctness gate.
- No full action-authoritative GPU runtime claim is made.
- Visual feedback is display-only.
- No action authority or weight authority is added.
- No hidden semantic, teacher, topology, memory, UI, or GPU bypass is added.
- No active bulk neural readback is added.
- No S12, G25, or P37 is created.
- No release tag is created.
- Screenshots, logs, target artifacts, model files, caches, generated media,
  and temporary outputs remain untracked.

## Final Roadmap Position

CA44A extension work is for CA44 readiness only.

CA44 remains blocked until independent external tester evidence exists.

CA45 is not authorized.

CA roadmap continuation remains stopped.
