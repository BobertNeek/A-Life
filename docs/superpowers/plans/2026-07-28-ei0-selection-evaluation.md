# EI0 Selection and Evaluation Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the P33 genome lab into a deterministic packed-log intelligence battery and managed multi-level breeding system.

**Architecture:** Add one tooling module for exposure-aware intelligence evaluation and one for managed breeding selection. Evaluation consumes validated `PackedExperienceRecord` traces, keeps unknown metrics explicit, and emits separate cognitive and seven-axis objective vectors. Selection uses Pareto preservation plus deterministic lexicase ordering, protects wild and minority lineages, and breeds only legal unrelated pairs through the existing P33 crossover contract.

**Tech Stack:** Rust 2021, `alife_core` contracts, existing `alife_world::ScenarioFixture` traces, Serde JSON, Cargo integration tests.

## Global Constraints

- Modify only `crates/alife_tools`, its tests, fixtures, reports, and this focused plan.
- Consume `alife_core` and `alife_world` contracts without editing either crate.
- Keep all evaluation and selection offline. Do not add runtime policy, neural execution, or action authority.
- Keep ecological, cognitive, social, group, stability, efficiency, and diversity scores separate.
- Treat missing exposure as unknown. Never convert it to zero.
- Treat hidden promotion evidence as invalid without assistance, exposure, adapter, foundation, lineage, source-run, and compute provenance.
- Never use fixed benchmark answers as the sole breeding or promotion signal.
- Preserve wild reservoirs, minority lineages, and useful specialists.
- Pair a high-cognition, low-survival exception only with a robust unrelated mate. Never pair two fragile exceptions.
- Put every cognitive-introgression offspring in a higher-scrutiny probation cohort with sibling and population controls.
- Evaluate persistent packs and randomized teams. Reject apparent group gains caused by a free rider.
- Use real packed logs from committed scenario fixtures for acceptance. Synthetic objective vectors remain limited to focused unit tests.
- Do not merge to `main`; the supervisor owns integration.

---

### Task 1: Exposure-aware intelligence battery

**Files:**
- Create: `crates/alife_tools/src/p33_evaluation.rs`
- Modify: `crates/alife_tools/src/lib.rs`
- Test: `crates/alife_tools/tests/intelligence_battery.rs`

**Interfaces:**
- Consumes: `alife_core::{GenomeId, LineageId, PackedExperienceRecord, Validate}`.
- Produces: `BatterySuite`, `BatteryTrial`, `TrialTrace`, `EvaluationProvenance`, `CognitiveMeasures`, `ObjectiveVector`, `BatteryReport`, and `evaluate_battery(&BatterySuite) -> Result<BatteryReport, EvaluationError>`.
- Produces: `ScoreEstimate { value: Option<f32>, samples: u32 }`; `None` means unexposed and is never scored as zero.

- [ ] **Step 1: Write failing contract tests for provenance and unknown exposure**

```rust
#[test]
fn hidden_promotion_requires_complete_provenance() {
    let mut suite = complete_hidden_suite();
    suite.trials[0].provenance.compute.adapter.clear();
    let error = evaluate_battery(&suite).unwrap_err();
    assert!(matches!(error, EvaluationError::MissingPromotionProvenance { .. }));
}

#[test]
fn unexposed_domains_remain_unknown_instead_of_zero() {
    let report = evaluate_battery(&ecology_only_suite()).unwrap();
    assert!(report.measures.learning.value.is_none());
    assert!(report.objectives.cognitive.value.is_none());
}
```

- [ ] **Step 2: Run the focused test and confirm the new module/API is missing**

Run: `cargo test -p alife_tools --test intelligence_battery --no-fail-fast`

Expected: FAIL because `alife_tools::p33_evaluation` does not exist.

- [ ] **Step 3: Implement the validated battery data model**

Implement these public contracts with `Serialize`, `Deserialize`, `Clone`, and `PartialEq`:

```rust
pub enum BatteryLayer { PermanentAnchor, ProceduralBreeding, HiddenPromotion }
pub enum TrialDomain { Ecology, Learning, Transfer, Reversal, DelayedMemory, Abstraction, SocialContribution }
pub enum TrialPhase { Baseline, Acquisition, Transfer, Reversal, DelayRecall, ActiveGroup, MemberRemoved, Replacement }
pub enum TeamMode { Individual, PersistentPack, RandomizedTeam }
pub enum AssistanceKind { Teacher, PlayerPossession, SemanticPrior, HiddenReward }

pub struct ComputeProvenance {
    pub adapter: String,
    pub backend: String,
    pub dispatches: u64,
    pub neural_ticks: u64,
    pub elapsed_micros: u64,
    pub energy_milliunits: u64,
    pub budget_units: u64,
}

pub struct LineageProvenance {
    pub lineage_id: LineageId,
    pub genome_id: GenomeId,
    pub ancestor_genome_ids: Vec<GenomeId>,
    pub population_share: f32,
    pub genome_novelty: f32,
}

pub struct EvaluationProvenance {
    pub source_run_id: String,
    pub foundation_id: String,
    pub foundation_version: u32,
    pub exposure_count: u32,
    pub assistance: Vec<AssistanceKind>,
    pub compute: ComputeProvenance,
    pub lineage: LineageProvenance,
}

pub struct TrialTrace {
    pub phase: TrialPhase,
    pub records: Vec<PackedExperienceRecord>,
}

pub struct BatteryTrial {
    pub test_id: String,
    pub layer: BatteryLayer,
    pub domain: TrialDomain,
    pub team_mode: TeamMode,
    pub seed: u64,
    pub variant_id: String,
    pub answer_fingerprint: Option<String>,
    pub hidden_set_id: Option<String>,
    pub focal_organism_id: u64,
    pub provenance: EvaluationProvenance,
    pub traces: Vec<TrialTrace>,
}

pub struct ScoreEstimate {
    pub value: Option<f32>,
    pub samples: u32,
}

pub struct CognitiveMeasures {
    pub learning: ScoreEstimate,
    pub transfer: ScoreEstimate,
    pub reversal: ScoreEstimate,
    pub delayed_memory: ScoreEstimate,
    pub abstraction: ScoreEstimate,
    pub social_contribution: ScoreEstimate,
}

pub struct ObjectiveVector {
    pub ecological: ScoreEstimate,
    pub cognitive: ScoreEstimate,
    pub social: ScoreEstimate,
    pub group: ScoreEstimate,
    pub stability: ScoreEstimate,
    pub efficiency: ScoreEstimate,
    pub diversity: ScoreEstimate,
}

pub enum EvaluationFlag {
    AnchorProceduralGap,
    FixedAnswerOverfit,
    GroupFreeRider,
}

pub struct BatteryReport {
    pub schema_version: u16,
    pub suite_id: String,
    pub packed_record_count: usize,
    pub measures: CognitiveMeasures,
    pub objectives: ObjectiveVector,
    pub flags: Vec<EvaluationFlag>,
    pub promotion_eligible: bool,
}
```

Validation must reject empty trials, zero seeds, invalid packed records, non-finite or out-of-range novelty and population shares, zero compute budgets, and incomplete hidden-promotion provenance. Hidden trials must have zero exposure, no assistance, and a nonempty hidden-set ID.

- [ ] **Step 4: Run the focused tests and confirm provenance and unknown-state behavior**

Run: `cargo test -p alife_tools --test intelligence_battery hidden_promotion_requires_complete_provenance -- --exact`

Run: `cargo test -p alife_tools --test intelligence_battery unexposed_domains_remain_unknown_instead_of_zero -- --exact`

Expected: PASS for both exact tests.

- [ ] **Step 5: Write failing behavior tests for every measure and hostile flag**

Add literal, hand-derived trace cases that prove learning, transfer, reversal, delayed memory, abstraction, social contribution, and compute efficiency. Add hostile fixtures for fixed-answer overfitting, anchor-only benchmark gaming, and group free-riding.

The free-rider fixture makes `MemberRemoved` outperform `ActiveGroup`. The fixed-answer fixture reuses one fingerprint across distinct seeds and variants. The benchmark-gaming fixture scores permanent anchors at least `0.35` above procedural trials.

- [ ] **Step 6: Run the target and confirm measure aggregation is missing**

Run: `cargo test -p alife_tools --test intelligence_battery --no-fail-fast`

Expected: FAIL on measure and hostile-flag assertions.

- [ ] **Step 7: Implement packed-log scoring and seven-axis objectives**

Use validated packed-log fields:

```text
trace performance = 0.45 * success rate
                  + 0.25 * normalized positive reward
                  + 0.20 * (1 - prediction error)
                  + 0.10 * (1 - absolute energy delta)
```

Derive learning from baseline-to-acquisition improvement and acquisition speed. Derive transfer, reversal, delayed memory, and abstraction only from matching phases and domains. Derive social contribution from active-minus-removed performance, with replacement as a control. Require persistent-pack and randomized-team evidence before group score is known. Derive stability from cross-seed dispersion, efficiency from performance per compute budget, and diversity from genome novelty plus inverse lineage share. Keep promotion eligibility false when a required cognitive measure is unknown or a hostile flag exists.

- [ ] **Step 8: Run the complete battery target**

Run: `cargo test -p alife_tools --test intelligence_battery --no-fail-fast`

Expected: PASS with no ignored tests.

- [ ] **Step 9: Commit the evaluation slice**

```powershell
git add crates/alife_tools/src/p33_evaluation.rs crates/alife_tools/src/lib.rs crates/alife_tools/tests/intelligence_battery.rs
git commit -m "feat: add EI0 intelligence battery"
```

---

### Task 2: Real packed-log fixture and report pipeline

**Files:**
- Create: `crates/alife_tools/tests/fixtures/p33_ei0_real_battery.json`
- Create: `crates/alife_tools/reports/ei0_real_fixture_report.json`
- Modify: `crates/alife_tools/src/p33_evaluation.rs`
- Modify: `crates/alife_tools/src/bin/p33_genome_lab.rs`
- Test: `crates/alife_tools/tests/intelligence_battery.rs`

**Interfaces:**
- Consumes: `alife_world::{ScenarioFixture, ScenarioName}` and the Task 1 battery API.
- Produces: `ScenarioBatteryFixture::from_json_file`, `ScenarioBatteryFixture::run`, and CLI command `evaluate-fixture --fixture <path> --out <path>`.
- Produces: a JSON `BatteryReport` with record count, source runs, objectives, hostile flags, and promotion eligibility.

- [ ] **Step 1: Write the failing real-fixture integration test**

```rust
#[test]
fn committed_scenario_fixture_evaluates_real_packed_logs_deterministically() {
    let fixture = ScenarioBatteryFixture::from_json_file(real_fixture_path()).unwrap();
    let first = fixture.run().unwrap();
    let second = fixture.run().unwrap();
    assert_eq!(first, second);
    assert!(first.packed_record_count >= 8);
    assert!(first.objectives.ecological.value.is_some());
    assert!(first.objectives.cognitive.value.is_none());
    assert!(first.objectives.social.value.is_none());
    assert!(first.objectives.group.value.is_none());
    assert!(!first.promotion_eligible);
}
```

- [ ] **Step 2: Run the exact fixture test and confirm the adapter is missing**

Run: `cargo test -p alife_tools --test intelligence_battery committed_scenario_fixture_evaluates_real_packed_logs_deterministically -- --exact`

Expected: FAIL because `ScenarioBatteryFixture` and its committed fixture do not exist.

- [ ] **Step 3: Add the committed scenario manifest and adapter**

Include at least two seeds and permanent/procedural cases drawn from all eight existing `ScenarioName` variants. Each case declares layer, domain, phase, team mode, variant, source run, foundation, exposure, assistance, lineage novelty/share, and honest `HeuristicBaseline` compute provenance. Map each scenario run to one honest `ObservedOutcome` phase. Never duplicate or relabel an identical scenario and seed as baseline/acquisition, removal/replacement, transfer, reversal, or delayed recall. Unsupported measures remain `UNKNOWN`. `ScenarioBatteryFixture::run` executes `ScenarioFixture::with_seed`, collects each tick's real `packed_record`, builds `BatterySuite`, and calls `evaluate_battery`.

- [ ] **Step 4: Run the exact fixture test and confirm real-log determinism**

Run: `cargo test -p alife_tools --test intelligence_battery committed_scenario_fixture_evaluates_real_packed_logs_deterministically -- --exact`

Expected: PASS and `packed_record_count >= 8`.

- [ ] **Step 5: Write the failing CLI report test**

Run the Cargo-provided `p33_genome_lab` binary with `evaluate-fixture`, parse its output as `BatteryReport`, and assert its suite ID, real packed-record count, separate objective fields, and `promotion_eligible: false`.

- [ ] **Step 6: Run the exact CLI test and confirm the command is unknown**

Run: `cargo test -p alife_tools --test intelligence_battery evaluate_fixture_cli_writes_machine_readable_report -- --exact`

Expected: FAIL with an unknown-command diagnostic.

- [ ] **Step 7: Implement the CLI and generate the committed report**

Run:

```powershell
cargo run -p alife_tools --bin p33_genome_lab -- evaluate-fixture --fixture crates/alife_tools/tests/fixtures/p33_ei0_real_battery.json --out crates/alife_tools/reports/ei0_real_fixture_report.json
```

Expected: exit 0 and a report whose source backend is explicitly `HeuristicBaseline`, whose unsupported cognitive/social/group measures are `UNKNOWN`, and whose promotion eligibility is false.

- [ ] **Step 8: Run the full fixture and CLI target**

Run: `cargo test -p alife_tools --test intelligence_battery --no-fail-fast`

Expected: PASS, including deterministic real-log and hostile tests.

- [ ] **Step 9: Commit the real-evidence slice**

```powershell
git add crates/alife_tools/src/p33_evaluation.rs crates/alife_tools/src/bin/p33_genome_lab.rs crates/alife_tools/tests/intelligence_battery.rs crates/alife_tools/tests/fixtures/p33_ei0_real_battery.json crates/alife_tools/reports/ei0_real_fixture_report.json
git commit -m "feat: evaluate real packed-log battery fixtures"
```

---

### Task 3: Authoritative composite-genome integration checkpoint

**Files:**
- No implementation files change until the biology dependency is present on this branch.
- Modify after integration: this focused plan with exact authoritative signatures.

**Interfaces:**
- Must consume the biology branch's composite `CreatureGenome`, seeded reproduction result, and lineage provenance contracts from `alife_core`.
- Must not use legacy `BrainGenome` or `p33_evolution::crossover_genomes` as the final breeding substrate.

- [ ] **Step 1: Commit Task 2 and checkpoint the supervisor**

Report the Task 2 commit, focused verification, real report summary, and explicit unsupported measures. Request the shortest integration path for the authoritative biology contracts.

- [ ] **Step 2: Stop before managed-selection tests or code until the dependency is present**

Verify the current branch exposes composite genome validation, deterministic seeded reproduction, parent and lineage provenance, and offspring viability. If any contract is absent, send one exact interface request and continue no managed-breeding implementation.

- [ ] **Step 3: Replace this checkpoint with an exact self-reviewed Task 3 plan after integration**

The revised task must use the integrated public signatures in its tests and implementation. It must retain Pareto/lexicase selection, wild preservation, minority and specialist retention, inbreeding rejection, robust-mate-only cognitive introgression, and controlled probation cohorts.

---

### Task 4: Acceptance hardening and documentation

**Files:**
- Modify: `crates/alife_tools/README.md`
- Modify: `docs/architecture/evolution_genome_lab.md`
- Modify: `crates/alife_tools/tests/intelligence_battery.rs`
- Modify: `crates/alife_tools/tests/managed_selection.rs`

**Interfaces:**
- Documents the Task 1-3 APIs and exact real-fixture command.
- Adds no runtime or cross-crate interface.

- [ ] **Step 1: Complete the hostile acceptance matrix**

Ensure exact tests cover benchmark gaming, shared-ancestry and shared-lineage inbreeding, missing hidden provenance, fixed-answer overfitting, persistent/randomized team free-riding, fragile-fragile rejection, unknown exposure, wild preservation, minority and specialist retention, and deterministic reports, pairings, and offspring.

- [ ] **Step 2: Run both focused targets once after acceptance changes**

Run: `cargo test -p alife_tools --test intelligence_battery --test managed_selection --no-fail-fast`

Expected: PASS with no ignored tests.

- [ ] **Step 3: Document evidence boundaries and operation**

State that the committed report proves deterministic tooling over real packed logs from actual headless scenario fixtures. Its `HeuristicBaseline` backend is not hidden-promotion or GPU neural intelligence evidence. Document permanent, procedural, and hidden layers; provenance requirements; unknown exposure; Pareto/lexicase selection; introgression; probation; wild preservation; both team modes; and hostile flags.

- [ ] **Step 4: Run formatting and the complete crate suite**

```powershell
cargo fmt --all -- --check
cargo test -p alife_tools --no-fail-fast
```

Expected: both commands exit 0 with zero failed tests.

- [ ] **Step 5: Run repository boundary checks once**

```powershell
& ./scripts/check_core_boundaries.ps1
& ./scripts/docs_check.ps1
```

Expected: both scripts exit 0. If an unrelated pre-existing failure occurs, record its exact output and keep scoped crate evidence separate.

- [ ] **Step 6: Inspect final scope and generated evidence**

```powershell
git status --short
git diff --check
git diff --stat main...HEAD
Get-Content -Raw crates/alife_tools/reports/ei0_real_fixture_report.json | ConvertFrom-Json | Select-Object suite_id,packed_record_count,promotion_eligible
```

Expected: only owned files changed, `git diff --check` exits 0, report record count is nonzero, and promotion eligibility is false.

- [ ] **Step 7: Commit documentation or final hardening changes**

```powershell
git add crates/alife_tools/README.md docs/architecture/evolution_genome_lab.md crates/alife_tools/tests/intelligence_battery.rs crates/alife_tools/tests/managed_selection.rs
git commit -m "docs: record EI0 selection evidence boundaries"
```

- [ ] **Step 8: Record branch readiness without merging**

Run: `git status --short --branch && git log --oneline main..HEAD`

Expected: clean `codex/ei0-selection-evaluation` worktree with all coherent commits listed and no merge to `main`.
