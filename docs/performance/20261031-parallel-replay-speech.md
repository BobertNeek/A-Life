# Parallel replay speech: conditional prototype

**This does not accelerate current continuous same-individual PPO.** Its
[window producer](../../crates/alife_game_app/src/foundation_training_cycle/continuous_world.rs)
sets `speech_targets` to all `None` and `auxiliary` to `None`.
[The trainer](../../crates/alife_training/src/trainer.rs) clears
`replay_speech_active` during preparation and enables it only when a target is
present. Both affected speech stages are already skipped on that path. Zero
incremental saving is expected there, based on those source conditions. No
production selection enables this candidate, and PR223 is unchanged.

This isolated prototype applies only to replay that actually dispatches speech
supervision. Its standalone shader preserves the original WGSL as an exact
prefix, then adds one 32-lane workgroup per tick. Each forward output keeps its
original summation order across six autoregressive steps. The reverse seed uses
shared step deltas and private per-lane history; its outgoing-edge gather
preserves the original output/index summation order, including zero summands.
No neural step, target, state update or optimizer age is omitted.

The disposable software-Vulkan fixture uses N2048, 32,768 synapses, 64 ticks,
16 burn-in ticks, four microsteps/tick, and a mask training every third synapse.
Every loss tick has supervision for all six speech steps and 32 outputs.
Candidate adjoints are synthetic fixed values; this is not a full PPO objective,
observed world trajectory, accepted checkpoint, or learning-capability assay.

| Seven alternating paired updates | Serial median | Parallel median | Less component elapsed |
| --- | ---: | ---: | ---: |
| Initial, unprofiled | 137.084 ms | 128.081 ms | 6.57% |
| Final frozen source, unprofiled | 118.152 ms | 113.191 ms | 4.20% |
| Final, with timestamp markers | 130.391 ms | 121.375 ms | 6.91% |

The final unprofiled cohort is the primary receipt. Its ranges overlap, and one
pair was 0.4% slower. The profiled forward kernel median fell from 7.930 to
3.978 ms; the speech seed fell from 6.954 to 3.377 ms. These are software-adapter
component observations, not physical RTX or whole-training gains.

Fourteen empty timestamp-marker passes bracket 13 device stages. Marker overhead
is included in those spans; profiled host time also includes query resolution.
Separate instrumented/uninstrumented pairs changed wall medians by +0.88% in
the first cohort and -2.46% in the final cohort. This variation does not establish
a negative profiling cost. Speech comprised about 11.5–11.7% of the instrumented
fixture's device interval; that share does not describe the continuous path.

Host intervals include command encoding/finish, common adjoint Vec allocation
and buffer initialization, queue submit and submission-index wait. They exclude
replay fixture packing/setup, pipeline creation, validation readbacks and receipt
output. Before timing, `queue.submit([])` and a device wait flush and finish all
setup uploads. First cold pairs are retained. Both sides continue matched
optimizer histories through seven updates.

Fifteen short cases match the independent original-shader oracle byte-for-byte
through state, outputs, gradients, weights, moments and integer ages. Cases cover
forward-only, seed-only and combined changes, burn-in, inactive/partial/full
microsteps, active dendrites, structural edges, dense/sparse/absent target weights
and ages above 2^24. Absent-label kernel cases force dispatch with zero weights;
the existing high-level all-None skip remains unchanged. All five buffers also
match after seven updates in every execution cohort. Naga validates all 19
entrypoints and barrier uniformity. Strict release Clippy, formatting and
whitespace checks pass.

Adapter: CPU llvmpipe, LLVM 19.1.7, Mesa 25.0.7-2+deb13u1, Vulkan,
`LP_NUM_THREADS=2`. Host: Linux x86_64, AMD EPYC 9V74, five visible CPUs.
Compiler: rustc 1.99.1 (`58cf8f9c35b355199727627d07ae34a4e2846d67`),
compiler LLVM 23.1.1. All dependencies use the actual Cargo release profile.
[Raw evidence](20261031-parallel-replay-speech-trials.json) retains every cohort,
including cold pairs, source hashes and final binary hash. Initial observations
did not capture full binary hashes; final receipts bind frozen source/binary
hashes before and after measurement.

Reproduce on this branch from the repository root:

```bash
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life/target/cloud-speech
cargo test --release -p alife_training --features gpu-tests --lib parallel_speech_matches_serial_through_adam -- --ignored --nocapture
ALIFE_SPEECH_STAGE_ASSAY_PATH=speech-stages.json cargo test --release -p alife_training --features gpu-tests --lib replay_speech_stage_share_assay -- --ignored --nocapture
ALIFE_PARALLEL_SPEECH_ASSAY_PATH=speech-update.json cargo test --release -p alife_training --features gpu-tests --lib parallel_speech_execution_assay -- --ignored --nocapture
ALIFE_SPEECH_PROFILE=1 ALIFE_PARALLEL_SPEECH_ASSAY_PATH=speech-profiled.json cargo test --release -p alife_training --features gpu-tests --lib parallel_speech_execution_assay -- --ignored --nocapture
cargo clippy --release -p alife_training --features gpu-tests --lib --tests -- -D warnings
```

Any future physical qualification must be coordinated by the sole PC owner.
Without the cloud software-ICD environment, prefix the correctness/assay command
with `ALIFE_DISPATCH_REQUIRE_HARDWARE=1` to refuse CPU adapters. Legacy non-replay
speech must retain its original entrypoint: the experimental entries are
replay-only. Physical-device, larger-window and real speech-bearing curriculum
qualification remain open; this prototype supplies no 10x training claim.
