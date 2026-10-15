# Next training run: remove repeated setup before redesigning collection

Inspected main: `69f00c05ec6774c3655ae87c5d4baa5df3861929`, fetched and
confirmed on 2026-10-15 in the saved A-Life cloud environment. Implementation
branch: `codex/next-training-serial-cuts-20261015`.

Two independent cuts are prepared:

- `3474ccba`: retain the trainer's immutable device-local pipelines across replay
  shape changes. A normal uninterrupted 16-decision cycle needs one set of 17
  pipelines instead of three sets (51 creations).
- `8d05c0f6`: acquire each compressed replay file once, authenticate its complete
  bytes, then decode those same bytes. Length, digest, schema, typed state,
  causal continuity and behavior validation remain real on every load.

**No RTX3050 speedup is measured here.** The CPU I/O fixture gives essentially
equal timings: 493.699 ms old versus 491.305 ms new per 200 loads, with overlapping
sample ranges. The reader cut reduces file traffic and removes the hash/decode
race; it is not evidence of a substantial wall-time gain. The pipeline cut has
the stronger immediate opportunity, but requires a later Windows/GPU receipt.

The supplied observations of about 63.3 accepted fresh episodes/hour on October
14 and about 94/hour in today's preliminary interval are context, not a matched
benchmark or a causal result of these changes. The latter includes startup
assessments and a deadline handoff. No production machine was accessed, no GPU
training or GPU tests were launched, and no owner was contacted. PR58's HUD work
was neither checked out nor modified. PR51's actual capture and exact restore
verification remain unchanged.

## Complete serial path and what its work protects

All source links below are pinned to the inspected main unless stated otherwise.

| Boundary | Actual work and protection |
| --- | --- |
| Host launch | [run_grab_food_night.py:687](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/scripts/run_grab_food_night.py#L687) hashes the pinned binary and current checkpoint files and checks lane ownership before every child. [run_child:431](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/scripts/run_grab_food_night.py#L431) creates a native process and log files and polls it every 0.5 s with resource/STOP guards. These detect external edits, competing work and failed/overbudget children. |
| Source admission | [load_cycle_continuation:297](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L297) reads the sealed receipt, canonical foundation, actor and value JSON. It verifies objective compatibility, policy/optimizer age, exact actor weight bits and the exported asset binding before output ownership/GPU construction. |
| World/device setup | [cycle:2583](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L2583) constructs the new seeded world and actual lesson arrangement. [backend:2877](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_gpu_backend/src/closed_loop_runtime.rs#L2877) acquires the device and builds production kernels once per child. Staging backends already share its device, queue and immutable kernels; they are **not** additional device acquisitions. |
| Initial durable world | [cycle:2693](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L2693) validates/attaches a portable base, captures actual GPU state, publishes the exact save and reopens the durable boundary. [attach:127](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime/checkpoint_runtime.rs#L127) itself publishes/reopens the preliminary base first. The exact publication is required for later sleep journal/recovery authority. |
| Collection | [staged_tick:858](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L858) captures before inference and again after inference. [capture_training_state:4840](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_gpu_backend/src/closed_loop_runtime.rs#L4840) reads the slot's mutable arena. Both are actual GPU snapshots; lifetime learning and biology intervene before the next decision, so an adjacent snapshot cannot simply replace either one. |
| Replay persistence | [writer:4016](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training.rs#L4016) checks source/continuity/patch evidence; [write_replay_record:4196](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training.rs#L4196) compresses, hashes, flushes and syncs each record before rename. Later loads authenticate disk bytes and independently validate decoded state. Append validation cannot replace load validation. |
| Trainer initialization | [cycle:2848](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L2848) creates training scratch, restores the actor and captures/verifies actual GPU Adam/weights once. PR51's verified serialized bytes become replay identity. |
| Values and PPO | [cycle:3170](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L3170) reloads replay for frozen old-value prediction and verifies the restored value head. [PPO:1219](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_training/src/ppo.rs#L1219) separately checks the frozen on-policy ratio, then reloads/evaluates windows per epoch, accumulates gradients, gates KL, preflights the value update and independently backs up its head before joint apply/rollback. |
| Export/handoff | [cycle:3420](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L3420) reads trained GPU weights, writes/redecodes the actual canonical file, builds a separate cohort and requires a real GPU decision. Trainer rebind checks topology and actual GPU weights before final actor/value capture and receipt publication. |
| Host acceptance | [runner:805](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/scripts/run_grab_food_night.py#L805) validates acquisition, reward, fresh realized pose, policy/Adam continuity, source hashes and artifact size before advancing the accepted lineage. It rewrites the cumulative run receipt. Frozen Grab and Eat assessments remain distinct evidence rather than fresh examples. |

## Ranked implementation priority

Opportunity estimates below are **work-count estimates**, not GPU timings. For
scale only, 94 accepted/hour corresponds to an amortized 38.3 s/accepted example;
saving 1 s would improve that illustrative rate about 2.7%, and saving 5 s about
15%. This is not a measured per-episode component breakdown. Do not add these
opportunities: persistent continuation would subsume some smaller cuts.

### 1. Retain trainer pipelines across scratch reallocations — implemented

[prepare_replay:306](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_training/src/trainer.rs#L306)
calls `TrainerGpuState::new_for_replay` on every changed tick count, burn-in or
dendrite shape. [The constructor:1713](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_training/src/trainer.rs#L1713)
rebuilds one shader module and 17 pipelines, although their WGSL, entry points,
explicit binding layout and compilation options do not depend on those shapes.
Per-window offsets live in buffers, not pipeline specialization constants.

For 16 decisions with one bootstrap, no sleep and both default PPO epochs:
initial scratch -> 17-row values -> 16-row loss makes three sets. Retention
removes **34 pipeline creations and two module/layout builds per episode**;
3000 such episodes would remove 102,000 pipeline creations. Estimate savings as
the measured cost of those two repeated constructor kernel sections, not 34
times an assumed driver cost: drivers may already cache part of compilation.

Smallest slice: clone the same-device pipeline/layout handles when replacing
scratch; retain fresh buffers/bind groups, size checks and both actual GPU
weight/Adam copies. This is the implemented slice. Risk is a stale resource
binding or accidental cross-device reuse. The constructor is private and reuse
comes only from this trainer's prior state. The expanded GPU regression cycles
1/17/16 rows and burn-in changes, comparing logits, full numerical replay and
serialized complete checkpoints after nonzero Adam updates. It **compiles but
was not executed** here. Existing CPU WGSL validation and trainer tests pass.

### 2. Pack sealed checkpoint readbacks into one submission — design only

[checkpoint:1433](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_training/src/trainer.rs#L1433)
reads moments, weights and update ages separately. Each reaches
[read_word_buffer:1511](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_training/src/trainer.rs#L1511),
which creates staging, submits, maps and waits. The trainer is sealed, with no
update between these reads. Each ordinary cycle captures the initial and final
actor: **six submissions/waits can become two**, with the same 512 KiB per
N2048 capture and the same actual GPU source bytes. PR51 does not address this
within-capture synchronization cost.

Smallest slice: issue two source copies (weights and the contiguous three-lane
optimizer buffer) into disjoint ranges of one staging buffer; map once; decode
float lanes with finite checks and ages as integer words. Keep the exact restore
comparison and all final capture calls. Estimate: four readback setup/fence
overheads per ordinary episode; bytes are not reduced. Risk: mixed lane offsets,
loss of integer ages or incomplete error propagation. Before adoption, compare
every bit, including signed zero and nonzero moments/ages, with the old capture;
exercise mapping/device failure and restore -> next Adam update equivalence on
the same Windows adapter. No runtime implementation or GPU evidence is claimed.

### 3. Bounded serial continuation reusing the device — largest ceiling, design only

[runner:796](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/scripts/run_grab_food_night.py#L796)
launches a new executable per fresh example, repeating device/production-kernel
construction, trainer construction and checkpoint parsing. Compare the two
small patches first. Then prototype **two sequential episodes per worker**, not
parallel collection from one stale policy. Keep a fresh world/realized pose,
16 learner decisions, temperature 32, prior-on failure receipts and the complete
actor/value Adam update order for each episode.

Smallest slice: retain only device/immutable kernels, construct an independent
runtime through existing `new_staging_like_live`, and continue to write and
independently reopen every sealed cycle. Python validates each published episode
and acknowledges its hashes before the worker advances. STOP/resource/deadline
checks remain active while waiting for acknowledgements. A dead worker restarts
from the last host-accepted seal; partial work remains unaccepted. Personal
memory, chemistry, eligibility and RNG state must not leak across fresh worlds.

Estimate: for k serial cycles, device/production-kernel setup falls from k to 1;
the setup-only saving ceiling is `(k-1)/k` (50% at k=2, 87.5% at k=8). This is
**not** an end-to-end estimate. Only measured setup fraction can convert it into
one. Keeping actual disk reopen/restore initially makes failure recovery
reviewable before considering in-memory head continuity. Risks are loss of
process isolation, resource retention and accepted-lineage races. Required
regressions: cold/warm equality of complete ordered receipts/head bits for the
same seed schedule; termination before/after seal and acknowledgement; recovery
only from accepted state; no extra examples counted for next-cohort probes.

The existing next-cohort probe cannot simply become the next training example:
it currently uses the preceding seed and an unconfigured lesson world. Its
staging backend already shares production kernels, so removing it would not
remove another full production kernel build.

### 4. Retain a bounded verified PPO window and separate persistent buffers — design only

[cycle:3178](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L3178)
loads 17 value rows. The [PPO callback:3344](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L3344)
then reloads 16 loss rows for frozen verification and each of two default epochs:
**65 record decodes/validations for 16 fresh decisions** in this uninterrupted
case. Epochs remain training repeats. They are not 65 fresh examples.

Smallest slice: cache one verified 16-row loss window under source identity,
record digests/segment/indices and phenotype hash, for just this update transaction.
To retain detection of actual changed/corrupt disk files, authenticate compressed
bytes on every use and only reuse typed decode/validation when they exactly match
the cached identity. Keep on-policy ratio, fresh forward passes, KL and updated
gradient calculations; those inputs change after updates. Evict on any key change
or memory-budget pressure. Estimate: eliminate up to 32 repeated typed record
decodes/validations per ordinary episode; file authentication remains. This
synthetic I/O fixture is not enough to predict full record-validation cost.

Separately, [scratch replacement:329](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_training/src/trainer.rs#L329)
also recreates weights/optimizer, zeros them and GPU-copies 512 KiB at each shape
change, and repacks/sorts topology metadata. Split trained buffers from scratch
in a later independent change; keep genetic weights, both moments, integer ages
and mask resident. This saves two such copies (1 MiB) in the ordinary 17 -> 16
case including the initial non-replay -> 17 replacement. It has more binding risk
than pipeline retention. Validate shape oscillation, masked/frozen weights and
next-update equality, including fresh dendrites and cohort invalidation. Neither
cache nor persistent-buffer design is implemented here.

### 5. One authenticated file acquisition per replay load — implemented, small

[old loader:4242](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training.rs#L4242)
hashes a file, rewinds and decodes from the same mutable file handle. It can read
a different image after hashing. The new loader reads at most expected length +
one byte, rejects short/growing data, checks full BLAKE3 before invoking zstd,
then performs all existing decode and semantic checks from that buffer. There is
no across-load integrity cache. The extra compressed buffer is bounded by 256
MiB and fits the existing encoded-size x8 plus record-limit admission estimate.

Estimate: one file acquisition instead of hash pass plus decoder file reads for
each of the normal 65 loads. Fixture compressed length is 120,859 bytes; this
removes roughly 7.5 MiB of repeated file traffic per corresponding 65-load
synthetic workload. CPU timing below does **not** establish material elapsed-time
savings. Risk is peak memory rather than changed cognition. Bound/error/bit
tests pass; per-load schema, exact-state and continuity tests remain active.

### 6. Avoid rebuilding sensory input merely to prime a synchronous prior — hypothesis

[prime_foundation_semantic_prior:6078](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime.rs#L6078)
builds a perception index/draft immediately before collection;
[staged_tick:570](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L570)
builds them again for actual cognition. The deterministic provider generates
synchronously on a cache miss in [prepare_inner:546](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime/semantic_prior.rs#L546).
Priming an async local provider can hide latency; treating both modes alike may
duplicate a full sensory preparation in the current deterministic campaign.

Smallest investigation: CPU compare deterministic `prime -> prepare` with
`prepare` for the **actual same current draft**, hearing expiry, active-hint state,
provider failure and developmental gain. Then check real runtime sequencing:
resident/body synchronization and sleep eligibility occur between the current
prime and preparation. Only skip synchronous priming if all neural input bits
and failure behavior agree; preserve prior-on and logged unaided continuation.
Estimate: up to one extra index/draft construction per prime attempt, including
sleep/no-progress attempts; elapsed cost unknown. No code change here because
priming also mutates provider history/controller state and its metric trace.

## Smaller candidates examined and rejected or deferred

- Initial durable setup publishes a preliminary base and then an exact base
  without a learner tick between them. A future explicit "attach exact initial
  boundary" could publish one generation, **after** real GPU capture and with
  independent reopen validation. Do not delete the exact base: sleep journal
  authority depends on it. [publish_snapshot:1372](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_runtime/src/checkpoint_assets/durable_manifest.rs#L1372)
  and [rebind:275](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime/checkpoint_runtime.rs#L275)
  authenticate authority generations, assets and journals. Consolidating their
  interfaces needs failure-injection evidence; it is not a safe blind dedup.
- Preliminary and focused recall in [cpu_preparation:135](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/gpu_live_runtime/cpu_preparation.rs#L135)
  and line 223 have different inputs/purposes: recall informs attention, then
  attention changes candidate routing and novelty. Reusing the first result
  would alter cognition. Training's audited memory/topology policy remains.
- A checkpoint export, disk reopen, new-cohort decision and optimizer rebind
  test different failure boundaries. Reusing GPU capture bytes is already PR51;
  do not substitute a CPU source object for actual restored-device evidence.
- PPO's frozen ratio pass precedes all updates. Later-window equality after a
  prior minibatch is not equivalent. Keep it, KL decisions, value preflight and
  independently submitted rollback backup. Reusing immutable inputs is allowed;
  reusing outputs/gradients after updates is not.
- Per-row `sync_all` is potentially expensive, while partial episodes are never
  resumed. A seal-level group-sync protocol could amortize it, but all record
  contents/renames must become durable before accepting a seal, including sleep
  failures. No fsync was removed without that persistence design.
- The 0.5 s child poll introduces 0..0.5 s exit-detection delay (0.25 s mean only
  under a uniform-phase assumption). An interruptible bounded `process.wait`
  could remove most of this while retaining guard cadence. It is a small ceiling
  relative to worker/device reuse; no production wait policy was changed.
- Binary/checkpoint hashing and JSON parse in both native and Python cover
  distinct trust boundaries. A file's unchanged mtime is not proof its contents
  are unchanged. Keep actual authentication. The growing `night.json` rewrite
  and recursive artifact-size walk merit timing at thousands of episodes, but
  there is no evidence they dominate this run.
- Logging is mostly explicit phase transitions, 64-world-tick progress and
  once-per-child receipt output, rather than an unconditional log flush on each
  neural tick. No basis was found to prioritize disabling logs or failure evidence.

## CPU verification and measurement limits

Linux cloud CPU only, AMD EPYC 9V74/KVM, five exposed logical CPUs, four-core
cgroup quota, Rust 1.98.1. No adapter/device acquisition was made by these checks.

- `cargo +stable test --locked -p alife_training --lib`: 13 passed.
- `cargo +stable test --locked -p alife_game_app --features foundation-training
  --lib replay_io_tests`: 3 passed, timing test ignored in the ordinary run.
- The emitted current foundation-training test executable with filter
  `foundation_ --test-threads=1`: 89 passed, one timing test ignored. This includes
  Grab/Eat legality/biology, sampling, sealed optimizer handoff, objective
  continuity, PR51's exact-byte checks, source/output protection and replay codec.
- Python `unittest discover -s scripts -p 'test_grab_food*.py'`: 67 passed.
- GPU regression **compile only**: `cargo +stable check --locked -p
  alife_training --features gpu-tests --test wgsl_foundation_trainer` passed.
- Strict Clippy passed for `alife_training --lib` and `alife_game_app --features
  foundation-training --lib --tests --no-deps`, both with `-D warnings`.
- Formatting, diff whitespace, core boundaries and 77/77 documentation assertions
  passed. Windows PowerShell checks and real GPU execution remain untested.

The [CPU receipt](evidence/20261015-next-training-replay-io.json) includes every
timing sample and source/executable identities. The fixture is a synthetic
N2048-shaped `TrainingSequence` with nonzero random weight offsets, signed zero
and zstd/bincode encoding. It is **not a captured episode**, a complete record's
behavior/patch validation, an optimizer run or a neural benchmark. Both paths
use the same binary and decode the same input; the old path is a literal frozen
hash/rewind/file-decode path. Open, metadata, hashing, decoding, allocation and
destruction are timed. File setup and warmup are outside the timer; typed
bit-equivalence is checked by a separate regression. Authentication remains timed. App code has test-profile opt-level 2 with debug assertions;
dependencies have their repository test settings, not a release build.

Nine alternating old/new pairs, 200 loads per sample, affinity CPU 0, no
concurrent builds: old median 493.699 ms (457.747..547.118); new median 491.305 ms
(465.698..514.908). The 0.5% median difference is below the observed variability;
no material wall-time improvement is established. This supports prioritizing
setup/fence reductions before more filesystem tuning.

Raw output and preserved executable:
`/workspace/cloud-cpu-validation/next-run-serial-cuts-20261015/`.
Reproduce by building the foundation-training library tests with the recorded
toolchain/settings, then running the emitted binary's exact ignored test
`foundation_training::replay_io_tests::replay_verified_read_cpu_timing` with
`--ignored --nocapture --test-threads=1`, preferably pinned to one CPU and without
other builds. The benchmark writes only a create-new temporary synthetic file.

## Falsifiable next-run timing plan

Use the existing owner's eventual matched receipts; do not interrupt today's
production. Keep binary/source pinned, identical seed schedule, 16 trained
decisions, temperature 32, deterministic prior/on, curriculum and assessments.
Report fresh accepted episodes separately from rows, replay and epochs.

Existing [timing.json:3127](https://github.com/BobertNeek/A-Life/blob/69f00c05ec6774c3655ae87c5d4baa5df3861929/crates/alife_game_app/src/foundation_training_cycle.rs#L3127)
and cycle receipt timings are incomplete: `collection_seconds` starts **after**
world/device/initial durable setup; `update_seconds` starts **after** old-value
prediction and GAE and ends **before** export/next-cohort/final checkpointing.
Host episode seconds include native execution and acceptance, but exclude the
subsequent frozen assessment and cumulative receipt publication. Subtracting
collection + update from host seconds locates an unassigned serial remainder;
it does not identify the component responsible.

Add a separate profiling receipt around source decode, world/lesson creation,
device/production kernels, attach/exact publication/reopen, trainer kernels,
initial exact optimizer verification, actual collection, replay append/sync,
old-value reload/forward/readback, PPO load/forward/gradient/preflight/update,
export/admission/final readbacks and host hash/validation/process gaps. Include
call/byte counts and peak host/device allocation. Do not feed timings into policy.

Test the small pipeline change first on matched short cold runs. If its two
removed kernel-build sections are negligible, reject it as a major speed lever
and rank warm device continuation by measured per-child setup fraction. If
checkpoint/readback fences dominate, prototype packed capture before a worker
protocol. If collection itself dominates, measure priming and audited preparation
without weakening the causal world step. Report sleep/terminal episodes
separately rather than quietly excluding difficult examples.

These changes preserve AOA-INV-003/009/010, AOA-AUTH-002, AOA-PERSIST-002/004 and
AOA-PERF-001/005: one GPU authority, shared causal organism semantics, explicit
failure, exact learned-state persistence and bounded, source-bound evidence.
No migration, trained-checkpoint discard, CPU neural fallback, reduction of
brain/population/curriculum, learning/biology switch-off or fabricated hints is
introduced. Whole-game competence and Windows/GPU performance remain Unknown.
