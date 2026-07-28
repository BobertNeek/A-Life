# CAR08 Review Report - UI Readability Review

Review: CAR08 - UI readability review
Branch: `codex/CAR08-ui-readability-review`
Verdict: `PASS_WITH_NOTES`
Reviewed tranche: CA05-CA08
Next plan recommendation: CA09 after user/ChatGPT consultation

## Scope Reviewed

CAR08 reviewed the phase-B UI/readability tranche:

- CA05 structured graphical panels
- CA06 camera, mouse selection, follow, and zoom polish
- CA07 inspector bars and readable creature state
- CA08 first sensory feedback layer with display-only VFX/audio-stub cues

The review checked the roadmap reports, plan requirements, implementation surface, tests, validation protocol, and current user-provided screenshots. No CA09 save/load implementation was started.

## Files Inspected

- `docs/creatures_agi_roadmap_pack/review_gates/CAR08_ui-readability-review.md`
- `docs/creatures_agi_roadmap_pack/plans/CA05_mockup-inspired-ui-layer-1-structured-panels.md`
- `docs/creatures_agi_roadmap_pack/plans/CA06_camera-mouse-selection-follow-and-zoom-polish.md`
- `docs/creatures_agi_roadmap_pack/plans/CA07_inspector-bars-and-readable-creature-state.md`
- `docs/creatures_agi_roadmap_pack/plans/CA08_first-sensory-feedback-layer-vfx-and-procedural-audio-stubs.md`
- `docs/creatures_agi_roadmap_pack/status/CA05_STRUCTURED_PANELS.md`
- `docs/creatures_agi_roadmap_pack/status/CA06_CAMERA_SELECTION.md`
- `docs/creatures_agi_roadmap_pack/status/CA07_INSPECTOR_BARS.md`
- `docs/creatures_agi_roadmap_pack/status/CA08_SENSORY_FEEDBACK_CUES.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `docs/creatures_agi_roadmap_pack/GLOBAL_INVARIANTS.md`
- `docs/creatures_agi_roadmap_pack/VALIDATION_PROTOCOL.md`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- User-provided local screenshots in the chat thread, not committed to the repo.

## Commands Run

Read-only inspection commands:

```powershell
git status --short --branch
git rev-parse --short HEAD
git rev-parse --short origin/main
git branch --show-current
rg -n "A-Life GPU Alpha|Read-only Inspector|Visual Guide|Play Feedback|H_shadow|full action|Entity|fallback|Controls|Sensory|pulse|audio" crates/alife_game_app/src/bevy_shell.rs crates/alife_game_app/tests/app_shell.rs
rg -n "S12|G25|P37|full action-authoritative|Bevy Entity|Entity\(" docs/creatures_agi_roadmap_pack crates/alife_game_app/src crates/alife_game_app/tests
```

Focused CA05-CA08 validation:

```powershell
cargo test -p alife_game_app --features bevy-app --test app_shell ca05 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca06 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca07 -- --nocapture
cargo test -p alife_game_app --features bevy-app --test app_shell ca08 -- --nocapture
cargo test -p alife_game_app --test app_shell graphical_controls -- --nocapture
```

Result: PASS.

Full validation:

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

Result: PASS.

Visual smoke validation:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Result: PASS. The normal smoke selected `GpuPlastic` with claim `CpuShadowGuardedStaticPlusLiveHShadow`. The forced fallback smoke selected `CpuReference` with `HardwareUnavailable` and no GPU performance claim.

## Findings By Severity

### BLOCKER

None.

### HIGH

None.

### MEDIUM

None requiring remediation before CA09.

### LOW

1. The current screenshot evidence still shows a dense text-heavy UI surface at the review point. The tranche is no longer a single debug wall, but CA09 and later UI work should keep reducing long technical lines and should reserve detailed GPU/readback/parity language for compact details surfaces.
2. Some screenshot labels overlap the world scene when markers are close together. This is acceptable for the current review gate but should be revisited in later polish/content plans.
3. The current evidence is local/user-provided screenshot evidence, not an independent external human playtest. This does not block CA09, but it should remain explicit in later alpha evidence gates.

### INFO

- The UI keeps the GPU-first identity visible while preserving the boundary that CPU shadow remains the gate.
- CA08 sensory cues are display-only and use stable-ID-derived presentation markers; they do not mutate cognition, issue actions, or rewrite weights.
- The reviewed tranche did not create S12, G25, P37, or a release tag.

## Invariant Status

- `alife_core` boundary: PASS. CA05-CA08 reports and inspected code show the tranche is confined to `alife_game_app` UI/presentation surfaces and tests; no Bevy/wgpu/GPU dependency is introduced into `alife_core`.
- Stable IDs: PASS. Player-facing inspector and overlay tests assert no `Entity(` leakage, and selection/follow are stable-ID based.
- GPU truthfulness: PASS. Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`; the UI and telemetry still state that CPU shadow remains the gate and do not claim full action-authoritative runtime.
- CPU fallback: PASS. Fallback remains available and visibly degraded/safety-scoped rather than removed.
- Learning boundary: PASS. CA08 cues are display-only. H_shadow/lifetime learning remains governed by existing post-seal contracts; the cue layer does not alter learning.
- Artifact policy: PASS. No screenshots, logs, captures, target artifacts, or large assets are tracked by this review.

## User-Facing Status

The graphical surface now presents a recognizable alpha playground instead of a single debug text wall:

- title/status panel is visible,
- GPU mode/fallback boundary is visible,
- read-only stable-ID inspector is visible,
- creature, food, hazard, and feedback cue markers are visible,
- bottom controls/guide and play feedback panels are visible,
- CPU-shadow gate and no full action-authoritative claim remain visible.

The current surface is usable enough to proceed to CA09, but it still reads like a technical alpha. Future plans should continue moving debug detail out of the default player view.

## Evidence Gaps

- No independent human external tester evidence was captured during CAR08.
- Screenshot evidence confirms visibility but not full manual input comfort.
- The current screenshots show one paused/early-tick state; longer unscripted visual behavior should be rechecked as the save/load and configuration surfaces arrive.

## Fix Prompt If Needed

No fix prompt is required before CA09. If ChatGPT/user wants to turn the LOW notes into immediate cleanup, use this narrow prompt:

```text
Apply only CAR08 LOW readability cleanup. Do not start CA09. Keep GPU-first UI semantics, stable IDs, CPU shadow gate, CPU fallback, and no full action-authoritative claim. Shorten dense technical overlay lines, reduce world-label overlap where low-risk, and update only tests/docs needed for those readability refinements. Use Windows wrapper validation and do not create S12/G25/P37 or a release tag.
```

## Consultation Packet

Commits reviewed:

- CA05-CA08 tranche as merged into `main` through `fe52415`.

Files changed by CAR08:

- `docs/creatures_agi_roadmap_pack/reviews/CAR08_REVIEW_REPORT.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

Validation:

- Full validation commands passed on the CAR08 branch and are rerun after merge to `main`.

Known limitations:

- UI remains technical-alpha, not final player UX.
- Independent human external tester evidence is still pending for this tranche.
- GPU runtime remains CPU-shadow guarded and not full action-authoritative.

Disputed decisions:

- None. The review treats remaining readability issues as LOW notes, not blockers.

Recommendation:

- Continue to CA09 after user/ChatGPT consultation. CA09 should implement the player-facing save/load menu without expanding the GPU claim or weakening validation.

Prompt for user/ChatGPT:

```text
Review docs/creatures_agi_roadmap_pack/reviews/CAR08_REVIEW_REPORT.md. Decide whether CAR08 PASS_WITH_NOTES is acceptable to proceed to CA09, or whether the LOW readability notes must be fixed before CA09. Do not request CA09 implementation unless you explicitly accept the CAR08 gate.
```

## Verdict

`PASS_WITH_NOTES`

CA09 may proceed only after the CAR08 consultation stop is acknowledged by the user/ChatGPT.
