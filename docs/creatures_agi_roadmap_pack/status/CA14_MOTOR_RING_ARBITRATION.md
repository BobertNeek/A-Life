# CA14 - Competitive motor ring arbitration presentation/runtime bridge

Status: implemented on `codex/CA14-motor-ring-arbitration-bridge`.

## Scope

CA14 adds a fixed-channel motor ring presentation over the existing structured
P09 action proposal and arbitration path. The ring is display/runtime evidence:
it exposes competing action channels without making the UI, GPU, semantic,
teacher, memory, or topology systems able to issue actions directly.

## Behavior

- Motor channels are presented in fixed order: idle, approach, eat, flee, sleep,
  and inspect.
- Channel scores are derived from structured `ActionProposal` records.
- Winner evidence is produced by the existing CPU reference arbitration path.
- Graphical status and inspector overlays include a compact motor ring section.
- Runtime control panel signatures include motor ring winner and margin data.
- The CA14 smoke command seals a normal patch after the winning action is
  selected through the existing live brain loop.

## Evidence command

```powershell
cargo run -p alife_game_app --bin alife_game_app -- motor-ring-arbitration-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Expected evidence includes:

- `schema=alife.ca14.motor_ring_arbitration_presentation.v1`
- `winner=Eat`
- `patch_sealed=true`
- `no_direct_bypass=true`
- `Boundary: normal arbitration; no direct bypass`

## Invariants

- `alife_core` is unchanged.
- Bevy remains feature-gated and presentation-only.
- CPU fallback and CPU shadow parity are unchanged.
- No full action-authoritative GPU claim is introduced.
- Motor ring display is not an action source and cannot bypass P09 arbitration.
- Player-facing output uses stable IDs only, not Bevy Entity IDs.
- No active bulk neural readback is introduced.

## Limitations

CA14 visualizes and records action competition, but it does not tune action
policy, add endocrine/homeostasis behavior, or implement non-scripted movement.
Those remain later roadmap items.
