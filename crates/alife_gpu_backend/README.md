# alife_gpu_backend

wgpu/WebGPU backend boundary for sparse GPU-authoritative neural compute.

This crate owns class-bucketed neural heaps, generation-checked handles,
closed-loop WGSL pipelines, sealed-outcome waking plasticity, and bounded
readback receipts. The world owns legality and measured outcomes. Production
neural execution has no live CPU shadow, parity-gated handoff, or automatic CPU
neural fallback.

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

## Closed-loop sealed-outcome plasticity

Production waking learning uses the same seven class-bucket heaps as selection.
A waking dispatch stages candidate-specific recurrent and decoder eligibility.
Only the matching sealed `ExperiencePatch` can apply three-factor credit and
atomically swap the inactive fast-weight and eligibility banks. Reward, pain,
homeostatic improvement, frustration, and novelty are validated provenance;
the bounded neuromodulator value is multiplied by the compiled receptor plan.
The resulting `H_fast` affects the next encounter before sleep.

The hardware gate is:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -j 1 -- --nocapture
```

## P27 supertile routing masks

P27 adds the hierarchical active-mask contract used by legacy static-forward
diagnostics and phenotype packing:

- microtiles remain 16x16 neurons.
- each supertile covers 8x8 microtiles, a 128x128 macro region.
- each supertile mask stores 64 microtile bits split into low/high 32-bit
  words.
- CPU-side routing descriptors are derived from `alife_core` lobe/routing
  metadata and reject invalid lobe references.
- active tile masks are deterministic and may be derived from lobe cadence,
  sensory activity, biological tile budget, or static fixture masks.
- P25 CPU diagnostic plans use the shared P27 mask helper for early exit;
  behavior must match unmasked execution whenever skipped regions have no
  source contribution.
- counters track skipped supertiles, skipped microtiles, active tiles, active
  synapses, routing descriptors evaluated, and mask boundary failures.

Dispatch-level culling is deliberately deferred. P27 establishes shader
early-exit and host-side mask packing first; P29 owns runtime performance tiers.
The P25 path remains a historical diagnostic. It is not a production neural
authority surface. Active gameplay exposes only compact selection and learning
receipts; bulk diagnostics and export staging remain boundary-scoped.

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

## GPU-required neural runtime and no-readback boundary

`NeuralClosedLoopGpu` requires a validated adapter and the seven-binding
closed-loop heap layout. Hardware or device failure returns the typed neural
backend-unavailable result and stops learned actions. `HeuristicBaseline` is an
explicit, separately selected comparison policy; it is never an error fallback.

Runtime boundary rules:

- active gameplay may stage compact selection and learning receipts only.
- active gameplay does not expose bulk neural, per-synapse, per-lobe, or weight
  readback.
- diagnostics/export readbacks are allowed only at frame, sleep, manual
  validation, or performance-report boundaries.
- historical P25 and P27/P28 diagnostic counters are not product
  active-gameplay neural authority APIs.

The throttling policy protects sensory, metabolic, motor, and homeostatic lobes
first. When GPU neural timing exceeds budget, non-essential association,
lexicon, memory, and working-memory lobes decimate before sensory/motor cadence
is reduced.

Performance-report commands:

```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --all --gpu-runtime
```

Reports record the exact adapter, class, phenotype, payload, commit/tree, and
status. Missing measurements remain `Unknown`; historical CPU diagnostic data
cannot satisfy a GPU neural performance gate. The production closed-loop
pipelines use exactly seven storage-buffer bindings per shader stage.
