# G17 Audio/VFX Feedback Readability

G17 adds a headless-safe feedback polish layer that maps existing validated product
events into optional presentation cues. The cues are non-authoritative: they do
not issue actions, mutate cognition, change save state, or bypass sealed patch
review.

The smoke path derives cues from:

- G06 sealed survival outcomes: food reward, missing affordance, hazard pain, and sleep.
- G10 perception-only teacher cue presentation.
- G15 save/load UX completion events.
- G05 read-only stable-ID inspector selection.

Placeholder assets live under `content/fixtures/g17/`. They are intentionally
tiny text/procedural descriptors rather than production audio or VFX payloads.
Missing optional polish assets use procedural fallbacks and are reported in the
G17 smoke summary.

Smoke command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- feedback-polish-smoke crates/alife_world/tests/fixtures/p34
```

Validation remains headless by default. Graphics/audio integration is a later
product concern; G17 only freezes the event mapping and manifest contract.
