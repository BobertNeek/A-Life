# CA24 - Teacher avatar, gestures, speech tokens, and perception-only gating

Status: complete on branch `codex/CA24-teacher-avatar-gestures-speech-tokens`.

CA24 extends the graphical school surface so teacher cues are visible/audible world events rather than hidden metadata.

Implemented behavior:

- Teacher cue dispatch includes a speech token, gesture marker, object highlight, and feedback cue.
- The graphical school overlay exposes speech and gesture channels as perception-only lesson cues.
- The Bevy scene spawns stable-ID teacher cue markers for the teacher avatar and lesson cue objects.
- The `T` school toggle hides teacher cue markers when school mode is off.
- Lesson verification remains based on sealed patches.
- Teacher cue metadata cannot directly select actions, bypass arbitration, inject hidden vectors, or rewrite weights.

Focused command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- teacher-world-cues-smoke
```

Boundary notes:

- CA24 does not use a local semantic provider or SLM prior.
- CA24 does not implement real model inference.
- CA26 owns the real local semantic embedding provider.
- CA27 owns the real local SLM prior boundary.
- Teacher and school systems remain perception/context only.
