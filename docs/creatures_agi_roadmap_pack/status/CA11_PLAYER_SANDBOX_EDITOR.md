# CA11 - Player sandbox editor v1

Status: complete.

CA11 adds a player-facing sandbox editor smoke path over the existing stable-ID
world editor contract. The smoke loads a CA10 environment scenario from the
versioned manifest, proves editing is pause-gated, places and removes food,
hazard, and obstacle markers, and roundtrips the edited scenario through the
portable save contract.

## Command

Run the default GPU alpha sandbox edit smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- player-sandbox-editor-smoke --scenario gpu-alpha
```

Optionally write the edited scenario save to a caller-provided path:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- player-sandbox-editor-smoke --scenario gpu-alpha --output target/playtest_evidence/ca11/edited_save.json
```

The output path is optional and validation does not commit generated saves.

## Behavior

- Known scenarios are selected through `crates/alife_game_app/environment_manifest.json`.
- Editing requires paused/editor mode; live edits are rejected.
- Food, hazard, and obstacle markers can each be placed and removed.
- The edited scenario is saved through the P34 portable save format with stable
  IDs, not Bevy entity IDs.
- Existing G13 editor smoke remains available for lower-level editor contract
  validation.

## Boundaries

- No Bevy/wgpu/GPU dependency changes were added to `alife_core`.
- The editor does not mutate creature cognition directly.
- The command does not claim full action-authoritative GPU runtime.
- CPU fallback and CPU shadow parity remain unchanged.
- No screenshots, logs, target artifacts, or generated saves are committed.

## Focused evidence

Planned CA11 focused commands:

```powershell
cargo test -p alife_game_app ca11 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- player-sandbox-editor-smoke --scenario gpu-alpha
cargo run -p alife_game_app --bin alife_game_app -- player-sandbox-editor-smoke --scenario p34
cargo run -p alife_game_app --bin alife_game_app -- graphical-controls-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Next manifest plan: CA12.
