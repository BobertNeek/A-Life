# CA06 Camera, Mouse Selection, Follow, and Zoom Polish

Status: implemented on `codex/CA06-camera-mouse-selection-follow-zoom-polish`.

CA06 adds stable-ID mouse selection to the graphical GPU alpha playground. Left-click picking resolves only through the Bevy stable-ID map, updates the read-only inspector, focuses the camera on the selected presentation object, and records a player-facing stable-ID event. Keyboard pan, zoom, orbit, and `F` follow remain available.

Player-facing controls now advertise left-click selection plus pan/zoom/orbit/follow without exposing Bevy `Entity` IDs. The selected ring remains presentation-only and follows the selected stable-ID position.

Boundaries:
- No `alife_core` changes.
- No Bevy `Entity` IDs in player-facing text.
- Visual/camera state remains presentation-only.
- GPU product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU fallback and CPU shadow parity remain intact.

Focused evidence:
- `cargo test -p alife_game_app --features bevy-app --test app_shell ca06 -- --nocapture`
- `cargo test -p alife_game_app --test app_shell graphical_controls -- --nocapture`
- graphical smoke and forced fallback smoke per validation protocol.
