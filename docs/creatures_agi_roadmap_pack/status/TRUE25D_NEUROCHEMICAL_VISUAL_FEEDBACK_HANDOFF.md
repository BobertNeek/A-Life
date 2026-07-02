# True 2.5D Neurochemical Visual Feedback Handoff

Created: 2026-07-01

## Current State

- Current branch: `codex/true25d-neurochemical-visual-feedback-phase5`
- Classification: CA44A-ext-05, a CA44A extension slice.
- Original Phase 5 status: committed, merged to `main`, post-merge validated,
  and pushed.
- Continuation addendum status: `codex/true25d-endocrine-asset-feedback-phase6`
  was committed, merged to `main`, post-merge validated, and pushed as a
  CA44A-ext-05 addendum.
- Later render-bypass proof status: committed, merged to `main`,
  post-merge validated, and pushed as a CA44A extension proof slice.
- Current addendum status: `codex/true25d-endocrine-gltf-feedback-contract`
  adds a GLB-internal endocrine visual-feedback metadata contract for the
  active creature assets. It remains a CA44A-ext-05 addendum, does not advance
  the CA roadmap, and does not unblock CA44 without independent external tester
  evidence.
- This handoff was refreshed after the Phase 5 branch was fast-forwarded to
  the current `main` tip so it does not describe stale uncommitted work.
- CA roadmap continuation remains stopped.
- This work does not advance the CA roadmap.
- CA44 remains blocked until independent external tester evidence exists.
- CA45 was not started.
- The next roadmap item remains CA44 after evidence, not CA45.

## Implemented In This Working Tree

Runtime code:

- `crates/alife_game_app/src/bevy_shell.rs`
  - Added `GraphicalTrue25dNeurochemicalCueKind`.
  - Added `GraphicalTrue25dNeurochemicalCue`.
  - Added `GraphicalTrue25dNeurochemicalFeedbackResource`.
  - Added native low-poly/material cue presentation for:
    - hunger glow;
    - pain spike;
    - stress/desaturation aura;
    - energy trail;
    - sleep bloom;
    - H_shadow learning biolume.
  - Spawned cues in default True 2.5D Player View.
  - Added update logic so cues follow the selected creature and refresh from
    `CreatureVisualSnapshot` plus bounded graphical GPU telemetry.

Tests:

- `crates/alife_game_app/tests/app_shell.rs`
  - Added
    `true_25d_player_view_has_display_only_neurochemical_world_cues`.
  - Test asserts six cue roles exist, are True 2.5D assets, are not legacy
    2D sprite fallbacks, are anchored to the selected creature, and are
    display-only/no-action/no-weight-authority.

Docs:

- `docs/creatures_agi_roadmap_pack/status/TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK.md`
  - Records objective, implementation summary, boundaries, evidence, known
    limitations, invariants, and CA44A extension classification for this
    Phase 5 slice.
- `docs/creatures_agi_roadmap_pack/status/TRUE25D_ENDOCRINE_ASSET_FEEDBACK.md`
  - Records the CA44A-ext-05 addendum that moves display-only feedback onto the
    selected creature root through bounded posture/material-shell presentation.
  - This addendum does not advance the CA roadmap and does not unblock CA44.
- `docs/creatures_agi_roadmap_pack/status/TRUE25D_ENDOCRINE_GLTF_FEEDBACK_CONTRACT.md`
  - Records the CA44A-ext-05 addendum that requires active creature GLB files to
    carry matching endocrine-feedback metadata.
  - This addendum proves a versioned display-only GLB contract, not authored
    keyframe animation clips or action-authoritative GPU runtime.

## Checks Already Run

Passed:

```powershell
cargo fmt --all
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d_player_view_has_display_only_neurochemical_world_cues -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell true_25d -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca39 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
cargo fmt --all -- --check
```

Notes:

- The first focused Bevy test run timed out before reporting a result due to
  build/link time. Rerun with a longer timeout passed.
- The first 30-second graphical smoke timed out at the tool budget, but no
  A-Life process was left running. Rerun with a longer timeout passed.
- Default graphical smoke selected `GpuPlastic` on local RTX 3050/Vulkan with
  fallback `None`.
- Forced fallback smoke selected `CpuReference` with
  `HardwareUnavailable` and made no GPU claim.

## Current Branch Validation

The CA44A-ext-05 GLB feedback-contract addendum has rerun the required branch
validation:

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

Result: PASS. The all-features test suite passed without needing the
`CARGO_BUILD_JOBS=1` linker-flake workaround in this run.

Graphical evidence for this addendum:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. The default graphical smoke selected the local `GpuPlastic`
path with fallback `None`; the forced fallback smoke selected `CpuReference`
with `HardwareUnavailable` and made no GPU-performance claim.

## Boundaries And Invariants

- `alife_core` has not been changed.
- No Bevy, wgpu, renderer, app, model-runtime, or tool dependency was added to
  `alife_core`.
- Visual feedback is display-only.
- No action authority was added.
- No weight authority was added.
- No semantic, teacher, topology, memory, UI, or GPU bypass was added.
- CPU fallback remains available.
- CPU shadow parity remains the gate.
- No full action-authoritative GPU runtime claim was added.
- No active bulk neural readback was added.
- No S12, G25, or P37 was created.
- No release tag was created.
- No screenshots, logs, target artifacts, model files, caches, or generated
  media should be committed.
- Blender path was not used in this slice; it remains available for future
  mesh calibration work.

## Resume Steps

1. Confirm branch and worktree:

   ```powershell
   git status --short --branch
   ```

2. Review the passed branch validation listed above.
3. Run Graphify update if available and appropriate after code changes.
4. Review scope and invariants.
5. Commit only the CA44A-ext-05 GLB feedback contract files if validation
   passes:

   ```powershell
   git add crates/alife_game_app/src/true_25d_assets.rs crates/alife_game_app/src/app_bundle_ingestion.rs crates/alife_game_app/src/bevy_shell.rs crates/alife_game_app/src/tests.rs crates/alife_game_app/tests/app_shell.rs crates/alife_game_app/assets/true_25d_alpha_v1/creature_idle.glb crates/alife_game_app/assets/true_25d_alpha_v1/creature_hurt.glb crates/alife_game_app/assets/true_25d_alpha_v1/true_25d_manifest.json docs/creatures_agi_roadmap_pack/status/TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK.md docs/creatures_agi_roadmap_pack/status/TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK_HANDOFF.md docs/creatures_agi_roadmap_pack/status/TRUE25D_ENDOCRINE_GLTF_FEEDBACK_CONTRACT.md
   git commit -m "Add CA44A True 2.5D endocrine GLB feedback contract"
   ```

6. Push/merge only after validation and review pass.
7. Post-merge validate `main` and push `main`.
8. Stop. Do not continue the CA roadmap automatically and do not start another
   True 2.5D extension automatically.
