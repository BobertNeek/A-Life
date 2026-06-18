# alife_game_app

Playable-sim app shell crate.

The default path is headless and validates P34 config/assets without requiring
graphics, GPU, semantic providers, school UI, or Bevy runtime support. The
optional `bevy-app` feature constructs a minimal Bevy app shell with the
existing adapter plugin. G02 adds feature-gated visible placeholder entities
from the P34 portable save, but it still does not run live creature cognition.

CI-safe smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34
```

Feature-gated Bevy construction smoke:

```powershell
cargo test -p alife_game_app --features bevy-app
```

G02 visible-world signature smoke, no graphics required:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- visible-signature crates/alife_world/tests/fixtures/p34
```

G02 feature-gated visible Bevy scene construction:

```powershell
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- visible-world-smoke crates/alife_world/tests/fixtures/p34
```

The visible-world smoke constructs deterministic placeholder entities from the
P34 portable save and binds Bevy entities to stable IDs through the adapter-local
map. It still does not run the G03 live brain/game tick bridge.
