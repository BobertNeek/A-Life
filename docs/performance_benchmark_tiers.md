# P20 benchmark harness and performance tiers

Status: v1 CPU-reference smoke harness.

The P20 harness measures deterministic headless scenario execution across the
population tiers required by the performance contract: 1, 10, 50, 100, 250,
and 500 agents. It is a measurement scaffold, not an optimization pass. Normal
CI runs only the 1-agent and 10-agent smoke tiers so validation remains stable
without Bevy, wgpu, GPU adapters, or device readback.

## Commands

Smoke report:

```bash
cargo run -p alife_tools --bin benchmark_tiers
```

The report is written to `target/artifacts/benchmark_tiers.md`.

Manual CPU-only upper tiers:

```bash
cargo test -p alife_tools --test benchmark_tiers -- --ignored --nocapture
cargo run -p alife_tools --bin benchmark_tiers -- --all
```

The 50/100/250/500 CPU-only tiers are expected-slow manual measurements. They
exist to expose scaling movement early; they are not release gates and they do
not imply that CPU reference execution is the final 500-agent path.

## Required tiers

| Tier | CI mode | CPU-only status |
|---:|---|---|
| 1 | smoke | required fast |
| 10 | smoke | required fast |
| 50 | manual ignored | expected-slow |
| 100 | manual ignored | expected-slow |
| 250 | manual ignored | expected-slow |
| 500 | manual ignored | expected-slow |

## Metrics

Each `BenchmarkRun` records:

- tick time
- memory usage estimate
- patch throughput
- memory/topology update time
- neural projection time
- sleep consolidation time
- scenario success count and attempt count

The current CPU-reference harness times whole scenario runs and derives
success, patch, memory, topology, and sleep counters from the P17/P18
headless scenario layer. Fine-grained neural projection timing is reserved as
a metric field but remains zero until later reference/GPU instrumentation can
produce truthful sub-stage timings.

## Biological compute budget data

`ComputeBudgetPolicy` is generated from `BrainClassSpec` and records:

- active synapse and tile budgets
- essential lobes that retain reservation under pressure
- non-essential lobes that decimate first
- throttling thresholds for non-essential decimation, warm-cadence fallback,
  and sleep-only fallback
- fallback update frequency in Hz

`UpdateRatePolicy::v1_defaults()` captures configurable cadence bands for hot,
warm, and cold agents across sensory/motor, endocrine/homeostatic, arbitration,
plasticity, memory expectancy, topology, sleep, and logging/export profiles.

## CPU reference vs expected GPU budget

The CPU reference harness is authoritative for causal scenario execution and
normal validation. It is not expected to meet the final 60 FPS population
budget at upper tiers. GPU plans must keep comparing against CPU reference
fixtures before making performance claims.

Expected split:

| Path | Role | Budget expectation |
|---|---|---|
| CPU reference | correctness oracle and smoke benchmarks | tier 1/10 CI, upper tiers manual |
| GPU backend | future sparse projection acceleration | P24-P29 parity and no-readback gates |
| Bevy adapter | product runtime host | not required for P20 benchmarks |

P20 deliberately does not optimize P15/P17/P25 internals and does not
implement P29 runtime integration.

