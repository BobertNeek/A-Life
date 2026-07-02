# True 2.5D Neurochemical Visual Feedback Handoff

Created: 2026-07-01

## Current State

- Current branch: `codex/true25d-neurochemical-visual-feedback-phase5`
- Classification: CA44A-ext-05, a CA44A extension slice.
- Base `HEAD`: `118318f`
- `origin/main`: `118318f`
- Working tree: dirty, uncommitted Phase 5 changes are present.
- Branch has not been committed, pushed, merged, or post-merge validated.
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

## Full Validation Still Pending

Full validation was interrupted after `cargo fmt --all -- --check` passed.

Still run before commit/merge:

```powershell
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

Use `CARGO_BUILD_JOBS=1` for all-features tests if the known MSVC linker flake
appears.

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

2. Run the remaining full validation listed above.
3. Run Graphify update if available and appropriate after code changes.
4. Review scope and invariants.
5. Commit the branch if validation passes:

   ```powershell
   git add crates/alife_game_app/src/bevy_shell.rs crates/alife_game_app/tests/app_shell.rs docs/creatures_agi_roadmap_pack/status/TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK.md docs/creatures_agi_roadmap_pack/status/TRUE25D_NEUROCHEMICAL_VISUAL_FEEDBACK_HANDOFF.md
   git commit -m "Add CA44A True 2.5D neurochemical visual feedback"
   ```

6. Push/merge only after validation and review pass.
7. Post-merge validate `main` and push `main`.
8. Stop. Do not continue the CA roadmap automatically and do not start another
   True 2.5D extension automatically.
