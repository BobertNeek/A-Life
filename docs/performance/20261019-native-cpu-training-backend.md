# Native CPU training backend: numerical stage

This is a dated implementation/evidence report, not architecture authority.
It covers the explicit `alife_training/cpu-backend` feature. Gameplay keeps its
existing GPU execution path; no automatic fallback or live CPU shadow is added.

The feature executes the compiled sparse graph directly in Rust without device
creation or a Vulkan ICD. It does not invoke the legacy CPU reference controller.
`CpuFoundationTrainer` owns founder weights and Adam state outside every
`CpuOrganismState`. `FoundationReplayBackend` supplies the common host replay
boundary for CPU and GPU implementations. Adjoint rows exclude burn-in.

## Implemented numerical boundary

- Sparse recurrent routes, cadence and activity masks; detached lifetime plus
  alpha-times-fast offsets; structural edges; target-indexed nonlinear dendrites;
  receptor projection gain and threshold; intrinsic activation/leak; activity
  EMA and metabolic homeostasis.
- Candidate, bounded memory and cognitive decoder channels, innate bias, sparse
  BPTT with detached burn-in, and six-step recurrent speech supervision.
- Joint representative/six-channel PPO likelihood, exact conditional entropy
  and its coupling derivative, clipping, receipt temperature, and optional
  imitation marginalization. Legal masks are caller evidence, never inferred
  from action success. One batch version is checked before updates; stale
  cohorts are rejected. Archived demonstrations are not admitted as on-policy
  PPO rollouts.
- Memory-bounded, sample-weighted microbatch accumulation with one actor/value
  update per effective batch. The critic supports both detached return fitting
  and the native full sampled Bellman residual derivative through current and
  actual next features, including the bias derivative.
- Native `FoundationTrainerCheckpoint` schema 2 and
  `PpoValueHeadCheckpoint` restoration/export. Global and per-weight ages remain
  `u32`; frozen coordinates retain their weights, moments and ages. Founder
  rebind validates all non-genetic graph/ABI coordinates before preserving the
  optimizer across an exported/recompiled cohort. Updates preflight all
  coordinates before committing.
- Private recurrent/somatic state, elapsed-tick eligibility, joint-selection
  deduplication, receptor-projected Hebbian/Oja waking updates and developmental
  ceilings. Pending credit binds organism, phenotype, sequence, frame, action,
  feature digest, dispatch, side and normalized joint motor selection. Only a
  sealed patch with measured physiology and canonical receptor data can supply
  credit. Zero-microstep collection is explicitly rejected until an inactive
  activation-bank checkpoint is implemented; zero-step detached replay remains
  supported.

## Validation

Run on a machine without a GPU/ICD:

```sh
VK_ICD_FILENAMES=/no-vulkan-icd.json cargo test --locked -p alife_training --features cpu-backend --lib cpu::
VK_ICD_FILENAMES=/no-vulkan-icd.json cargo test --locked -p alife_training --features cpu-backend --test cpu_backend --test checkpoint_validation --test blocked_legacy_pipelines -- --nocapture
cargo clippy --locked -p alife_training --features cpu-backend --lib --test cpu_backend -- -D warnings
```

The deterministic N2048 fixtures check recurrence formulas, finite-difference
recurrent/decoder/speech gradients, detached burn-in, zero-step replay, joint PPO
probabilities/entropy gradients, imitation, residual critic Adam, private state,
and native checkpoints at actor age 16,684 and critic age 19,800. Integer-age
coverage includes 16,777,217 and overflow rejection. The test suite emits actual
CPU wall time for a complete numerical PPO batch, including useful accepted
optimizer updates. Its `canonical_valid_decisions` is explicitly zero: fixture
rows do not certify organism behavior or production throughput.

The opt-in `cpu-backend,gpu-tests` test compares the same replay/gradient fixture
against required hardware with absolute/relative tolerances. It is compiled in
CPU CI, but hardware execution remains Unknown until a real-GPU receipt exists.
No llvmpipe or CPU-backed Vulkan adapter is accepted as CPU-backend evidence.

Local cloud validation passed 13 CPU integration tests, 5 direct plasticity/state
unit tests and 4 existing checkpoint/gate regressions, strict Clippy, hardware-test
source compilation and 77 documentation assertions. An unoptimized numerical
batch of four PPO rows performed one actor and one critic update in 0.066875797
seconds on `native-cpu`, x86_64, Rust-reported parallelism 4. Canonical valid
decisions were zero. This records numerical work and wall time only.

## Remaining integration: Blocked, not a completed CPU training cycle

The canonical collector and headless training CLI still need an explicit CPU
closed-loop evidence variant and a shared canonical host. Core sealed neural
evidence currently names `NeuralClosedLoopGpu`. This feature does not label CPU
decisions as GPU decisions or bypass `require_canonical_training_biology`.
The existing legacy evaluators must continue failing before device/world
mutation. Biology, memory, cognitive context, sleep/consolidation, native full
organism persistence and source-package admission must remain causal in the
collector integration; this numerical stage does not replace those mechanisms.

The accepted policy7400 package has not been supplied in this environment. Its
32,768 actor weights, actor age 16,684 and critic age 19,800 have not been loaded,
reset, synthesized, or changed. Exact accepted-package continuation and useful
canonical-decision throughput remain Unknown. No speedup claim is made.

## Requirement trace

| Requirement | Scope/evidence |
| --- | --- |
| AOA-ADM-007, AOA-TIME-006 | Explicit offline CPU placement; honest CPU receipt; gameplay unchanged. |
| AOA-BRAIN-006/007/009/011/014 | Recurrent/somatic/dynamic sparse equations and gradient/formula checks with tolerances. |
| AOA-LEARN-002/003/005/010 | Bound joint eligibility, sealed receptor credit, distinct waking/plasticity state; canonical collector integration remains Blocked. |
| AOA-AUTH-003, AOA-INV-009 | One organism-owned biology path is required; no independent synthetic physiology is introduced or certified. |
| AOA-PERSIST-002/004 | Native optimizer checkpoints preserve integer ages and moments; full accepted organism-package continuation remains Unknown. |
