# CA18 - Multi-creature graphical population v1

Status: complete on branch `codex/CA18-multi-creature-graphical-population-v1`.

## Scope

CA18 expands the GPU alpha graphical scenario from a deterministic single-creature slice into a bounded three-creature graphical population:

- the `gpu_alpha` fixture now contains three stable-ID agent objects plus food, hazard, and obstacle markers;
- the graphical scene renders all fixture agents through the existing stable-ID presentation path;
- the app exposes a CA18 population smoke summary with stable-ID creature selection order;
- the Bevy shell shows a compact population panel and stable-ID social proximity cue lines;
- Tab cycles selected creatures by stable ID;
- per-creature selection remains read-only and presentation-only.

## Evidence

Focused command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- graphical-population-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Expected evidence:

- `creatures=3`
- `cap=8`
- `selected=1`
- nonzero social cues
- product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`

## Boundaries

CA18 does not change `alife_core`, save schemas, neural contracts, CPU shadow parity, CPU fallback, GPU runtime authority, or P09 action arbitration. Social proximity cues are display-only and cannot emit actions. The current claim is still not full action-authoritative GPU runtime.

## Known Limitations

The CA18 population is a bounded visible population slice, not a full ecosystem claim. CA19-CA22 own richer resource ecology, lifecycle, behavior metrics, and long-run ecological balancing.
