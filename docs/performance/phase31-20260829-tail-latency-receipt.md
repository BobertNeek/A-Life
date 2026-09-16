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

## 2026-09-01 asynchronous persistence and pacing follow-up

The earlier pacing rejection above remains the historical result while sleep-journal publication was synchronous. The follow-up first moved sealed, generation-checked journal publication off the update thread, then repeated the one-tick production pacing experiment on top of that repair.

| Metric | Original diagnostic | Async persistence | Async plus pacing |
| --- | ---: | ---: | ---: |
| Source | `3d02c05a` | `2766e344` | `7416ec15` |
| Completed world ticks | 561 | 903 | 873 |
| True TPS | 9.349 | 14.972 | 14.546 |
| Average FPS | 46.61 | 25.63 | 30.31 |
| Frame p50 | 9.26 ms | 11.49 ms | 11.38 ms |
| Frame p95 | 42.89 ms | 221.86 ms | 104.33 ms |
| Frame p99 | 312.30 ms | 540.99 ms | 160.01 ms |
| Maximum frame | 1594.68 ms | 785.44 ms | 257.99 ms |
| Update-thread journal enqueue | synchronous | 0.663 ms total | 0.587 ms total |
| Ordinary full-neural snapshots | 0 | 0 | 0 |

The asynchronous run completed five journal worker transactions with zero failures, a pending-entry peak of 12, an idle persistence terminal state, and a clean exit. The paced run completed four journal worker transactions with the same bounded peak, zero failures, an idle terminal state, and a clean exit. Exact checkpoint capture, atomic manifest CAS, rollback, stale-generation rejection, and shutdown draining remained enabled.

The retained worst-frame evidence identifies the pacing effect directly. Before pacing, 79 of the worst 100 frames executed four ticks and 11 executed three. After pacing, all retained worst frames executed one tick. Pacing reduced p95 by 53%, p99 by 70%, and the maximum stall by 67%, while true TPS decreased 2.8%. The scheduler preserved unspent planned ticks in its existing bounded accumulator; it did not skip cognition, reduce population, or weaken persistence.

The pre-run machine samples found no competing game process. Listed background CPU was 14.6% before the asynchronous run and 4.8% before the paced run; ADB accounted for 3.5-4.0%. This load difference prevents treating small changes as precise, but it does not explain the elimination of every three- and four-tick retained slow frame.

The async repair and pacing change are accepted as layered intermediate improvements. The full performance phase remains open because the paced p95 is still above the original 42.89 ms diagnostic. Mean runtime-tick cost was 50.69 ms in the paced run, making ordinary tick work the next measured seam.

Evidence:

- Async persistence: `target/artifacts/phase31-performance/sleep-journal-clean-rerun-valid-20260901-000548/performance.json`, SHA-256 `d848c595be5d7d4f96d4f6622ca0751b5a4a5e7ec7ffbdfff4088d6e555eba04`.
- Async plus pacing: `target/artifacts/phase31-performance/sleep-journal-paced-clean-20260901-001722/performance.json`, SHA-256 `968894077d86734fd6f1ed221201a28532c851495e7417257091f081a5ee2af4`.
