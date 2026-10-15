# Bounded memory-linked game exploration, 2026-10-15

## Scope and status

This is an implementation/qualification note, not a revision of the controlling
v2.0 architecture. It describes the isolated adaptive-exploration change based on
main `69f00c05ec6774c3655ae87c5d4baa5df3861929`.

The production GPU candidate selector now has a bounded exploratory perturbation.
The intent is Creatures-style observable behavior: familiar, settled routines
retain their preferences; unfamiliar objects and unresolved learned gaps permit
small variations, while urgent bodily state suppresses that variation. This is a
simple biologically inspired control mechanism, not a model of human cognition,
a new synthetic biology subsystem, or a claim of biologically validated constants.

Actual GPU behavioral acceptance, same-adapter checkpoint continuation, live
performance and tuning across trained N512/N1024/N2048 policies are **Unknown**.
CPU/shader checks do not establish those outcomes. No gameplay, GPU training or
benchmark run on the user's computer is part of this qualification.

## Causal inputs and the explicit layout limit

- Existing preparation derives focal-object novelty as one minus the organism's
  indexed, episode-derived familiarity with that tracked object. This is not a
  confidence score or whole-situation understanding. No focal tracked object
  yields zero remembered novelty; memory eviction can legitimately restore novelty
- Familiarity is currently `1 - 1 / (1 + retained_observations)`, so ordinary
  practical familiarity approaches one gradually and can retain tiny perturbations.
  Exact deterministic argmax is guaranteed only at zero novelty/gap evidence or
  full urgency; there is no newly invented familiarity deadband
- Curiosity is the existing phenotype/body-derived drive. Baseline curiosity alone
  cannot warm a fully familiar, gap-free routine
- For existing 54-lane cognitive-extension phenotypes, candidate learning-input
  lane 50 carries the target-bound unresolved `gap_match`. The preceding memory
  context pass populates this lane from the finalized cognitive projection
- Standard 36-lane phenotypes do not carry that projection. They get the
  familiarity/novelty and curiosity coupling, but **no unresolved-gap warming**.
  The selector checks the stride before reading lane 50. No phenotype migration,
  foundation replacement, acquired-state reset or layout expansion is introduced
- The neighboring legacy `prior_residual` lane is deliberately excluded. Its
  current producer computes confidence times mean absolute remembered outcome,
  not prediction mismatch. Correcting or renaming that semantic concern is
  separate work
- Existing memory expectancy, genetic + lifetime + alpha * fast weights, sealed
  outcome learning and sleep consolidation remain the sources of learned logits.
  Exploration neither overwrites them nor substitutes a host-authored action

The unresolved-gap path is evidence of uncertainty associated with a target; this
slice does not establish full targeted information-seeking (AOA-EVAL-006), a
world-model confidence estimator, or learned long-horizon exploration.

## Provisional calibration

The constants are centralized in `closed_loop_decode.wgsl`, with deliberately
narrow scalar reconstruction only for requested selector diagnostics and tests.
There is no new user-facing tuning/configuration system.

For each candidate:

```
evidence = max(focal_object_novelty, supported_candidate_gap_match)
critical_need = clamp((max(hunger, fatigue, temperature_stress) - 0.65) / 0.35, 0, 1)
urgency = max(fear, pain, critical_need)
scale = 0.5 * evidence * (0.6 + 0.4 * curiosity) * (1 - urgency)
score = raw_learned_logit + scale * uniform_noise_in_open_interval(-2, 2)
```

These are engineering bounds awaiting measured tuning. With no novelty/gap evidence,
scale is exactly zero, preserving existing argmax and lowest-index ties. Full
urgency also makes scale zero. The scale is bounded to [0, 0.5] absolute logit units;
each perturbation is strictly between -1 and 1. A candidate more than 2 logit units
below a winner cannot replace it through exploration. Higher learned margins do
not increase the noise, and an extreme losing candidate cannot inflate it.

This conservative bound can be ineffective on large trained logit gaps. That is
a declared calibration limit, not something hidden by per-row margin/range
normalization. It is bounded exploration strength, **not exact Boltzmann/softmax
temperature**. Uniform noise avoids the tie/index bias produced by clipping many
Gumbel upper-tail samples to one value.

## Durable identity, channels and authority

The stateless counter hashes both halves of the durable organism ID and full world
tick plus a 128-bit logical candidate key. That key includes action ID, primitive,
family, stable target entity and/or grounded target position, and duration bounds.
It excludes candidate enumeration index, observation slot, GPU slot/generation,
packed offsets, dispatch generation, request order and retry count. Distinct target
or primitive alternatives do not intentionally share a draw. An exact duplicate
logical candidate shares the same draw and retains the existing lowest-index tie
rule. Changing a candidate's meaning legitimately changes its key.

The organism ID is read from the existing host-validated pending-eligibility
frame template; the full perception-header tick is checked against that template. Candidate keys are appended immediately after it as unscored
transient metadata and checked against the host's exact private identity records.
They are reconstructed from existing saved world/action fields: no mutable RNG
counter or new persistent neural state is needed. Same-tick retries reuse a draw;
row reordering, slot relocation and dispatch-generation changes cannot change it.
Exact runtime GPU checkpoint continuation still requires a measured acceptance run.

Representative and factorized motor selectors call the same candidate score
function. Channel masks, candidate validity, representative forced-slot override,
joint bundle assembly, world legality and outcome sealing retain their existing
boundaries. Raw logits are never replaced by perturbed values: selection receipts,
memory deltas and training likelihood evidence still describe the learned logits.
Opt-in selector diagnostics retain explanatory raw argmax data, advertise the
versioned adaptive policy and separately verify its scalar arbitration criterion.
Legacy disabled-exploration receipts remain explicitly versioned.

## Training and work cost

Explicit training-rollout requests bypass the game perturbation hook. Their
caller-specified fixed temperature, categorical GPU sampler, demonstrator path and
PPO likelihood calculation remain unchanged. No training default is copied into
ordinary play.

No new compute entry point, neural pass, storage binding, ordinary GPU readback,
neural CPU fallback or memory search is added. The existing two bounded selector
scans each calculate one candidate-local scalar score; candidates remain capped at
32. Transient upload identity metadata costs 16 bytes per candidate, at most 512
bytes per active row. Arena/staging capacity planning includes that bound. Host
identity hashing is bounded by the same candidate count. These structural costs
are not a measured performance result or an assertion of zero overhead. Existing
work receipts still count decoded candidates and neural operations; they do not
separately count the additional scalar hash/perturbation arithmetic.

## Requirement trace and remaining acceptance

- AOA-TIME-001, AOA-PERSIST-001/002/004: logical stateless identity, full tick,
  existing saved causal inputs; CPU host upload/relocation checks qualify the
  wiring, while exact GPU resume remains Unknown
- AOA-MEM-002/003, AOA-ATT-002: existing recalled familiarity and target-bound gap
  evidence modulate GPU arbitration without issuing actions from the memory store
- AOA-MOTOR-003: retain within-channel selection and existing joint coordination
- AOA-BIO-002/018/022: body/chemistry supply cues; neural-side arbitration derives
  bounded effects, and no drive is relabeled as reward. This qualifies ownership
  placement, not heritable exploration calibration: the new gains/thresholds are
  shared provisional shader constants
- AOA-PERF-001/003: fixed-size metadata and bounded candidate work, no new search,
  pass or readback. Runtime timing and behavior-per-compute remain Unknown

Before promotion: run the same saved creature and fixed world inputs on the same
adapter; verify cold/novel/gap/urgent choices, representative/channel legality,
continuation across save/restore into changed residency, learned preferences and
sleep/lifetime retention. Measure ordinary tick wall/GPU time and upload bytes
against the source-matched baseline. Tune only from those receipts. Draft PR/CPU
checks are not release or merge approval.

## CPU/source qualification

Coordinated low-memory cloud runs use one Cargo build job, no incremental build,
and no `gpu-tests` feature. The backend `--all-targets --features training-rollout`
run passes 204 tests across 27 test targets (zero failures). This includes actual
Naga validation of production and feature-composed shaders, current seven-binding
reflection, the exact host batch builder, candidate permutation/target identity,
serialized perception-frame and pending-template roundtrips, slot/row/dispatch
relocation, bounded distribution sweeps, and the training bypass. An earlier
production-only focused run passes 66 unit, 14 exploration and 16 WGSL tests.
Hardware-gated test targets report zero executed tests; that is not GPU acceptance.

The fixed-size uniform sweeps are mathematical fixtures tied to the shader's
arithmetic and identity source. They qualify the bounded control rule and test
wiring, not a CPU neural backend or a measured GPU parity/performance oracle.

The app's CPU causal regression also passes. It derives the real thirteen-lane
position/velocity/drive source, establishes a controlled matched predictor target,
then uses `observe_frozen` and production `apply_prediction_evidence` to compute a
0.75 residual from controlled successor values. That residual creates and reinforces
one target-A `PredictionError` gap while target B stays unchanged. Finalized recall
and the exact cognitive upload retain it despite acquired object familiarity.
The fixture then changes only the actual uploaded gap in the shared, shader-bound
mathematical selector rule: fixed logits, logical identities and frame-derived
state give 0 versus 1415 alternate choices over 4096 fixed ticks. The cold margin
exceeds its complete pair's noise bound. Controlled successor extrema and supplied
contact outcomes are fixture boundaries, not observed physical-world transitions,
GPU decisions, full targeted curiosity, or gameplay/performance acceptance.

Final bounded gates also pass the legacy training selector-receipt CPU regression,
strict Clippy for all targets of `alife_gpu_backend`, `alife_training` and
`alife_game_app` with `foundation-training`, workspace formatting, core static
boundaries and all 77 documentation assertions. This is an affected-source CPU
qualification; full-workspace CI and the GPU acceptance gates are separate.
