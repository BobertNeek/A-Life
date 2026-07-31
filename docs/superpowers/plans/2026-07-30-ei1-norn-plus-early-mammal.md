# Era 1 Norn-Plus Early Mammal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans with superpowers:test-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the complete N2048 Era 1 evolutionary program and honest promotion evidence for Norn-plus early-mammal cognition without entering Era 2 or scaling brain class.

**Architecture:** Extend the merged EI0 genome, habitat, archive, evaluation, and shared GPU session. `alife_core` owns versioned Era 1 evidence contracts, `alife_world` owns deterministic unscored trial worlds, `alife_training` owns GPU-authoritative trial execution and ablations, and `alife_tools` owns evolution orchestration and the promotion artifact. No layer may select actions except the production WGSL brain; the world remains authoritative for legality and outcomes.

**Tech Stack:** Rust 2021, wgpu/Vulkan, WGSL, serde JSON, BLAKE3, `alife_core`, `alife_world`, `alife_runtime`, `alife_gpu_backend`, `alife_training`, `alife_school`, `alife_archive`, and `alife_tools`.

## Global Constraints

- Source of truth: `docs/master_spec.md`, ADR-024 through ADR-030, `docs/schooling_and_teacher_architecture.md`, and the approved evolutionary-intelligence selection design.
- Preserve the promoted GPU-authoritative N2048 foundation. Do not add a CPU policy shadow, parity gate, fallback, or alternate scorer.
- World snapshots produce unscored candidates. Selection never injects actions, answers, targets, scores, or rewards.
- Ordinary births inherit DNA and foundation identity only. Lifetime weights, memories, learned vocabulary, eligibility, and transient state start empty.
- Missing or invalid evidence remains `Unknown` or blocks promotion. It never becomes zero, inferred success, or a relabelled heuristic fixture.
- Bind every receipt to seed, organism, genome, parents, lineage, generation, phenotype, foundation, sensor profile, world digest, assistance, adapter, backend, source commit, and source tree.
- Preserve the EI0 evidence lock. Do not edit `alife_training/src/active_battery.rs`; Era 1 uses a separate driver and first validates the committed EI0 report.
- Run Cargo and GPU jobs serially on the RTX 3050. A wrapper timeout is not a failed test; inspect the existing process before retrying.
- Exclude packs, coordinated hunting, complex tools, advanced schooling, possession, open-ended language, Era 2, N4096, and all brain-class changes.
- Brain scaling remains unauthorized. The final report may measure an N2048 plateau but may only recommend a later review.

---

### Task 1: Canonical Era 1 Evidence Contract

**Files:**

- Create: `crates/alife_core/src/era1_evaluation.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Test: `crates/alife_core/tests/era1_evaluation.rs`

**Interfaces:**

- Produces: `Era1Ability`, `Era1Control`, `Era1EvidencePartition`, `Era1TrialIdentity`, `Era1TrialReceipt`, `Era1MatchedComparison`, and `Era1PlateauWindow`.
- Consumes: `MetricReading`, `PolicyBackend`, `BrainClassId`, `GenomeId`, `LineageId`, `OrganismId`, `PhenotypeHash`, and canonical four-word digests.
- Rule: `Era1Ability::ALL` contains `FlexibleForaging`, `HazardAvoidance`, `SpatialMemory`, `DelayedChoice`, `RewardReversal`, `ObjectTransfer`, `MultiStepProblem`, `IndividualRecognition`, `Imitation`, `GroundedLanguage`, and `PostSleepRetention`.

- [ ] **Step 1: Write RED contract tests.** Prove an exact complete GPU receipt validates; zero IDs/digests, non-N2048 class, heuristic backend, assistance in hidden evidence, malformed parent/generation provenance, duplicate cells, and fake measured exposure fail; absent cells remain `MetricReading::Unknown`.
- [ ] **Step 2: Run RED.**

```powershell
cargo test -p alife_core --test era1_evaluation -j 1
```

Expected: compile failure because the Era 1 types do not exist.

- [ ] **Step 3: Implement minimal versioned contracts.** Use fixed enums and bounded vectors. Validation checks identity and completeness but never calculates neural behavior.
- [ ] **Step 4: Run GREEN plus existing life-evaluation tests.**

```powershell
cargo test -p alife_core --test era1_evaluation --test life_evaluation -j 1
```

Expected: all tests pass, including literal `Unknown` preservation.

- [ ] **Step 5: Commit.**

```powershell
git add crates/alife_core/src/era1_evaluation.rs crates/alife_core/src/lib.rs crates/alife_core/tests/era1_evaluation.rs
git commit -m "feat(core): define Era 1 evidence contracts"
```

### Task 2: Deterministic Early-Mammal Trial Worlds

**Files:**

- Create: `crates/alife_world/src/era1_trials.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Test: `crates/alife_world/tests/era1_trials.rs`

**Interfaces:**

- Produces: `Era1WorldFamily`, `Era1TrialPhase`, `Era1TrialManifest`, `Era1WorldTransition`, and `build_era1_trial_world(&Era1TrialManifest) -> Result<HeadlessWorld, ScaffoldContractError>`.
- Consumes: `HeadlessScenarioBuilder`, stable tracked objects, `HeadlessWorldSignatureDigest`, spatial speech, and ordinary `HeadlessWorld` action legality/outcomes.
- World families: foraging/hazard maze, delayed location, reward reversal, transformed objects/layout, two-step access problem, familiar/novel individual, peer demonstration, and grounded vocabulary.

- [ ] **Step 1: Write RED deterministic-world tests.** Require byte-stable manifests and world digests for equal seeds, changed digests for held-out transforms, no semantic truth in grounded slots, no scored candidates, phase transitions at exact ticks, and distinct familiar/novel tracked individuals.
- [ ] **Step 2: Run RED.**

```powershell
cargo test -p alife_world --test era1_trials -j 1
```

- [ ] **Step 3: Implement the smallest real worlds.** Reuse `HeadlessWorld` and ordinary editor transitions. Do not add a task-answer field or a scripted action path.
- [ ] **Step 4: Run GREEN with headless and speech boundaries.**

```powershell
cargo test -p alife_world --test era1_trials --test headless_world_harness --test spatial_speech -j 1
```

- [ ] **Step 5: Commit.**

```powershell
git add crates/alife_world/src/era1_trials.rs crates/alife_world/src/lib.rs crates/alife_world/tests/era1_trials.rs
git commit -m "feat(world): add deterministic Era 1 trials"
```

### Task 3: GPU Trial Driver and Causal Ablations

**Files:**

- Create: `crates/alife_training/src/era1_trials.rs`
- Modify: `crates/alife_training/src/lib.rs`
- Test: `crates/alife_training/tests/era1_trials.rs`

**Interfaces:**

- Produces: `Era1TrialRunner::new_required`, `Era1TrialRunner::run`, and complete `Era1TrialReceipt` values.
- Consumes: exact expressed `CreatureGenome`, built-in N2048 foundation, `GpuAuthoritativeSession`, `MemorySidecarState`, `recall_frame`, `tick_memory_batch`, sealed world outcomes, GPU sleep consolidation, and Task 2 manifests.
- `Era1Control::PlasticityDisabled` discards the exact pending eligibility transaction after sealing; intact trials apply it.
- `MemoryDisabled` finalizes an empty candidate-memory bank and never observes patches into it; intact trials recall and observe the organism-owned bank.
- `SleepDisabled` skips the matched consolidation boundary; intact trials consolidate once and verify post-wake behavior.
- `SocialDisabled` removes peer perception/demonstration while leaving the subject, physical objects, tick budget, and neural runtime unchanged.

- [ ] **Step 1: Write RED causal-loop tests.** Require exact perception/world/pending-eligibility binding, world execution before learning, memory context on every GPU dispatch, sealed-patch memory updates, and typed ablation receipts. Reject mismatched worlds, genomes, phenotypes, pending transactions, and cross-organism memory.
- [ ] **Step 2: Run RED with the GPU-required test filter.**

```powershell
cargo test -p alife_training --test era1_trials causal_ --features gpu-tests -j 1
```

- [ ] **Step 3: Implement one shared trial loop.** Keep condition differences inside the four explicit ablation branches. Do not fork or reimplement candidate scoring.
- [ ] **Step 4: Run GREEN on the RTX 3050/Vulkan adapter.** Require `NeuralClosedLoopGpu`, matching dispatch/outcome counts, at least one intact learning receipt, and one intact sleep commit.
- [ ] **Step 5: Commit.**

```powershell
git add crates/alife_training/src/era1_trials.rs crates/alife_training/src/lib.rs crates/alife_training/tests/era1_trials.rs
git commit -m "feat(training): run causal Era 1 GPU trials"
```

### Task 4: Learning, Transfer, Social, and Grounded-Language Episodes

**Files:**

- Modify: `crates/alife_training/src/era1_trials.rs`
- Modify: `crates/alife_core/src/language.rs`
- Modify: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_school/src/language_nursery.rs`
- Test: `crates/alife_training/tests/era1_learning.rs`
- Test: `crates/alife_core/tests/grounded_language_contracts.rs`
- Test: `crates/alife_core/tests/evolutionary_genetics.rs`
- Test: `crates/alife_school/tests/language_nursery.rs`

**Interfaces:**

- Produces: acquisition, reversal, delayed, held-out transfer, post-sleep, social-transfer, and reproduced-offspring partitions for all eleven abilities; `UtteranceGroundingReceiptV2`; and a nonzero bounded Era 1 starter-token table.
- Consumes: the same Task 3 brain handle across acquisition and test phases, `LanguageNursery`, spatial teacher/peer utterances, `LanguageGroundingLedger`, GPU-selected `Vocalize`, and raw token receipts.
- Starter vocabulary is read only from expressed `CreatureGenome.predisposition.starter_vocabulary`; token zero is never inherited or heard. Acquired bindings require one exact utterance, speaker, tracked object or agent, selected action, target, sequence, tick, and successful sealed outcome.

- [ ] **Step 1: Write RED behavioral tests.** Require improvement from early acquisition to late acquisition, correct reversal after a contingency swap, delayed choice after cue removal, transfer to unseen material/layout, remembered individuals across changed positions, imitation after a peer demonstration, learned word grounding with translation/SLM off, and retention after automatic sleep.
- [ ] **Step 2: Add hostile tests.** Token relabelling, mere co-occurrence, wrong speaker/target, replay, novel peer identity, memory reset, route/condition ablations, copied lifetime state, token zero, or host-authored speech must not pass.
- [ ] **Step 3: Run RED.**

```powershell
cargo test -p alife_training --test era1_learning --features gpu-tests -j 1
cargo test -p alife_school --test language_nursery -j 1
```

- [ ] **Step 4: Implement the minimal phase protocols.** Extend nursery records only where exact exposure/demonstration/sealed-outcome provenance is missing. Do not add advanced curriculum or free-form language.
- [ ] **Step 5: Run GREEN and commit.**

```powershell
git add crates/alife_training/src/era1_trials.rs crates/alife_training/tests/era1_learning.rs crates/alife_core/src/language.rs crates/alife_core/src/evolutionary_genetics.rs crates/alife_core/tests/grounded_language_contracts.rs crates/alife_core/tests/evolutionary_genetics.rs crates/alife_school/src/language_nursery.rs crates/alife_school/tests/language_nursery.rs
git commit -m "feat(training): prove Norn-plus adaptive learning"
```

### Task 5: Multi-Generation Evolution Program

**Files:**

- Create: `crates/alife_tools/src/era1_evolution.rs`
- Modify: `crates/alife_tools/src/lib.rs`
- Modify: `crates/alife_tools/Cargo.toml`
- Test: `crates/alife_tools/tests/era1_evolution.rs`

**Interfaces:**

- Produces: `Era1EvolutionConfig`, `Era1GenerationReceipt`, `Era1LineageReceipt`, and `run_era1_evolution`.
- Consumes: EI0 `CreatureGenome::reproduce/express`, `run_managed_selection`, Wild/Managed habitat permissions, Task 3/4 trials, `LineageLibrary`, and portable composite saves.
- Bounded default matrix: four lineages, three evaluation seeds, two held-out world transforms, intact plus four matched controls, and two ordinary-birth generations. GPU trials run serially and cache only digest-identical immutable inputs.

- [ ] **Step 1: Write RED evolution tests.** Require `validate_committed_ei0_exit_gate_report` first, distinct viable lineages, exact seeded reproduction, preserved wild reservoir, managed cognitive introgression rules, archived conceptions/births/deaths, complete trial partitions, and empty acquired state in every child.
- [ ] **Step 2: Add selection-shortcut negatives.** Reject fixed-answer exposure, assistance, missing controls, reused hidden worlds, parent-pair fabrication, low-survival-exception pairing, archive tampering, and objectives derived from `Unknown`.
- [ ] **Step 3: Run RED.**

```powershell
cargo test -p alife_tools --test era1_evolution -j 1
```

- [ ] **Step 4: Implement the bounded program.** Selection consumes only completed trial receipts and ecological life statistics. It never writes actions, reward, memory, language bindings, or weights.
- [ ] **Step 5: Run the smallest real two-generation GPU slice, inspect costs, and commit.**

```powershell
git add crates/alife_tools/src/era1_evolution.rs crates/alife_tools/src/lib.rs crates/alife_tools/Cargo.toml crates/alife_tools/tests/era1_evolution.rs
git commit -m "feat(tools): run bounded Era 1 evolution"
```

### Task 6: Honest Promotion, Ablation, and Plateau Gate

**Files:**

- Create: `crates/alife_tools/src/era1_promotion.rs`
- Create: `crates/alife_tools/src/bin/era1_promotion.rs`
- Modify: `crates/alife_tools/src/lib.rs`
- Test: `crates/alife_tools/tests/era1_promotion.rs`

**Interfaces:**

- Produces: `Era1PromotionReport`, `Era1PromotionVerdict`, `Era1ControlComparison`, `Era1HardwareCost`, and `Era1PlateauAssessment`.
- Consumes: Task 5 receipts and no raw neural state.
- Promotion requires every ability measured in intact held-out descendants, every seed/lineage/world/reproduction grouping positive, and an aggregate intact margin of at least 5 percentage points over each relevant disabled control. A comparison with fewer than twelve matched cells is `Unknown` and blocks promotion.
- Plateau requires three consecutive generation windows whose intact held-out aggregate improves by less than one percentage point while ecological stability and diversity do not regress. It records review eligibility only; this task cannot change brain class.

- [ ] **Step 1: Write RED report tests.** Hand-check literal matrices for PASS, `Unknown`, regression, missing cell, hostile exposure, assistance, control contamination, source mismatch, and false plateau cases.
- [ ] **Step 2: Run RED.**

```powershell
cargo test -p alife_tools --test era1_promotion -j 1
```

- [ ] **Step 3: Implement deterministic grouping and verdict derivation.** Report all subgroup scores and exact hardware costs. Never average away a failing seed, lineage, held-out family, or descendant cohort.
- [ ] **Step 4: Run GREEN and commit.**

```powershell
git add crates/alife_tools/src/era1_promotion.rs crates/alife_tools/src/bin/era1_promotion.rs crates/alife_tools/src/lib.rs crates/alife_tools/tests/era1_promotion.rs
git commit -m "feat(tools): derive the Era 1 promotion gate"
```

### Task 7: Reproducible RTX Evidence and Handoff

**Files:**

- Create: `crates/alife_tools/reports/era1_promotion_report.json`
- Create: `docs/architecture/era1_norn_plus.md`
- Modify: `docs/architecture/evolution_genome_lab.md`
- Modify: `docs/creatures_agi_roadmap_pack/ROADMAP_OVERVIEW.md`
- Test: `crates/alife_tools/tests/era1_promotion.rs`

**Interfaces:**

- Produces: a source-bound report and `validate_committed_era1_promotion_report`.
- Report verdict may be PASS or BLOCKED according to actual evidence. The program is complete only when the artifact validator can recompute every receipt and no required field is fabricated; Era 2 remains dormant even if Era 1 passes.

- [ ] **Step 1: Write RED artifact tests.** Require exact Git commit/tree and source-blob digest, report-schema validation, genome/foundation/WGSL/world/save/archive/trial digests, RTX adapter/API, complete ability/control matrix, cost totals, no assistance, no hidden policy, no class scaling, and an explicit Era 2 out-of-scope marker.
- [ ] **Step 2: Commit the final source state, then run the CLI from that exact commit.**

```powershell
cargo run -p alife_tools --bin era1_promotion -- --out crates/alife_tools/reports/era1_promotion_report.json
```

- [ ] **Step 3: Run final serialized gates once.**

```powershell
cargo test -p alife_core --test era1_evaluation -j 1
cargo test -p alife_world --test era1_trials -j 1
cargo test -p alife_training --test era1_trials --test era1_learning --features gpu-tests -j 1
cargo test -p alife_tools --test era1_evolution --test era1_promotion -j 1
cargo fmt --all -- --check
powershell -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -ExecutionPolicy Bypass -File scripts/docs_check.ps1
git diff --check
```

- [ ] **Step 4: Inspect the report, request independent review, and commit the evidence lock.** Do not merge or push from this branch.

## Plan Self-Review

- Spec coverage: all eleven Era 1 abilities, four causal disabled controls, seeds, lineages, held-out worlds, ordinary reproduction, GPU authority, exact provenance, hardware cost, and plateau-before-scaling are assigned to concrete tasks.
- Interface ownership: core defines evidence; world defines environments and outcomes; training executes GPU trials; school exposes perception-only lessons; tools orchestrate selection and evidence.
- Reuse: EI0 genetics, habitats, selection, composite saves, archives, active challenge vocabulary, and the shared GPU session remain authoritative.
- Scope: no packs, hunting, tools, possession, advanced school, open language, Era 2, N4096, or class promotion.
- Placeholder scan: every step names concrete files, interfaces, failure evidence, proving checks, and commit boundaries.
- Type consistency: Tasks 2-7 consume the exact Task 1 enums/receipts; Task 6 reads only validated Task 5 receipts; Task 7 binds the final committed source and artifacts.
