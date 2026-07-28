# CA02 GPU Alpha Fixture

Status: CA02 complete.

## Scope

CA02 makes the default GPU alpha fixture a tiny deterministic world with all
four first-playtest markers: creature, food, hazard, and obstacle. The change is
fixture/content and presentation evidence only. It does not change neural
runtime authority, action arbitration, save schema, or `alife_core`.

## Changes

- `crates/alife_world/tests/fixtures/gpu_alpha/tiny_save.json` now contains:
  - creature `stable:1`,
  - food `stable:2`,
  - hazard `stable:3`,
  - obstacle `stable:4`.
- The Bevy alpha overlay and launcher text now describe the GPU alpha fixture as
  creature + food + real hazard + obstacle.
- Productization docs and first-tester checklist now ask testers to look for
  both hazard and obstacle markers.
- The existing visible-world fixture regression now asserts one agent, one
  food, one hazard, one obstacle, and stable IDs `[1, 2, 3, 4]`.

## Evidence

Focused regression:

```powershell
cargo test -p alife_game_app --test app_shell gpu_alpha_fixture -- --nocapture
```

Focused smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- visible-world-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Graphical evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback evidence:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Boundaries

- P34 remains unchanged for compatibility and headless regression coverage.
- The new obstacle uses stable ID `4`; no Bevy entity IDs are persisted or shown
  as player-facing identifiers.
- Bevy visuals remain presentation-only.
- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU shadow parity remains the GPU proposal gate.
- No full action-authoritative GPU runtime is claimed.
- No screenshots, logs, target artifacts, release tags, S12, G25, or P37 were
  created.

## Remaining Work

- CA03 owns visible intent, movement, targeting, and interaction feedback.
- CA04 owns reset/terminal-state stability beyond the existing smoke path.
