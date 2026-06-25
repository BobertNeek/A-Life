# CA17 - Hazard avoidance, pain, sleep, and failure recovery

Status: complete on `codex/CA17-hazard-pain-sleep-failure-recovery`.

## Scope

CA17 adds a bounded app-level smoke path for the current live creature loop:

- visible hazard sensory salience,
- normal `FLEE` proposal selection through existing arbitration,
- hazard contact pain/fear outcome evidence,
- rest/sleep transition evidence,
- recoverable invalid-target failure followed by a sealed recovery tick.

The change does not add a new roadmap phase, does not make GPU mandatory, and
does not change `alife_core` dependencies.

## Evidence

Focused command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- hazard-recovery-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Observed focused result:

- hazard visible in the alpha fixture: true,
- hazard salience: positive,
- player-facing hazard avoidance cue: true,
- flee selected as `Move` / `HeadlessActionIds::FLEE`,
- hazard distance increases after flee,
- hazard contact increases pain and fear in a sealed patch,
- rest action enters forced recovery sleep and reduces fatigue,
- invalid target produces a recoverable failure patch,
- the next normal tick seals successfully,
- no terminal stagnation,
- stable-ID output only.

## Known Limitations

CA17 keeps this as a small deterministic product smoke. It does not claim
emergent long-horizon hazard avoidance or full action-authoritative GPU runtime.
CPU shadow parity and CPU fallback remain intact.

Next executable item: `CAR17` core gameplay review hard stop.
