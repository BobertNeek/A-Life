# CA16 - Non-scripted movement, approach, eat, and affordance loop

Status: complete on `codex/CA16-non-scripted-affordance-loop`.

## Scope

CA16 makes the GPU alpha fixture exercise a visible approach-then-eat loop
through existing live proposals, P09 arbitration, world action legality, and
sealed patches. It does not add scripted action forcing and does not change
`alife_core`.

## Evidence Added

- `gpu_alpha` food starts outside eat reach so the first live tick selects
  `APPROACH`/`Move`.
- The second live tick reaches normal eat distance and selects `EAT`/`Interact`.
- Food is consumed through the existing world action path.
- Hunger decreases and energy increases after consumption.
- A deterministic `affordance-loop-smoke` command records the action sequence,
  distance change, sealed patches, reward/energy deltas, and stable-ID-safe
  signature.
- App-shell tests assert no Bevy `Entity` token appears in the CA16 evidence.

## Boundaries

- No Bevy, wgpu, renderer, GPU, semantic, or school dependencies were added to
  `alife_core`.
- The live loop still emits action proposals and uses normal arbitration.
- The world layer remains authoritative for contact, reach, consumption, and
  outcomes.
- GPU claims are unchanged; CPU shadow parity remains the gate.
- No full action-authoritative GPU runtime is claimed.
- No screenshots, logs, target artifacts, release tags, S12, G25, or P37 were
  created.

## Focused Commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- affordance-loop-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo test -p alife_game_app --test app_shell ca16 -- --nocapture
```

Graphical changes are covered by the standard graphical smoke and forced
fallback smoke for this tranche.
