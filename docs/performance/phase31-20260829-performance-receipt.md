# Phase 3.1 gameplay performance receipt, 2026-08-29

Scenario: release `production-voxel`, Vulkan, NVIDIA GeForce RTX 3050, `MinSpecComfort1080p`, 1920x1080, seed 31082706, six founders, GPU-required cognition, 5-second warmup, then one 60-second measurement.

Baseline source: `c0d685151df2800cbee0ed1121cb6955667a125d`
Final measured source: `03fd3bf1a918e5660d84ca8e513d414ac2cabe8f`

| Metric | Baseline | Final | Change |
| --- | ---: | ---: | ---: |
| Completed world ticks | 529 | 574 | +8.5% |
| True simulation throughput | 8.82 TPS | 9.55 TPS | +8.3% |
| Runtime CPU per completed tick | 61.43 ms | 60.03 ms | -2.3% |
| Preparation CPU per completed tick | 27.74 ms | 24.86 ms | -10.4% |
| Frame p50 | 8.43 ms | 9.15 ms | +8.5% |
| Frame p95 | 47.74 ms | 51.18 ms | +7.2% |
| Frame p99 | 291.87 ms | 327.67 ms | +12.3% |

The first measured defect was false scheduler throughput. The baseline made 998 tick attempts but advanced only 529 world ticks. Checkpoint publication returned an untyped empty result, and the Bevy loop counted every attempt as progress. The repair adds a typed checkpoint-publication wait, ends that catch-up batch, preserves only its unspent bounded debt, polls once on each later frame, and records completed world ticks as throughput. The final run recorded 2,770 attempts, 574 completed ticks, 2,196 typed checkpoint-publication waits, and zero checkpoint-failure waits. The larger wait count is expected because cheap blocked frames now poll once per rendered frame. It is no longer reported as simulation progress.

Rollback cloning was measured rather than removed. The final run cloned on all 574 completed ticks and on zero blocked ticks. `world.clone()` cost 20.56 ms total and `residents.clone()` cost 153.75 ms total, or 0.30 ms per completed tick combined. This is too small to justify replacing the atomic rollback transaction.

Preparation profiling identified sleep eligibility and replay work as the largest substage. Static tracing then found that replay availability called `build_sleep_replay_batch`, which maps the full mutable GPU slot. The final repair reads the generation-checked host replay event count for availability. Exact replay bytes are still read when consolidation requires them. Preparation fell from 27.74 to 24.86 ms per completed tick.

Ordinary neural readback remains bounded and batched. The final run used 379 selection maps for 838 organism rows at 80 bytes per row, and 379 learning maps at 64 bytes per row. Mean batch size was 2.21 organisms. Ordinary full neural snapshot calls and bytes were both zero. Eleven asynchronous exact checkpoint transactions completed, covering 66 exact neural organism captures. Their overlapped wall time was 48.83 seconds, while update-thread poll CPU time was 157.67 ms.

Evidence:

- Baseline: `target/artifacts/phase31-performance/opt-baseline-c0d68515-20260829-114313/baseline-performance.json`
- Final: `target/artifacts/phase31-performance/opt-final-03fd3bf1-20260829-132339/final-performance.json`
- Final launch: `target/artifacts/phase31-performance/opt-final-03fd3bf1-20260829-132339/launch-receipt.txt`

Focused checks passed: scheduler debt regression, truthful receipt validation, replay availability metadata unit check, production feature compile, release build, and bounded production shutdown. No training, evolution, asset changes, population reduction, synchronous full-population fallback, or broader optimization occurred.

Remaining limits: frame tails did not improve in this scenario, and sleep eligibility and replay work still used 10.44 seconds of the final measurement. Further work needs a separate measured pass inside the real sleep scheduler. Target-indexed dendritic spans, dispatch layout, buffer layout, and integrated-test speed were not changed because this pass did not measure them as the next production bottleneck.
