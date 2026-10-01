# Founder pretraining care improvements — 2026-10-01

Implementation evidence, based on `8c4c881eec7d3b3eccdea53ced8b108844b69782`.
Branch: `codex/founder-pretraining-dispositions-20261001`.
This report does not amend the controlling v2.0 architecture.

## Small first slice

- **Input fix:** both scaffold and expressed founders have bilateral chemical
  sensing and proprioception. Encoder construction covers distinct available
  lanes before duplicating them; hunger, fatigue, pain and temperature ports
  remain individually observable. Shared modality lane ranges still allocate
  unique coordinates. No object identity or food category enters these ports.
- **Action learning fix:** targetless candidates have a constant feature23 basis.
  Rest/hold also carry posture21; vocal opportunities carry vocal22, separating
  the two existing Other-family choices. These describe opportunities, not
  preference or reward. The world supplies them without scores; existing GPU
  motor readouts and eligibility consume them. The core perception validator
  validates the exact action descriptor separately from physical primitives.
  Object candidate terrain/temperature lanes keep their existing meaning.
- **Disposition experiment:** urgency-scaled, smaller decoder priors encourage
  hungry investigation before contact, taste-supported ingestion at contact,
  aversion to harmful taste under danger, and resting under fatigue. Approach
  uses physical noncontact and nonzero bearing, not distant object chemistry.
  Gain constants fall from 112/128/96 to 16/24/8 before inherited traits scale
  them; rest changes from hunger suppression to positive fatigue gain 8.
  This retains neural arbitration and permits bounded learned readouts to
  compete. A hunger-driven organism may investigate a rock: food recognition
  and navigation remain learned problems.
- **Credit experiment:** new founder defaults use the already implemented
  SignedChoiceReadouts receptor on ActionCandidate and MemoryContext (and
  CognitiveContext when enabled). Positive prediction surprise remains on
  recurrent and speech receptors. The new action/memory profile removes its
  positive surprise coefficient while retaining signed consequences and novelty.
  No new learning knobs, reward labels, projections or runtime CPU brain exist.

Source ownership: `genome.rs`, `evolutionary_genetics.rs`, phenotype
`io_compile.rs`, `decoder.rs`, `learning.rs`, world `candidate_enumerator.rs`,
and GPU `closed_loop_decode.wgsl`. A required narrow extension in core
`perception.rs` admits the new exact intrinsic descriptor; leaving its zero-only
validation unchanged rejected the production world draft. No scheduler,
checkpoint, pipeline codec, graphics, teacher or CPU regression files changed.

Requirements addressed by source/CPU evidence: AOA-GEN-001/002/003,
AOA-SENSE-001/002, AOA-INV-006, AOA-LEARN-001/003/006/007/010.
Useful learned gameplay and production GPU causality remain **Unknown**.

## Current actor handoff: retain state, change the intended design

[Address comparison](founder-pretraining-address-check-2026-10-01.json) compares
base and candidate N2048 GroundedTerrainVisionV1 compilation at seeds 1, 2, 8, 60010,
for both scaffold and mature expressed founders. In all eight pairs:

- All 2,048 neuron and 32,768 synapse persistent addresses **and packed indices**
  are identical. Every weight's source/target, decoder head/family/input/output
  coordinate, genetic value and alpha is identical. Projections, route ABI,
  plasticity ABI, lobe layout and neuron dynamics are identical.
- Sensor assignments change 104→146 for scaffold and 396→439 for expressed.
  The sensor encoder, candidate prior digest, plasticity receptor/plan digest,
  compiler-input digest and phenotype hash intentionally change.
- Synapse receptor indices can be renumbered. Rebuild immutable receptor tables
  and bindings from the new phenotype; do not copy the previous receptor-index
  table merely because weight coordinates match.
- Language, speech/memory decoder shapes, replay capture and sleep plans remain
  identical in these fixtures. Candidate dimensions remain 24 features and 8
  families. No recurrent/action/memory weight count changed.

The ONE current actor/checkpoint was not available here. Desktop integration
should perform the same exact map comparison on that actor, retain a byte-for-byte
prechange checkpoint, and use the existing snapshot/asset APIs for a deliberate
same-address design rebind. This is not a request for historical-format support.

1. Settle its pending outcome transaction under the original phenotype before
   crossing the metadata boundary. A pending receipt binds the old phenotype and
   perception; relabeling it would fabricate causal provenance.
2. If the exact addresses/indices match, copy immutable trained weight bits and
   both lifetime/fast banks without reinitialization. Keep optimizer moments,
   master weights, step counters and RNG state when continuing offline training.
   Copy activations, neuron homeostasis, both eligibility banks, active-side/bank
   selectors and generations using those same addresses. Preserve the actual
   biological/world state, personal memories, learned lexicon and identity.
3. Use the new intended sensor ports, care priors and signed choice receptor
   bindings. Preserve genotype seeds, topology, learning rates, bounds and sleep
   parameters that belong to the current actor rather than replacing its whole
   genome with a fresh scaffold. Sensor/development allowlists must admit the
   added channels. A serialized old genome keeps its explicit old credit policy;
   installing the new policy on the current actor is an intentional design edit.
4. Reissue foundation metadata around the actual trained payload with existing
   `FoundationWeightAsset::from_trained_weights(new_phenotype, original_weights,
   original_training_stage)` where appropriate. This asset does not contain
   optimizer or lifetime state; preserve those separately through existing
   checkpoints. Update phenotype/source bindings explicitly. Retain historical
   replay/experience provenance; do not rewrite an old perception receipt to
   pretend it was produced by the new ports. Audit replay admission at this
   design boundary rather than silently dropping the journal.

Address equality justifies copying compatible arrays; it does not claim identical
behavior after an intended sensory/prior/receptor change. Actual actor transfer
is **Blocked here by unavailable checkpoint**, owned by desktop integration.

## Cheap verification and integration follow-ups

Commands use `/workspace/cloud-cpu-validation/env.sh`; no long training or GPU
hardware test was run.

- `cargo test -p alife_core --lib`: 22 passed.
- `cargo test -p alife_world --lib founder_basis_tests`: 1 passed, including a
  real terrain world draft, separated intrinsic choices and absent distant taste.
- `cargo test -p alife_world --test perception_candidates`: 20 passed.
- `cargo test -p alife_gpu_backend --test closed_loop_wgsl`: 16 passed including
  Naga parsing/validation. `closed_loop_gpu_behavior` has zero enabled tests
  without `gpu-tests`; this supplies no GPU behavior evidence.
- `cargo test -p alife_core --test phenotype_compiler`: 46 passed, 3 fixture
  failures. Shared vision expects 120 assignments instead of 162. Shared
  smell/taste adds a duplicate founder Smell gene. The causal sensor mutation
  adds a duplicate founder Proprioception gene. CPU regression owner should
  derive expected receptor counts, remove the default test modality before
  constructing an explicit shared pair, and prepare the causal sensor fixture
  without its existing Proprioception before testing addition. Keep duplicate
  gene rejection intact.
- `grounded_object_slots`: 12 passed, 3 failures involving pre-existing PLAY
  activation/count expectations; all three reproduce at the base commit with
  an isolated target directory (`/tmp/founder-baseline-target`).
- Formatting, whitespace, core boundary checks and docs assertions (77/77) pass.

The unowned training `evolution.rs` PlasticityReceptor and BiochemicalSensitivity
mutations reconstruct parameters with `try_new`, which resets the optional choice
credit policy. Their owner should preserve `source.action_candidate_credit_profile()`
when rebuilding parameters before using those mutations; no edit was made here.

Desktop's cheap causal check: use the preserved current brain to inspect receptor
admission and hungry/fatigued logits, then run a short care/navigation replay
with sampling and with game argmax. Zero-meal diagnostics do not yet isolate the
cause, and the CPU checks above do not prove that these changes solve it.

## Next scoped fix: eligibility follows simulation time

AOA-TIME-001/002/003/004/005 and AOA-LEARN-002 apply. Current WGSL decays by one
factor per decision, regardless of elapsed world ticks. It copies inactive
recurrent traces without decay. At the current 20 world ticks/s, .95 implies a
13.51-decision half-life (0.676s only at one decision/tick). Slow/fast learning
bands change rates, not this timescale.

Propose `decay_elapsed = decay_reference ^ elapsed_simulation_ticks` with the
existing factor defined at one current world tick, explicit 0/1 edge handling,
and a resident last-committed-trace tick. Decay all existing traces, including
inactive routes; add local coactivation only for actual active work. Initialize
an admission anchor at the actor's current simulation tick, persist it, advance
it only on successful trace commit, and reset it with eligibility during sleep.
A discarded transaction must leave the committed anchor unchanged. Use simulation
elapsed time rather than render cadence, wall time or neural microstep count.
This corrects timing semantics first; useful longer action-credit horizons remain
an experiment, not a justified arbitrary new constant.

Exact follow-up ownership to coordinate before edits: GPU
`closed_loop_learning.rs`, `closed_loop_pipeline.rs`,
`closed_loop_buffers/abi.rs`, `closed_loop_buffers/bucket.rs`,
`closed_loop_runtime.rs`, `closed_loop_runtime/tick.rs`,
`closed_loop_checkpoint.rs`, `closed_loop_sleep.rs`, plus shaders
`closed_loop_abi.wgsl`, `closed_loop_eligibility.wgsl`, and
`closed_loop_plasticity.wgsl` (commit the anchor with the trace). Existing GPU
ABI/reflection tests must follow the header/state change. Desktop currently owns
this pipeline/integration surface. Core/world clock adapters are needed only if
the chosen implementation cannot consume the already authoritative frame tick.
Timing is a proposal only; none of these files changed in this slice.
