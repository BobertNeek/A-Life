# CA09 - Player-Facing Save/Load Menu

Status: implemented on `codex/CA09-player-facing-save-load-menu`.

CA09 makes save/load accessible from the graphical GPU alpha surface. The
graphical app now exposes a player-facing save/load panel with:

- `M` to open or close the save/load menu.
- `F5` to write the manual save slot.
- `F9` to load the manual save slot.
- Stable-ID slot metadata in the overlay.
- Readable error display for invalid save/schema cases.
- Explicit `partial_load=false` behavior after failed loads.

The implementation reuses the existing P34/G15 portable save contract and
`SaveSlotManager`; it does not introduce a new save schema and does not store
Bevy Entity IDs, GPU handles, renderer handles, or other engine-local IDs.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell ca09 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- graphical-save-load-menu-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Graphical smoke remains:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
```

Forced fallback remains:

```powershell
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Boundaries

- GPU-first presentation is unchanged.
- CPU shadow remains the correctness gate.
- Save/load uses portable stable IDs only.
- Invalid loads do not partially replace the current session.
- No full action-authoritative GPU runtime claim is made.
- No release tag was created.
