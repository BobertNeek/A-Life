# G20 Onboarding, Help, Tutorials, and Player Documentation

This guide is the current first-run path for the playable-sim phase. It is deliberately headless-first: the CPU/reference path should work without GPU hardware, Bevy graphics, a semantic provider, or school UI.

## First Run

From the repository root:

```powershell
cargo run -p alife_tools --bin p35_playground -- run-headless crates/alife_world/tests/fixtures/p34
```

Expected result: a deterministic tiny world report with at least one sealed patch and one packed log. This proves the playable shell can load P34 assets/configs, run a creature tick, and summarize the result without optional providers.

## Controls Reference

The current product surface exposes these controls as deterministic state or smoke surfaces:

| Control | What It Means | Evidence |
|---|---|---|
| Pause | Stop automatic tick progression before inspection | G03/G05 paused tick tests |
| Step | Advance one deterministic headless brain/world tick | G03 live-brain bridge |
| Run | Resume deterministic ticking after pause | G05 camera/inspector controls |
| Select | Inspect a stable `WorldEntityId`, never an engine-local entity | G02/G05 stable-ID selection |
| Inspect | Read drives, hormones, action, sleep state, and last sealed patch | G05 inspector and G14 cognition debug |
| Save/Load | Validate P34 stable IDs, schemas, and asset manifests | G15 save/load UX |

The default headless path is the reliable onboarding path. Graphics/Bevy controls are optional/manual until a local graphics environment is available.

## Tutorial Script

The committed tutorial script is:

```text
examples/g20/tutorial_food_hazard_sleep_inspection.json
```

Run the focused tutorial commands:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- onboarding-help-smoke
cargo run -p alife_game_app --bin alife_game_app -- playable-survival-loop-smoke
cargo run -p alife_game_app --bin alife_game_app -- creature-inspector-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
```

The tutorial covers food reward, missing affordance failure, hazard pain, rest/sleep, read-only inspection, and G19 balance reporting.

## Optional Demos

Optional demos are still optional:

```powershell
cargo run -p alife_tools --bin p35_playground -- school-demo
cargo run -p alife_tools --features semantic-demo --bin p35_playground -- semantic-demo
cargo run -p alife_tools --bin p35_playground -- gpu-fallback
```

GPU fallback may honestly report CPU fallback unless runtime feature, availability, and validation flags are set. Do not treat CPU fallback output as measured GPU performance.

## Troubleshooting

| Symptom | What To Check | Command |
|---|---|---|
| GPU unavailable or unvalidated | CPU fallback should be selected and no GPU performance claim should be made | `cargo run -p alife_tools --bin p35_playground -- gpu-fallback` |
| Graphics unavailable | Use the headless path; graphics smoke remains optional/manual | `cargo run -p alife_tools --bin p35_playground -- run-headless crates/alife_world/tests/fixtures/p34` |
| Schema mismatch or missing asset | Validate the manifest before running optional demos | `cargo run -p alife_tools --bin p35_playground -- validate-manifest examples/p35/playground_manifest.json` |
| Windows script invokes WSL | Use the Git Bash PowerShell wrappers | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1` |
| Balance looks scripted | Read G19 known limitations; the smoke exposes metrics instead of hiding them | `cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke` |

Windows validation wrappers:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

## Content Extension Path

G16 content authoring remains the supported way to add small worlds, lessons, and creature presets:

```powershell
cargo run -p alife_tools --bin g16_content_authoring -- validate-pack content/fixtures/g16/content_pack_manifest.json
cargo run -p alife_tools --bin g16_content_authoring -- validate-world content/fixtures/g16/worlds/tiny_meadow_world.json
cargo run -p alife_tools --bin g16_content_authoring -- validate-lesson content/fixtures/g16/lessons/grounded_food_lesson.json
cargo run -p alife_tools --bin g16_content_authoring -- validate-creature content/fixtures/g16/creatures/nano_forager.json crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json
```

Content files must remain versioned, small, stable-ID based, and perception-only where they describe lessons.
