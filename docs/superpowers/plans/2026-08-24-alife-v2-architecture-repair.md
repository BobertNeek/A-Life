# A-Life v2.0 architecture repair implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair the highest-impact production architecture violations against the controlling v2.0 `AOA-*` ledger without training or capability campaigns.

**Architecture:** Keep the organism as the canonical identity while preserving separate world, body, biochemical, developmental, cognitive, adapter, teacher, and semantic-prior ownership. Replace the fixed controller with a bounded sparse chemical graph, make brain-body coupling bidirectional through typed frames, and seal learning from measured before/after organism state rather than world-authored reward. Version every changed durable contract and fail closed on incompatible state.

**Tech stack:** Rust 2021, serde, Bevy-independent `alife_core` and `alife_world` contracts, the existing GPU-authoritative production runtime, WGSL unchanged unless compilation proves a receptor upload must cross it.

**Spec:** `docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`

## Global constraints

- The stable requirement registry is `docs/architecture/requirement_registry.csv`.
- LOCKED INVARIANT and LOCKED INTERFACE requirements control lower categories.
- Do not train, pretrain, fine-tune, evolve, school, or optimize a brain.
- Do not create learned weights, germline prior assets, or training receipts.
- Preserve GPU neural authority and score-free world candidate enumeration.
- Use RED-first focused tests. Run Cargo serially with `-j 1`.
- Verification is formatting, static boundary checks, targeted compilation, and focused causal tests only.
- Do not run corpora, ablations, population scaling, long GPU journeys, curriculum runs, or evolutionary experiments.
- Grade conservatively: STRUCTURAL is not CAUSAL, CAUSAL is not CAPABILITY-VALIDATED, and CAPABILITY-VALIDATED is not ROBUST-SCALED.

---

### Task 1: Sparse biochemical graph and typed brain coupling

**Requirements:** AOA-BIO-001 through AOA-BIO-023, AOA-BIO-028, AOA-GEN-003, AOA-GEN-004, AOA-PERF-001, AOA-PERF-003.

**Files:**

- Create: `crates/alife_core/src/biochemical_graph.rs`
- Modify: `crates/alife_core/src/biochemistry.rs`
- Modify: `crates/alife_core/src/chemistry.rs`
- Modify: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Modify: `crates/alife_core/src/version.rs`
- Test: `crates/alife_core/tests/biochemical_graph_v2.rs`

**Interfaces:**

- Produce a versioned, bounded `BiochemicalPhenotype` containing typed species, sparse reactions, emitters, receptors, and neuroemitters.
- Produce `BiochemicalGraphState`, `DriveFrame`, `NeuralReceptorFrame`, `NeuralEmissionFrame`, and deterministic biochemical work receipts.
- `BiochemistryState` owns graph state and derives compatibility drive/endocrine views at its authoritative tick.
- The neural phenotype projects receptor activations into local target classes. Biochemistry never emits finished threshold, attention, salience, motor-confidence, or learning-rate values.

- [ ] **Step 1: Write failing graph and coupling tests**

  Cover phenotype bounds, sparse work scaling, conservation, typed loci, species-dependent receptor effects, and a neural emission changing later chemistry without direct body mutation.

- [ ] **Step 2: Run the RED test**

  Run: `cargo test -p alife_core --test biochemical_graph_v2 -j 1`

  Expected: compile failure because the v2 graph and frames do not exist.

- [ ] **Step 3: Implement the bounded graph and migration phenotype**

  Add explicit schema versions and limits. Compile the existing founder chemistry loci into one valid reference graph, but make that graph the live authority rather than another inert representation.

- [ ] **Step 4: Remove `NeuralModulation` from biochemical authority**

  Replace it with `NeuralReceptorFrame`. Move all final neural effect calculations behind neural phenotype receptor profiles.

- [ ] **Step 5: Run focused GREEN tests**

  Run: `cargo test -p alife_core --test biochemical_graph_v2 -j 1`

### Task 2: Genetic and region ownership cleanup

**Requirements:** AOA-GEN-001 through AOA-GEN-004, AOA-BRAIN-001, AOA-BRAIN-003, AOA-REG-001 through AOA-REG-003, AOA-INV-007.

**Files:**

- Modify: `crates/alife_core/src/genome.rs`
- Modify: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_core/src/lobe.rs`
- Modify: `crates/alife_core/src/phenotype/inputs.rs`
- Modify: `crates/alife_core/src/phenotype/record.rs`
- Modify: `crates/alife_core/src/evidence_digest.rs`
- Test: `crates/alife_core/tests/v2_genetic_ownership.rs`

**Interfaces:**

- Remove independently authoritative endocrine constants and drive thresholds from `BrainGenome` construction inputs.
- Keep neural receptor references as typed biochemical species/receptor-class links only.
- Replace algorithm-named activation policies with generic input-coupled, recurrent, output-coupled, and dormant execution roles. Region names remain developmental anchors and routing metadata.

- [ ] **Step 1: Write failing ownership tests**

  Assert that changing chemistry changes the biochemical phenotype without changing duplicate brain-owned chemistry, and that no region label selects a cognitive algorithm.

- [ ] **Step 2: Run the RED test**

  Run: `cargo test -p alife_core --test v2_genetic_ownership -j 1`

- [ ] **Step 3: Migrate compiler inputs and digests**

  Remove duplicated fields from current construction and bump the affected genome/phenotype schema. Retain explicit legacy deserialization only where it can fail or transform deterministically.

- [ ] **Step 4: Run focused GREEN tests**

  Run: `cargo test -p alife_core --test v2_genetic_ownership -j 1`

### Task 3: Measured physiology and reward-free experience ABI

**Requirements:** AOA-INV-002, AOA-WORLD-003, AOA-BODY-005, AOA-BODY-006, AOA-BIO-018, AOA-BIO-024 through AOA-BIO-028, AOA-LEARN-002 through AOA-LEARN-010, AOA-AUTH-004, AOA-FAIL-002.

**Files:**

- Modify: `crates/alife_core/src/experience.rs`
- Modify: `crates/alife_core/src/learning.rs`
- Modify: `crates/alife_core/src/memory.rs`
- Modify: `crates/alife_core/src/memory/candidate_recall.rs`
- Modify: `crates/alife_core/src/packed_log.rs`
- Modify: `crates/alife_core/src/reference_brain.rs`
- Modify: `crates/alife_world/src/headless.rs`
- Test: `crates/alife_world/tests/v2_measured_experience.rs`

**Interfaces:**

- `BodyEventDelta` carries physical facts only.
- A new measured physiology record binds full before/after body and biochemical references, actual homeostatic change, and derived biological value.
- `PostActionOutcome` keeps physical result, biological value, prediction error, and social/teacher evidence distinct. It contains no world-authored reward.
- Learning derives receptor-local credit from eligibility, biological value, receptor signals, and prediction error.

- [ ] **Step 1: Write failing causal tests**

  Test that changed canonical biology cannot seal with a default delta, hazardous food can have nutrition plus aversive biological value, and the world cannot supply reward.

- [ ] **Step 2: Run the RED test**

  Run: `cargo test -p alife_world --test v2_measured_experience -j 1`

- [ ] **Step 3: Implement and version the experience transaction**

  Derive the physiology record from the exact registered `biology_before` and `biology_after` states. Update memory and learning to consume distinct fields.

- [ ] **Step 4: Add explicit legacy migration or rejection**

  Old experience payloads that contain host reward must not silently become v2 biological value. Migrate only when before/after evidence proves the value, otherwise return an explicit incompatible-state error.

- [ ] **Step 5: Run focused GREEN tests**

  Run: `cargo test -p alife_world --test v2_measured_experience -j 1`

### Task 4: Production organism and GPU-world causal loop

**Requirements:** AOA-AUTH-001 through AOA-AUTH-005, AOA-CTX-001, AOA-CTX-002, AOA-BIO-019 through AOA-BIO-026, AOA-INV-009, AOA-INV-011, AOA-PERSIST-001 through AOA-PERSIST-003.

**Files:**

- Modify: `crates/alife_world/src/organism.rs`
- Modify: `crates/alife_world/src/headless.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_runtime/src/session.rs` only if the typed frame crosses the shared session boundary
- Test: `crates/alife_world/tests/task_3_2a_atomic_action.rs`
- Test: focused unit tests in `crates/alife_game_app/src/gpu_live_runtime.rs`

**Interfaces:**

- The canonical organism record binds distinct body, biochemical, developmental, brain-owner, memory-owner, adapter, lifecycle, persistence, and presentation identities.
- Runtime caches are immutable tick-bound views and cannot advance canonical biology.
- Production sends the neural receptor frame into the decision context and returns a bounded neural emission frame to the registered world transaction.
- Action, chemistry, experience sealing, and cognitive work cost either commit together or leave the canonical record unchanged.

- [ ] **Step 1: Write failing authority and loop tests**

  Assert exact tick/identity binding, receptor influence on production decision input, neural emission influence on next chemistry, nonzero measured physiology when biology changed, and rollback after a late failure.

- [ ] **Step 2: Run the RED gates**

  Run: `cargo test -p alife_world --test task_3_2a_atomic_action -j 1`

  Run: `cargo test -p alife_game_app gpu_live_runtime::tests::production_v2_organism_loop --lib -j 1`

- [ ] **Step 3: Repair the registered transaction and runtime cache boundary**

  Thread the typed frames through the existing `WorldOrganismRecord` and GPU-authoritative tick. Do not create a shadow organism or CPU neural fallback.

- [ ] **Step 4: Run focused GREEN gates**

  Repeat the two RED commands.

### Task 5: Teacher and semantic-prior boundary repair

**Requirements:** AOA-INV-008, AOA-SLM-001 through AOA-SLM-007, AOA-TEACH-001 through AOA-TEACH-008, AOA-CULT-001, AOA-CULT-002, AOA-LEARN-010.

**Files:**

- Modify: `crates/alife_school/src/teacher.rs`
- Modify: `crates/alife_school/src/curriculum.rs`
- Modify: `crates/alife_school/src/verifier.rs`
- Modify: `crates/alife_semantic/src/semantic.rs`
- Modify: `crates/alife_semantic/src/providers.rs`
- Modify dependent app adapters only where compilation requires it
- Test: `crates/alife_school/tests/school_teacher_contracts.rs`
- Test: `crates/alife_semantic/tests/semantic_gaussian_adapter.rs`

**Interfaces:**

- Teacher cues are grounded speech, gesture, objects, demonstrations, or social feedback. No event or verifier calls a cue reward or writes neuromodulation.
- Semantic responses are bounded hypotheses with provenance. They cannot install authoritative concept IDs or memory entries.

- [ ] **Step 1: Write failing boundary tests**

  Assert that teacher contracts expose no reward-writing event and semantic responses cannot produce authoritative concept activation.

- [ ] **Step 2: Run RED tests**

  Run: `cargo test -p alife_school --test school_teacher_contracts -j 1`

  Run: `cargo test -p alife_semantic --test semantic_gaussian_adapter -j 1`

- [ ] **Step 3: Repair production schemas and callers**

  Replace reward wording and semantics with grounded observations and measured biological improvement checks. Route semantic suggestions through hypothesis/provenance fields only.

- [ ] **Step 4: Run focused GREEN tests**

  Repeat the two RED commands.

### Task 6: Consolidated verification and dated compliance report

**Requirements:** AOA-ADM-003, AOA-ADM-006, AOA-GRADE-001 through AOA-GRADE-010, AOA-MIG-001 through AOA-MIG-007.

**Files:**

- Create: `docs/architecture/compliance/2026-08-24-v2.0-source-repair-report.md`
- Create: `docs/architecture/compliance/2026-08-24-v2.0-source-repair-matrix.csv`
- Modify: `docs/ARCHITECTURE.md` only for current implementation facts changed by this pass

- [ ] **Step 1: Run formatting and static boundary checks**

  Run: `cargo fmt --all -- --check`

  Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`

  Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`

- [ ] **Step 2: Run targeted compilation and focused tests once**

  Run: `cargo check -p alife_core -p alife_world -p alife_runtime -p alife_semantic -p alife_school -p alife_game_app -j 1`

  Run only the focused tests named above. Do not start broad behavioral or accelerator campaigns.

- [ ] **Step 3: Perform one source-level v2.0 review**

  Trace every repaired requirement to exact production files and lines. Grade untouched requirements conservatively and record missing evidence as ABSENT, STRUCTURAL, PARTIAL, or CAUSAL.

- [ ] **Step 4: Write the dated report and matrix**

  For each repaired deficiency record `AOA requirement(s) -> defect -> repair -> production causal path -> remaining debt`. Include ABI/schema changes, rollback/migration limits, checks run, commit, and uncertainty.

- [ ] **Step 5: Review, stage, and commit only intended files**

  Inspect `git diff --check`, `git status --short`, and the staged file list before committing. Do not merge or start gameplay work.

### Task 7: Close the v2 founder-region and neuromodulatory ABI debt

**Requirements:** AOA-REG-001..010, AOA-BIO-019..023, AOA-BIO-027..028, AOA-LEARN-002..010, AOA-MIG-002..005.

**Files:**

- Modify: `crates/alife_core/src/lobe.rs`
- Modify: `crates/alife_core/src/learning.rs`
- Modify: `crates/alife_core/src/genome.rs`
- Modify: `crates/alife_core/src/phenotype/learning.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_learning.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_buffers/abi.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_buffers/upload.rs`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_replay_learning.wgsl`
- Test: `crates/alife_core/tests/v2_architecture_completion.rs`
- Test: focused GPU ABI tests in `crates/alife_gpu_backend/src`

**Interfaces:**

- `LobeKind` contains exactly the nine permanent founder homologues. `LegacyLobeKindV1` and a deterministic range migration preserve old neuron and weight indices without retaining the 17-region model as production authority.
- `NeuromodulatoryFrame` keeps bounded, inspectable lanes. `PlasticityReceptorProfile` projects those lanes locally. The GPU upload and WGSL consume the lane vector and receptor vector directly and contain no universal modulator value or sign.
- Production derives `NeuralReceptorEffects` from the complete tick-bound biochemical receptor frame. Those effects alter the GPU-bound cognitive frame and the plasticity lanes.

- [x] Write the focused contract tests and run them RED.
- [x] Implement the nine-homologue ABI and deterministic v1 range migration.
- [x] Implement the lane/receptor CPU and GPU ABI, then update waking and replay WGSL.
- [x] Route the authoritative biochemical receptor frame through production cognition and learning.
- [x] Run the focused tests GREEN.

### Task 8: Heritable chemistry, organ, embodiment, and organism state graph

**Requirements:** AOA-GEN-001..010, AOA-BIO-003..016, AOA-BODY-001..010, AOA-EMB-001..006, AOA-AUTH-001..005, AOA-INV-003, AOA-INV-009, AOA-INV-011.

**Files:**

- Modify: `crates/alife_core/src/biochemical_graph.rs`
- Modify: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_core/src/biochemistry.rs`
- Create: `crates/alife_core/src/embodiment.rs`
- Modify: `crates/alife_world/src/organism.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Test: `crates/alife_core/tests/v2_architecture_completion.rs`
- Test: `crates/alife_world/tests/v2_organism_state_graph.rs`

**Interfaces:**

- `BiochemicalGraphChromosome` owns bounded heritable species, reaction, emitter, receptor, and neuroemitter genes. Expression compiles only those genes. Recombination and validation operate on genes, never live concentrations.
- `BodyState` owns typed organ physiology. Compatibility scalars become checked projections of organs, not a second authority.
- `EmbodimentState` persists typed sensor/effector calibration and body-schema state independently of cognitive memory.
- `OrganismSubsystemStateGraph` binds concrete body/chemistry, brain, memory, and embodiment revisions to one organism and causal sequence. Runtime caches must validate and advance those revisions through the world record.

- [x] Write the focused contract tests and run them RED.
- [x] Move graph topology into the chromosome and integrate founder expression/recombination.
- [x] Add typed organs and independent embodiment state.
- [x] Replace owner-only authority with the validated subsystem state graph and production revision commits.
- [x] Run the focused tests GREEN.

### Task 9: Deterministic migrations and grounded teacher authority

**Requirements:** AOA-MIG-001..007, AOA-PERSIST-001..004, AOA-TEACH-001..008, AOA-LEARN-010.

**Files:**

- Create: `crates/alife_core/src/migration_v2.rs`
- Modify: `crates/alife_core/src/version.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Modify: `crates/alife_school/src/teacher.rs`
- Modify: `crates/alife_school/src/runner.rs`
- Modify: `crates/alife_school/src/curriculum.rs`
- Test: `crates/alife_core/tests/v2_schema_migrations.rs`
- Test: `crates/alife_school/tests/school_teacher_contracts.rs`

**Interfaces:**

- Every changed durable contract has an explicit source-version decoder, deterministic transformer, migration receipt, and unsupported-version error. Deserialization never resets or fills missing authoritative state silently.
- `TeacherPlanner` returns only a bounded `LessonPlan`. `EmbodiedTeacherActor` is the sole converter from approved teacher acts to ordinary world/perception events. The runner accepts actor output, never planner output.

- [x] Write deterministic migration and teacher-bypass tests and run them RED.
- [x] Implement version transitions and fail-closed loaders.
- [x] Split planner diagnostics from embodied acts and route every school runner path through the actor.
- [x] Run focused tests GREEN.

### Task 10: Repeat the consolidated review and publish Phase 2

- [x] Run one economical static/compile/focused-test pass.
- [x] Trace the repaired production paths against every affected AOA requirement.
- [x] Update `docs/architecture/ALife_v2.0_Compliance_2026-08-24.md` and its CSV matrix with exact grades and remaining violations.
- [x] Commit only intended files, compare against `origin/main`, push `aoa-v2-architecture-repair-20260824`, and stop before Phase 3.
