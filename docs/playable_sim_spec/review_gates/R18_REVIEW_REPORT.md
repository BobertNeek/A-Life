# R18 Review Report

Review: R18 - Population/performance/scalability review before G19

Branch: `codex/R18-population-performance-review`

Verdict: PASS

G19 may proceed: yes

## Summary

G18 is ready to hand off to G19. The population performance policy defines
bounded target tiers, keeps CPU/headless behavior as the measured CI path,
documents manual 50/100/250/500 population commands, and explicitly records GPU
runtime performance as unknown unless hardware timing is measured. The LOD
projection is presentation/cadence-only and tests preserve the population
behavior signature.

## Findings by Severity

### Blocker

None.

### High

None.

### Medium

None.

### Low

- Manual population tiers 50/100/250/500 and GPU runtime timing remain manual
  evidence. This is acceptable for G18/R18 because the docs and reports label
  them as manual or unknown, not measured product performance.
- The playable fun/balance judgment is still based on smoke-scale behavior.
  G19 should tune long-run ecology/social/lifecycle balance with the G18
  performance policy in place.

## Checklist

| Review item | Result | Evidence |
|---|---|---|
| Population caps, update cadence, and LOD policies are deterministic and bounded | PASS | `PopulationPerformancePolicy::v1_defaults` defines tiers 1/10/50/100/250/500, hot/warm/cold LOD, and finite validated cadence bands. |
| Sensory/motor and survival-critical work retain priority | PASS | Tests assert sensory/motor and action arbitration cadence stays above nonessential cognition; homeostasis/action/sensory flags remain protected under throttling. |
| Performance claims are measured or unknown/manual | PASS | `target/artifacts/benchmark_tiers.md` records tier 1 and 10 CPU smoke values; G18 docs mark upper tiers manual and GPU unknown. |
| CPU fallback remains available and correct | PASS | `population-performance-smoke` selected `CpuReference`; GPU runtime report selected `CpuReference` with `HardwareUnavailable`. |
| GPU acceleration remains optional and no-readback-safe | PASS | GPU runtime report says no active gameplay neural readback is true and records fallback instead of timing claims. |
| Ecology/social/lifecycle loops remain stable at reviewed counts | PASS | Existing G07/G08/G09 tests remained green in full workspace validation; G18 smoke exercises two-creature population behavior. |
| Long-run memory/topology/log growth bounded or capped | PASS_WITH_LIMITATION | Existing release/headless soak and population caps remain active; G19 should review longer balance horizons. |
| Sim is ready for G19 tuning rather than only test-passing | PASS_WITH_LIMITATION | Current smoke demonstrates visible population policy and feedback; G19 should focus on play feel, balance, and long-run stability. |
| No false GPU performance claims | PASS | Searches found GPU performance references consistently state unknown/manual or CPU fallback is not GPU timing evidence. |
| Optional GPU/graphics path not required by headless tests | PASS | Default validation and wrappers passed without GPU/graphics hardware. |
| `alife_core` remains engine-independent | PASS | `scripts/check_core_boundaries.ps1` and `cargo tree -p alife_core` passed. |

## Performance Evidence

Focused commands run for R18:

```powershell
cargo run -p alife_tools --bin benchmark_tiers
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_game_app --bin alife_game_app -- population-performance-smoke crates/alife_world/tests/fixtures/p34
```

Observed benchmark smoke report:

- Tier 1 CPU/headless tick time: 1.408 ms.
- Tier 10 CPU/headless tick time: 7.351 ms.
- Manual expected-slow tiers 50/100/250/500 are documented, not CI gates.

Observed GPU runtime report:

- Backend requested: `GpuStatic`.
- Backend selected: `CpuReference`.
- Fallback reason: `HardwareUnavailable`.
- GPU neural time: unknown.
- No active gameplay neural readback: true.

Observed G18 product smoke:

- Creatures: 2.
- Scheduler steps: 4.
- Sealed patches: 4.
- Backend: `CpuReference`.
- Throttle level: 2.
- Nonessential decimation: 4.
- LOD: `full`.
- Golden behavior preserved: true.

## Validation Commands

Run on G18 before merge and on main after G18 merge:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

R18 branch validation rerun after this report:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
```

## Known Limitations

- GPU hardware timing evidence was not available on this run. The report records
  CPU fallback and unknown GPU neural time.
- Manual upper population tiers remain manual expected-slow checks.
- G19 should prioritize long-run play balance, ecology/social/lifecycle tuning,
  and whether the minimum playable target of 10 creatures feels good rather than
  merely passing smoke tests.

## Fix Prompt

No fix prompt required.

## Recommendation for G19

Proceed with G19 only after explicit user authorization. G19 should use G18's
bounded population performance policy as the guardrail for long-run balance
tuning and should not convert manual or fallback evidence into product GPU
claims.
