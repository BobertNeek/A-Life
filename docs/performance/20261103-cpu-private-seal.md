# CPU collection: retain the completed private-state seal

Baseline: main `b2f7a81f36acc733313c0db627c730cfe5245f93`; implementation
`1e72c568191576908d947868076db20e788d474a`. This change affects
the supported native CPU collector used by serial and cooperative training.
It does not change replay, neural arithmetic, PPO, Adam, checkpoint formats,
GPU execution or the separate predictive experiments.

## Measured cost and change

Fresh public N2048 reference workloads collected 65 actual decisions each under
object-slot and terrain-vision sensors, then trained the native residual PPO
learner. Initial collection took 2,367.55 / 2,356.97 ms. Temporary phase timers
on another reference run measured admission at 698.71 / 669.94 ms out of
2,406.93 / 2,283.95 ms collection. Admission includes founder validation and
resident authority checking; these are not exclusive resident-hash timings.
Waking credit took only 50.16 / 48.49 ms, so receptor arithmetic was left alone.

At each completed boundary, the host already computes a full digest of private
neural state, memory, topology, predictor, sleep, replay and sampling context
and seals it into the canonical world record. Before the next step, it formerly
serialized and hashed that unchanged private state again. The private resident
has no external mutable alias; callers can mutate the world through `world_mut`.

The host now retains the completed brain/memory seal in an unexported field,
bound to the sampling seed and counter. Step admission compares the current
world record against that seal. All resident seals are invalidated in the staged
host before a step can mutate private state or the shared counter. Recovery
sleep also invalidates its resident before mutation. Successful transactions
recompute and reseal the complete state before committing; failed staged
transactions leave the original host and seal intact. Snapshots clone the owned
boundary. Cold restore still recomputes the full digest from actual decoded
checkpoint state and compares it with the restored world before establishing a
process-local seal. No seal is loaded from serialized data.

## Supported CLI measurement

One cloud Linux x86_64 VM reported AMD EPYC 9V74, five visible CPUs, a four-CPU
quota and 16 GiB memory limit. Rust 1.99.1 / LLVM 23.1.1; test profile with
`alife_training` opt-level 3, core/world/runtime opt-level 2, debug assertions
enabled and debug symbols disabled. No LTO or native-CPU compiler flags.
Both executable variants were built before timing. The reference CLI was rebuilt
from a clean detached checkout of exact main; both executables and all timed
logs were checked for absence of the temporary `CPU_PROFILE` instrumentation.
Each process was pinned to CPU 2; no build or other test ran concurrently. VM contention/frequency remained
uncontrolled. This is not a release-build or target-PC result.

Eight CLI processes ran in ABBAABBA order over four matched disposable founder
seeds, 847401..847404. Each used `--fixture --ticks 65 --temperature 0.9
--replay-execution reference` and a new output directory. These are deliberate
matched replayability fixtures, not claims of historically unused experiment
seeds. Each pair retained 65 native decisions and 64 training rows. Seed 847401
stopped for KL after one paired actor/critic update; each other pair completed
two updates. Budgets and stopping decisions match within every pair.

| Boundary, median seconds | Reference | Retained seal | Wall-time change |
| --- | ---: | ---: | ---: |
| External complete CLI process | 3.9357 | 3.4175 | 13.2% less |
| Internal collection component | 2.2601 | 1.7276 | 23.6% less |
| Internal update component | 0.1970 | 0.2063 | 4.7% more |

The measured complete-process ratio is **1.15x** for this small workload.
External time includes founder compilation/admission, world setup, collection,
training, checkpoint serialization and output. The learner was not optimized;
its slower timing in this run is reported rather than attributed to a learner
change. Interleaving reduces ordering bias but does not eliminate VM variability.
Component medians do not sum to a process median. There is no confidence interval, long-life scaling result, behavioral learning claim or GPU speed claim.

[Raw samples, phase scopes, source/binary hashes and output hashes](20261103-cpu-private-seal.json)
include every CLI trial. The one-off native benchmark also preserves seven
learner samples per profile: learner medians changed from 159.92 / 198.89 ms
to 182.14 / 244.52 ms in those sequential runs, another reason not to attribute
a learner gain to this collection-only change.

## State and integrity verification

All ten non-timing CLI output files match byte-for-byte in every seed pair,
including the compressed private collector checkpoint, collection actor state,
compiler inputs, both learned optimizer checkpoints, trained foundation and
training handoff. Cycle receipts differ only in collection/update/elapsed times
and executable identity. The trained artifacts remain local and unpublished.

The separate native object-slot/terrain-vision benchmark produced identical
decision hashes, complete private-state hashes and complete actor/critic/update
receipt bytes: 4,688,876 bytes, SHA-256
`948831e152fe9d679404200661574d714d724d579bb28f85c01860ac29981709`.
That fixture retains nonzero moments, frozen coordinates and unequal integer
optimizer ages including 16,777,217. The native rollout regression additionally
requires actual recurrent first-moment updates. Replay/BPTT arithmetic is
unchanged.

Local verification passed: 37 CPU unit tests and 58 integration tests (three
explicitly opt-in benchmarks ignored), strict Clippy, formatting and the
feature-disabled build. The native ignored benchmark was run separately.

Focused regressions cover changed world brain/memory seals before preparation,
atomic failure, cold private-state tampering, snapshot versus cold continuation,
recovery sleep and an awake peer advancing the shared counter while another
resident sleeps. Existing CPU recurrent-gradient, Adam and checkpoint tests
provide the adjacent learning/state checks.

## Reproduction commands

Use the same compiler and test-profile settings for both revisions. On this VM,
the existing cloud environment script selected Cargo/Rust homes, target directory,
two build jobs, no incremental compilation/debug symbols and one test thread.

```sh
source /workspace/cloud-cpu-validation/env.sh
cargo test --locked --config profile.test.package.alife_training.opt-level=3 \
  -p alife_training --features cpu-backend --test cpu_collector \
  native_collected_replay_ledger -- --ignored --nocapture
```

Set `ALIFE_NATIVE_BENCH_TAG` and `ALIFE_BENCH_OUTPUT_DIR` before that command.
The clean CLI baseline was built with:

```sh
git worktree add --detach /workspace/A-Life-cpu-baseline \
  b2f7a81f36acc733313c0db627c730cfe5245f93
cargo build --locked --profile test \
  --config profile.test.package.alife_training.opt-level=3 \
  --manifest-path /workspace/A-Life-cpu-baseline/Cargo.toml \
  -p alife_training --features cpu-backend --bin train_cpu_foundation
```

Keep a copy of each built `train_cpu_foundation` before switching revisions.
Execute each of these with a fresh output path, in the recorded ABBAABBA order:

```sh
taskset -c 2 "$ARM_BIN" --fixture --output "$NEW_OUTPUT" \
  --seed "$SEED" --ticks 65 --temperature 0.9 --replay-execution reference
```

Use an external monotonic process timer; read `collection_seconds` and
`update_seconds` from `cpu-cycle.json`. Compare all other output bytes, and compare
cycle receipts after excluding only the four fields listed in the raw evidence.
All original fixture states and outputs were retained. Initial unoptimized,
overlapping setup attempts were stopped and excluded; preliminary CLI runs
concurrent with compilation were also excluded. Independent review then found
profile timers in an earlier copied CLI reference. That contaminated pilot and
its 20.5% figure were withdrawn before publication; only the clean rerun above
enters the reported comparison. All pilot states/logs remain preserved locally.

Verification commands (same environment, `VK_ICD_FILENAMES=/no-vulkan-icd.json`
for CPU tests):

```sh
cargo test --locked --config profile.test.package.alife_training.opt-level=3 \
  -p alife_training --features cpu-backend --lib cpu:: -- --test-threads=1
cargo test --locked --config profile.test.package.alife_training.opt-level=3 \
  -p alife_training --features cpu-backend --test cpu_backend \
  --test cpu_collector --test cpu_cooperative_round --test backend_handoff \
  --test checkpoint_validation -- --test-threads=1
cargo clippy --locked -p alife_training --features cpu-backend --lib --bins \
  --test cpu_backend --test cpu_collector --test cpu_cooperative_round \
  --test backend_handoff --test checkpoint_validation -- -D warnings
cargo check --locked -p alife_training --no-default-features
cargo fmt --all --check
```
