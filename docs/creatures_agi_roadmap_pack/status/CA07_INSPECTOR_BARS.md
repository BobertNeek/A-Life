# CA07 Inspector Bars and Readable Creature State

Status: implemented on `codex/CA07-inspector-bars-readable-creature-state`.

CA07 converts the graphical alpha read-only inspector from dense debug text into
a compact creature-state panel. The inspector now shows:

- selected stable ID and mapping status,
- awake/sleep status plus current animation/expression,
- energy, health, hunger, fatigue, and fear bars,
- recent action and target stable ID,
- patch/sealed-state summary,
- H_shadow learning count and last delta,
- compact fallback/backend technical footer.

The inspector remains read-only and stable-ID based. Detailed GPU safety
language is kept concise: CPU shadow remains the gate, `full_auth=false` is
visible, and no Bevy `Entity` IDs are exposed in player-facing text.

Boundaries:
- No `alife_core` changes.
- No save/schema/public core contract changes.
- No GPU claim upgrade; product claim remains
  `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU fallback and CPU shadow parity remain intact.
- Bars are presentation-only and mirror existing `CreatureVisualSnapshot` cues.

Focused evidence:
- `cargo test -p alife_game_app --features bevy-app --test app_shell ca07 -- --nocapture`
- `cargo test -p alife_game_app --features bevy-app --test app_shell inspector -- --nocapture`
- `cargo test -p alife_game_app --test app_shell graphical_controls -- --nocapture`
- graphical smoke and forced fallback smoke per validation protocol.
