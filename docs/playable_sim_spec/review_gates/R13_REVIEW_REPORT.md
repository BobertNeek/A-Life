# R13 Review Report - Retrospective Product Boundary Review

Review: R13 - Retrospective product boundary review after G13 before G14
Branch: codex/R13-retrospective-product-boundary-review
Date: 2026-06-18
Verdict: FIX_REQUIRED

## Summary

R13 reviewed G01-G13 after the missed G03/G06/G12 human checkpoints. The implemented product surfaces generally preserve the required backend boundaries: headless execution stays available, visible world and editor surfaces use stable IDs, school and semantic inputs remain perception/context only, GPU product paths remain optional and fallback-safe, and `alife_core` remains engine-independent.

G14 should not start yet. The accumulated `alife_game_app` implementation has become too large and monolithic for another major visualization/debugging feature. A behavior-preserving module split is required before G14 so cognition visualization does not turn the app crate into a god object.

G14 may proceed: no
Module split required before G14: yes

## Findings By Severity

### HIGH

R13-HIGH-001 - `crates/alife_game_app/src/lib.rs` is too large and monolithic for G14.

Evidence:
- `crates/alife_game_app/src/lib.rs` is 6001 lines and about 220 KiB.
- The single file contains schema constants and implementation surfaces for G01 through G13: app shell, visible world presentation, live brain bridge, creature visuals, camera/inspector, survival loop, ecology loop, population/social loop, lifecycle/lineage, school mode, semantic provider display, GPU product telemetry, world editor, and Bevy feature resources.
- The R13 gate explicitly asks whether `alife_game_app` module organization is still maintainable before G14. G14 would add cognition visualization and a debug timeline, which would worsen this concentration of unrelated surfaces.

Required remediation:
- Split `crates/alife_game_app/src/lib.rs` into focused modules before G14.
- Preserve existing public APIs, CLI behavior, schemas, tests, and feature gates.
- Do not implement G14 during the split.

### MEDIUM

None.

### LOW

R13-LOW-001 - The user-requested `docs/playable_sim_spec/03_global_invariants.md` and `docs/playable_sim_spec/04_validation_and_release_policy.md` paths do not exist in the imported v2 pack.

Evidence:
- The canonical imported equivalents are `docs/playable_sim_spec/GLOBAL_INVARIANTS.md` and `docs/playable_sim_spec/VALIDATION_PROTOCOL.md`.

Impact:
- No product boundary issue was found. This is a naming mismatch in the R13 prompt source list.

### INFO

R13-INFO-001 - Graphify could not be used through the CLI.

Evidence:
- `graphify-out/graph.json` exists, but the `graphify` CLI was not on PATH in this session.
- Direct inspection of `graphify-out/graph.json` showed existing crate nodes, but the graph did not include the newer playable-sim `alife_game_app`/G-plan surfaces, so direct source inspection was authoritative for R13.

## Review Checklist

| Check | Status | Evidence |
|---|---|---|
| G01 app shell remains feature-gated and headless-safe | PASS | `alife_game_app` default features are empty; `bevy-app` and `gpu-runtime` are optional; app tests cover headless config startup. |
| G02 visible world uses stable IDs and no engine-local persistence | PASS | Visible world structs use `WorldEntityId`; P34 persistence stores stable IDs and adapter remap tables, not engine handles. |
| G03 live brain loop bridge preserves CPU oracle and sealed patch order | PASS | App tests assert sealed live-brain patches and compatibility with manual headless CPU ticks. |
| G04 creature visuals are presentation only and do not mutate cognition | PASS | Visual snapshots are presentation resources; no cognition mutation path was found in visual smoke surfaces. |
| G05 inspector is read-only and stable-ID based | PASS | App tests assert `inspector.read_only`, stable selected IDs, sealed patch summaries, and local Bevy mapping outside the model. |
| G06 survival loop is not overclaimed as full gameplay | PASS | G00 and docs state the repo is still a scaffold/reference implementation; survival smoke exercises a loop without claiming final gameplay. |
| G07 ecology is deterministic/bounded and not unbounded simulation | PASS | Ecology smoke uses deterministic fixtures, bounded resource metrics, and explicit validation. |
| G08 population/social loop stays bounded and perception/modulatory | PASS | Tests cover population cap, stable order, social context as perception, and no social direct action count. |
| G09 lifecycle/lineage keeps genetic/lifetime separation | PASS | Tests assert genetic baseline immutability and no inherited lifetime state by default. |
| G10 school mode remains perception-only and verifier uses sealed patches | PASS | Tests assert teacher cue perception-only semantics, no direct motor bypass, and verifier sealed patch counts. |
| G11 semantic/SLM provider is optional, bounded, non-authoritative, cannot act, cannot mutate weights | PASS | Tests assert disabled provider safety, fake provider bounded context, no semantic action bypass, and no weight rewrite. |
| G12 GPU product hardening is optional/fallback-safe, no active neural readback, no false hardware claims | PASS | Tests assert CPU fallback by default, invalid GPU config fallback, no active readback, and current `--gpu-runtime` manual command. |
| G13 world editor uses stable IDs, bounded edits, P34 save/load, no cognition mutation | PASS | World editor smoke validates stable IDs, edit caps/rejection, P34 portable save round-trip, sealed resumed patch, and zero cognition direct mutation. |
| `alife_game_app` module organization is still maintainable | FIX_REQUIRED | `src/lib.rs` is 6001 lines and contains G01-G13 surfaces in one file. Split is required before G14. |
| No P37 exists | PASS | Tracked-file search found no P37 plan file. |
| P36 gates remain intact | PASS | `docs/release_checklist.md`, `docs/final_status_report.md`, and Windows wrapper docs remain present and current. |
| `alife_core` remains engine-independent | PASS | `crates/alife_core/Cargo.toml` has no Bevy, Avian, wgpu, renderer, tooling, or app dependencies; boundary validation is required below. |

## Exact Fix Prompt

Use this prompt before starting G14:

```text
You are applying the R13 remediation before G14.

Do not start G14.
Do not implement cognition visualization or debug timelines.
Do not create P37.
Do not change runtime behavior.
Do not change public APIs unless required solely to preserve existing exports after module splitting.
Do not edit alife_core.
Do not weaken tests.

Task:
Perform a behavior-preserving module split of crates/alife_game_app/src/lib.rs before G14.

Scope:
- Split the current monolithic lib.rs into focused modules for existing G01-G13 surfaces.
- Suggested modules:
  - app_shell
  - visible_world
  - live_brain_bridge
  - creature_visuals
  - camera_inspector
  - survival_loop
  - ecology_loop
  - population_social
  - lifecycle_lineage
  - school_mode
  - semantic_provider_display
  - gpu_product_telemetry
  - world_editor
  - bevy_shell
- Keep lib.rs as a small module/export hub.
- Preserve all existing public names, schemas, CLI commands, tests, feature gates, and docs.
- Do not move Bevy/GPU dependencies into alife_core.
- Do not implement G14.

Validation:
Run on Windows without plain bash:
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo test -p alife_game_app --test app_shell
cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
cargo run -p alife_game_app --bin alife_game_app -- world-editor-smoke

Receipt:
R13 remediation receipt
Files changed:
Module split summary:
Public APIs preserved:
Commands run:
Results:
Invariant checks:
G14 readiness:
Stopped before G14: yes
```

## Validation Commands Run

R13 branch validation:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo test -p alife_game_app --test app_shell
cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
cargo run -p alife_game_app --bin alife_game_app -- world-editor-smoke
```

Main merge validation repeats the same required command set after merge.

## Known Limitations

- R13 did not implement remediation. It produced the required review report and exact fix prompt only.
- R13 did not run or start G14.
- GPU hardware performance remains manual/unknown unless the documented hardware flags and validation are explicitly enabled.
- Graphify CLI was unavailable in this session, so direct source inspection was used for the new playable-sim surfaces.

## Recommendation For G14

Do not start G14 until the R13 module-split remediation has passed validation. After the split, G14 can add cognition visualization/debug timeline surfaces on top of focused app modules without extending the current monolith.
