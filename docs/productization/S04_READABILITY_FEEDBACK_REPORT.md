# S04 Readability And Feedback Report

Status: implemented on `codex/S04-world-readability-feedback`.

## Scope

S04 improves the graphical playground's presentation layer only. It does not
change cognition, action arbitration, save/load schema, GPU policy, or
`alife_core`.

## Visual Blueprint

Generated implementation reference:

- `C:\Users\PC\.codex\generated_images\019eaf55-9e91-75b0-bedf-5c6bbb6ea17e\ig_04e8508f269586cd016a389fca8418819ab44f317939dbfeed.png`

The blueprint emphasized color+shape coding, stable-ID labels, CPU fallback
status, read-only inspector state, and non-authoritative feedback badges. The
implemented S04 shell follows that direction using lightweight Bevy sprites,
text badges, and overlays rather than committed art assets.

## Implemented Presentation Cues

- Creature markers now carry `[@] creature` stable-ID badges.
- Food markers now carry `[+] food` badges and nutrition text.
- Hazard, obstacle, and token classes are documented in the in-window legend as
  `[!] hazard`, `[#] obstacle`, and `[T] token` for fixtures that include them.
- The graphical world scale and badge offsets were adjusted so the P34 creature
  and food target remain readable beside the inspector.
- The S04 feedback overlay is derived from the existing feedback-polish smoke
  summary, whose survival entries come from sealed outcomes.
- The feedback overlay exposes success, pain, sleep, failure, curiosity, asset
  fallback, and CPU fallback context as display-only information.
- The launcher script now prints a readability line so manual testers know what
  the graphical window should show.

## Evidence

Screenshots are local evidence under `target/` and are not committed:

- `target/playtest_evidence/S04/screenshots/s04_readability_feedback_window.png`
- `target/playtest_evidence/S04/screenshots/s04_readability_feedback_window_final.png`
- `target/playtest_evidence/S04/screenshots/s04_readability_feedback_window_revised.png`

The revised screenshot shows:

- persistent A-Life graphical window
- CPU Reference fallback status
- selected creature marker and stable-ID badge
- food marker and nutrition badge
- read-only inspector
- readability legend with hazard, obstacle, token, sleep, pain, success, failure,
  and curiosity cues
- non-authoritative display feedback derived from sealed outcome labels

Native Computer Use screenshot capture still failed on this Windows 10 machine
with `SetIsBorderRequired failed: No such interface supported (0x80004002)`.
The Alt+Print clipboard fallback captured the active A-Life window at
`1402x914`.

## Validation Evidence

Focused checks run during implementation:

```powershell
cargo test -p alife_game_app --test app_shell bevy_feature_s04_readability_feedback_is_display_only --features bevy-app
cargo test -p alife_game_app s01_graphical_launcher_script_uses_persistent_window_commands
cargo check -p alife_game_app --all-targets --features bevy-app
cargo run -p alife_game_app --bin alife_game_app -- feedback-polish-smoke crates/alife_world/tests/fixtures/p34
cargo run -p alife_game_app --bin alife_game_app -- playable-survival-loop-smoke
```

Manual graphical evidence:

```powershell
cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- graphical-playground-smoke --seconds 60 crates/alife_world/tests/fixtures/p34
```

## Known Limitations

- The P34 fixture contains one creature and one food object. Hazard and obstacle
  world-object readability is represented by the supported taxonomy, legend, and
  sealed survival feedback evidence rather than by a live P34 graphical hazard.
- S04 uses text/sprite placeholders only. No full art pipeline or audio pipeline
  is added.
- The graphical shell remains a developer/playtest surface, not a polished
  consumer game UI.
- Computer Use native window screenshots remain unavailable on this Windows 10
  build; Alt+Print is the reliable capture path.

## Next

Proceed to S05 only after S04 is reviewed, merged, and main validation passes.
