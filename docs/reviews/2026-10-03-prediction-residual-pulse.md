# Prediction-residual endocrine continuity — 2026-10-03

Authorized repair of the PredictionResidual part of finding 3 in the
[biology continuity audit](2026-10-02-biology-continuity-audit.md). Base:
`3ee6b0351b4c0b2949c93d5e61dd430a69b0078c`, which includes the taste and current-age
repairs. Publication branch: `phase3-prediction-residual-pulse`. No main merge,
training, GPU execution, or desktop run forms part of this evidence.

## Result and timing

The shared production sealing path measures grounded successor prediction error
after the action. Previously this residual entered learning credit but never
activated the inherited PredictionResidual neuroemitters. It now stages one
organism-owned pulse before sealing measured physiology. The next successful
biology transition applies that pulse through the existing biochemical graph,
then clears it. This can be an awake action, idle update, or sleeping update.
Catch-up consumes one pulse rather than repeating it for each elapsed tick.

The existing inherited pathways release adrenaline and learning signal, scaled by
the creature's expressed graph. The resulting chemical state reaches subsequent
receptors and neural input through the existing production bridge. This change
does not claim a measured improvement in behavior or learning efficacy.

The pulse carries versioned source tick, experience sequence, neural dispatch
generation, activity, and confidence. Its source tick is the exact post-action
biology tick. The enclosing staged world/resident transaction either seals the
experience and pulse together or restores the preceding state. An immutable
failed biology advance retains the pulse; a failed organism-record update also
restores its body state reference. A second unconsumed pulse cannot overwrite it.

## Persistence and preserved ownership

The optional pulse lives inside the existing canonical `BiochemistryState`, and
its digest participates in the organism's existing body state reference. World
save/load therefore retains a queued pulse and retains its absence after
consumption. No history scan recreates a pulse on restore.

When no pulse is queued, serialization omits the new optional field. A genuine
pre-change fixture generated on exact base `3ee6b035` verifies legacy biology
decoding and byte identity; a full organism-record golden verifies the unchanged
body digest and serialized record. No phenotype/compiler identity, acquired
neural bank, genetic plasticity mask, GPU ABI, shader, or six-founder construction
changes accompany this repair. AOA-BIO-020 and AOA-BIO-024 remain the relevant
bounded bridge and distinct prediction/value/reward requirements.

Source points in this revision: [production staging](../../crates/alife_game_app/src/gpu_live_runtime.rs#L4895),
[state and validation](../../crates/alife_core/src/biochemistry.rs#L656),
[next-update consumption](../../crates/alife_core/src/biochemistry.rs#L873), and
[atomic record staging](../../crates/alife_world/src/organism.rs#L707).

## Canonical training dependency

This is shared production behavior, including canonical N2048 foundation
training. The supported entry point calls `foundation_training_cycle`; that cycle
constructs `GpuLiveBrainRuntime::new_profiled_foundation_training` and advances it
through `tick_outcome`. That runtime uses the same sealing function modified here.
Thus the pulse changes chemistry and subsequent receptor inputs in training
experience collection as well as deployed lives. It is a correctness dependency
under the requested fixes-before-training ordering, though the old executable can
launch without it. Accelerator effect size remains Unknown.

## Independent review and verification

R2 source review by a separate SOL 6.1 agent found no blocker, including incoming
frame validation, provenance, legacy encoding, sealing order, exact body identity,
and late-tick retry. Builds and tests use CPU only with `CARGO_BUILD_JOBS=1` and
`RUST_TEST_THREADS=1`. The publication receipt records command results and remote
CI status for the final commit; this source-bound report does not certify a later
revision.

Focused coverage includes delayed chemical consequence, once-only consumption,
save/load on each side of consumption, stale and malformed neural frame rejection,
invalid persisted provenance, overwrite rejection, catch-up, idle/sleep consumption,
legacy byte/digest identity, failed record retry, and two-creature late-tick rollback
with distinct residuals and an exact successful retry. The CPU production sealing
test also checks that measured physiology contains the canonical queued post-state
and exact source sequence/tick/activity.

All ten focused regressions pass on this revision (five core, three world
integration, one two-creature rollback, and one production sealing test):

```sh
cargo test -p alife_core --test prediction_residual_pulse
cargo test -p alife_world --test prediction_residual_pulse
cargo test -p alife_world --lib prediction_residual_pulse_tests
cargo test -p alife_game_app --features foundation-training --lib seal_prepared_selection_uses_world_biology_receipt_as_resident_authority
```

Workspace formatting, `git diff --check`, core/world Clippy with all targets and
warnings denied, core boundary checks, and all 77 documentation assertions pass.

The existing chemistry/development/nociception suites (24 tests) and world unit,
save/load, atomic-action, and organism-registry suites (120 tests, including the
new rollback regression) also pass. These CPU suites cover existing death,
reproduction, and transactional persistence behavior alongside the repair.

## Related source-trace conclusions; no additional repair

`plasticity_enabled = false` is an inherited projection-expression flag, not a
training-completion lock. It suppresses compiled alpha on that route, but the
N2048 section-policy fallback, decoder handling, and other weight operations do
not establish a universal immutable-weight contract. The explicit
`LifetimePlasticityBand::Fixed` policy gives selected section weights zero
lifetime receptor rate. However, all 15 current canonical N2048 routes select
zero Fixed-band synapses: the `slow` and `fast` helpers put zero in the fixed
count, and the mixed route also explicitly puts zero there. Fresh foundation
optimization instead constructs a
trainable mask covering compiled synapses, or restores the prior optimizer mask.
Therefore selecting innate fixed routes after training needs an explicit policy;
reinterpreting every inherited false flag as a global freeze is not this repair.
Existing masks and exact acquired-state compatibility are preserved.

Trace: [inherited expression](../../crates/alife_core/src/evolutionary_genetics.rs#L1497),
[alpha handling](../../crates/alife_core/src/phenotype/topology_compile.rs#L526),
[lifetime receptor handling](../../crates/alife_core/src/phenotype/learning.rs#L656),
[canonical bands](../../crates/alife_core/src/foundation.rs#L126), and
[training optimizer mask](../../crates/alife_game_app/src/foundation_training_cycle.rs#L1246).

SocialState still lacks a production emission source. Physical positive contact
already has its own oxytocin/body-event path. The sensed agent affinity comes from
the other object's static `social_affinity`; the similarly named presentation
projection does not establish an observer-owned bond. Episodic trust/fear biases
are derived from that affinity and proximity. More useful existing evidence is
the organism-owned `CandidateMemoryStoreV2`, whose per-tracked-object sealed
experiences retain consequence, success, danger, energy, contact, and prediction
error and feed ordinary recall. That is a bounded substrate for meaningful
individual interactions, not yet a complete persistent care/bond model. A future
SocialState bridge should derive a GPU-owned summary from such ordinary personal
evidence after its intended meaning is coordinated (AOA-MEM-007). This change adds
no host-authored social intent or new relationship feature.

Trace: [sensed affinity](../../crates/alife_world/src/headless.rs#L3454),
[episodic affinity biases](../../crates/alife_core/src/memory.rs#L2101), and
[per-target sealed consequence memory](../../crates/alife_core/src/memory/candidate_recall.rs#L6).

Coverage gaps: no GPU causal experiment, whole-life competence campaign,
statistical fertility/lifespan balance, or end-to-end acquired neural checkpoint
experiment. Those limitations do not negate the verified CPU state transition and
shared production caller, but they limit behavioral claims.
