# Frozen-gradient full PPO release assay

The experimental cache reduced elapsed time in five paired runs of the actual
PPO controller on software Vulkan. Median paired reduction was **19.06%**
(1.235× paired ratio); individual reductions ranged from 14.35% to 21.48%.
Separately, original/cache medians were 4.245073/3.482260 seconds, a **17.97%**
ratio-of-medians reduction (1.219×). Five pairs establish no confidence interval.

This is a synthetic controller-component result using the legacy allocator,
without the separate capacity-reuse candidate. It is not physical GPU or
whole-training throughput, biological learning, or evidence of a 10× gain.
No default, CLI, schema, shader or normal device-admission behavior was changed.

| Pair | First mode | Original seconds | Cache seconds | Paired reduction |
| --- | --- | ---: | ---: | ---: |
| Warmup, retained | Original | 4.239087 | 3.529993 | 16.73% |
| 1 | Cache | 4.245073 | 3.436119 | 19.06% |
| 2 | Original | 4.451253 | 3.558802 | 20.05% |
| 3 | Cache | 4.330130 | 3.400057 | 21.48% |
| 4 | Original | 4.236155 | 3.482260 | 17.80% |
| 5 | Cache | 4.163954 | 3.566376 | 14.35% |

Exactly one warmup pair and five alternating measured pairs ran; all raw times
are retained, with no retries or additional assays. Actual adapter information
was CPU llvmpipe LLVM 19.1.7, Mesa 25.0.7, Vulkan. Cargo `--release` optimized all
dependencies. A dedicated target forced current local core/storage/world/backend/
runtime/training compilation; optimized third-party dependency artifacts came
from a completed release build. Compilers were quiet during the assay.

Both paths invoke the same actual controller with identical immutable Arc
windows and full actor/value checkpoints. The procedural N2048/S32768 fixture
uses every third synapse as trainable: 10,923 coordinates. Windows with 256 ticks / burn-in 0 and
384 ticks / burn-in 128 each contribute 256 loss rows; every tick uses all four microsteps,
without speech labels or auxiliary loss. GPU-generated collection log
probabilities and values are fixed once outside timing. The target uses 2 epochs,
effective batch 8, value coefficient 0, entropy 0 and target KL 100; these are chosen
fixture settings, not a complete production workload. Full configs are in the
[receipt](20261031-frozen-gradient-release.json).

A prior actual accepted update with value coefficient .5 seeds nonzero actor and
value-head history. Before each clock, both modes restore that exact boundary,
precreate the 384-tick replay and PPO objective, assert persistent bits, and flush
all setup submissions. The clock includes the whole controller call: frozen
validation, Arc loaders, packing, forwards, objective, BPTT/ordered accumulation,
normal metric/status waits, KL checks and Adam. Construction, collection,
checkpoint restore, initial allocation, setup flush, additional proof readbacks
and comparisons are outside the clock.

After every pair, all 14 captured sections/**9,711,889 u32 words**, plus receipt
bits and six loader indices, matched exactly. Each mode accepted two actor steps
(ending at 3 after warmup), changed 287 trainable weights and had 177 coordinates
with nonzero first/second moments. All 10,923 trainable integer ages advanced
by 2; all 21,845 frozen coordinates preserved weight/moment/age bits. Nonzero
value-head parameters/moments stayed bit-exact at value step 1. The cache requested
278,544 bytes, cached/reused exactly two raw vectors, and was discarded before
update; the second epoch performed fresh original evaluation. Full logical
state/output, all five value planes and objective fields were compared.

Whole-process peak child RSS was **530,424 KiB (517.99 MiB)**, including fixed
windows, software GPU buffers and proof captures. It is not incremental cache
memory or VRAM. The source-only maximum eight-window cache bound remains
1,114,176 requested bytes; no whole-training memory or throughput claim follows.

Measured source was clean commit `470dde2e37843c3f93af908f3b08664508145cfe`.
Binary SHA256 was `13e8224ab14b51b08091f77eacc2990b98b47ac4ede3dcaa4d16035c2face271`.
The receipt preserves the tree, source/library/binary hashes, both build logs,
all trials and process context. Root independently recomputed the statistics
and checked the raw JSON, binary/library hashes and all six measured source hashes. Later comment/lint-only changes clarify preparation wording and retain the
runtime release guard under strict Clippy; no timing behavior changed. Strict
release all-target Clippy passed. The preceding
[correctness gate](20261031-frozen-gradient-cache.md) additionally covers unequal
3/4-loss windows, positive value updates, failure ordering and KL restart.

```sh
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life-frozen-gradient-cache/target/frozen-release
export ALIFE_SOFTWARE_PPO_CAPTURE_DIR=/workspace/software-ppo-frozen-release
export ALIFE_FROZEN_ASSAY_STOP_FILE=/tmp/alife-frozen-gradient-release-stop
cargo test --release --locked -p alife_training --features gpu-tests --test software_ppo_controller frozen_gradient_assay::representative_actual_ppo_frozen_gradient_release_assay -- --exact --ignored --test-threads=1 --nocapture
```

Physical GPU qualification, combination with capacity reuse, nonterminal replay,
speech/auxiliary and memory-cap fallbacks, and forced actor-apply/value rollback
remain unqualified. This stays experimental, without a PR or default promotion.
