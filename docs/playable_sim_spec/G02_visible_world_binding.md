# G02 Visible World Binding

G02 adds a deterministic, feature-gated visible-world presentation layer in
`alife_game_app`.

The default path remains headless and CI-safe. The visible Bevy scene is behind
the existing `bevy-app` feature and uses adapter-local `BevyEntityMap` bindings
from stable `WorldEntityId` values to Bevy `Entity` handles. Bevy entities are
not serialized and do not enter `alife_core`.

## Data Source

The visible presentation loads the committed P34 tiny fixture:

- `crates/alife_world/tests/fixtures/p34/tiny_config.json`
- `crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json`
- `crates/alife_world/tests/fixtures/p34/tiny_save.json`

The portable save remains the source of stable object IDs, labels, positions,
kinds, and organism IDs. The app restores the corresponding headless world and
compares object counts/signatures before spawning visible placeholders.

## Placeholder Presentation

G02 defines simple debug-safe placeholder presentation metadata:

- ground: ground plane
- agent: creature capsule
- food: sphere
- hazard: cone
- obstacle: cube
- token: billboard

These are lightweight app/adapter presentation records. G02 does not implement
final creature rendering, animation, camera UX, live cognition stepping, or the
G03 live brain/game tick bridge.

## Smoke Commands

Headless signature, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- visible-signature crates/alife_world/tests/fixtures/p34
```

Feature-gated Bevy scene construction:

```powershell
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- visible-world-smoke crates/alife_world/tests/fixtures/p34
```

The Bevy smoke command constructs the app, spawns deterministic placeholder
entities, binds stable IDs through the adapter-local map, and updates once. It
does not require a product window or hardware GPU evidence.
