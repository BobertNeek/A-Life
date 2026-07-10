# GPU-Authoritative Closed-Loop Brain Design

**Date:** 2026-07-09
**Status:** Approved implementation design
**Repository baseline:** `origin/main` at `16ba2abc`
**Implementation branch:** `codex/brain-gpu-closed-loop`

## 1. Objective

Replace the current split cognitive architecture with one causally closed,
GPU-authoritative brain. Current perception, recurrent neural state, candidate
scoring, action selection, outcome credit, waking plasticity, sleep
consolidation, and later behavior must form one observable causal loop.

This design implements the recommendations from the Alife Brain Architecture
Analysis without preserving the current heuristic controller as the implicit
brain. It preserves the engine-neutral contracts, sparse class-bucketed
storage, structured actions, world-authoritative legality, sealed
`ExperiencePatch`, deterministic seeds, and GPU-oriented execution boundaries.

The full goal remains P0 through P3. Work is divided into integration slices so
each causal layer can be validated, but no slice redefines the final objective.

## 2. Controlling decisions

1. `NeuralClosedLoopGpu` is the normal neural policy.
2. Production neural execution runs once on the GPU. There is no live CPU
   shadow, per-tick CPU parity copy, parity-gated handoff, or silent CPU neural
   fallback.
3. `HeuristicBaseline` remains an explicit, separately labelled policy for
   comparison and emergency product diagnostics. It is never selected because
   GPU neural execution failed.
4. If a required GPU adapter, feature, pipeline, or buffer cannot be created,
   neural mode reports `NeuralBackendUnavailable` and performs no learned
   action. It does not claim a neural tick occurred.
5. Pure CPU math may remain in a test-only `reference_debug` surface for
   contract tests and offline fixture generation. It cannot be linked into the
   production neural tick or used as a second live brain.
6. Initial production neural capacity classes are N512, N1024, and N2048.
   Existing larger tier identifiers remain readable for save compatibility but
   are research-gated until they pass causal, soak, memory, and performance
   gates.
7. The existing `cpu_shadow_*`, parity, and auto-with-CPU-fallback runtime
   contracts, telemetry, CLI claims, tests, and documentation are removed or
   migrated. They are not retained as dead compatibility architecture.
8. GPU authority does not weaken the world boundary. The world still enumerates
   candidates, validates the selected structured command, executes it, and
   measures the outcome.

## 3. Current-state defects this design removes

The current default mind constructs an empty sparse projection schema, advances
old neural state before gathering current perception, accepts caller-scored
`ActionProposal` values, applies a candidate-invariant symbolic bias, and then
updates behaviorally inactive `h_shadow`. The live bridge creates scores such as
food `0.72`, hazard `0.66`, inspect `0.38/0.42`, and idle `0.28`. Sleep is forced
and consolidated by harness code rather than advanced by the canonical brain
state machine. Memory truncates its default query after the 16 visual channels,
and topology capacity can turn an ordinary long run into a terminal brain
transaction.

The current GPU path does not close that loop. It receives handcrafted
saliences, produces a small action summary, compares against a CPU shadow, and
then injects the returned values back into heuristic proposal construction.
This design replaces that path rather than extending it.

## 4. Architectural ownership

### 4.1 `alife_core`: versioned contracts only

`alife_core` remains engine- and GPU-library-independent. It owns:

- `BrainCapacityClass`
- `BrainGenome`
- `BrainPhenotypeManifest`
- `PhenotypeHash`
- `SensorProfile`
- `PerceptionFrame`
- `ActionCandidate`
- `CandidateFeatureVector`
- `NeuralActionSelection`
- `NeuromodulatorSample`
- `SleepState`
- `ExperiencePatch`
- portable save/export records

It does not execute a production neural tick. The current 1,100-line
`reference_brain.rs` orchestration is decomposed and retired from production.
Reusable state contracts move to focused modules; CPU reference execution moves
behind test-only or developer-only compilation.

### 4.2 `alife_world`: perception and legality

The world owns a `CandidateEnumerator` that produces potentially legal,
unscored candidates from the same world snapshot used to build perception.
It also remains the final authority for command legality and measured outcome.

The candidate enumerator may expose observed mechanical facts. It may not add a
utility, desirability, danger score, food score, action prior, or learned value.

### 4.3 `alife_gpu_backend`: cognitive execution

The GPU backend owns:

- deterministic phenotype buffer compilation;
- sparse topology and structure-of-arrays weight buffers;
- sensory/body/homeostasis encoding;
- recurrent neural microsteps;
- candidate feature encoding and motor decoding;
- GPU winner selection;
- eligibility accumulation;
- post-outcome three-factor fast plasticity;
- sleep consolidation and safe double-buffered structural swaps;
- compact action and diagnostic readback.

The backend exposes one shared `GpuClosedLoopBackend` plus lightweight,
backend-instance-scoped, generation-checked `GpuBrainHandle` values whose
fields are private. A handle from another backend instance or a prior slot
generation is rejected before any buffer access. The backend owns one
device/queue, shared pipelines, and class-bucketed SoA pools; a per-creature
handle never owns or duplicates those resources. Device and pipeline types
never cross into `alife_core` or `alife_world`.

### 4.4 `alife_game_app`: scheduling and policy selection

The app selects one explicit policy:

```text
NeuralClosedLoopGpu
HeuristicBaseline
```

It schedules world snapshots, GPU submissions, compact readbacks, action
execution, outcome uploads, sleep phases, save checkpoints, and diagnostics.
It does not construct neural proposal scores.

## 5. Core data model

### 5.1 Capacity is not cognition

`BrainClassSpec` remains only a legacy-save adapter; production uses
`BrainCapacityClass` keyed by stable N512/N1024/N2048 `BrainClassId` values. A
capacity class does not embed `BrainScaleTier`. A capacity class
defines ceilings and execution alignment only:

- maximum neurons;
- maximum active synapses and tiles;
- maximum candidates and object slots;
- buffer alignment and storage formats;
- compute, memory, and readback budgets;
- permitted microstep range;
- supported GPU feature floor.

Capacity records are validated constructors, not forgeable bags of public
limits. Their canonical identity covers every execution-ABI requirement and
bounded dimension used by admission or dispatch.

It does not define semantic capability, fixed lobe meaning, or a cognitive
claim. N512, N1024, and N2048 are the initial enabled classes.

### 5.2 Deterministic phenotype compiler

At birth, development milestones, sleep structural compilation, or explicit
offline migration, a compiler maps:

```text
BrainGenome
+ BrainCapacityClass
+ DevelopmentState
+ SensorManifest
+ MotorManifest
+ deterministic seed
-> BrainPhenotype
```

`BrainPhenotype` contains:

- compiled lobe/module ranges;
- sensor encoder assignments;
- candidate decoder assignments;
- projection routes and sparse indices;
- immutable genetic weights;
- per-route density and tile allocations;
- alpha and plasticity receptor parameters;
- neuron biases, leaks, activation functions, delays, and cadences;
- structural-growth permissions;
- global and per-route budgets;
- stable canonical phenotype hash.

Every accepted evolvable genome field must affect the compiled phenotype or be
removed from the production genome. Compiler tests mutate each accepted field
and assert a phenotype or behavioral difference.

Over-budget genomes are deterministically repaired according to documented
priority rules or rejected with a typed compile error. The compiler never
silently allocates beyond the capacity class.

### 5.3 Single ownership and structure-of-arrays storage

Immutable topology and genetic parameters are stored once in phenotype-owned
GPU buffers. Mutable state is stored in separate GPU pools:

- activation A;
- activation B;
- accumulators;
- lifetime-consolidated weights;
- immediately active fast weights;
- eligibility traces;
- optional shadow/audit journal;
- neuron homeostatic state;
- candidate descriptors and logits;
- compact diagnostics.

The duplicated `CpuNeuralState.projections` and `NeuralProjectionSchema`
ownership is removed. The active path does not clone complete schemas, memory
banks, or topology maps per tick.

## 6. Perception and unscored candidates

### 6.1 Same-tick perception frame

The world emits one `PerceptionFrame` containing:

- organism and tick IDs;
- active `SensorProfile`;
- sensory channels/object slots;
- body pose, velocity, and proprioception;
- drive and endocrine state;
- bounded episodic retrieval context;
- unscored `ActionCandidate` records.

Perception and candidates must come from the same authoritative world snapshot.
The brain cannot advance before that frame is gathered.

The causal record uses two explicit digests. `PerceptionBaseDigest` identifies
the same-tick world, body, homeostatic, and unscored-candidate frame used to
query episodic memory. After bounded retrieval context is attached,
`PerceptionFrameDigest` identifies the complete ordered GPU input. Decision and
seal validation bind both digests plus the retrieval-context digest, so memory
can neither create a circular query nor enter neural execution outside the
audited causal record.

### 6.2 Action candidate contract

An `ActionCandidate` carries:

- action ID and action kind;
- candidate action family used for neural decoding and learning;
- tick-local candidate index;
- optional target entity and target position for command transport;
- relative bearing, distance, and velocity;
- required effort and duration bounds;
- observed shape, material, color, chemistry, contact, and terrain features;
- observed affordances allowed by the active sensor profile;
- sensor confidence and visibility/contact evidence.

It carries no score. Raw world entity IDs are excluded from the feature vector
used by the decoder. They are transported only so the selected candidate can be
turned into a structured command.

For grounded experiments, generic candidate families are enumerated for
observed objects without revealing their class. For example, an unfamiliar
object can yield inspect, approach, avoid, contact, or ingest candidates; the
world determines whether execution succeeds and what outcome follows.

## 7. Explicit sensor profiles

Every run, save, patch, benchmark, and behavioral report records one profile:

### `PrivilegedAffordanceV1`

Retains bounded semantic affordance channels such as food, hazard, mate,
shelter, tool, glyph, or teacher cue. This profile isolates learning above
object recognition and supports comparisons with historical behavior.

### `GroundedObjectSlotsV1`

Provides egocentric object slots with bearing, distance, relative velocity,
color, material, shape, chemical gradients, temperature, contact,
proprioception, and terrain properties. It contains no food, hazard, teacher,
or object-class truth labels.

Behavioral claims must name the profile. Results from the privileged profile
cannot be reported as perceptual grounding.

## 8. GPU neural tick

The authoritative waking tick is:

1. Gather and validate the same-tick `PerceptionFrame`.
2. Upload bounded sensory, body, homeostatic, episodic, and candidate records.
3. Clamp encoded inputs into their compiled populations.
4. Run the phenotype's deterministic recurrent microsteps. The initial default
   is three; valid phenotypes may select two through four.
5. Encode every candidate and decode a neural logit from motor state and
   candidate features.
6. Apply motor lateral inhibition and deterministic GPU winner selection.
7. Read back only the selected candidate index, logit, confidence, and bounded
   diagnostic counters.
8. Construct the structured command and let the world validate and execute it.
9. Observe the outcome and seal the `ExperiencePatch`.
10. Upload the compact post-outcome credit packet.
11. Update eligibility-gated fast weights on the GPU.
12. Record episodic and topology sidecar observations.

The runtime reads no bulk activation or weight state during active play.

### 8.1 Neural dynamics

Each microstep applies per-population or per-neuron leak, bias, activation,
route cadence, and optional delay:

```text
a_i(next) = (1 - leak_i) * a_i
          + leak_i * phi_i(bias_i + input_i + sum_j(w_ij * a_j))
```

Routing metadata is executable, not decorative. Activity policy, cadence,
priority, and projection type determine dispatch behavior. Activity-dependent
BrainATP cost is accumulated from active neurons, tiles, synapses, and
microsteps.

### 8.2 Candidate-conditioned decoder

For candidate `k`, `family_k` is the learned discriminator. `ActionKind`
remains command-ABI metadata only because multiple opposite mechanical actions
can share one legacy kind:

```text
phi_k   = encode_candidate(observed candidate features)
logit_k = decoder_bias(family_k)
        + dot(motor_latent, decoder_projection(family_k, phi_k))
        + bounded homeostatic modulation
```

Decoder projection weights and motor assignments come from the compiled
phenotype. No bridge-provided score enters this equation.

## 9. Immediate causal plasticity

Effective weight semantics become:

```text
W_effective = W_genetic + W_lifetime + alpha * H_fast
```

`H_fast` affects the next neural dispatch immediately. `H_shadow` is retained
only if useful as an audit, rollback, precision, or pending structural journal;
it is never the sole waking learning target.

Eligibility accumulates during recurrent and decoder activity:

```text
e_ij(t) = lambda_e * e_ij(t-1) + F(pre_i, post_j, selected_candidate)
```

After the patch is sealed, a bounded neuromodulator is computed from reward
prediction error, pain, frustration, novelty, homeostatic improvement, and the
developmental receptor profile:

```text
delta H_fast = eta * alpha * M(t) * e_ij
             - eta_norm * post_j^2 * W_effective
```

Candidate/action-head eligibility provides target-specific credit. Oja remains
a normalization term, not the behavioral credit signal.

Learning batches are patch-gated and replay-protected. Invalid, duplicate,
out-of-range, or non-finite updates never commit.

## 10. Canonical sleep state machine

Sleep advances through the normal scheduler:

```text
Awake -> EnteringSleep -> Consolidating -> Waking -> Awake
                    \-> ForcedRecoverySleep -/
```

- Awake ticks evaluate fatigue, BrainATP, sleep pressure, and recovery signals.
- Entering sleep emits no new action.
- The transition into Consolidating submits exactly one GPU consolidation job
  for a unique cycle ID.
- Consolidation replays selected episodes through bounded replay eligibility,
  promotes bounded `H_fast` content into lifetime weights, prunes or grows
  within budgets, compacts sidecars, and prepares a double-buffered structural
  swap. Replay payloads are persisted and must measurably affect the staged
  consolidation result; replay metadata alone is not sufficient.
- Waking restores bounded homeostatic state before actions resume.

Cycle IDs and phase state are saved so load/retry cannot consolidate twice.
Interrupted sleep, save/load in every phase, automatic wake, and retained
post-wake behavior are required tests.

## 11. Episodic memory and topology

### 11.1 Episodic retrieval

The 16-element prefix encoder is replaced by a versioned, stratified feature
encoder that reserves dimensions for every supported sensory group, drives,
hormones, candidate action family, `Other` action ID, and target descriptor.
Queries are candidate-conditional:

```text
Q(state, candidate action family, action ID for `Other`, observed target features)
```

Retrieval returns candidate-local context for the next GPU perception frame: a
target latent may match the same tracked target across action families, while
an action-family value matches only the exact candidate family. Both are
consumed by that candidate's decoder path; target-outcome context is never
pooled into a frame-global recurrent bias that can leak onto unrelated
candidates. Retrieval does not add one scalar to every candidate and cannot
replay an action command.

The bank uses deterministic eviction and merge rules. Capacity saturation is a
normal degradation condition, never a terminal tick error.

### 11.2 Topology sidecar

The concept/topology ledger is an organism-local diagnostic and analysis
sidecar. It observes sealed patches and may contribute bounded curiosity
context only through explicit neural input channels in experiments that enable
it. Sidecar capacity or mutation failure cannot roll back an already sealed
world transaction or prevent other organisms from advancing.

Concept, edge, simplex, and gap capacities use deterministic merge, eviction,
or summary replacement. Exhaustion never aborts cognition. Persistent raw world
entity IDs are replaced by tracked-object/episodic bindings where identity must
survive beyond a tick.

## 12. Policy and failure semantics

### 12.1 Explicit policy selection

`HeuristicBaseline` contains the historical handcrafted behavior and is labelled
as such in UI, telemetry, saves, and reports. It shares world perception,
candidate enumeration, command execution, and experience logging, but not the
GPU neural decoder or learning claims.

`NeuralClosedLoopGpu` never calls the baseline to fill a missing neural result.

### 12.2 GPU failures

- Adapter or required feature unavailable: return `NeuralBackendUnavailable`.
- Device lost: stop learned actions, retain the last portable checkpoint, and
  require explicit runtime recovery.
- Pipeline or layout mismatch: reject before dispatch.
- Invalid candidate: suppress before upload and record a diagnostic.
- No candidates: enumerate one explicit idle candidate rather than fabricate a
  score.
- Non-finite GPU diagnostic: reject the staged tick, emit no action, and do not
  commit plastic state.
- Capacity pressure: deterministically truncate, merge, or evict according to
  the owning subsystem's policy.

No error path silently claims success, changes policy, or runs a second brain.

Admission accounting distinguishes logical committed slot bytes from
physically allocated class-bucket bytes, unused bucket capacity, shared backend
bytes, and peak in-flight/growth-swap bytes. Removing a brain reclaims logical
admission immediately; retained wgpu bucket allocation is reported honestly
rather than claimed as physical deallocation.

### 12.3 GPU-native validation

Removing CPU shadow execution does not remove validation. The GPU path uses:

- validated upload records and offsets;
- buffer bounds and canary regions;
- finite/range diagnostic counters;
- dispatch generation and completion receipts;
- deterministic seed and input replay on the same adapter;
- metamorphic tests such as lesioning, zeroing, input perturbation, and
  plasticity ablation;
- compact manual diagnostic snapshots outside the active loop;
- hardware integration tests on the real Vulkan adapter.

## 13. Save, replay, and migration

Portable saves record:

- genome and phenotype hash;
- capacity class;
- sensor and motor manifest versions;
- active sensor profile;
- lifetime and fast plastic state asset references;
- eligibility checkpoint policy;
- homeostasis and sleep phase/cycle ID;
- memory/topology summaries;
- GPU-required policy identity plus backend provenance; the requirement is
  derived from policy/save kind rather than serialized as a redundant boolean.

Legacy supported-tier saves without a phenotype are migrated by deterministically
compiling the saved genome and seed. A hash mismatch without a tested migration
is rejected. Large legacy tiers remain loadable for inspection/export but
cannot enter production neural mode until promoted.

Replay determinism is defined for the same phenotype hash, inputs, seed,
backend version, adapter class, and tolerance contract. Cross-vendor bitwise
identity is not claimed without evidence.

## 14. Removal and migration targets

The implementation removes or replaces:

- scored proposals in `BrainTickInput`;
- heuristic proposal construction as the default live bridge;
- neural advance before sensory gathering;
- empty default projection schemas;
- `CpuNeuralState` from production execution;
- duplicated projection ownership;
- `bias_proposals` candidate-invariant memory/topology bias;
- waking Oja updates that modify only inactive `h_shadow`;
- external harness-driven sleep progression;
- topology-capacity terminal errors;
- `cpu_shadow_parity`, `cpu_shadow_ms`, `cpu_shadow_checked`, and related
  control/claim fields;
- `AutoWithCpuFallback` for neural policy;
- reports that describe GPU scores as neural authority while they are fed back
  into heuristic proposal construction.

Renderer fallback policy is a separate concern and is not changed merely
because neural CPU fallback is removed.

## 15. Acceptance evidence

### 15.1 Closed causal loop

- Same candidates with different sensory input produce different GPU logits.
- Same sensory frame with lesioned weights changes the selected policy.
- Zeroing neural weights produces a documented loss of learned neural behavior.
- With `HeuristicBaseline` disabled, neural mode still selects and executes an
  action on the GPU.
- Same phenotype, seed, adapter, and inputs replay within the defined tolerance.
- Production neural runtime contains no CPU shadow dispatch or parity gate.

### 15.2 Genome and phenotype causality

- N512, N1024, and N2048 compile nonempty sparse phenotypes within global and
  per-route budgets.
- Mutating connectome, density, lobe allocation, alpha, sensor, motor, or
  development genes changes the phenotype hash and relevant allocation.
- Unsupported neutral evolvable fields are rejected or removed.

### 15.3 Learning causality

- A painful encounter increases avoidance of the matching target features on
  the next encounter before sleep.
- A rewarding encounter increases target-conditional approach/ingest value.
- Ablating the neuromodulator removes that learning effect.
- Unrelated candidates do not receive the same learned bias.

### 15.4 Sleep and bounded cognition

- Sleep entry, exactly-once consolidation, automatic wake, interruption, and
  no-action phases pass.
- Save/load passes in every sleep phase.
- Learned behavior remains after consolidation and wake.
- A 10,000-plus-tick cognition soak keeps memory, topology, candidates, and GPU
  buffers bounded without a terminal capacity error.

### 15.5 Grounding and profiles

- Every behavioral report names its sensor profile.
- Grounded profile uploads contain no semantic food/hazard/teacher class bits.
- Privileged and grounded results are reported separately.
- Activity-dependent BrainATP cost changes with active neural work.

### 15.6 GPU authority and readback

- The real hardware receipt names the selected Vulkan adapter and required GPU
  backend with no neural fallback.
- Action selection, fast plasticity, and sleep consolidation execute through
  WGSL pipelines.
- Active-loop readback is limited to the selected action and bounded counters.
- Device-unavailable tests return the typed unavailable state.
- Source and telemetry scans find no production `cpu_shadow` or neural
  auto-fallback contract.

## 16. Implementation slices

### Slice A: GPU causal core

Introduce the candidate/perception/phenotype contracts, compiler, GPU
structure-of-arrays buffers, sensory encoder, recurrent microsteps,
candidate-conditioned decoder, GPU winner selection, explicit policy selection,
and direct live-bridge cutover. Remove CPU shadow control and default heuristic
proposal scoring. Prove P0 acceptance on N512 and N1024, then N2048.

### Slice B: GPU causal learning and sleep

Add eligibility buffers, three-factor fast plasticity, immediate behavioral
effect, automatic sleep scheduling, GPU consolidation, save/load phase state,
and safe structural swaps. Prove P1 and sleep acceptance.

### Slice C: memory, topology, and grounding

Replace memory encoding and recall, make topology nonfatal, add both sensor
profiles, tracked-object bindings, candidate-conditional retrieval, and the
10,000-tick soak. Prove P2 and grounding acceptance.

### Slice D: scaling and cleanup

Enforce global/per-route budgets, activity-dependent ATP, memory ceilings,
populated phenotype benchmarks, tier promotion gates, legacy save migration,
documentation/ADR updates, and removal of superseded backend code and claims.
Prove P3 and final completion audit.

Each slice is integrated only after its own behavioral and architectural gates
pass. The persistent goal is complete only after all four slices and the final
requirement-by-requirement audit pass.

## 17. Documentation decision

This design-only commit does not yet amend the controlling architecture ADRs.
The first implementation-plan task will add an ADR that supersedes the old
scaffold-only restriction, establishes GPU-authoritative neural execution, and
removes live CPU shadow/fallback semantics. Deferring that ADR until written
spec review prevents the controlling decision log from getting ahead of the
reviewed design.
