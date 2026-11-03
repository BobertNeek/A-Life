# Disposable packed-block feasibility assay

This branch studies layout and precision, not a replacement training backend.
It starts from main `102e0e720c1641b9e899ff7c53b39a9780b49f37` after CPU PR233.
No accepted/private checkpoint, production default, PC setting or held predictive
branch is used. Full fresh fixtures remain local; only source and aggregate
measurements are published.

## Feasibility and compute boundary

The inspected N2048 compiler emits **96 complete 16×16 recurrent blocks**, covering
24,576 of 32,768 genetic synapses. The other 8,192 decoder coordinates retain
their original identities and are not packed into recurrent GEMMs. The block
layout is discovered and checked from actual compiled endpoints/routes, with
duplicate/missing/partial blocks rejected rather than padded or renumbered.

The selected executor reports AMD EPYC 9V74, five visible CPUs, four-CPU quota,
16 GiB limit, Rust 1.99.1 and NumPy 2.3.5. It has no NVIDIA device nodes,
`nvidia-smi`, `nvcc`, PyTorch, CuPy or Triton. CUDA/Tensor Core execution, actual
RTX capability, GPU kernel time and GPU end-to-end performance are **unmeasured**.
The optional CUDA runner refuses this executor with exit 2 and does not substitute
CPU output as GPU evidence. No security, boot or unlock operation is attempted.

## Two deliberately distinct computations

The Rust test-only FP32 oracle is serial; B counts independent windows, not a
concurrent GEMM. It packs current genetic values and each window's
actual tick-specific lifetime/fast offset, but preserves the original global-ID
addition order within every target. It computes `activation * (genetic + offset)`
before addition, matching native CPU replay. Structural edges, dendrites,
activation/leak, metabolic state, EMA and all decoders use the native equations.
The optional graph hook, its field and the entire assay are guarded by `cfg(test)`;
production binaries have no added runtime field, branch, execution flag or save
field. It is absent by default even in tests.

The sidecar instead studies a plausible GEMM split: one shared genetic tile
matrix times independent window activations, plus a separate FP32 per-window
offset matvec, followed by tile scatter. Genetic GEMM shape is
`[96,16,16] × [96,16,B]`. Offsets remain `[96,B,16,16]`, never averaged/shared
across windows. This changes reduction and rounding order even with TF32 off.
The genetic payload is 96 KiB; recurrent offsets alone cost 96 KiB per window
per tick (1.5 MiB at B=16), before other private inputs or transfers.

The CUDA mode compares the identical packed component with IEEE FP32 requests
and TF32 requested only for genetic GEMM. Master arrays remain FP32; offsets
always request IEEE FP32. It records hardware/API versions and precision settings,
refuses forcing environment overrides, and records packing, transfer, CUDA event,
wall and whole-sidecar scopes separately. An enabled TF32 flag is permission,
not proof that Tensor Cores executed; profiler kernel/counter evidence is still
required. See [PyTorch CUDA precision controls](https://docs.pytorch.org/docs/stable/notes/cuda.html)
and [NVIDIA cuBLAS compute modes](https://docs.nvidia.com/cuda/cublas/index.html).

## Actual windows and integrity

The local bridge captured **16 separate fresh native collector instances**, each
with 17 actual decisions, one 16-loss-row window and its real bootstrap: 272
decisions total. Instances share a public founder and initial world geometry,
with distinct sampling seeds 73101..73116 and separate private lifetime state.
All 16 sampled private-offset vectors differ. This is deliberate homogeneous
fixture collection, not evidence that an ordinary single trajectory supplies a
batch of 16 independent windows. B=32/64 are not measured.

For every window, the ordinary native frozen-feature/old-policy admission and
a copied actor/critic update pass without changing tolerances. Original source
actor and complete host checkpoint bytes remain unchanged. The sidecar samples
the actual previous activation at tick 8, microstep 0. All 96 tiles are active in
these actual samples; synthetic tests supply disabled-route/cadence coverage.

The strict Rust oracle matches complete native tapes and all gradient bits for
ragged independent windows, burn-in, zero/reduced microsteps, route/cadence masks,
per-window/tick offsets, structural additions, dendrites and decoder contexts.
Two successive actor Adam updates retain nonzero moments, frozen coordinates
and integer ages including 16,777,217. A second check runs the **unchanged full
native residual-PPO cohort** through the test hook: actor and critic checkpoints
and update receipts match byte-for-byte. Deliberately corrupting an old-policy
probability still rejects before state mutation. A wrong packed lane is also
detected by full-tape comparison.

This preserves CPU backward, including its strict-interior metabolic and
memory/cognitive clamp derivatives. It does not assume generic framework
autograd is equivalent. Existing WGSL backward reconstructs activation with a
`leak > 1e-8` guard, whereas CPU backward uses stored `tape.activated`; tiny leaks
and cancellation can differ. Full GPU recurrence, nonlinear backward and
optimizer qualification remain required before any backend promotion.

## Observations and timing limits

The NumPy canonical-ID oracle matches all actual native expected sums bit-for-bit.
The dense split FP32 component changes 12,310 output values in bits: maximum
absolute drift `7.152557e-7`, relative L2 `1.172068e-7`. Local linear VJP maximum
absolute drifts are `1.311302e-6` for shared genetic gradients, zero for per-window
offset gradients and `4.768372e-7` for activation gradients. Diagnostic tolerances
pass; these are **not** neural training or PPO acceptance tolerances.

The B16 NumPy component in the complete process measured 1.142 ms packing and
0.794 ms median dense
compute (five samples after two warm-ups). The Python canonical loop is a
correctness oracle, not a native performance baseline. The sidecar's 0.603 s
total includes parsing/identity/oracle/local VJP and repeated component work,
but excludes interpreter startup, imports and file IO. The process orchestrator
separately includes those costs and fresh native collection/export: **13.686 s**
total, with 12.807 s bridge process and 0.879 s Python component process. This
single-run process boundary includes 272 fresh decisions and native updates on
copies; it is not a full packed neural training loop. CUDA is
unavailable, so there is no CUDA kernel-time or Tensor Core speedup result.

The Rust ledger separates cold plan construction, current genetic-plus-one-tick
packing, full recurrent forward/BPTT/Adam and the enclosing residual-PPO learner.
It uses test profile (`alife_training` opt-level 3), one thread, sequential arms
and small synthetic windows. CPU affinity was not pinned in this experiment.
Setup/plan creation is outside the learner timer;
packing, native admission, full forward/backward, critic work and Adam are inside
the residual-PPO scope. No collection/disk/transfer is inside that timer. Raw
samples and the separate whole-prototype process boundary are retained in the
aggregate evidence; no statistical or production/GPU speed claim is made.

Cold plan construction median was 0.718 ms (21 samples); genetic plus one-tick
packing median was 0.02385 ms (21 samples). Five-sample medians below are
milliseconds. The large ranges, grouped arm order and shared executor preclude
a throughput conclusion from these numbers.

| Independent windows | Forward/BPTT/Adam native | Packed oracle | Full residual-PPO native | Packed oracle |
| --- | ---: | ---: | ---: | ---: |
| 1 | 5.750 | 5.793 | 19.802 | 21.989 |
| 8 | 47.979 | 52.037 | 177.404 | 184.335 |
| 16 | 94.727 | 90.047 | 415.028 | 332.229 |

[Aggregate evidence](packed-block-feasibility.json) contains every sample, exact
boundaries, the fixture SHA-256 (not its payload), native admission receipts and
the explicit CUDA refusal. Validation passed 39 CPU unit tests, 15 CPU backend
integration tests, both strict assay tests, the explicit assay measurement and
native bridge, 10 Python tests, strict Clippy, formatting and a production CPU
binary check.

## Reproduce without a private checkpoint

```sh
source /workspace/cloud-cpu-validation/env.sh
ALIFE_PACKED_OUTPUT_DIR=/tmp/packed-ledger cargo test --locked \
  --config profile.test.package.alife_training.opt-level=3 \
  -p alife_training --features cpu-backend --lib packed_block_assay \
  -- --include-ignored --nocapture
ALIFE_PACKED_OUTPUT_DIR=/tmp/fresh-packed ALIFE_PACKED_BRIDGE_BATCH=16 \
  cargo test --locked --config profile.test.package.alife_training.opt-level=3 \
  -p alife_training --features cpu-backend --test cpu_collector \
  export_fresh_native_packed_block_fixture -- --ignored --nocapture
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 \
  scripts/experiment_packed_tiles.py /tmp/fresh-packed/fresh-native-block-fixture.json \
  --device cpu --repeats 5 --warmup 2
python3 scripts/run_packed_block_fixture.py "$BUILT_CPU_COLLECTOR_TEST_BINARY" \
  /tmp/fresh-packed-complete-process --batch 16 --device cpu
python3 -m unittest discover -s scripts -p test_experiment_packed_tiles.py
```

Output paths must be fresh. Both exporters refuse fixture overwrite and preserve
failed prefixes. Future hardware work needs an admitted CUDA device, new native
independent windows for larger B, full recurrent/gradient/optimizer drift tests,
native old-policy admission, synchronized end-to-end measurements and actual
Tensor Core profiler evidence. Those gates are not replaced by this layout assay.
