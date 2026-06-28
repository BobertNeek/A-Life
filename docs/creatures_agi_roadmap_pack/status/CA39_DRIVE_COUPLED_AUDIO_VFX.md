# CA39 Drive-Coupled Audio and VFX

## Plan

CA39 - Drive-coupled audio and VFX.

## Branch

`codex/CA39-drive-coupled-audio-vfx`

## Files Changed

- `content/fixtures/g17/feedback_polish_manifest.json`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/drive_coupled_audio_vfx.rs`
- `crates/alife_game_app/src/lib.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `docs/creatures_agi_roadmap_pack/status/CA38_CREATURE_ANIMATION_STATE_MACHINE.md`
- `docs/creatures_agi_roadmap_pack/status/CA39_DRIVE_COUPLED_AUDIO_VFX.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

## Runtime Code Changed

Yes. CA39 adds a display-only cue mapping layer that converts existing sealed
feedback events and runtime H_shadow telemetry into player-readable
audio/VFX cue descriptors.

## Core APIs Changed

No. `alife_core` was not changed.

## Docs Changed

Yes. This CA39 status file records the implementation and CA38 main status was
updated from pending to merged/validated as a factual reference.

## Public APIs Changed

Yes, within `alife_game_app` only:

- `Ca39DriveCueKind`
- `Ca39RuntimeCueEvidence`
- `Ca39DriveCue`
- `Ca39DriveAudioVfxSummary`
- `ca39_drive_audio_vfx_summary`
- `ca39_drive_audio_vfx_summary_from_graphical`
- `ca39_drive_audio_vfx_panel_text`
- `ca39_drive_audio_vfx_panel_text_from_graphical`
- `run_drive_coupled_audio_vfx_smoke`
- CLI command `drive-coupled-audio-vfx-smoke <fixture-root>`

## Tests Added/Changed

- Added headless CA39 cue mapping tests for hunger satisfaction, hazard pain,
  sleep/rest, and learning pulse.
- Added a CA39 smoke test that verifies honest runtime claims, no action
  authority, no weight authority, no cognition mutation, and no active bulk
  readback.
- Added Bevy-feature coverage for the player-readable CA39 cue panel.
- Extended the feedback manifest test to require CA39 procedural learning-pulse
  descriptors.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell ca39 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca39 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- drive-coupled-audio-vfx-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- drive-coupled-audio-vfx-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- feedback-polish-smoke crates/alife_world/tests/fixtures/gpu_alpha
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Observed CA39 smoke summary without `gpu-runtime` feature:

```text
schema=alife.ca39.drive_coupled_audio_vfx.v1 version=1 cues=4 active=3 audio=4 vfx=4 backend=CpuReference fallback=Some("FeatureDisabled") h_shadow_apps=0 gate=true no_readback=true no_actions=true no_weights=true claim=None full_action_authoritative=false
```

This is expected for a non-GPU CLI smoke. The learning pulse remains configured
and becomes active only when H_shadow applications are present in runtime
telemetry.

Observed CA39 smoke summary with `gpu-runtime` feature on local RTX/Vulkan
hardware:

```text
schema=alife.ca39.drive_coupled_audio_vfx.v1 version=1 cues=4 active=4 audio=4 vfx=4 backend=GpuPlastic fallback=None h_shadow_apps=4 gate=true no_readback=true no_actions=true no_weights=true claim=CpuShadowGuardedStaticPlusLiveHShadow full_action_authoritative=false
```

Observed graphical smoke selected `GpuPlastic`, kept CPU shadow parity true, and
reported no fallback. The bounded graphical slice did not apply H_shadow during
that specific 30-second run, so the learning cue remained configured but
inactive there. Forced fallback smoke selected `CpuReference` with
`HardwareUnavailable` and made no GPU claim.

## Validation Results

Focused validation is passing. Full branch validation and post-merge validation
are recorded in the final CA39 receipt.

## Invariant Checks

- `alife_core` unchanged and remains engine-independent.
- CA39 cues are display-only and non-authoritative.
- CA39 cues cannot emit actions, mutate cognition, or rewrite weights.
- CPU shadow parity remains the correctness gate.
- CPU fallback remains available and visibly reported.
- Product runtime claim remains honest; no full action-authoritative GPU claim
  was added.
- No active bulk neural readback was added.
- Stable IDs remain the player-facing identity surface.
- No Bevy Entity IDs are used in CA39 player-facing text.

## Known Limitations

- Audio remains procedural/stub-labeled; CA39 does not add binary audio assets.
- Cue activity depends on the underlying runtime evidence. In CPU fallback,
  H_shadow learning pulse is configured but inactive.
- This plan does not add CA40 onboarding/tutorial behavior or CA39 production
  audio mixing.

## Artifacts Tracked

No screenshots, logs, target artifacts, model files, caches, or generated media
are tracked.

## Release/Tag Status

No release tag was created.

## alife_core Dependency Status

`alife_core` remains dependency-clean; this branch does not add Bevy, wgpu,
renderer, windowing, model-runtime, or app dependencies to core.

## Main Status

Pending merge and post-merge validation.

## Next Plan

CA40 - Onboarding tutorial first session.
