# Experimental paired GPU replay

The two-window actor prototype preserved the serial result bit for bit, but
produced **no meaningful speedup** on the representative software-device assay.
Keep it on the experimental branch. These results justify neither production
enablement nor a physical-GPU, whole-PPO or retained-training throughput claim.

Source baseline is `8873770c`; candidate source is
`0903ede8899bd9f0b2ebbd24232d4d2e375dbb7b`. The
[receipt](20261031-paired-gpu-actor.json) contains full source/binary hashes,
raw trial output, correctness/fault logs and release Clippy output. The binary
hash identifies the final representative assay; provenance of supplemental
checks before lint-only cleanup is explicit in the receipt. Independent source
and report review found no blocking issues; this was not formal PR approval.

The current continuous path forms two replay windows for 512 loss rows:
256 ticks/burn-in 0 and 384 ticks/burn-in 128. Both contribute 256 loss rows
and scale 0.5 to one existing effective batch; its configured batch size remains
8. The windows have independent recorded recurrent boundaries and local-weight
context under the same actor weights. Pairing them does not connect their
recurrent trajectories, change a world tick, or change the Adam cadence.

The test-only shader uses z for window identity because y already indexes ticks
in candidate/seed kernels. Separate packed ranges and original headers preserve
per-window arithmetic. Ragged forward guards and relative backward ordinals
preserve each window's original temporal order. Per-window gradients remain
separate; the **unchanged original shader** validates/adds gradient 0 and then
gradient 1 before one original Adam update. No floating atomics or cross-window
tree reduction are used. Production code has no reference to the candidate.

The timed workload used N2048, 32,768 synapses, all four active microsteps on
every tick, two candidates, dendrites and structural context. Speech targets
were absent and all three speech kernels were skipped. Candidate adjoints were
fixed synthetic inputs, rather than outputs of the actual PPO objective.

All dependencies used the actual Cargo `--release` profile. The host was an AMD
EPYC 9V74 VM with a four-CPU quota and 16 GiB limit. The selected adapter was
Vulkan llvmpipe, LLVM 19.1.7, Mesa 25.0.7, with two software-driver threads.
Parent/compiler work was quiet during these trials. Earlier correctness runs
had populated the shader cache; cold-device behavior was not established.

| Trial order | Serial seconds | Paired seconds | Serial / paired |
| --- | ---: | ---: | ---: |
| Serial, paired | 1.654886 | 1.662584 | 0.995370 |
| Paired, serial | 1.748128 | 1.684965 | 1.037486 |
| Serial, paired | 1.686573 | 1.710319 | 0.986116 |

The ratio of medians was 1.000955; the median paired ratio was 0.995370.
Serial ranged 1.654886–1.748128 s and paired ranged 1.662584–1.710319 s.
Three alternating pairs, retaining the first trial, establish no confidence
interval or tail-latency claim. Source-counted `queue.submit` calls fell from 11 to 6
and explicit host readback waits from 5 to 3 in this harness; these are not
physical driver submission counters, and the reduction did
not produce a wall-time improvement here.

Timers include host command encoding/finish, submission, compact readback
allocation/copy/map/wait, actual WGSL forward/BPTT, ordered accumulation, Adam
and its final receipt. Resource/pipeline setup and input uploads are excluded;
an explicit submission and wait completes every deferred setup upload before
either timer starts. Full-buffer oracle reads run outside timing. The forward
availability boundary copies two real neural logits per window; it does not
evaluate/read actual PPO metrics. Whole-process peak RSS was **838,800 KiB
(819.14 MiB)**, including both baseline/candidate resources, the software driver,
setup and oracle reads. This is neither VRAM nor incremental production memory.

Every u32 in state, outputs, gradients, the ordered accumulated gradient, weights
and optimizer matched the serial oracle after each representative trial. Small
release cases additionally covered inactive/partial microsteps, unequal loss
counts with 2/5 and 3/5 scales, and optional speech. Integer optimizer ages above
2^24 and frozen signed-zero moments were preserved. Injecting NaN adjoints into
the second window rejected before Adam and left weights, moments and ages
unchanged. Strict release Clippy passed. These checks use disposable synthetic
inputs and no real checkpoint.

Full PPO integration remains absent: the global frozen-policy validation pass,
behavior likelihood checks, actual PPO objective, the original sample-weighted
host KL reduction/gate, receipt/controller bookkeeping, per-window value-gradient
scratch, ordered value accumulation and actor/value rollback. A future integration
must preserve their ordering and failure behavior before physical qualification.
Two native-sized packed training regions for the representative pair require
about 101.78 MiB plus route words; two 384-tick regions require about 122.12 MiB
plus route words. Two 512-tick regions would exceed a 128 MiB storage-binding
limit. The fixture checks actual device binding/buffer limits and admits exactly
two windows. These arithmetic bounds are not measured VRAM usage.

Reproduce in the prepared software environment:

```sh
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life-window-batch/target/pair-release
cargo test --release --locked -p alife_training --features gpu-tests --lib paired_ragged_windows_match_serial_ordered_gradients_and_adam -- --ignored --nocapture
cargo test --release --locked -p alife_training --features gpu-tests --lib paired_invalid_second_gradient_cannot_apply_adam -- --ignored --nocapture
ALIFE_PAIR_TRIALS=3 cargo test --release --locked -p alife_training --features gpu-tests --lib paired_ragged_windows_complete_actor_assay -- --ignored --nocapture
```

The driver setup is described in the earlier
[throughput-pipeline evidence](20261031-throughput-pipeline.md). Normal runtime
admission continues to reject software adapters. Set
`ALIFE_DISPATCH_REQUIRE_HARDWARE=1` to make this test fixture reject them too.
