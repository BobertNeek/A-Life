# Phase 3.1 frame-tail receipt, 2026-08-29

Scenario: release `production-voxel`, Vulkan, NVIDIA GeForce RTX 3050, `MinSpecComfort1080p`, 1920x1080, seed 31082706, six founders, GPU-required cognition, 5-second warmup, then one 60-second measurement.

The diagnostic source `64e880dce42e8028cf76962c78bc4e8e6184b5fa` added bounded profiling only. It retained the worst 100 frames above 25 ms with per-frame runtime, checkpoint, scheduler, presentation, GPU-readback, persistence, and update-stage deltas. Normal runs do not install this profiling resource.

| Metric | Diagnostic | Paced post-fix | Paced confirmation |
| --- | ---: | ---: | ---: |
| Completed world ticks | 523 | 498 | 517 |
| True TPS | 8.71 | 8.30 | 8.62 |
| Average FPS | 45.09 | 48.56 | 53.00 |
| Frame p50 | 9.71 ms | 9.91 ms | 9.17 ms |
| Frame p95 | 43.72 ms | 84.22 ms | 69.38 ms |
| Frame p99 | 343.37 ms | 187.80 ms | 174.97 ms |
| Maximum frame | 1315.10 ms | 524.06 ms | 496.16 ms |
| Frames over 100 ms | 94 | 112 | 97 |

The diagnostic identified four-tick catch-up batches as the largest p99 amplifier. Sixty-five of the retained worst 100 frames ran four completed world ticks and averaged 372.9 ms. A paced scheduler experiment limited 1x play to one completed tick per render frame and restored unspent work to the existing bounded debt accumulator.

The experiment reduced p99 by 45-49%, reduced the maximum stall by 60-62%, and raised average FPS. It also made moderate stalls more frequent, worsened p95 by 59-93%, and did not reproduce higher true TPS. This failed the requirement to improve both p95 and p99 without reducing throughput. Commit `c2c7a39e3712e74c6705d0ddb90caf09e9944b63` was reverted by `6b84ab312d05295e67a6adc8ebc77dd9c8bb1067`.

The remaining measured source seam is synchronous compact sleep-journal publication on the update thread. `GpuLiveCheckpointDurability::publish_sleep_journal_entries` loads the current durable journal, merges and sorts entries, publishes the replacement, reloads the manifest and journal, and validates equality. In the paced confirmation's retained worst frames, the 17 frames with sleep-persistence work averaged 307.7 ms. The other 83 averaged 153.4 ms. The largest ordinary publication calls consumed 260-387 ms each without GPU checkpoint capture or full-neural readback.

No run budget remained to move this filesystem publication and reload validation off the update thread while preserving ordered journal authority, atomic manifest CAS, exact checkpoint identity, and fail-closed validation. That is the next bounded repair seam.

Evidence:

- Diagnostic: `target/artifacts/phase31-performance/tail-diagnostic-64e880dc-20260829-170929/diagnostic-performance.json`
- Paced post-fix: `target/artifacts/phase31-performance/tail-postfix-c2c7a39e-20260829-172738/postfix-performance.json`
- Paced confirmation: `target/artifacts/phase31-performance/tail-confirm-c2c7a39e-20260829-172938/confirm-performance.json`

All three runs exited 0 on the same physical Vulkan adapter with six GPU-authoritative founders, zero checkpoint-failure waits, zero ordinary full-neural snapshot calls, zero ordinary full-neural snapshot bytes, and bounded shutdown. No training, evolution, brain asset, rendering-quality, population, cognition, persistence-authority, or rollback-transaction change occurred.
