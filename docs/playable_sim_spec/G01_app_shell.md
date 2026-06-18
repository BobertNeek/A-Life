# G01 Graphical App Shell

Status: G01 app-shell implementation notes.

## Crate Choice

G01 uses a dedicated `alife_game_app` crate instead of adding another
`alife_tools` command or a loose example. The product app shell is runtime
startup policy, not offline tooling, and it should become the natural owner for
later G02+ visible-world and product-launcher work.

The default `alife_game_app` path remains headless and CI-safe. The Bevy
construction path is feature-gated behind `bevy-app`.

## App States

The shell defines an explicit app-state trace:

- `Boot`
- `LoadConfig`
- `DevMenu`
- `Running`
- `Paused`
- `Shutdown`

The state machine is deterministic and rejects invalid transitions. It is a
startup shell only; it does not spawn visible world content or run gameplay
systems.

## Config and Asset Loading

The shell loads and validates P34 runtime config and asset manifests through
the existing `alife_world::persistence` APIs:

- runtime config: `RuntimeConfig::from_json_file` and `validate`
- asset manifest: `AssetManifest::from_json_file` and `validate_with_root`

This preserves stable-ID and asset-reference policy. The app shell does not
serialize engine-local IDs or bypass P34 validation.

## CI-Safe Commands

Default headless startup smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34
```

Explicit config/manifest validation:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- validate-config crates/alife_world/tests/fixtures/p34/tiny_config.json crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json crates/alife_world/tests/fixtures/p34
```

Focused test gate:

```powershell
cargo test -p alife_game_app --all-targets
```

## Feature-Gated Bevy Shell

The `bevy-app` feature compiles a minimal Bevy app with `MinimalPlugins` and
the existing `AlifeBevyAdapterPlugin`.

```powershell
cargo test -p alife_game_app --features bevy-app --all-targets
```

Manual Bevy construction smoke:

```powershell
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- bevy-smoke crates/alife_world/tests/fixtures/p34
```

This path constructs the app shell and adapter scheduling resources. It does
not claim product rendering, camera, visible world binding, creature rendering,
input UX, or gameplay. Those remain G02+ work.

## Boundaries

- `alife_core` is unchanged and remains engine-independent.
- Bevy and adapter types stay out of `alife_core`.
- GPU, semantic, school, and graphics paths remain optional.
- Headless CPU startup remains the default validation path.
- No gameplay, visible world content, save slot UX, release packaging, or G02+
  behavior is implemented in G01.
