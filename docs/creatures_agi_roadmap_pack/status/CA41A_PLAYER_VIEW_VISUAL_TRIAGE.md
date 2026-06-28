# CA42A Player View Visual Triage

Plan: CA42A - Player-view visual triage before CA43
Branch: `codex/CA42A-player-view-visual-triage`

## Summary

The default graphical playground now starts in Player View instead of presenting
the full internal diagnostic dashboard. Player View keeps the GPU alpha world
visible, collapses event/debug text, hides world-space stable-ID labels, hides
topology/social lines, and preserves a compact HUD, creature inspector, controls
strip, and GPU/fallback chip.

Dev Overlay and Full Debug remain available through explicit launch view modes.
The smoke summary reports Player View screenshot-acceptance fields so default
presentation regressions are visible in command output.

## Files Changed

- `crates/alife_game_app/src/graphical_playground.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/gpu_graphics_performance.rs`
- `crates/alife_game_app/src/tests.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `scripts/run_graphical_playground.ps1`
- `docs/creatures_agi_roadmap_pack/status/CA41A_PLAYER_VIEW_VISUAL_TRIAGE.md`

## Runtime Code Changed

Yes. The change is presentation-only:

- added `GraphicalPlaygroundViewMode` with `player`, `dev-overlay`, and
  `full-debug`;
- made Player View the default launch/smoke mode;
- gated world-space debug labels, action badges, intent/social lines, and
  teacher debug labels behind explicit diagnostic modes;
- collapsed default event and HUD text;
- reduced terrain wash opacity so the map reads as world art rather than a
  debug heatmap.

Simulation semantics, action authority, GPU/CPU correctness, CPU fallback, CPU
shadow parity, and save data are unchanged.

## Public APIs Changed

Yes, app-level launch API only:

- `GraphicalPlaygroundLaunchConfig` now carries `view_mode`;
- `GraphicalPlaygroundLaunchSummary` now carries `view_mode` and
  `player_view_acceptance`;
- `graphical-playground` accepts `--view-mode player|dev-overlay|full-debug`;
- `scripts/run_graphical_playground.ps1` accepts `-ViewMode`.

No `alife_core` APIs changed.

## Tests Added/Changed

- Added CA42A tests for default Player View acceptance.
- Added CA42A tests proving Dev Overlay and Full Debug remain available.
- Added CA42A tests proving compact Player HUD hides patch/claim/stable-ID
  spam.
- Updated existing graphical UI tests to use Full Debug helper when checking
  diagnostic text.

## Focused Evidence

Focused CA42A tests passed:

```powershell
cargo test -p alife_game_app --features "bevy-app gpu-runtime" --test app_shell ca42a -- --nocapture
cargo test -p alife_game_app --features "bevy-app gpu-runtime" --test app_shell first_graphical_alpha_playtest_docs_and_launcher_are_current -- --nocapture
cargo test -p alife_game_app --features "bevy-app gpu-runtime" --test app_shell graphical_gpu_launch_config_defaults_gpu_first_and_preserves_cpu_choice -- --nocapture
```

Graphical smoke evidence passed:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded -ViewMode dev-overlay
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

The default Player View smoke summary reported:

- `view_mode=player`;
- `dev_overlay_hidden=true`;
- `full_debug_hidden=true`;
- `event_feed_collapsed=true`;
- `stable_labels_hidden=true`;
- `terrain_alpha_max=0.16`;
- `internal_spam_hidden=true`;
- `topology_lines_hidden=true`;
- `teacher_debug_hidden=true`.

A local untracked visual check was captured at
`target/playtest_evidence/CA42A/player_view_after_triage_bevy_late.png` and
confirmed that the default view now prioritizes terrain/object presentation over
the previous debug dashboard.

Full validation passed before merge:

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

## Invariant Checks

- No simulation semantics change.
- No action-authority change.
- CPU fallback preserved and made explicit.
- CPU shadow parity preserved.
- No full action-authoritative GPU claim.
- No Bevy/wgpu/GPU dependency leak into `alife_core`.
- Stable IDs remain portable; Bevy Entity IDs remain hidden from
  player-facing text.
- Dev/Full Debug diagnostics remain opt-in.

## Known Limitations

- Player View still uses placeholder 2D primitives and procedural dressing; it
  is a cleaner alpha presentation, not finished art.
- Dev Overlay and Full Debug are launch-time modes, not in-window hot toggles.
- CA43 remains unstarted.

## Artifacts / Release

- No screenshots, logs, target artifacts, model files, or cache files should be
  tracked.
- No release tag created.

## Main Status

Validation passed. Merge to `main` is pending.

Next plan remains CA43 after explicit continuation. This branch stops before
CA43.
