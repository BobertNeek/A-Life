# alife_gpu_backend

wgpu/WebGPU backend boundary for sparse GPU-authoritative neural compute.

This crate owns class-bucketed neural heaps, generation-checked handles,
closed-loop WGSL pipelines, sealed-outcome waking plasticity, and bounded
readback receipts. The world owns legality and measured outcomes. Production
neural execution has no live CPU shadow, parity-gated handoff, or automatic CPU
neural fallback.

Active upload sizing includes perception, candidate digests, eligibility,
episodic memory, cognitive records, and neural receptor effects. Cognitive
records must be counted before capacity checks and final payload validation.

## Closed-loop sealed-outcome plasticity

Production waking learning uses the same seven class-bucket heaps as selection.
A waking dispatch stages candidate-specific recurrent and decoder eligibility.
Only the matching sealed `ExperiencePatch` can apply three-factor credit and
atomically swap the inactive fast-weight and eligibility banks. Reward, pain,
homeostatic improvement, frustration, and novelty are validated provenance;
the bounded neuromodulator value is multiplied by the compiled receptor plan,
and Oja homeostatic normalization is gated by `abs(local_third_factor * credited_eligibility)`
so zero-credit ordinary activity preserves `genetic + lifetime` weights.
The resulting `H_fast` affects the next encounter before sleep.

The hardware gate is:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -j 1 -- --nocapture
```

## P27 supertile routing masks

P27 adds the hierarchical active-mask contract used by phenotype packing:

- microtiles remain 16x16 neurons.
- each supertile covers 8x8 microtiles, a 128x128 macro region.
- each supertile mask stores 64 microtile bits split into low/high 32-bit
  words.
- CPU-side routing descriptors are derived from `alife_core` lobe/routing
  metadata and reject invalid lobe references.
- active tile masks are deterministic and may be derived from lobe cadence,
  sensory activity, biological tile budget, or static fixture masks.
- counters track skipped supertiles, skipped microtiles, active tiles, active
  synapses, routing descriptors evaluated, and mask boundary failures.

The routing code provides shader early-exit and host-side mask packing. Compact
selection and learning results support the live loop; bulk diagnostic and exact
state captures use explicit transaction boundaries.

## P28 sleep/offline recompaction

P28 adds host-side structural recompaction and autophagy contracts for safe
sleep/offline boundaries:

- imports P16 `StructuralEditBatch` records into backend edit plans.
- accepts prune, consolidate, and recompaction-hint candidates; strength,
  weaken, and synaptogenesis edits remain explicitly unsupported/deferred.
- rebuilds a scratch `GpuUploadBuffers` set deterministically instead of
  mutating active buffers in place.
- prunes only zero-effective, decayed trace slots under the v1 autophagy policy.
- reports byproduct decay events and a bounded BrainATP recovery signal as
  diagnostics for future sleep/autophagy tuning.
- emits an old-to-new remap table, affected projection/tile refs, routing/mask
  preservation diagnostics, and autophagy markers.
- stages a double-buffer replacement that can either be rejected while keeping
  the old active upload or atomically swapped at a sleep/offline boundary.

The P28 WGSL file is a contract stub, not dynamic GPU allocation or shader-side
recompaction. This developer contract does not by itself demonstrate a production
sleep or performance feature.

## Gameplay runtime and state capture

Gameplay requires a validated GPU adapter and the closed-loop heap layout.
Hardware or device failure returns an unavailable result and stops learned
actions. Explicitly selected comparison policies are separate from the gameplay
neural path and do not act as device-error fallbacks.

The actual runtime owns generation-checked slots, bounded selection results,
learning transactions, and exact state capture. Bulk capture and restore preserve
private state and occur at sealed boundaries for persistence, diagnostics, and
training; they do not create a CPU neural shadow. See
[`closed_loop_runtime.rs`](src/closed_loop_runtime.rs) and
[`closed_loop_exact_capture.rs`](src/closed_loop_exact_capture.rs).

The real GPU diagnostic supports selected classes, sensor profiles, populations,
and custom targets. See [Development](../../docs/DEVELOPMENT.md#gpu-measurements)
for its current command. Record observed device work and report missing
measurements explicitly; CPU estimates are not GPU timing.
