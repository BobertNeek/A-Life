# alife_game_app

G01 playable-sim app shell crate.

The default path is headless and validates P34 config/assets without requiring
graphics, GPU, semantic providers, school UI, or Bevy runtime support. The
optional `bevy-app` feature constructs a minimal Bevy app shell with the
existing adapter plugin, but it still does not spawn visible gameplay content.

CI-safe smoke:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34
```

Feature-gated Bevy construction smoke:

```powershell
cargo test -p alife_game_app --features bevy-app
```
