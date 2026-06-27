# CA31 - Player Lab Tools for Behavior Comparison

Status: implemented on `codex/CA31-player-lab-tools-behavior-comparison`.

## Summary

CA31 adds a bounded A/B behavior comparison lab for players and developers. It
selects two manifest scenarios, runs each in an isolated deterministic copy,
records compact behavior signatures, renders a read-only comparison panel, and
can export a small Markdown report on request.

## Player/Developer Surface

- CLI smoke: `behavior-comparison-lab-smoke`.
- Defaults compare the manifest default scenario against the first alternate
  scenario.
- Optional arguments:
  - `--manifest <path>`
  - `--a <scenario-id>`
  - `--b <scenario-id>`
  - `--ticks <N>`
  - `--out <path>`
- The report includes creature/food/hazard/object counts, sealed patch counts,
  final action/target labels, and deterministic behavior signatures.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell ca31 -- --nocapture
```

Observed:

- CA31 tests validate A/B scenario comparison, behavior signatures, read-only
  panel text, small report export, stable-ID safety, and no hidden action or
  cognition authority.

```powershell
cargo run -p alife_game_app --bin alife_game_app -- behavior-comparison-lab-smoke --a gpu-alpha --b p34 --ticks 8
```

Observed:

- The runner compares `gpu-alpha` with `p34`.
- The summary reports different behavior signatures.
- The report stays below the small-report cap.
- The panel states `No hidden training mutation`.

## Boundaries

- `alife_core` was not changed.
- Scenario runs are isolated copies, not mutations of the active graphical
  runtime.
- The comparison panel is read-only and stable-ID based.
- Semantic, topology, memory, UI, and GPU surfaces cannot emit actions through
  this lab path.
- CPU shadow parity remains the gate.
- Product GPU claim is unchanged; CA31 does not claim full action-authoritative
  GPU runtime.
- Exported reports are local artifacts and are not committed.

## Known Limitations

- CA31 compares compact deterministic behavior signatures, not full raw neural
  tensors or bulk trace exports.
- Deeper offline trace comparison remains future scope after CAR31 review.

## Next Plan

CAR31 - Cognition inspection review.
