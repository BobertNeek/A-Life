# Replay worker burst risk assay

The worker added a small amount of elapsed time in three 32-row CPU fixture
pairs. This establishes neither a collection-overlap benefit nor a formal
production regression bound. No default behavior changed in this evidence branch.

| Order | Serial ms | Worker ms | Worker elapsed increase |
| --- | ---: | ---: | ---: |
| Serial, worker | 57.124 | 57.811 | 1.202% |
| Worker, serial | 55.735 | 55.907 | 0.309% |
| Serial, worker | 55.754 | 57.132 | 2.471% |

The median paired increase was **1.202%**. Separately, the serial median was
55.754373 ms and the worker median 57.131799 ms, a 2.471% ratio-of-medians
increase. The first pair is retained; three alternating pairs establish no
confidence interval or tail-latency claim. No 64-row or follow-up timing run
was performed.

Both paths moved identical owned records through the existing `EncodedWindow`
and unchanged codec/cohort writer. Timers include append validation/encoding,
file writes, queue backpressure, final worker drain, durable seal and publication.
All fixture construction, list cloning, cohort/window construction and persistent
thread creation were outside timing. The `start()` enqueue was also outside
timing, but has no acknowledgment; its trivial residual processing could remain
inside the first timed append.

This is an N2048/32,768-synapse **typed-record** fixture: individually validated
CPU-world patches plus synthetic neural context, not a coherent captured or
acquired neural trajectory. It excludes `FoundationReplayWriter` snapshot
conversion/cross-capture continuity, Windows process-memory guards, production
retention/cache costs, registered run quota/admission, GPU work and real collector
overlap. No artificial producer work or sleep was added. Storage loading reads
and authenticates actual bytes before any optional cache lookup; its false
admission callback denies retention, not lookup. The isolated unique-source assay
does not populate that decoded-record cache.

After both timers closed, every reference, index digest and full bundle matched.
Each of the 32 rows in both files was authenticated, loaded and compared bit for
bit; changing the expected policy source was rejected. The matched input was
6,083,463 uncompressed bytes and each sealed bundle 4,036,851 bytes, with one
durable cohort commit and data synchronization per side.

The actual Cargo `--release` build optimized all dependencies. The host was an
AMD EPYC 9V74 Linux VM with a four-CPU quota and 16 GiB limit; files used workspace
overlayfs. Physical storage and cold-disk behavior were not established. GPU
selection was disabled for this filtered test. Whole-process peak child RSS was
**65,492 KiB (63.96 MiB)**, including fixture setup, both variants, buffers and
verification; it is not incremental worker memory or VRAM.

Measured source is clean commit `36b52b1413da9f3fe968a2f13aa716701115732c`, tree
`3a0a1e1bb57a84eff08cbe0acefda582f382544b`, based on merged main `a9288027`.
The later evidence changes only a cache-retention comment and documentation.
The app build script embeds no Git revision. The
[receipt](20261031-replay-worker-burst.json) preserves all raw pairs, build output,
source/tree/file/binary hashes and process context. Root independently checked
the statistics, binary hash and all six measured source hashes. Focused formatting,
whitespace checks and strict release Clippy also passed.

Reproduce with the same release profile and otherwise quiet compiler work:

```sh
source /workspace/cloud-cpu-validation/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life-replay-burst/target/burst-release
ALIFE_REPLAY_BURST_ROWS=32 VK_ICD_FILENAMES=/no-vulkan-icd.json cargo test --release --locked -p alife_game_app --features foundation-training --lib replay_worker_burst_risk_assay -- --ignored --nocapture --test-threads=1
```

Actual training benefit still requires matched collection plus replay drain/seal
on the real capture path, preserving exact learning/persistence and counting
inclusive host time. This burst result alone does not justify enabling or
disabling the production worker.
