# S00 Computer-Use Product Playtest Report

Branch: `codex/S00-computer-use-playtest-evidence`

Date: 2026-06-21

Scope: player-facing and dev-facing product surfaces exposed by the current app,
CLI, playground docs, smoke commands, scripts, and graphical path. This is not a
source-function audit and not a new implementation phase.

## Executive Summary

The current A-Life build is **validated and usable as a developer/headless CPU
playground**, but it is **not yet ready as a normal player-facing graphical game**.

Evidence collected in `target/playtest_evidence/S00/` shows:

- Full default and all-features validation passed.
- 38 product/dev-facing CLI commands passed.
- The requested but nonexistent `alife_game_app content-authoring-smoke`
  subcommand was captured as `COMMAND_MISSING`; the actual G16 content-authoring
  CLI passed through `g16_content_authoring validate-pack`.
- The graphical launcher command passed, but it ran `visible-world-smoke`,
  printed a CLI summary, and exited. It did not leave a persistent game window.
- Computer Use was re-run after the local runtime repair. App/window
  enumeration, accessibility inspection, keyboard input, and Alt+PrintScreen
  active-window screenshot fallback worked. The graphical A-Life smoke still
  left no persistent product window to inspect.
- GPU runtime diagnostics selected CPU fallback with `HardwareUnavailable`;
  no GPU hardware performance was measured.

## Environment

See `target/playtest_evidence/S00/environment/environment.md`.

Key local environment:

- OS: Windows 10 Home 10.0.19045.
- CPU: Intel(R) Core(TM) i7-3770K CPU @ 3.50GHz.
- RAM: 31.97 GiB.
- GPU/display: NVIDIA GeForce RTX 3050, driver 32.0.15.8180.
- Screens: 1536x864 primary plus 1280x720 and 1344x840 secondary displays.
- GUI session: available.
- Computer Use: available after local runtime repair. Native WGC screenshot
  remains machine-limited, so active-window evidence uses the Alt+PrintScreen
  clipboard fallback.

## Validation Summary

All validation commands passed. Logs are under
`target/playtest_evidence/S00/logs/validation/`.

| command | result | log |
|---|---|---|
| `cargo fmt --all -- --check` | pass | `target/playtest_evidence/S00/logs/validation/cargo_fmt.log` |
| `cargo check --workspace --all-targets` | pass | `target/playtest_evidence/S00/logs/validation/cargo_check_workspace_all_targets.log` |
| `cargo test --workspace --all-targets` | pass | `target/playtest_evidence/S00/logs/validation/cargo_test_workspace_all_targets.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass | `target/playtest_evidence/S00/logs/validation/cargo_clippy_workspace_all_targets.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1` | pass | `target/playtest_evidence/S00/logs/validation/scripts_check_ps1.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1` | pass | `target/playtest_evidence/S00/logs/validation/scripts_check_core_boundaries_ps1.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1` | pass | `target/playtest_evidence/S00/logs/validation/scripts_docs_check_ps1.log` |
| `cargo tree -p alife_core` | pass | `target/playtest_evidence/S00/logs/validation/cargo_tree_alife_core.log` |
| `cargo check --workspace --all-features --all-targets` | pass | `target/playtest_evidence/S00/logs/validation/cargo_check_workspace_all_features_all_targets.log` |
| `cargo test --workspace --all-features --all-targets` | pass | `target/playtest_evidence/S00/logs/validation/cargo_test_workspace_all_features_all_targets.log` |

## CLI Function Coverage Summary

Full matrix: `docs/productization/S00_FUNCTION_COVERAGE_MATRIX.md`.
Raw logs: `target/playtest_evidence/S00/raw_command_output/`.

| class | count | status |
|---|---:|---|
| Passing product/dev CLI command surfaces | 38 | pass |
| Missing command requested in playtest prompt | 1 | `alife_game_app content-authoring-smoke` is not in current CLI usage |
| Validation commands | 10 | pass |
| Graphical dry-run command | 1 | pass |
| Graphical real launch command | 1 | pass as smoke, unavailable as interactive app |

Notable CLI evidence:

- `release-candidate-smoke` reported zero blockers, no tag, GPU manual/unknown,
  and graphics manual/not measured.
- `product-qa-smoke` reported zero blockers and the current `--gpu-runtime`
  command.
- `p35_playground run-all` reported one sealed patch, school enabled, semantic
  false, CPU reference selected, and six sample paths.
- `benchmark_tiers --gpu-runtime` wrote `gpu_runtime_performance.md`, selected
  `CpuReference`, and recorded `HardwareUnavailable`. This is CPU fallback data,
  not measured GPU performance.

## Graphical And Computer-Use Findings

Computer Use was re-run after the local runtime repair and was functional for
the parts needed in this pass:

- `sky.list_apps()` and `sky.list_windows()` returned targetable windows.
- Accessibility inspection succeeded on a safe File Explorer probe window.
- Keyboard input through Computer Use succeeded with `Alt+PrintScreen`.
- The Alt+PrintScreen clipboard fallback saved a bounded active-window PNG:
  `target/playtest_evidence/S00/screenshots/013_computer_use_alt_printscreen_active_window.png`.
- After the graphical launcher completed, Computer Use found zero targetable
  A-Life/Bevy/playground windows.

Evidence:

- `target/playtest_evidence/S00/raw_command_output/computer_use_after_repair_window_enumeration.log`
- `target/playtest_evidence/S00/raw_command_output/computer_use_after_repair_accessibility.log`
- `target/playtest_evidence/S00/raw_command_output/computer_use_after_repair_post_graphical_windows.log`
- `target/playtest_evidence/S00/raw_command_output/graphical_real_launch_after_computer_use_repair.log`

Native Computer Use screenshot capture was not relied on because this Windows 10
machine still reports a WGC interface limitation. The reliable local path is
Computer Use keyboard input plus Alt+PrintScreen clipboard capture.

Graphical command tested:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1
```

Result: pass as a CLI/Bevy smoke. It printed:

```text
G02 visible world Bevy smoke objects=2 stable_map=2 ground=true ...
```

It then exited with code 0 and left no persistent window. Therefore the
following player-facing interactions were unavailable in the graphical path:

- move camera / pan / zoom / orbit
- select creature
- open inspector/debug overlay
- pause
- step once
- resume/run
- inspect action/drive/sleep state
- trigger save/load UI
- open school/teacher view
- open semantic/GPU/status view
- exit from an interactive app window

Detailed interaction evidence:
`target/playtest_evidence/S00/reports/graphical_interactions.md`.

## Screenshot Table

Full index: `docs/productization/S00_SCREENSHOT_INDEX.md`.

| screenshot | status | meaning |
|---|---|---|
| `target/playtest_evidence/S00/screenshots/001_desktop_before_launch.png` | manual | Actual desktop capture proving GUI session/screen capture fallback. |
| `target/playtest_evidence/S00/screenshots/002_launch_command_or_terminal.png` | manual/unavailable | Placeholder pointing to launch-command log. |
| `target/playtest_evidence/S00/screenshots/003_initial_window_or_failure.png` | unavailable | No persistent graphical app window after launcher. |
| `target/playtest_evidence/S00/screenshots/004_main_scene_visible.png` | unavailable | No main scene visible. |
| `target/playtest_evidence/S00/screenshots/005_camera_or_view_state.png` | unavailable | No camera/view controls testable. |
| `target/playtest_evidence/S00/screenshots/006_creature_visible_or_missing.png` | unavailable | No rendered creature visible. |
| `target/playtest_evidence/S00/screenshots/007_world_objects_food_hazard_visible_or_missing.png` | unavailable | No rendered food/hazard/resource objects visible. |
| `target/playtest_evidence/S00/screenshots/008_inspector_or_debug_overlay.png` | unavailable | No graphical inspector/debug overlay. |
| `target/playtest_evidence/S00/screenshots/009_save_load_or_menu_surface.png` | unavailable | No graphical save/load/menu surface. |
| `target/playtest_evidence/S00/screenshots/010_school_semantic_or_optional_demo_surface.png` | unavailable | No school/semantic UI surface. |
| `target/playtest_evidence/S00/screenshots/011_gpu_fallback_or_status_surface.png` | unavailable | No GPU/status UI surface. |
| `target/playtest_evidence/S00/screenshots/012_exit_or_shutdown_state.png` | pass/manual | Launcher exited cleanly; no window to close. |
| `target/playtest_evidence/S00/screenshots/013_computer_use_alt_printscreen_active_window.png` | pass/manual | Repaired Computer Use active-window screenshot fallback captured File Explorer, not A-Life product UI. |

## Product Playability Answers

| question | answer | evidence |
|---|---|---|
| Does a normal player see a game world? | No. | Graphical launcher exits after CLI smoke; no persistent scene. |
| Is there a controllable camera? | No evidence / unavailable. | No interactive viewport. |
| Is there a visible creature? | No graphical evidence. | CLI creature visual smoke passes; no rendered window. |
| Can the player inspect creature state? | No graphical evidence. | CLI inspector/debug smokes pass; no UI overlay. |
| Can the player see food/hazards/resources? | No graphical evidence. | CLI visible-world/ecology smokes pass; no rendered scene. |
| Can the player pause/step/run? | No graphical evidence. | CLI paused/fixed/live tick smokes pass; no UI controls. |
| Can the player save/load from UI? | No graphical evidence. | CLI save/load UX smoke passes; no menu surface. |
| Can the player understand what happened without reading logs? | No. | Current product evidence is log/CLI centered. |
| Is the graphical path more than a dry-run? | Barely: it runs a Bevy smoke and exits. | `graphical_real_launch.log`. |
| What is still CLI/dev-tooling only? | Most functions: brain tick, visual signatures, inspector, survival, ecology, population, lifecycle, school, semantic, GPU fallback, editor, cognition debug, save/load UX, performance, balance, QA, and release candidate evidence. | Coverage matrix. |

## Findings By Severity

### BLOCKER

None for the currently supported headless CPU product path. Validation and CLI
smoke gates passed.

For a normal player-facing graphical release, the lack of a persistent playable
window would be release-blocking. It is classified below as HIGH because current
project docs still define the supported path as headless CPU plus deterministic
product smoke.

### HIGH

1. No persistent graphical game window is launched.
   - Evidence: `target/playtest_evidence/S00/raw_command_output/graphical_real_launch.log`.
   - Impact: a normal player cannot see or play the game through the graphical
     path.

2. No interactive camera, creature selection, inspector, pause/step/run, or
   menu flow is testable through GUI.
   - Evidence: `target/playtest_evidence/S00/reports/graphical_interactions.md`.
   - Impact: core game UX remains CLI/dev-tooling only.

3. Repaired Computer Use found no targetable A-Life product window after the
   graphical smoke.
   - Evidence: `target/playtest_evidence/S00/raw_command_output/computer_use_after_repair_post_graphical_windows.log`.
   - Impact: the previous Computer Use setup failure is no longer the blocker;
     the product still does not expose an interactive graphical window.

### MEDIUM

1. `alife_game_app content-authoring-smoke` is not a real subcommand.
   - Evidence: `target/playtest_evidence/S00/raw_command_output/alife_game_app_content_authoring_smoke.log`.
   - Mitigation: the actual `g16_content_authoring validate-pack` CLI passed.

2. GPU hardware performance remains unknown.
   - Evidence: `target/artifacts/gpu_runtime_performance.md` reports
     `Backend selected: CpuReference` and `Fallback reason: HardwareUnavailable`.

3. Screenshot evidence includes placeholders for unavailable A-Life UI
   surfaces.
   - Impact: useful for documentation, but not a substitute for real GUI
     screenshots.

### LOW

1. The current graphical script name can over-suggest an interactive playground.
   It is currently a smoke wrapper around `visible-world-smoke`.

2. CLI coverage is broad but player interpretation still requires reading logs.

### MANUAL_EVIDENCE_MISSING

- Real graphical app window.
- Real camera/selection/inspector/menu interaction screenshots.
- Real GPU hardware performance measurements.
- Real A-Life product-window screenshots through Computer Use. Computer Use now
  works, but there was no A-Life window to capture.

## Current Playable Status

- **Headless CPU/dev playground:** ready and validated.
- **Normal graphical game for a player:** not ready.
- **Release/tag:** not recommended from this evidence unless explicitly scoped to
  the headless CPU playground and deterministic product smoke suite.

## Release Blockers

- No blockers for the documented headless CPU supported path.
- Blocker for normal-player graphical release: no persistent interactive game
  window and no GUI controls evidence.

## Manual Evidence Still Missing

- Persistent graphical game window screenshots.
- Camera/selection/inspector/save-load/school/GPU status UI interactions.
- GPU hardware performance with validation flags set and real hardware selected.

## No-Scope-Expansion Checks

- No G25 created.
- No P37 created.
- No release tag created.
- No runtime code changed.
- `target/playtest_evidence/S00/` is untracked evidence only.
