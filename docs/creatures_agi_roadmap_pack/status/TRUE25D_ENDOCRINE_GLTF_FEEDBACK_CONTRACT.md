# True 2.5D Endocrine GLB Feedback Contract

Classification: CA44A-ext-05 addendum

Branch: `codex/true25d-endocrine-gltf-feedback-contract`

This is a CA44A extension slice. It does not advance the CA roadmap. CA44
remains blocked until independent external tester evidence exists. CA45 was not
started. The next roadmap item remains CA44 after evidence, not CA45.

## Objective

Move the existing True 2.5D neurochemical visual feedback proof one step closer
to direct asset feedback by requiring the committed creature GLB assets to carry
a validated endocrine-feedback contract in their own glTF metadata.

## Implementation Summary

- Added an `alife.ca44a.true25d_endocrine_asset_feedback.v1` contract to the
  active True 2.5D asset manifest.
- Embedded the same contract into the GLB `asset.extras` metadata for:
  - `creature_idle.glb`;
  - `creature_hurt.glb`.
- Added manifest validation that requires the GLB metadata and manifest
  contract to match exactly for the endocrine-capable creature roles.
- Added validation fields to the True 2.5D manifest summary and app-bundle
  summary.
- Added runtime receipt fields proving the Player View endocrine feedback path
  is backed by the validated GLB contract.
- Added tests for missing contracts, malformed authority/channel contracts, and
  manifest/GLB mismatch rejection.

## Contract Channels

The asset contract is display-only and declares:

- posture channels from adrenaline and pain companion drive;
- animation-speed channels from adrenaline and pain companion drive;
- material channels from cortisol, dopamine, low-hunger companion, and learning
  companion;
- particle channels from dopamine and learning companion;
- no action authority;
- no weight authority.

The runtime source remains bounded:

```text
alife_core.EndocrineSnapshot::to_array plus bounded drive companions
```

## What This Proves

- The active creature GLB assets are no longer generic meshes from the
  validator's point of view; they carry a versioned endocrine visual-feedback
  metadata contract.
- The manifest and GLB metadata must agree or validation fails.
- Player View receipts expose that the direct asset-feedback contract was
  validated before endocrine presentation is reported.
- The endocrine path remains display-only.

## What This Does Not Prove

- This does not add new Blender-authored keyframe animation clips. Current GLB
  files still do not claim authored skeletal/keyframe animation tracks.
- This does not change world simulation, navigation, sensory authority,
  ExperiencePatch sealing, action arbitration, GPU neural correctness, or CPU
  shadow parity.
- This does not claim full action-authoritative GPU runtime.
- This does not unblock CA44 without independent external tester evidence.

## Focused Evidence

Focused checks:

```powershell
cargo test -p alife_game_app true_25d_assets -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_creature_asset_feedback -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
```

Results:

- True 2.5D asset validation: PASS, 5 tests.
- App bundle production asset discovery: PASS, including
  `true_25d_endocrine_feedback_contract_validated=true`.
- Focused endocrine Bevy receipt test: PASS.
- Broader True 2.5D Bevy app-shell filter: PASS, 10 tests.

Graphical evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Results:

- Default graphical smoke: PASS. Local preflight selected `GpuPlastic` on the
  RTX 3050/Vulkan path, fallback `None`, and the smoke exited cleanly.
- Forced fallback smoke: PASS. The smoke selected `CpuReference` with
  `HardwareUnavailable` fallback and made no GPU-performance claim.

Branch validation:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

Result: PASS. `cargo tree -p alife_core` still contains no Bevy, wgpu,
renderer, app, model-runtime, or tool dependency leak.

## Invariant Checks

- `alife_core` unchanged.
- No Bevy/wgpu/app dependency leak into `alife_core`.
- Visual feedback remains display-only.
- No action authority added.
- No weight authority added.
- No hidden vector injection added.
- CPU fallback preserved.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU claim.
- No S12, G25, or P37 created.
- No release tag created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media should be tracked.

## Next Plan

Roadmap continuation remains stopped. CA44 remains the next roadmap item after
independent external tester evidence is provided. Do not start CA45 from this
status document.
