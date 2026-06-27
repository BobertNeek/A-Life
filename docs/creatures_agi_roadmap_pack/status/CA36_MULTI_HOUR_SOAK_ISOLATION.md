# CA36 Multi-Hour Soak Isolation

## Status

Complete on branch `codex/CA36-multi-hour-soak-isolation`.

CA36 adds a CI-safe manual soak isolation protocol for the existing headless,
GPU, and graphical evidence paths. It does not add a new neural runtime mode,
does not change action authority, and does not run multi-hour work in normal
validation.

## What Changed

- Added `alife.ca36.multi_hour_soak_isolation.v1` as a protocol/report schema.
- Added `multi-hour-soak-isolation-smoke [--out path]`, which writes a
  markdown protocol report under `target/ca36_soak_isolation/` by default.
- Recorded 10k+ manual commands for:
  - GPU sustained-learning soak.
  - GPU long-run soak.
  - Headless ecological 10k soak.
- Recorded bounded graphical and forced-fallback smoke commands.
- Added process/memory monitoring instructions using `Get-Process`,
  `WorkingSet64`, and `PrivateMemorySize64`.
- Added required precision/drift counters for CPU shadow parity, first parity
  failure tick, H_shadow delta bounds, compact readback bytes, sealed
  patch/packed log monotonicity, and process memory samples.

## Manual Evidence Boundary

The CA36 smoke command validates the protocol text and writes an untracked
report. It is not itself multi-hour evidence.

Manual 10k+ evidence commands:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 10000 --report-every 1000 --json target/ca36_soak_isolation/gpu_sustained_learning_10k.json
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 10000 --report-every 1000 --json target/ca36_soak_isolation/gpu_longrun_10k.json
cargo test -p alife_game_app --test app_shell ca22_manual_10k_ecological_soak -- --ignored --nocapture
```

Optional multi-hour runs repeat the GPU sustained-learning 10k command until a
time window ends, sampling process memory into `target/ca36_soak_isolation/`.

## Invariants

- `alife_core` remains unchanged and engine-independent.
- CPU fallback remains available.
- CPU shadow parity remains the correctness gate.
- No full action-authoritative GPU runtime claim is made.
- No active bulk neural readback is added.
- Report artifacts are written under `target/ca36_soak_isolation/` and must not
  be committed.
- No screenshots, logs, model files, `S12`, `G25`, `P37`, or release tag are
  created by this plan.

## Focused Evidence

```powershell
cargo test -p alife_game_app --test app_shell ca36_soak_isolation -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- multi-hour-soak-isolation-smoke
```

Existing bounded smoke commands remain the runtime evidence checks:

```powershell
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 100 --report-every 25
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 100 --report-every 25
```

## Known Limitations

- CA36 records the isolation protocol and bounded smoke evidence; multi-hour
  manual evidence remains operator-run.
- Timing remains local-hardware specific and is not release readiness evidence.
- The product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.

Next: CAR36 performance and parity review.
