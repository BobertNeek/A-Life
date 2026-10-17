# Packed actor checkpoint readback

Date: 2026-10-17. Initial integration base: main `28770f3b7f2692be02c0aa686c554309bc4703b9`.
The published branch also composes main `4d5b04b89b2261d57c566ff74ec67ee8ff8a4f2a`
after the coastal asset/checker publication, preserving its shipping profiles and
existing CI plant checks. Those changes do not alter the trainer.

## Scope and evidence status

This is a narrowly scoped, unmeasured optimization candidate. No GPU regression,
intended-adapter parity, training run, peak-allocation measurement or runtime
speedup is claimed. The historical candidate was prepared on `73f8ecf0`; both its
implementation and adapter-test patches apply cleanly after the now-merged
[pipeline reuse change](https://github.com/BobertNeek/A-Life/pull/59). Current-main
integration has been inspected independently; historical source-only checks are
not treated as qualification of the new source.

`FoundationTrainer::checkpoint` still captures the current authoritative device
state at every call. It copies weights and all contiguous optimizer words into
one temporary staging buffer, submits once, maps once and waits once. It retains
no state across captures, optimizer updates, replay scratch changes or cohort
rebinds. The encoder contains only copies, with no neural update.

The schema, phenotype/source identities, Adam step/config, stage mask, exact
weights, both moments and integer ages remain unchanged. Restore, replay
preparation, rebind and standalone `read_weights` retain their existing bodies.
An unfinished accumulation still rejects before capture. The serial cycle's
initial/final real captures, restored-device comparisons, disk reopen, successor
decisions and persistence validations are untouched.

## Byte layout and admission

For `n` synapses, staging contains four adjacent `4n`-byte lanes: weights, first
moment, second moment, and integer update ages. One copy reads `[0, 4n)` from the
weights buffer; another reads `[0, 12n)` from the optimizer buffer to offset `4n`.
Copies are word-aligned, including odd counts. Mapping starts at zero over the
complete `16n` range. Native-word reconstruction retains the old bytemuck word
interpretation without requiring an aligned host subslice. `f32::from_bits`
preserves finite float bits, signed zeros and subnormals. Ages remain integers,
including patterns that would be NaNs as floats.

Checked arithmetic, nonzero count, host span and device maximum-buffer admission
precede allocation. A testable source-size guard rejects either undersized
buffer before copy encoding. Current admitted metadata includes eight record
words plus at least one outgoing-ID word per synapse, at least `36n` bytes;
this dominates the new `16n` allocation. Keep explicit readback checks even if
that metadata representation changes.

Moment floats are checked before weights, preserving the old rejection order.
Wrong length or nonfinite float lanes return `MalformedReadback`. Poll/map errors
retain the existing `GpuSubmission` pattern. On successful mapping, the view is
dropped and staging unmapped before a decode success or failure returns. Early
errors drop the temporary buffer; fault-injected device-loss/map/submission
cleanup has not been executed.

## Static work counts and tradeoff

Per actor checkpoint, by source inspection:

- Staging allocations, submissions, maps and waits: three become one
- Buffer copies: three become two
- Transferred bytes: unchanged `16n`
- Largest individual staging allocation: `8n` becomes `16n`
- At 32,768 weights: staging grows from 256 KiB to 512 KiB, an extra 256 KiB

These are static counts, not timing or net peak-memory results. Pipeline reuse
retains immutable pipelines/layouts across shape changes; it does not combine
checkpoint readbacks, so these mechanisms remain independent. The observed
8.91-second world-checkpoint envelope reported elsewhere includes different
work and cannot be attributed to this actor helper. CPU decoding cost and the
larger simultaneous staging range could offset some benefit; measure them.

## CPU and source qualification

Eight isolated production-helper `rustc` tests pass in unoptimized and optimized
builds with debug assertions. Their wrapper supplies only a minimal error enum;
the exact production helper and its tests are unchanged. They cover byte-exact
finite lanes and integer ages, signed
zeros, subnormals, NaN-like ages, unaligned subslices, odd counts, 32,768-weight
layout/decoding, overflow/host/device limits, exact/larger/undersized source
buffers, wrong lengths, nonfinite rejection, and independent successive results.
Focused helper Clippy with warnings denied passes.

The real-adapter test is enabled with `gpu-tests` but must be run explicitly
on an intended adapter. A separate hardware-free CI workflow adds no-run compilation and strict Clippy
for that test source. It preserves the existing CI file, whose exact bytes are
pinned by the coastal art profile. Ordinary workspace CPU
tests execute the helper regressions without enabling GPU execution.

Independent source review found no static correctness blocker and mechanically
checked that restore/replay/rebind/readers are unchanged. The adapter oracle
matches the old exact-main checkpoint body after receiver/type-path
normalization and never invokes the packed helper or decoder.

## Prepared real-adapter regression, execution pending

The test compares separate legacy captures before and after packed capture,
including payload bits, metadata and serialized identity. It seeds nonzero
moments and ages, signed-zero float lanes, a frozen subnormal weight, changes
replay tick count, executes real WGSL Adam updates, restores both captures against
uninterrupted updates, accepts a cohort rebind and rejects active accumulation.
It requires hardware rather than silently skipping. This is a synthetic
zero-microstep optimizer/serialization fixture, not grounded curriculum,
behavioral improvement or founder-promotion evidence. Tick-count changes do not
exhaust burn-in/dendrite shapes; empty active accumulation does not exhaust
partially accumulated gradient states.

From a complete checkout carrying this change, when the training owner has made
the intended adapter available:

```sh
cargo test --locked -p alife_training --features gpu-tests --lib trainer::packed_checkpoint_gpu_tests::packed_checkpoint_matches_separate_captures_and_next_adam_after_shape_and_rebind -- --exact --nocapture --test-threads=1
```

Record source and binary identity, toolchain, intended adapter, exit status and
complete output. Passing elsewhere cannot establish intended-adapter acceptance.
Then measure matched cold/warm capture time and staging/host/device peak memory.
Preserve the same learning recipe, prior, resource mode, threads, priority,
inter-cycle gaps and observer settings. Failure injection and wider shape/real
training continuation coverage remain separate open evidence limits.

## Architecture trace

`AOA-PERF-001` and `AOA-PERF-003`: explicit bounded copies, sizes and work counts.
`AOA-PERF-005`: runtime and scaling benefit remain unknown without measurement.
`AOA-PERSIST-002` and `AOA-PERSIST-004`: retain complete state and explicit errors;
actual continuation equivalence remains an execution gate. `AOA-OBS-004`: capture
is a derived read of current authoritative state, with no update.

This changes an implementation mechanism, not the controlling architecture.
