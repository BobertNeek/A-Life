# Phase 3.1 sleep sidecar receipt, 2026-09-03

Scenario: release `production-voxel`, Vulkan, NVIDIA GeForce RTX 3050, `MinSpecComfort1080p`, 1920x1080, seed 31082706, six founders, GPU-required cognition, 5-second warmup, then one 60-second measurement.

Baseline source: `e1ede639a10ed2ecdd233d4650c9013486f0a4f9`

Measured repair source: `3f67a7c04b41c46a6e702119396eac06e77b9a67`

| Metric | Baseline | Repair | Change |
| --- | ---: | ---: | ---: |
| Completed world ticks | 1,151 | 1,154 | +0.3% |
| True simulation throughput | 19.183 TPS | 19.232 TPS | +0.3% |
| Average frame cadence | 87.50 FPS | 92.14 FPS | +5.3% |
| Runtime CPU per completed tick | 25.98 ms | 25.53 ms | -1.7% |
| Frame p50 | 5.02 ms | 4.74 ms | -5.6% |
| Frame p95 | 47.49 ms | 44.79 ms | -5.7% |
| Frame p99 | 79.68 ms | 77.52 ms | -2.7% |
| Maximum frame | 186.47 ms | 204.89 ms | +9.9% |
| Frames over 100 ms | 22 | 26 | +18.2% |
| Sleep consolidation CPU | 1,604.77 ms | 1,469.44 ms | -8.4% |
| Sleep phase-data CPU | 1.98 ms | 0.13 ms | -93.2% |

The sleep scheduler previously ran bounded memory and topology sidecars when a cycle started, then reran them after each successful replay chunk according to a cadence counter. A single cycle could therefore perform the same CPU-side causal work nine times. The repair runs these sidecars once, before sealing the replay identity, and retains the existing bounded plan fields for serialized and API compatibility. Exact GPU replay, generation checks, durable journal publication, rollback, and checkpoint boundaries remain unchanged.

A focused regression reproduced nine sidecar calls with the old scheduler and now requires one call per cycle. Thirteen GPU sleep contract tests and six automatic sleep scheduler tests passed. The production feature compile, release build, and the focused RTX 3050 Vulkan exact-authority sleep integration test also passed.

The matched production run completed normally. Fifteen asynchronous checkpoint transactions completed. Persistence was idle and healthy at shutdown. Ordinary full-neural snapshot calls and bytes remained zero.

This is a valid redundant-work removal, not a complete sleep-performance solution. True throughput changed by only 0.3%, and the single-run maximum and hitch count worsened. The broad sleep wrapper still used 7.63 seconds. Of that, 5.58 seconds remains attributed to `sleep_scheduler_other`, which also includes surrounding retained-learning, resident synchronization, ATP accounting, and record replacement. True GPU replay still contains blocking readback stages. Those paths need separate state-machine work, not an unsafe cache of mutable GPU replay state.

An attempted sealed-replay cache was rejected before this commit. It caused deterministic GPU-authority failure near tick 120 during exact checkpointing. No cache code or temporary diagnostic code remains in the branch.

Evidence:

- Baseline: `target/artifacts/phase31-performance/compile-profile-e1ede639-20260902-230216/performance.json`
- Repair: `target/artifacts/phase31-performance/sleep-sidecar-once-3f67a7c0-20260903/performance.json`

No training, evolution, brain assets, population, scheduler pacing, rollback semantics, or presentation behavior changed.
