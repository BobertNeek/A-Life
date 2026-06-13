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
