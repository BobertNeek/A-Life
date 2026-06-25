# CA10 - Configuration-driven environment launcher

Status: complete.

CA10 adds a versioned app-owned environment manifest at
`crates/alife_game_app/environment_manifest.json`. The graphical launcher can now
select known alpha scenarios by ID instead of requiring users or scripts to pass
raw fixture paths.

## Manifest

- Schema: `alife.ca10.environment_manifest.v1`
- Default scenario: `gpu-alpha`
- Included scenarios:
  - `gpu-alpha` - player-facing GPU alpha fixture with creature, food, hazard,
    and obstacle markers.
  - `p34` - legacy persistence fixture for headless compatibility validation.

## Commands

List known scenarios:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- list-environments
```

Validate/select a scenario:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- environment-launch-smoke --scenario gpu-alpha
```

Launch the graphical alpha through the manifest:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -Scenario gpu-alpha -GpuMode static-plastic-cpu-shadow-guarded
```

Legacy fixture-root CLI arguments remain supported for compatibility, but the
Windows launcher no longer hardcodes the fixture path.

## Boundaries

- No runtime behavior is made GPU-mandatory.
- CPU fallback and CPU shadow parity remain intact.
- `alife_core` has no Bevy/wgpu/GPU dependency changes.
- Scenario IDs are player-readable and bad selections report known scenario IDs.

## Focused evidence

Planned CA10 focused commands:

```powershell
cargo test -p alife_game_app ca10 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- list-environments
cargo run -p alife_game_app --bin alife_game_app -- environment-launch-smoke --scenario gpu-alpha
cargo run -p alife_game_app --bin alife_game_app -- environment-launch-smoke --scenario p34
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun -Scenario gpu-alpha -GpuMode static-plastic-cpu-shadow-guarded
```

Next manifest plan: CA11.
