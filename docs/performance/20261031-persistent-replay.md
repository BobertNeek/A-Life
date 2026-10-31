# Persistent recurrent replay experiment

This candidate is **disabled in production**. It preserves the tested replay
results but is slower on the available software Vulkan adapter. It does not
establish a physical GPU speedup or improved training throughput.

The experimental shader copies the original WGSL, then adds single-workgroup
forward and backward sweeps. Positive 32-microstep chunks reduce dispatch count
while preserving every microstep, recurrent dependency and detached burn-in.
Production retains the dynamic-header, per-microstep dispatch implementation
from base commit `c50ffdb9`; no runtime selection enables this shader.

The final release assay uses synthetic N2048 replay with 32,768 synapses, 64 ticks,
16 burn-in ticks, four microsteps/tick, speech supervision, an active dendrite and
a structural edge. Each side continues its matched optimizer history through
seven alternating paired updates. The measured interval sums command encoding
and finish, queue submit and submission-index wait. It excludes fixture packing,
pipeline creation, validation readbacks and receipt output. It is not a full PPO
or training-session timer, and uses no GPU timestamp query.

| Workgroup width, chunk 32 | Dynamic baseline median | Persistent median | More elapsed time |
| --- | ---: | ---: | ---: |
| 128 | 128.232 ms | 189.226 ms | 47.6% |
| 256 | 130.276 ms | 191.711 ms | 47.2% |

An earlier release cohort also slowed down: 34.2–46.6%. Both complete cohorts,
including cold pairs and ranges, are retained in the
[raw evidence](20261031-persistent-replay-trials.json). Reduced host recording
cost did not offset the increased wait. Physical GPU occupancy and driver costs
may differ; these software results provide no basis for enabling the candidate.

The adapter was CPU `llvmpipe (LLVM 19.1.7, 256 bits)`, Mesa
25.0.7-2+deb13u1, Vulkan, with `LP_NUM_THREADS=2`. Host: Linux x86_64,
AMD EPYC 9V74, five visible CPUs. Compiler: rustc 1.99.1
(`58cf8f9c35b355199727627d07ae34a4e2846d67`), compiler LLVM 23.1.1.
All dependencies used the actual Cargo release profile. Exact source hashes,
binary hashes, base commit and raw trial values are in the evidence JSON.

The final two release correctness tests passed. The persistent test compares
21 short cases against independent fixed-header bindings and the original
shader: 3/5/9 ticks, burn-in 0/2/1, widths 64/128/256, chunks 8/32 plus width
128 with chunk 3 cutting inside ticks. State, outputs, gradients, weights,
moments and integer optimizer ages match byte-for-byte. The execution assay
also checks all five buffers after seven matched updates at each width. The
fixtures include inactive/partial/full microsteps, speech, frozen coordinates,
signed zeros, valid age-zero moments and integer ages above 2^24.

Run the cloud reproduction from this branch's repository root:

```bash
source /workspace/cloud-cpu-validation/env.sh
source /workspace/vulkan-assay/env.sh
export CARGO_TARGET_DIR=/workspace/A-Life/target/cloud-persistent
cargo test --release -p alife_training --features gpu-tests --lib dispatch_matches_static_headers_through_adam -- --ignored --nocapture
ALIFE_PERSISTENT_ASSAY_PATH=/workspace/vulkan-assay/persistent-release-final-execution-trials.json cargo test --release -p alife_training --features gpu-tests --lib persistent_dispatch_execution_assay -- --ignored --nocapture
cargo clippy --release -p alife_training --features gpu-tests --lib --tests -- -D warnings
cargo fmt --check -p alife_training
```

Strict release Clippy, formatting and whitespace checks passed. For an explicitly
authorized physical-device assay, use the following in a shell (Git Bash on
Windows), without the cloud software-ICD environment. The hardware guard refuses
a CPU adapter. These tests use disposable synthetic state, not a real checkpoint:

```bash
ALIFE_DISPATCH_REQUIRE_HARDWARE=1 cargo test --release -p alife_training --features gpu-tests --lib persistent_dispatch_matches_static_headers_through_adam -- --ignored --nocapture
ALIFE_DISPATCH_REQUIRE_HARDWARE=1 ALIFE_PERSISTENT_ASSAY_PATH=persistent-hardware-trials.json cargo test --release -p alife_training --features gpu-tests --lib persistent_dispatch_execution_assay -- --ignored --nocapture
```

Physical RTX qualification, representative larger replay windows and complete
retained-learning/capability measurements remain open. No real checkpoint entered
this workspace; normal hardware-only runtime admission is unchanged.
