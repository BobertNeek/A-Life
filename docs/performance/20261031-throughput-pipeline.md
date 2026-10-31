# Recurrent command encoding and bounded replay preparation

The throughput branch removes repeated host work without changing causal world
ticks, neural arithmetic, replay contents, PPO cadence or acquired learner state.
The measured result is a **host command-encoding component gain**, not a 10×
training result. Current physical-device and retained-training gains are unknown.

The recurrent trainer uses one dynamic uniform-header binding and one compute
pass per forward/backward sweep. It records exactly the original dispatch order,
including local-backward before source-backward at every reverse microstep.
wgpu 29.0.3 gives each compute dispatch its own usage scope and emits the required
storage barriers inside a pass. The production WGSL is unchanged. Header range,
alignment and u32 dynamic-offset limits are checked before resource creation.

A separate persistent replay worker moves capture conversion, validation,
encoding and optional cache insertion beside continuous collection. It retains
one queued capture plus one in-flight capture, with an 8 MiB cap on each pair of
mutable snapshot arrays and conservative concurrent-memory admission. The same
writer/codec authenticates rows in FIFO order. Only a complete drain and durable
seal return usable references; failure, discard and drop cannot publish a partial
cohort. A worker is reused between windows and joined before cleanup.

Collection now closes before the worker's final drain. That tail is included in
`replay_seal_seconds`, so compare **collection plus replay seal**, and ultimately
inclusive retained-learning/hour. A smaller collection counter alone does not
establish a speedup. No worker throughput gain has been measured yet.

## Host encoding measurements

[Raw trials, source and binary identities, environment and limits](20261031-throughput-pipeline.json)
bind the measurements to trainer commit `c50ffdb9c00ec3c85fc825d736030aa4d978efea`
on main `a1ea75a03b5f2cbbfa55b0034303cead85d439b7`. The replay worker is commit
`ef668cc1bd50ca6e96fe0e64f0bc598b37919a70` and was not linked into this trainer assay.

The synthetic N2048 fixture has 2,048 neurons, 32,768 synapses, four microsteps,
two candidates per tick, nonempty dendrites and structural context. Seven paired
trials alternate the actual production sweep encoder with an independent original
fixed-header/pass-per-dispatch oracle in the same binary. The first cold pair is
retained. Timing covers recording both sweeps plus `encoder.finish()`; resource
creation, uploads, output destruction, submission, waits and neural execution are
outside the interval. Oracle setup includes pipeline compilation and is excluded
from the comparison. No GPU dispatch is submitted by this timing assay.

| Ticks / burn-in | Full-release original / candidate median ms | Host component ratio | Saved ms |
| --- | ---: | ---: | ---: |
| 256 / 0 | 5.405 / 1.649 | 3.28× | 3.757 |
| 384 / 128 | 6.597 / 2.079 | 3.17× | 4.518 |
| 512 / 128 | 9.964 / 2.828 | 3.52× | 7.135 |

The earlier test build optimized only `alife_training` to opt-level 3, leaving
wgpu validation unoptimized. Its corresponding medians were 53.663/22.482,
62.247/26.096 and 93.837/37.803 ms (2.39–2.48×). Those larger absolute savings
must not be substituted for the release result. Both profiles and every sample
remain in the raw data. Cold original outliers reach 21–25 ms in release; seven
pairs do not establish a formal confidence interval or a tail-latency claim.

Hardware here is a Linux AMD EPYC 9V74 VM with five visible CPUs, a four-CPU
quota, 16 GiB RAM and Rust 1.99.1. The selected Vulkan adapter is Mesa 25.0.7
llvmpipe/LLVM 19.1.7 with two software-driver threads. This is software-device
evidence. The full release assay ran after local compiler work stopped; VM
contention, CPU frequency and allocator history remain uncontrolled.

## Correctness and remaining qualification

The direct-device oracle compares every u32 in state, outputs, gradients,
weights and optimizer buffers after real WGSL forward/backward/Adam execution.
Three/five-tick cases cover burn-in 0/2, inactive/partial/full microsteps, speech,
dendrites and structural edges. Heterogeneous integer ages include values above
2^24; zero-age moments are valid and frozen signed zeros remain exact. These
fixtures do not certify current gameplay, acquired trajectories or useful
capability per hour. Normal hardware-only runtime admission remains unchanged.

The worker's CPU tests compare identical authenticated references and entire
serialized bundles over two windows and explicit segments. They also exercise
corruption, partial-row and output-budget failures, bounded queuing, drain,
discard, drop/join and panic propagation. They begin with typed records; the
Windows hardware gate below additionally exercises actual capture conversion,
continuity and cache/TLS behavior. The 52 relevant worker/replay/cohort/continuous
CPU tests and strict Clippy passed; the host's 45 retention/recovery tests passed.
The training crate's 75 receipt-lineage library tests and strict GPU-test-target
Clippy also passed. Full-release software equivalence passed independently of
the earlier test-profile run. No real trained checkpoint was used, and
`semantic_prior.rs` is unchanged.

Run these through the sole PC owner on the exact branch head, with a disposable
procedural fixture. Record actual adapter/driver/source and keep real checkpoints
protected:

```powershell
$env:ALIFE_DISPATCH_REQUIRE_HARDWARE = "1"
cargo test --locked -p alife_training --features gpu-tests --lib dynamic_dispatch_matches_static_headers_through_adam -- --ignored --nocapture --test-threads=1
cargo test --locked -p alife_game_app --features foundation-training,gpu-tests --lib captured_steps_match_serial_full_writer_including_explicit_segments -- --ignored --nocapture --test-threads=1
```

After equivalence, measure submitted GPU work and a matched continuous session
including collection/drain/seal, PPO, install, snapshot and exact restore. Report
retained rows and capability per inclusive host hour. The isolated persistent
single-workgroup sweep experiment is separate from the default implementation;
its reduced dispatch count needs physical-device measurement because reduced
occupancy may offset the saving. Capture/staging/codec integration and held fused
changes are outside this branch.
