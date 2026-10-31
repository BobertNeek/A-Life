# Test-only immutable packed-replay reuse

This bounded experiment supports a host serialization opportunity, with a clear
memory tradeoff. In the final synthetic eight-call host fixture, the median was
230.588 ms uncached and 138.564 ms cached (39.9% lower). The cache retains
106,762,752 bytes of packet capacity and raises tracked peak live operation
allocations from 64,453,120 to 109,536,748 bytes. It is not a GPU or end-to-end
training speedup, a lower-memory result, or a production admission decision.

Base source is `1d58e143b1b35128bedd7f15724147b325e4421c`. Everything new is behind
`cfg(all(test, feature = "cpu-backend"))`. Removing the three-line test-module
registration reproduces the base trainer source exactly. No application call
site, normal preparation/upload, neural forward, gradient, optimizer, checkpoint
or cognition behavior changes. The unfinished runtime scaffold was removed.

[All raw trials, memory samples, source/binary/log hashes and commands](20261031-cohort-packed-replay-experiment.json)
are committed separately from this interpretation. Earlier exploratory trials
are retained there as a distinct cohort, without pooling or selecting their
more favorable result.

## Source feasibility and cache lifetime

The existing [policy-local snapshot cache](https://github.com/BobertNeek/A-Life/blob/1d58e143b1b35128bedd7f15724147b325e4421c/crates/alife_game_app/src/foundation_training_cycle/replay_window_cache.rs#L26)
holds up to eight immutable `Arc<PpoTrainingWindow>` snapshots with a 512 MiB
conservative charge limit. Its hits still perform measured process-memory
admission. The [generic cohort interface](https://github.com/BobertNeek/A-Life/blob/1d58e143b1b35128bedd7f15724147b325e4421c/crates/alife_training/src/ppo.rs#L1275)
exposes only `Borrow<PpoTrainingWindow>`, erasing Arc identity. Window index or a
borrowed address alone is insufficient for safe reuse.

The prototype instead accepts an actual Arc and retains a Weak identity plus
the exact integer layout fields read by the production packer. Weak ownership
does not keep replay vectors alive. It prevents `Arc::get_mut`; `Arc::make_mut`
clones or dissociates the allocation, forcing a cache miss. Dead owners are
pruned at the next access. Cache lifetime is one synthetic cohort call, with at
most eight packets and a 256 MiB conservative charge cap. Metadata and packet
retention require an optional admission callback; refusal takes the original
uncached packer path. Hits call admission with zero additional charge. This is
a deterministic test callback, not the production Windows guard.

The [production packer](https://github.com/BobertNeek/A-Life/blob/1d58e143b1b35128bedd7f15724147b325e4421c/crates/alife_training/src/trainer.rs#L2400)
materializes immutable replay inputs, candidates, context, effective private
weight offsets, initial state and reserved zero lanes. Repeating it for the
same admitted immutable window adds no new input information. The prototype
keeps fresh `validate_for` before cache access. Dynamic headers remain separate;
unit tests exercise exact integer optimizer ages beyond 2^24, burn-in and gain
fields. A CPU sink test checks that copying the complete base packet clears
previously written speech-target lanes. That sink is not a queue-write test.

[Normal preparation](https://github.com/BobertNeek/A-Life/blob/1d58e143b1b35128bedd7f15724147b325e4421c/crates/alife_training/src/trainer.rs#L382)
validates before resetting speech state. Shape-hit and shape-change paths have
different device-check/allocation ordering. Any future integration must preserve
those errors, fresh headers and complete base uploads before speech targets.
Current GPU PPO already retains forward state within a window; fresh forwards
after weight updates remain necessary. This experiment caches no activations.

## Host timing conditions and results

The sequence fixture matches the eligible packing assay at `893e6395`. It
compiles an N2048 terrain phenotype (seed 99): 2,048 neurons, 32,768 synapses,
17 routes, four microsteps, 25 candidates/tick, 64 canonical dendritic branches
with 128 inputs, nonzero periodic inputs/offsets and zero structural edges.
Synthetic valid PPO batches supply Arc identity holders; no neural trajectory,
acquired checkpoint or PPO objective is evaluated.

The eight shapes are `512, 1, 256, 384, 256, 384, 256, 384`; burn-in is 128 for
384 and zero otherwise. They represent one uninterrupted 512-loss-row segment
plus bootstrap, followed by two windows' frozen pass and two attempted epochs.
The first two old-value shapes remain uncached. Source fixtures are built before
the clock and held throughout both modes. Header ages vary synthetically from
19,801 through 19,803 to stress refresh; this is not an optimizer-step model.
No optimizer runs or advances a real age.

Each call performs fresh validation, shape clone/comparison, warm metadata/layout
packing, packet creation or lookup, and fresh header construction. Per-call
temporaries and final cache teardown are included. Full-word equality for cached
256/384 packets is checked in an untimed pass; timed calls sample outputs, and
their checksums agree. The separate small-input tests compare every word across
54 deterministic shape/topology/candidate/context combinations.

| Final cohort | Trials after two warmups | Median ms | Observed min–max ms |
| --- | ---: | ---: | ---: |
| Uncached | 11 | 230.587565 | 227.268691–244.323123 |
| Cached | 11 | 138.563604 | 136.573539–144.973598 |

Modes alternate baseline/cache then cache/baseline by trial. The result is an
enclosing **selected host-helper sequence**, not complete `prepare_replay` or
PPO time. No loader/disk/RAM guard, device resource allocation, queue staging or
upload, speech upload, objective/value preparation, command encoding/submission,
wait, neural forward/backward, Adam or persistence is timed. These excluded
costs remain unknown here. There are no new exclusive component timings to add
to this enclosing measurement.

Static packet arithmetic reconciles the saved work: eight serialized payloads
total 405,875,028 bytes. Retaining the two cohort packets removes four repeat
serializations totaling 213,525,504 bytes, leaving 192,349,524 bytes materialized
in this eight-call host fixture. A future implementation must still upload all
405,875,028 base bytes for those eight calls, plus its separate speech payloads.
This does not reduce queue-copy bytes, dispatches, synchronization or Adam work.

An earlier status-only memory-instrumentation variant measured 212.415227 versus
117.799014 ms (11 trials/mode). Its complete raw trials remain in the evidence,
with the missing initial exact binary/source-file hash explicitly noted. The
final source manifest applies to the final cohort above. Neither cohort has a
formal confidence interval. Both use the unpinned AMD EPYC 9V74 cloud VM, five
visible CPUs, four-CPU quota, 16 GiB memory, Rust 1.98.1, test profile with training
opt-level 3, core/world/backend/runtime opt-level 2 and debug assertions enabled.
CPU frequency, VM contention, page and allocator history are uncontrolled.
Counters are disabled for timing, but the allocator wrapper's TLS check remains;
its overhead versus an unwrapped System allocator is unknown.

## Retention, tracked allocations and observed process residency

Memory is measured separately: two fresh processes for each of five modes,
each running only the six 256/384 cohort operations. Source vectors stay alive
through the final sample. Metadata is warmed before the operation counters.
The following uses decimal MB; these columns measure different things.

| Mode | Retained packet capacity MB | Peak live operation allocation MB | Before-drop smaps RSS MB, two trials | Reported whole-process VmHWM MB, two trials |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 0 | 64.453120 | 107.430–107.622 | 164.426–164.704 |
| Cache | 106.762752 | 109.536748 | 202.801–202.932 | 202.801–202.932 |
| Refuse retention before metadata allocation | 0 | 64.453120 | 107.557–107.586 | 164.573–164.663 |
| Evict after frozen pass | 0 | 109.536748 | 107.688–107.692 | 202.994–202.998 |
| Cap at the first retained packet | 42.710016 | 107.163280 | 145.756–145.818 | 202.760–202.834 |

The cache mode's conservative charge is 213,531,680 bytes, versus 106,762,752
payload capacity and 144 bytes of cache-entry Vec capacity. The charge doubles
payload and adds metadata/Weak-root allowances; it is not measured RSS. Before
drop, tracked live bytes are 106,766,480, including the payload, entry Vec and
3,584-byte current shape clone. Gross requested operation bytes fall from
348,261,444 to 134,736,084. The difference is exactly four removed packet
allocations minus the new 144-byte entry Vec. **Lower allocation traffic raises
both retained memory and peak live allocations in this fixture.**

Refusal reproduces baseline allocation totals and peak. Eviction releases packet
retention, but cannot undo the peak already reached in the frozen pass. Capping
at one packet lowers steady retention, while its 42.710 MB retained packet still
overlaps the uncached 64.053 MB temporary; its peak remains close to the full
cache. All modes reach zero tracked live operation bytes after cohort teardown;
post-drop smaps RSS is 107.430–107.692 MB while source fixtures remain alive.

The allocation counter follows successful Rust GlobalAlloc requests on the test
thread, including realloc replacement sizes. It excludes fixture construction,
sampling allocations, allocator rounding/arenas and other threads. It measures
requested live bytes, not physical residency. Linux VmRSS/VmHWM and smaps_rollup
RSS/PSS/private-dirty are separate boundary observations of the whole process,
including executable, fixture and harness. After-operation smaps samples miss
temporary packets that have already dropped; VmHWM retains the earlier peak.
Kernel accounting and separately timed reads can disagree (the exploratory run
contains such examples), so these are not a strict continuous peak bound. Lazy
zero pages and allocator release also prevent converting packet capacity or
gross requested bytes directly to RSS. Two processes/mode provide no deployment
confidence limit or Windows admission result.

## Admission limits and checks

The key unresolved ordering issue is that the app snapshot loader performs its
fresh RAM guard **before** trainer preparation. Retained packets could make that
guard reject before a later pack-cache eviction can run. A production design
needs eviction/headroom coordination before loading, not merely this cache's
post-load callback. Queue staging, overlapping shape resources and complete
weight/Adam device copies also require real-device peak measurements. The
Windows working-set/pagefile guard is unchanged and was not stubbed or executed.
The generic owned-loader path would need an explicit immutable Arc lease or a
safe uncached fallback. No default runtime integration or merge is admitted by
this experiment.

Five focused tests pass: the 54 valid matrix cases; twelve representative invalid
input cases before admission/lookup; Arc COW/replacement/expiry and changed-layout
misses; header/full-base sink behavior; and caps/refusal/eviction with identical
uncached output. Full CPU-feature library tests pass: 115 passed, three manual
assays ignored. Both manual assays pass, including ten fresh memory children.
Strict library/tests Clippy, the normal CPU-feature library check and formatting
pass. Source hashes prove normal trainer code is unchanged. These tests do not
establish GPU numerical equivalence, real Windows guard/error ordering, complete
canonical continuation, GPU fault handling or end-to-end learning performance.

Use the command prefix and suffixes in the raw evidence to reproduce. Before any
runtime proposal, falsify admission under pressured real Windows headroom, then
compare full replay outputs, gradients, weights, both Adam moments and exact
integer ages across optimizer updates on the same actual adapter. Preserve fresh
forwards, original failure ordering and complete base/speech uploads. Require an
enclosing GPU PPO improvement; the host-helper result alone is insufficient.
