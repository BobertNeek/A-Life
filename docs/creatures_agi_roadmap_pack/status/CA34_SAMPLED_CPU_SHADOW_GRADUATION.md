# CA34 Sampled CPU-Shadow Graduation Experiment

## Status

Complete on branch `codex/CA34-sampled-cpu-shadow-graduation-experiment`.

CA34 adds a manual-only sampled CPU-shadow experiment for the existing bounded
multi-creature GPU runtime. It does not graduate to full action-authoritative
GPU runtime.

## What Changed

- Added `alife.ca34.sampled_gpu_runtime.v1` as the sampled runtime evidence
  schema.
- Added `sampled-gpu-runtime-smoke <fixture-root>` with bounded manual options:
  - `--creatures N`
  - `--ticks N`
  - `--warmup-ticks N`
  - `--cpu-shadow-every N`
  - `--json path`
- Kept CA33 `batched-gpu-runtime-smoke` locked to every-creature/every-tick CPU
  shadow parity.
- Added a backend sampled static tick call that can skip CPU diagnostic work on
  non-sample ticks while reporting `cpu_shadow_checked=false`.
- Added fallback-on-first-sampled-parity-failure behavior. If a sampled check
  fails, later ticks use CPU proposals and the report records the first failure
  tick.

## Evidence Boundary

CA34 is a manual graduation experiment. Its product claim is sampled and
bounded, for example `SampledCpuShadowGuardedStaticPlusLiveHShadow` when local
GPU evidence supports it. It is not a player-facing release claim and not a
full action-authoritative GPU claim.

CPU fallback remains available. CPU fallback output is not GPU evidence.

## Invariants

- `alife_core` is unchanged and remains engine-independent.
- Stable IDs remain the only portable/player-facing creature identifiers.
- No Bevy `Entity`, wgpu resource, GPU buffer, adapter ID, or raw tensor enters
  core or portable output.
- No active bulk neural readback is added.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain
  unchanged.
- Semantic, teacher, UI, memory, topology, and GPU paths do not emit actions
  directly or bypass P09 action arbitration.
- No screenshots, logs, target artifacts, model files, `S12`, `G25`, `P37`, or
  release tag are created by this plan.

## Focused Evidence

Planned/required CA34 focused commands:

```powershell
cargo test -p alife_game_app --test app_shell ca34_sampled_gpu_runtime -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- sampled-gpu-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha --creatures 3 --ticks 4 --warmup-ticks 1 --cpu-shadow-every 2
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- batched-gpu-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha --creatures 3 --ticks 1 --cpu-shadow-every 1
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
```

Graphical smoke and forced fallback remain required when graphical/GPU product
paths are touched.

## Known Limitations

- The sampled experiment is bounded to small manual ticks and the GPU alpha
  population. It is not a long soak and does not replace CA36.
- CA34 samples static action-score parity. It does not remove CPU fallback or
  CPU oracle infrastructure.
- A parity failure immediately degrades to CPU proposals; the run does not keep
  using GPU proposals after a sampled failure.
