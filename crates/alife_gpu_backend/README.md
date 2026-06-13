# alife_gpu_backend

wgpu/WebGPU backend boundary for sparse neural compute.

This crate owns GPU resource planning, buffer/shader manifests, dispatch interfaces, and later WGSL integration. It must not define cognitive truth that is absent from the CPU reference path, and it must not add neural runtime kernels before the gated GPU plans.

## P25 static forward parity

P25 adds the first executable neural GPU path, limited to static forward
projection and activation finalization:

- pass 0 `clear_accumulators`: clears one i32 atomic accumulator per neuron and
  eight diagnostic counter words.
- pass 1 `sparse_projection_spmv`: consumes P24 tile metadata, supertile masks,
  packed synapse indices, and precomputed Q4096 effective weights.
- pass 2 `activation_finalize`: clamps i32 accumulator values into the Q32767
  activation write buffer.

Dispatch dimensions use `ceil(item_count / 64)` workgroups with a fixed 64-lane
workgroup size. Pass 0 item count is `neuron_count + 8`, pass 1 item count is
`packed_synapse_count`, and pass 2 item count is `neuron_count`.

Buffer assumptions:

- Activations are signed Q32767 i32 values.
- Effective weights are precomputed from P24 split buffers as
  `W_genetic_fixed + W_lifetime_consolidated + alpha * H_operational`, stored as
  signed Q4096 i32 values for this parity milestone.
- Dense16x16 and COO tiles are supported because P14/P24 already flatten them to
  packed synapse records.
- RowRun and ColumnRun remain unsupported until later parity plans.
- P25 uses identity activation finalization with clamp only. Nonlinear activation
  functions belong to later plans.

The normal active gameplay API still does not expose synchronous neural
readback. `run_static_forward_gpu_diagnostic` is a parity/export helper only.
Manual GPU parity can be run on machines with a wgpu adapter:

```bash
cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored
```

The current diagnostic bind group keeps the P24 contract buffers separate and
therefore requires an adapter limit of at least nine storage buffers in the
compute stage. Normal CI does not require this adapter path.

## P26 plasticity parity

P26 adds pass 3 `plasticity_update` as a diagnostic/parity path:

- reads previous activations plus pass-2 finalized activations, both signed
  Q32767 i32 values.
- applies fixed-point Oja math with 32-bit shader intermediates and seeded
  deterministic LFSR stochastic rounding.
- treats alpha as the plasticity gate: alpha=0 leaves the slot unchanged.
- writes `H_shadow` only. `W_genetic_fixed`, `W_lifetime_consolidated`, and
  `H_operational` remain immutable in this pass.
- records overflow, saturation, alpha-zero skip, active tile/synapse, mask skip,
  and unsupported-tile diagnostics.

The upload contract still stores H_shadow as signed INT16 fixed-point values.
The P26 WGSL diagnostic path widens H_shadow to i32 storage buffers because
portable WGSL storage buffers do not use 16-bit integer element arrays here; the
host result clamps back to INT16. This keeps the low-precision contract explicit
without changing P24 buffer views.

Manual GPU parity can be run on machines with a wgpu adapter:

```bash
cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored
```

The current diagnostic bind group requires an adapter limit of at least ten
storage buffers in the compute stage. Normal CI does not require this adapter
path.

## P27 supertile routing masks

P27 adds the shared hierarchical active-mask contract used by the P25 static
forward and P26 plasticity diagnostic paths:

- microtiles remain 16x16 neurons.
- each supertile covers 8x8 microtiles, a 128x128 macro region.
- each supertile mask stores 64 microtile bits split into low/high 32-bit
  words.
- CPU-side routing descriptors are derived from `alife_core` lobe/routing
  metadata and reject invalid lobe references.
- active tile masks are deterministic and may be derived from lobe cadence,
  sensory activity, biological tile budget, or static fixture masks.
- P25/P26 CPU diagnostic plans use the shared P27 mask helper for early exit;
  behavior must match unmasked execution whenever skipped regions have no
  source contribution.
- counters track skipped supertiles, skipped microtiles, active tiles, active
  synapses, routing descriptors evaluated, and mask boundary failures.

Dispatch-level culling is deliberately deferred. P27 establishes shader
early-exit and host-side mask packing first; P29 owns runtime performance tiers.
The P25 and P26 paths remain diagnostic parity paths. Passing local diagnostic
or ignored GPU tests does not prove product WebGPU portability, especially
because the current diagnostic bind groups use nine and ten storage-buffer
bindings respectively. Active gameplay APIs still must not require synchronous
neural readback; diagnostics and export staging remain the allowed readback
surfaces.

## P28 sleep/offline recompaction

P28 adds host-side structural recompaction and autophagy contracts for safe
sleep/offline boundaries:

- imports P16 `StructuralEditBatch` records into backend edit plans.
- accepts prune, consolidate, and recompaction-hint candidates; strength,
  weaken, and synaptogenesis edits remain explicitly unsupported/deferred.
- rebuilds a scratch `GpuUploadBuffers` set deterministically instead of
  mutating active buffers in place.
- prunes only zero-effective, decayed trace slots under the v1 autophagy
  policy, preserving static forward outputs.
- reports byproduct decay events and a bounded BrainATP recovery signal as
  diagnostics for future sleep/autophagy tuning.
- emits an old-to-new remap table, affected projection/tile refs, routing/mask
  preservation diagnostics, and autophagy markers.
- stages a double-buffer replacement that can either be rejected while keeping
  the old active upload or atomically swapped at a sleep/offline boundary.

The P28 WGSL file is a contract stub only. It does not implement dynamic GPU
allocation or shader-side recompaction. Active gameplay APIs still cannot
require per-synapse, weight, or bulk neural readback; P29 owns runtime
performance-tier integration.

## P29 optional GPU runtime and no-readback tiers

P29 adds the runtime selection and performance-reporting shell around the
diagnostic GPU lane. The CPU reference remains the default and the correctness
oracle. GPU static, GPU plastic, and GPU full modes are selectable through
configuration, but unsupported hardware, disabled features, validation failure,
or unavailable full-runtime support fall back to CPU with a typed reason.

Runtime boundary rules:

- active gameplay may stage compact action summaries only.
- active gameplay does not expose bulk neural, per-synapse, per-lobe, or weight
  readback.
- diagnostics/export readbacks are allowed only at frame, sleep, manual
  validation, or performance-report boundaries.
- P25/P26 ignored GPU parity tests and P27/P28 diagnostic counters are not
  product active-gameplay readback APIs.

The throttling policy protects sensory, metabolic, motor, and homeostatic lobes
first. When GPU neural timing exceeds budget, non-essential association,
lexicon, memory, and working-memory lobes decimate before sensory/motor cadence
is reduced.

Performance-report commands:

```bash
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --all --gpu-runtime
```

The smoke command writes `target/artifacts/gpu_runtime_performance.md`. Unless
`ALIFE_GPU_RUNTIME_AVAILABLE=1` and `ALIFE_GPU_RUNTIME_VALIDATED=1` are set, the
report records CPU fallback data and leaves GPU neural timing and 60 FPS target
status as unknown. This is intentional: the existing P25/P26 GPU paths are
diagnostic parity paths, and local ignored tests do not prove product WebGPU
portability.

Relevant optional environment flags:

- `ALIFE_GPU_RUNTIME_BACKEND=cpu|static|plastic|full`
- `ALIFE_GPU_RUNTIME_FEATURE=1`
- `ALIFE_GPU_RUNTIME_AVAILABLE=1`
- `ALIFE_GPU_RUNTIME_VALIDATED=1`
- `ALIFE_GPU_FULL_RUNTIME_AVAILABLE=1`

Current storage-buffer assumptions remain inherited from P25/P26/P27: the
static-forward diagnostic bind group uses at least nine storage buffers, and
the plasticity diagnostic bind group uses at least ten storage buffers. Future
runtime work may reduce or shard these assumptions, but must keep the
no-active-readback rule.
