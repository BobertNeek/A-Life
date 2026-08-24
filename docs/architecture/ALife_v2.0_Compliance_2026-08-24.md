# A-Life v2.0 Source Compliance Report — 2026-08-24

Architecture version: `v2.0` controlling

Implementation commit: `6bb1d815`

Review date: `2026-08-24`

Method: one consolidated source-level review of production causal paths, supported by targeted Rust compilation and focused causal tests. No training, pretraining, fine-tuning, evolution, curriculum run, behavioural corpus, ablation, scaling run, or long GPU journey was performed.

Environment: Windows, Rust workspace, serialized `-j 1` checks.

## Executive assessment

This pass repairs the most direct ownership and outcome-loop violations. The production GPU gameplay seal now sends typed brain summaries into organism chemistry, receives an authoritative biochemical transition through the world transaction, records actual before/after physiology, and derives learning evidence without a world-authored reward. Chemistry now has a bounded sparse active graph and no longer stores a finished brain-policy modulation object. The portable brain genome no longer duplicates endocrine constants or drive thresholds.

The repaired slice is **CAUSAL**, not capability-validated. No intelligence, learning quality, robustness, scale, or performance claim is made. The repository is not fully v2.0 compliant. The most important remaining violations are the legacy founder-region model, scalar GPU third-factor projection, incomplete evolvable biochemical genotype, incomplete chemistry-to-neural runtime projection, incomplete canonical organism ownership, legacy/reduced outcome paths, and missing deterministic migrations for the new schemas.

## Repaired deficiencies

### Sparse organism-owned biochemistry

`AOA-BIO-001, AOA-BIO-003, AOA-BIO-005..009, AOA-BIO-011..015, AOA-TIME-003..004, AOA-PERF-001` -> fixed named snapshots and handwritten deltas acted as the biochemical mechanism -> added a bounded sparse active graph with typed species, compartments, sparse conserved reactions, emitters, receptors, neuroemitters, causal ticks, bounded work receipts, and derived drive/endocrine views -> `CreatureGenome::express` builds `ChemistryPhenotype.biochemical`; `BiochemistryState` owns `BiochemicalGraphState`; every biology advance executes the graph and derives `HomeostaticSnapshot` (`crates/alife_core/src/biochemical_graph.rs:390`, `:636`; `crates/alife_core/src/biochemistry.rs:386`) -> founder construction still compiles a fixed reference graph from fixed chromosome loci; graph topology and receptor expression are not yet directly heritable structural genes.

Grade: **CAUSAL** for the reference phenotype and production biology transaction. Not capability-validated.

### Brain ↔ biochemistry interfaces

`AOA-BIO-019..023, AOA-BRAIN-005` -> chemistry published a finished `NeuralModulation` policy and production lacked a typed brain-to-chemistry transaction -> removed stored `NeuralModulation`; added versioned `NeuralEmissionFrame` and `NeuralReceptorFrame`; added targeted receptor classes and typed neuroemitters -> the GPU decision seal creates bounded arousal, motor-commitment, and executive emissions, and the atomic world motor transaction advances the authoritative organism chemistry with them (`crates/alife_game_app/src/gpu_live_runtime.rs:2608`; `crates/alife_world/src/headless.rs:1956`; `crates/alife_world/src/organism.rs:472`) -> receptor frames exist and are tested, but production GPU neural parameters still do not consume the full targeted receptor vector. Prediction residual is not yet returned to chemistry after the outcome in the same causal sequence.

Grade: brain-to-chemistry **CAUSAL**; chemistry-to-targeted-neural use **STRUCTURAL**.

### Physical outcome, physiology, prediction, and learning evidence

`AOA-INV-002, AOA-AUTH-004..005, AOA-WORLD-003, AOA-BODY-002, AOA-BODY-005..006, AOA-BIO-024..026, AOA-LEARN-002..004, AOA-LEARN-010` -> body events carried `reward_outcome`; the live GPU seal substituted zero homeostatic change and used world reward as learning evidence -> removed reward from `BodyEventDelta`; made world outcome reward zero; added `MeasuredPhysiologyTransition` that validates canonical before/after organism state and derives homeostatic, energy, and pain changes; made v1.1 sealed outcomes require it; changed learning to require measured physiology and use prediction residual separately (`crates/alife_core/src/experience.rs:1130`, `:1327`; `crates/alife_core/src/learning.rs:177`; `crates/alife_game_app/src/gpu_live_runtime.rs:2658`) -> legacy `PostActionOutcome::new` and `ReferenceOutcomeObservation` retain a reward-valence field for reduced/older ABIs, although the repaired production path forces zero. Some reduced evaluators still use host-authored outcome profiles and do not share the full production step.

Grade: repaired GPU gameplay seal **CAUSAL**. No learning capability validation was run.

### Genetics and cognitive ownership

`AOA-INV-004, AOA-GEN-001, AOA-GEN-003..004, AOA-MEM-005, AOA-BRAIN-008` -> `BrainGenome` duplicated endocrine baselines and drive thresholds expressed by chemistry -> removed those fields, their validation, their canonical phenotype digest inputs, and the compiler copy step; chemistry remains in `ChemistryPhenotype.biochemical` (`crates/alife_core/src/genome.rs:13`; `crates/alife_core/src/evolutionary_genetics.rs:546`; `crates/alife_core/src/phenotype/inputs.rs:304`) -> deprecated endocrine/drive gene type definitions remain for migration vocabulary; a deterministic old-genome migration is not implemented.

Grade: **CAUSAL** for current phenotype compilation; migration is **NOT IMPLEMENTED**.

### Region metadata versus handwritten algorithms

`AOA-BRAIN-001, AOA-BRAIN-003, AOA-REG-002` -> lobe names selected handwritten policies such as episodic recall, motor competition, and homeostatic control -> reduced `ActivationPolicy` to generic `InputCoupled`, `Recurrent`, `OutputCoupled`, and `Disabled` execution roles (`crates/alife_core/src/lobe.rs:254`) -> the legacy 17-region taxonomy and allocations remain and do not implement v2.0's nine founder homologues or derived-region contracts.

Grade: **STRUCTURAL** only.

### Canonical organism authority

`AOA-INV-003, AOA-INV-011, AOA-AUTH-001..003` -> `WorldOrganismRecord` bound genome, phenotype, biology, and lifecycle but did not declare brain, memory, and embodiment ownership -> added and validated `OrganismAuthorityBinding`, tying organism, brain owner, memory owner, and embodiment entity (`crates/alife_world/src/organism.rs:139`) -> the record still stores owner identities rather than the complete cognitive/memory persistence dependency graph. `ResidentCognition` retains tick-bound derived homeostasis/development caches outside the record.

Grade: **STRUCTURAL**.

### Teacher and semantic-prior boundaries

`AOA-INV-001, AOA-INV-006, AOA-INV-008, AOA-SLM-002..004, AOA-TEACH-003..005` -> teacher APIs called grounded social cues “visible reward/punishment,” lesson verification read reward, and semantic-provider concept bindings directly selected learner concept cells -> renamed teacher cues to social approval/disapproval; lesson verification now checks measured biological improvement; external semantic bindings become opaque compressed hypotheses and never populate authoritative concept salience (`crates/alife_school/src/teacher.rs:22`; `crates/alife_school/src/verifier.rs:54`; `crates/alife_semantic/src/semantic.rs:58`) -> planner and embodied teacher authority are not fully separated; semantic hypothesis provenance and learner accept/reject state remain incomplete.

Grade: **STRUCTURAL**, with focused causal conversion tests. Not capability-validated.

## State and schema map

| Contract | Previous | Current | Material change | Migration state |
|---|---:|---:|---|---|
| Chemistry | 1 | 2 | Sparse graph state, typed emitters/receptors, work receipt | Explicit incompatibility; deterministic V1 migration missing |
| Creature genome | 1 | 2 | Biochemical phenotype added; duplicated brain chemistry removed | Explicit incompatibility; migration missing |
| Brain genome registry | 1 | 2 | Endocrine constants and drive thresholds removed | Migration missing |
| Phenotype | 1 | 2 | Biochemical graph is expressed state | Migration missing |
| Experience | 3 / v1.1=4 | 4 / v1.1=5 | Measured physiology required for v1.1 sealed outcomes | Legacy ABI accepted only where explicitly constructed; save migration missing |
| Learning | 1 | 2 | `prediction_residual` replaces reward-prediction-error conflation; measured physiology required | Wire alias reads the old field name; semantic migration receipt missing |
| Teacher school | 1 | 2 | Social cues and biological verifier replace reward terms | Migration missing |
| Save | 1 | 2 | New authoritative chemistry and organism authority fields | Old exact saves fail explicitly; transformer and receipt missing |
| GPU learning/checkpoint ABI | source layout changed | v2 source layout | Field renamed to `prediction_residual` | Binary compatibility not validated |

The version registry changes are at `crates/alife_core/src/version.rs:61`. `CREATURE_GENOME_SCHEMA_VERSION` is 2 at `crates/alife_core/src/evolutionary_genetics.rs:15`.

## Focused verification

- `cargo check -p alife_game_app -p alife_school -p alife_semantic -j 1` — PASS.
- `cargo test -p alife_core --test biochemical_graph_v2 --test genetic_cognitive_ownership_v2 --test measured_physiology_v2 -j 1` — PASS, 7 tests.
- `cargo test -p alife_world --lib factorized_bundle_causally_couples_neural_emission_into_organism_chemistry -j 1` — PASS, 1 production transaction test.
- `cargo test -p alife_semantic --features gaussian-adapter --test semantic_gaussian_adapter semantic_context_keeps_external_bindings_opaque_and_caps_codes -j 1` — PASS, 1 test.
- `cargo test -p alife_school --test school_teacher_contracts teacher_channel_contract_only_allows_perception_inputs -j 1` — PASS, 1 test.
- `git diff --check` — PASS before the implementation commit.

No EI corpus, behavioural experiment, ablation, scale run, evolution, curriculum, teacher training, pretraining, fine-tuning, or brain optimization ran.

## Consolidated remaining violations and debt

1. **P0 — AOA-REG-001, AOA-REG-003..010:** production still uses the legacy 17-name lobe taxonomy. It lacks the nine founder-homologue ABI, floor-plus-share allocation, and derived-region provenance/recombination model.
2. **P0 — AOA-BIO-021, AOA-BIO-028, AOA-LEARN-007:** the GPU learning backend still collapses distinct evidence lanes into `NeuromodulatorSample.value` and projection `modulator_sign`. Targeted receptor-vector projection is not causal.
3. **P0 — AOA-GEN-002..003, AOA-GEN-006, AOA-BIO-010, AOA-MIG-004:** chemistry graph topology, emitter/receptor structure, and species selection are not directly genetically encoded and evolvable. The new graph currently compiles the reference founder's fixed loci.
4. **P0 — AOA-BIO-021..023:** chemistry-to-neural receptor frames are not consumed by the production GPU neural phenotype. Brain-to-chemistry is causal; the return path is incomplete.
5. **P0 — AOA-PERSIST-001..004, AOA-MIG-002..003, AOA-MIG-006:** schema numbers were advanced, but deterministic migrations, transformation receipts, exact-save round trips, and rollback tests are missing.
6. **P1 — AOA-AUTH-001..003, AOA-INV-009:** canonical organism authority binds owner IDs but not the complete live brain/memory/adapter objects. Reduced evaluators and reference paths do not all execute the same semantic organism step.
7. **P1 — AOA-BODY-001, AOA-BODY-004, AOA-BODY-009:** physiology remains a small scalar `BodyState`; typed organs, tissues, local dysfunction, actuator limits, and proprioceptive state are absent.
8. **P1 — AOA-EMB-001..006:** deterministic control is not fully separated into a replaceable embodiment adapter with independently persisted learned body-schema state.
9. **P1 — AOA-TEACH-001..008:** read-only planner and embodied teacher actor remain incompletely separated; teacher-off competence was not tested.
10. **P1 — AOA-SLM-001..007:** opaque hypotheses no longer inject concept IDs, but provenance, budgets, caching, learner acceptance/rejection, and service-off competence remain incomplete.
11. **P1 — AOA-MEM-005, AOA-INV-007:** no direct ordinary reproduction leak was found in this pass, but germline assimilation sanitization and memory-leak probes were not re-audited or run.
12. **P1 — AOA-BIO-013..014:** graph species record compartments and decay, but reactions do not yet model compartment transfer or broad natural timescale families.
13. **P2 — AOA-BIO-024:** legacy/reduced outcome structures still expose `reward_valence`. Production v2 sets it to zero and learning ignores it, but removal or explicit legacy isolation remains.

## Conservative grades

| Family | Grade | Basis |
|---|---|---|
| Sparse biochemical substrate | CAUSAL | Production organism biology advances the graph |
| Brain-to-biochemistry | CAUSAL | GPU seal passes a neural emission through the atomic world transaction |
| Chemistry-to-neural targeting | STRUCTURAL | Frame exists and is tested; GPU consumption missing |
| Measured physiology learning loop | CAUSAL | Production seal uses authoritative before/after state |
| Reward-free world/body event | CAUSAL for GPU gameplay path | Body event has no reward field; legacy outcome ABIs remain |
| Genetic chemistry ownership | CAUSAL | Current compiler has one chemistry owner |
| Region architecture | STRUCTURAL | Algorithmic policies removed; nine-homologue migration missing |
| Organism authority | STRUCTURAL | Owner binding exists; complete state graph missing |
| Semantic-prior boundary | STRUCTURAL | Direct concept activation removed; learner grounding incomplete |
| Teacher boundary | STRUCTURAL | Reward terminology/control removed; actor/planner separation incomplete |
| Capability validation | NOT RUN | Prohibited in this pass |
| Robust scaling | NOT RUN | Prohibited in this pass |

No family is graded **CAPABILITY-VALIDATED** or **ROBUST-SCALED**.

## Rollback

The implementation is isolated in branch `aoa-v2-architecture-repair-20260824`. Reverting implementation commit `6bb1d815` restores the previous schemas and causal paths. Because save/schema migrations are not implemented, do not publish V2 save assets from this branch as backward-compatible artifacts.

## Evidence matrix

The machine-readable matrix is `docs/architecture/ALife_v2.0_Compliance_2026-08-24.csv`.
