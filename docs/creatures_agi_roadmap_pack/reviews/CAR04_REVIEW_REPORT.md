# CAR04 Review Report

Verdict: PASS_WITH_NOTES

Review: CAR04 - Playable loop review before UI/content expansion
Branch: codex/CAR04-playable-loop-review
Reviewed baseline: 9898cd3
Date: 2026-06-25

## Scope reviewed

CAR04 reviewed the completed CA01-CA04 tranche:

- CA01 - GPU-first visible playable loop
- CA02 - GPU alpha fixture with creature, food, hazard, and obstacle
- CA03 - Visible intent, movement, targeting, and interaction feedback
- CA04 - Reset, terminal-state recovery, and alpha run-loop stability

The review question was whether a first user can press Space or N and see meaningful creature/world changes before expanding UI/content further.

## Files inspected

Primary tranche files inspected:

- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/gpu_live_runtime.rs`
- `crates/alife_game_app/src/interactive_runtime.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_world/tests/fixtures/gpu_alpha/tiny_save.json`
- `scripts/run_graphical_playground.ps1`
- `docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_CHECKLIST.md`
- `docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_REPORT.md`
- `docs/productization/GPU_FIRST_PLAYABLE_ALPHA_REPORT.md`
- `docs/productization/GRAPHICAL_GPU_PLAYABILITY_REPORT.md`
- `docs/creatures_agi_roadmap_pack/status/CA01_GPU_FIRST_VISIBLE_PLAYABLE_LOOP.md`
- `docs/creatures_agi_roadmap_pack/status/CA02_GPU_ALPHA_FIXTURE.md`
- `docs/creatures_agi_roadmap_pack/status/CA03_VISIBLE_INTENT_FEEDBACK.md`
- `docs/creatures_agi_roadmap_pack/status/CA04_RESET_TERMINAL_RECOVERY.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

Instruction and invariant files inspected:

- `AGENTS.md`
- `docs/AGENTS.md`
- `crates/alife_game_app/AGENTS.md`
- `crates/alife_world/AGENTS.md`
- `crates/alife_core/AGENTS.md`
- `docs/master_spec.md`
- `docs/architecture_decisions.md`
- `docs/creatures_agi_roadmap_pack/plan_manifest.json`
- `docs/creatures_agi_roadmap_pack/GLOBAL_INVARIANTS.md`
- `docs/creatures_agi_roadmap_pack/VALIDATION_PROTOCOL.md`
- `docs/creatures_agi_roadmap_pack/review_gates/CAR04_playable-loop-review-before-ui-content-expansion.md`

## Commands run

Repository/audit commands:

```powershell
git status --short --branch
git rev-parse --short HEAD
git rev-parse --short origin/main
git branch --show-current
git diff --name-only 18e0d46..HEAD
git diff --stat 18e0d46..HEAD
git log --oneline --merges -n 8
rg -n "S12|G25|P37|release tag|full action-authoritative|Entity\(" docs/creatures_agi_roadmap_pack crates/alife_game_app/src crates/alife_game_app/tests/app_shell.rs
```

Focused validation:

```powershell
cargo test -p alife_game_app --test app_shell graphical_runtime_overlay_is_gpu_first_without_false_pretick_events -- --nocapture
cargo test -p alife_game_app --test app_shell gpu_alpha_fixture -- --nocapture
cargo test -p alife_game_app --test app_shell graphical_runtime_event_feed_keeps_last_five_meaningful_events -- --nocapture
cargo test -p alife_game_app --test app_shell ca04 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Standard validation:

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

Result: all CAR04 branch commands passed.

## Findings by severity

BLOCKER: none.

HIGH: none.

MEDIUM: none.

LOW:

- CAR04-L1: The current UI is good enough to prove Space/N-visible change, but still uses technical overlay density and large text panels. This matches the roadmap boundary; CA05 owns structured panel UI.
- CAR04-L2: CA03's food/hazard cue labels are tied to deterministic GPU alpha fixture stable IDs (`stable:2` food and `stable:3` hazard). This is acceptable for the CA01-CA04 tranche; broader kind-driven targeting belongs in later runtime/UI work.
- CAR04-L3: CAR04 evidence is automated/local smoke evidence plus existing screenshot context. No new independent human external playtest was collected during this review gate.

INFO:

- The supplied visual evidence shows the alpha playground window opens with GPU mode visible, stable-ID markers, food/hazard/object labels, controls text, and read-only inspector panels. It also shows why CA05 should focus on structured panels and layout readability.

## Invariant status

PASS:

- `alife_core` was not modified by CA01-CA04.
- `cargo tree -p alife_core` remains dependency-clean and does not include Bevy, wgpu, renderer, GPU backend, semantic provider, or game-app dependencies.
- Stable IDs remain the player-facing identifier in overlay/inspector text; tests reject Bevy `Entity(` leakage.
- Bevy visuals remain presentation-only mirrors of runtime panel state.
- GPU mode remains `CpuShadowGuardedStaticPlusLiveHShadow`; the UI and telemetry keep the CPU shadow gate visible and do not claim full action-authoritative runtime.
- CPU fallback remains available and visibly degraded rather than silent.
- No active bulk neural readback was added.
- No release tag, S12, G25, or P37 was created.
- No screenshots, logs, benchmark artifacts, target artifacts, or large generated assets were committed.

## User-facing status

PASS_WITH_NOTES:

- A first user can launch the GPU alpha playground and see a persistent graphical window.
- The window shows creature, food, hazard, and obstacle presentation markers in the GPU alpha fixture.
- Pressing N / deterministic StepOnce advances a live tick, updates selected action/target, records an event feed, seals a patch, and keeps stable-ID text.
- Pressing Space / deterministic run control changes playback/run behavior and produces ticks in the control smoke.
- Pressing R / deterministic reset clears stale action/patch state, preserves stable IDs, and records restart guidance.
- Terminal-invalid or runtime failure guidance is player-visible: `Simulation stopped: <cause>. Press R to restart.`

The loop is playable enough to proceed into CA05 UI/content expansion after consultation. It is not yet polished enough to call a player-ready product surface.

## Evidence gaps

- No independent human external tester evidence was collected in CAR04.
- The current overlay can still feel like a technical dashboard; this should be handled by CA05 and later UI polish plans, not by expanding CAR04.
- The visible world currently proves a tiny deterministic alpha fixture, not broad ecology or unscripted behavior. Those are later roadmap phases.
- GPU path remains CPU-shadow guarded. It is intentionally not full action-authoritative.

## Fix prompt if needed

No fix prompt is required for BLOCKER/HIGH/MEDIUM issues because none were found.

Optional LOW cleanup prompt for CA05 planning context:

```text
Use docs/creatures_agi_roadmap_pack/reviews/CAR04_REVIEW_REPORT.md as input for CA05. Keep the GPU-first semantics and stable-ID boundaries intact, but replace the dense debug-dashboard feel with structured, readable first-player panels. Do not create S12/G25/P37, do not claim full action-authoritative GPU runtime, and preserve CPU shadow parity and fallback behavior.
```

## Next plan recommendation

Next executable plan: CA05 - Mockup-inspired UI layer 1: structured panels.

Hard stop: yes. Do not start CA05 until the user/ChatGPT consultation explicitly approves continuing past CAR04.

Suggested consultation handoff:

```text
Please review docs/creatures_agi_roadmap_pack/reviews/CAR04_REVIEW_REPORT.md.
CAR04 verdict is PASS_WITH_NOTES.
Can Codex proceed to CA05 - Mockup-inspired UI layer 1: structured panels?
```
