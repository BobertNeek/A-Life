# CA08 Sensory Feedback Cues

Status: implemented on `codex/CA08-first-sensory-feedback-layer`.

CA08 adds the first player-facing sensory feedback layer to the graphical alpha
without adding large assets or changing game authority. The Bevy presentation now
has display-only pulse markers and overlay rows for:

- reward/food feedback as a green pulse with `soft-ping` audio stub,
- hazard/pain feedback as a red pulse with `warning-pulse` audio stub,
- sleep/rest feedback as a blue pulse with `rest-chime` audio stub,
- H_shadow learning feedback as a teal pulse with `learn-spark` audio stub.

The cue panel is appended to the graphical event feed, and the visual guide
lists cue meanings. The pulse sprites are derived from existing stable-ID world
objects and existing feedback/GPU summaries; they do not issue actions, mutate
weights, or alter world state.

Boundaries:
- No `alife_core` changes.
- No save/schema/public core contract changes.
- No new audio files or large assets were added.
- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU shadow remains the gate; no full action-authoritative claim.
- Cue pulses are presentation-only and stable-ID based.

Focused evidence:
- `cargo test -p alife_game_app --features bevy-app --test app_shell ca08 -- --nocapture`
- graphical smoke and forced fallback smoke per validation protocol.

