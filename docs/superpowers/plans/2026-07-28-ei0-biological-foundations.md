# EI0 Biological Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real diploid creature genome, deterministic bounded reproduction, causal phenotype expression, and bounded multi-rate biochemistry/development to `alife_core`.

**Architecture:** Keep `BrainGenome` as the neural compiler input and add a composite `CreatureGenome` above it. Express the six chromosome families into a `CreaturePhenotype`, then feed its expressed `BrainGenome` through the existing phenotype compiler. Add a separate engine-neutral biological state machine that converts body events into bounded drives, hormones, development, sleep readiness, reproductive readiness, and neural modulation without action authority.

**Tech Stack:** Rust 2021, serde, existing `alife_core` contract types, deterministic SplitMix64-style seeded sampling, Cargo integration tests.

## Global Constraints

- Work only in `D:\A life\.worktrees\ei0-biological-foundations` on `codex/ei0-biological-foundations`.
- Modify only `crates/alife_core`, directly related tests, and this focused plan.
- Preserve existing `BrainGenome`, phenotype compiler, chemistry, lineage, foundation, and save-facing contracts.
- Production neural action selection remains GPU-authoritative; genome and chemistry never emit actions.
- Ordinary offspring inherit DNA and foundation identity, never lifetime weights, eligibility, memories, learned language bindings, or transient state.
- All trait values, mutation deltas, crossover segments, chemistry channels, cadence steps, and provenance collections remain bounded.
- The same parents and conception seed must produce byte-identical serialized offspring.
- Avoid concurrent Cargo or GPU gates that share outputs. Use an isolated `CARGO_TARGET_DIR` on a volume with free space.

---

### Task 1: Diploid chromosome and foundation contracts

**Files:**
- Create: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Modify: `crates/alife_core/src/error.rs`
- Test: `crates/alife_core/tests/evolutionary_genetics.rs`

**Interfaces:**
- Consumes: `BrainGenome`, `BrainClassId`, `GenomeId`, `LineageId`, `NormalizedScalar`, `Validate`.
- Produces: `ContinuousLocus`, `DiscreteAllele<T>`, `DiscreteLocus<T>`, `AlleleDominance`, six chromosome structs, `FoundationGeneticIdentity`, and `CreatureGenome`.

- [ ] **Step 1: Write failing diploid-expression and validation tests**

```rust
#[test]
fn continuous_loci_blend_both_alleles_and_discrete_loci_obey_dominance() {
    let blended = ContinuousLocus::mean(0.2, 0.8).unwrap();
    assert_eq!(blended.expressed().unwrap(), 0.5);

    let dominant = DiscreteLocus::new(
        DiscreteAllele::new(BodyFrame::Light, AlleleDominance::Recessive),
        DiscreteAllele::new(BodyFrame::Sturdy, AlleleDominance::Dominant),
    );
    assert_eq!(dominant.expressed(), DiscreteExpression::Single(BodyFrame::Sturdy));
}

#[test]
fn malformed_locus_bounds_and_excessive_mutation_delta_are_rejected() {
    assert_eq!(ContinuousLocus::with_bounds(0.5, 0.5, 1.0, 0.0).unwrap_err(),
               ScaffoldContractError::InvalidGeneticBounds);
    let mut genome = early_mammal_genome();
    genome.reproduction.max_mutation_delta = ContinuousLocus::mean(0.8, 0.8).unwrap();
    assert_eq!(genome.validate_contract().unwrap_err(), ScaffoldContractError::MutationOverflow);
}
```

- [ ] **Step 2: Run the focused test and confirm RED**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics -- --nocapture`

Expected: compile failure because the EI0 diploid types do not exist.

- [ ] **Step 3: Implement bounded loci and the six chromosome groups**

```rust
pub const MAX_CROSSOVER_SEGMENTS: u8 = 8;
pub const MAX_MUTATION_DELTA: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContinuousLocus {
    pub maternal: f32,
    pub paternal: f32,
    pub lower: f32,
    pub upper: f32,
    pub maternal_weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlleleDominance { Recessive, Dominant, Codominant }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscreteExpression<T> { Single(T), Codominant(T, T) }
```

Define fixed, versioned `BodyChromosome`, `BrainChromosome`, `ChemistryChromosome`, `DevelopmentChromosome`, `ReproductionChromosome`, and `PredispositionChromosome` records. Each family must contain at least one continuous locus. Body frame, brain class, mate preference, and starter-vocabulary profile use discrete loci. `CreatureGenome::early_mammal_founder` creates a valid heterozygous founder with explicit foundation and lineage identity.

- [ ] **Step 4: Run the focused test and confirm GREEN**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics -- --nocapture`

Expected: all Task 1 tests pass.

- [ ] **Step 5: Commit the contract slice**

```powershell
git add crates/alife_core/src/evolutionary_genetics.rs crates/alife_core/src/lib.rs crates/alife_core/src/error.rs crates/alife_core/tests/evolutionary_genetics.rs
git commit -m "feat(core): add diploid creature genome contracts"
```

### Task 2: Deterministic bounded reproduction and lineage provenance

**Files:**
- Modify: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_core/tests/evolutionary_genetics.rs`

**Interfaces:**
- Consumes: validated `CreatureGenome` parents and a nonzero `conception_seed: u64`.
- Produces: `CreatureGenome::reproduce`, `GeneticLineageProvenance`, `ChromosomeRecombinationRecord`, and `MutationRecord`.

- [ ] **Step 1: Write failing seeded-reproduction, serialization, and negative tests**

```rust
#[test]
fn seeded_reproduction_is_deterministic_bounded_and_records_both_parents() {
    let a = parent(101, BrainCapacityClass::N512_ID, 11);
    let b = parent(202, BrainCapacityClass::N512_ID, 11);
    let one = CreatureGenome::reproduce(&a, &b, 0xC0FFEE).unwrap();
    let two = CreatureGenome::reproduce(&a, &b, 0xC0FFEE).unwrap();
    assert_eq!(serde_json::to_vec(&one).unwrap(), serde_json::to_vec(&two).unwrap());
    assert_eq!(one.parent_genome_ids, vec![a.id, b.id]);
    assert!(one.provenance.recombination.iter().all(|r| r.segments <= MAX_CROSSOVER_SEGMENTS));
    assert!(one.provenance.mutations.iter().all(|m| m.after >= m.lower && m.after <= m.upper));
}

#[test]
fn reproduction_rejects_incompatible_classes_and_foundation_families() {
    let n512 = parent(1, BrainCapacityClass::N512_ID, 10);
    let n1024 = parent(2, BrainCapacityClass::N1024_ID, 10);
    assert_eq!(CreatureGenome::reproduce(&n512, &n1024, 9).unwrap_err(),
               ScaffoldContractError::IncompatibleGeneticClass);
}
```

Round-trip the complete offspring through `serde_json`, validate it after decoding, and assert ordinary offspring carry no lifetime or transient mind-state payload.

- [ ] **Step 2: Run the focused test and confirm RED**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics -- --nocapture`

Expected: compile failure because `CreatureGenome::reproduce` and provenance records do not exist.

- [ ] **Step 3: Implement gamete selection, crossover, mutation, and provenance**

```rust
impl CreatureGenome {
    pub fn reproduce(
        maternal: &Self,
        paternal: &Self,
        conception_seed: u64,
    ) -> Result<Self, ScaffoldContractError>;
}
```

Use one deterministic local RNG state derived from both genome IDs and `conception_seed`. Each chromosome starts on one parental homolog and may switch only while its crossover count is below the expressed `max_crossover_segments`. Select one gamete from each parent, mutate selected continuous alleles with a signed delta no larger than the expressed `max_mutation_delta`, reject non-finite arithmetic, and reflect results into locus bounds. Record every crossover count and changed allele. Select a child foundation only when both parents share the compatibility family and expressed brain class. Force ordinary-child inheritance flags to the non-Lamarckian default.

- [ ] **Step 4: Run the focused test and confirm GREEN**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics -- --nocapture`

Expected: deterministic reproduction, serialization, class/foundation rejection, and mutation bounds all pass.

- [ ] **Step 5: Commit reproduction**

```powershell
git add crates/alife_core/src/evolutionary_genetics.rs crates/alife_core/tests/evolutionary_genetics.rs
git commit -m "feat(core): reproduce diploid genomes deterministically"
```

### Task 3: Causal creature phenotype and existing brain compiler activation

**Files:**
- Modify: `crates/alife_core/src/evolutionary_genetics.rs`
- Modify: `crates/alife_core/src/phenotype/construction.rs`
- Modify: `crates/alife_core/src/phenotype/topology_compile.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Test: `crates/alife_core/tests/evolutionary_genetics.rs`
- Test: `crates/alife_core/tests/phenotype_compiler.rs`

**Interfaces:**
- Consumes: validated diploid `CreatureGenome`.
- Produces: `CreaturePhenotype`, `BodyPhenotype`, `ChemistryPhenotype`, `DevelopmentPhenotype`, `ReproductionPhenotype`, `PredispositionPhenotype`, and an expressed `BrainGenome` accepted by `PhenotypeCompiler`.

- [ ] **Step 1: Write failing tests proving every enabled family changes phenotype**

```rust
#[test]
fn each_chromosome_family_changes_its_causal_phenotype_surface() {
    let baseline = early_mammal_genome();
    let expected = baseline.express().unwrap();
    assert_ne!(mutate_body(&baseline).express().unwrap().body, expected.body);
    assert_ne!(mutate_brain(&baseline).express().unwrap().brain_genome, expected.brain_genome);
    assert_ne!(mutate_chemistry(&baseline).express().unwrap().chemistry, expected.chemistry);
    assert_ne!(mutate_development(&baseline).express().unwrap().development, expected.development);
    assert_ne!(mutate_reproduction(&baseline).express().unwrap().reproduction, expected.reproduction);
    assert_ne!(mutate_predisposition(&baseline).express().unwrap().predisposition, expected.predisposition);
}
```

Add a compiler test that changes the expressed plasticity locus, compiles both `BrainGenome` values, and observes changed compiled synapse alpha or phenotype hash. Add a test that a valid reproduced genome with non-founder IDs and seeds compiles deterministically.

- [ ] **Step 2: Run both focused tests and confirm RED**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics --test phenotype_compiler -- --nocapture`

Expected: failure because causal expression and offspring compiler support are absent.

- [ ] **Step 3: Implement expression and remove founder-only compiler restrictions**

```rust
impl CreatureGenome {
    pub fn express(&self) -> Result<CreaturePhenotype, ScaffoldContractError>;
}

impl CreaturePhenotype {
    pub fn development_state_at(&self, age: Tick) -> Result<DevelopmentState, ScaffoldContractError>;
}
```

Map body loci to physical phenotype values; brain loci to lobe ratios, sparse density, alpha/plasticity, sensor layout, motor affordances, and the immutable genetic seed; chemistry loci to `EndocrineProfile` and drive thresholds; development loci to milestones and critical periods; reproduction loci to mutation/crossover/fertility; predisposition loci to starter-token sets and bounded reflex, attraction, aversion, and social-attention biases. Keep these fields as modulation/configuration only.

Replace `validate_supported_inputs` founder-equality checks with invariant checks that accept validated offspring IDs, seeds, schedules, chemistry, reproduction, and plasticity. In `alpha_for`, set alpha to zero for a disabled projection and multiply enabled alpha by that projection's validated learning-rate scale. Keep dense debug alpha rejected.

- [ ] **Step 4: Run both focused tests and confirm GREEN**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics --test phenotype_compiler -- --nocapture`

Expected: all family-causality and brain-compiler tests pass.

- [ ] **Step 5: Commit phenotype activation**

```powershell
git add crates/alife_core/src/evolutionary_genetics.rs crates/alife_core/src/phenotype/construction.rs crates/alife_core/src/phenotype/topology_compile.rs crates/alife_core/src/lib.rs crates/alife_core/tests/evolutionary_genetics.rs crates/alife_core/tests/phenotype_compiler.rs
git commit -m "feat(core): express diploid genes into causal phenotypes"
```

### Task 4: Bounded multi-rate biochemistry and development links

**Files:**
- Create: `crates/alife_core/src/biochemistry.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Test: `crates/alife_core/tests/biochemistry_development.rs`

**Interfaces:**
- Consumes: `BodyPhenotype`, `ChemistryPhenotype`, `DevelopmentPhenotype`, `ReproductionPhenotype`, world-derived `BodyEventDelta`, and the existing `HomeostaticSnapshot`.
- Produces: `BodyState`, `BiochemistryState`, `BiochemistryCadence`, `NeuralModulation`, `DevelopmentReadiness`, and `ReproductionReadiness`.

- [ ] **Step 1: Write failing body-to-neural, cadence, sleep, reproduction, development, and authority tests**

```rust
#[test]
fn body_damage_and_energy_loss_causally_change_drives_hormones_and_neural_modulation() {
    let phenotype = early_mammal_genome().express().unwrap();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let next = state.advance(Tick(12), BodyEventDelta {
        energy: -0.30,
        damage: 0.40,
        ..BodyEventDelta::zero()
    }, &phenotype).unwrap();
    assert!(next.homeostasis.drives.fatigue > state.homeostasis.drives.fatigue);
    assert!(next.homeostasis.drives.pain > state.homeostasis.drives.pain);
    assert!(next.homeostasis.hormones.cortisol > state.homeostasis.hormones.cortisol);
    assert!(next.neural.learning_rate_scale < state.neural.learning_rate_scale);
}
```

Also prove fast hormone response can change before metabolic development ticks; slower metabolism, development, and reproduction update only on their cadence boundaries; sleep recovery lowers fatigue and sleep pressure; puberty and health gate reproduction; critical periods raise plasticity then close; values stay within `[0, 1]` during a long bounded loop; invalid body values are rejected. Serialize `NeuralModulation` and assert its exact allowed keys contain thresholds, salience, attention, plasticity, sleep, and development fields but no action, candidate, target, reward injection, or command field.

- [ ] **Step 2: Run the focused test and confirm RED**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test biochemistry_development -- --nocapture`

Expected: compile failure because the integrated biology state machine does not exist.

- [ ] **Step 3: Implement the multi-rate state machine**

```rust
impl BiochemistryState {
    pub fn new(
        phenotype: &CreaturePhenotype,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError>;

    pub fn advance(
        &self,
        next_tick: Tick,
        event: BodyEventDelta,
        phenotype: &CreaturePhenotype,
    ) -> Result<Self, ScaffoldContractError>;
}
```

Apply world/body deltas once at the causal boundary. Count crossed cadence boundaries with integer division, cap catch-up steps, and update fast chemistry, metabolism, development, and reproduction separately. Derive interoceptive drive deltas from body truth, derive bounded hormone deltas from drives/outcomes, call the existing chemistry modulation functions, multiply learning by active critical-period plasticity, expose sleep and migration readiness, and never construct or accept `ActionCommand`.

- [ ] **Step 4: Run the focused test and confirm GREEN**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test biochemistry_development -- --nocapture`

Expected: all causal, cadence, bound, sleep, reproduction, development, and authority tests pass.

- [ ] **Step 5: Commit integrated biology**

```powershell
git add crates/alife_core/src/biochemistry.rs crates/alife_core/src/lib.rs crates/alife_core/tests/biochemistry_development.rs
git commit -m "feat(core): connect bounded biochemistry and development"
```

### Task 5: Compatibility, scope, and final evidence

**Files:**
- Modify only files already listed if verification finds a regression.

**Interfaces:**
- Consumes: all EI0 public contracts and tests.
- Produces: clean branch commits and exact verification receipts for supervisor integration.

- [ ] **Step 1: Format and inspect the intended diff**

Run: `cargo fmt --all -- --check`

Run: `git status --short; git diff --check; git diff --stat main...HEAD`

Expected: formatting and whitespace checks pass; changed paths stay within assigned ownership.

- [ ] **Step 2: Run focused EI0 tests**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo test -p alife_core --test evolutionary_genetics --test biochemistry_development --test phenotype_compiler`

Expected: zero failures.

- [ ] **Step 3: Run the complete crate gate**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; $env:CARGO_INCREMENTAL='0'; cargo test -p alife_core --all-targets -j 1`

Expected: every `alife_core` unit and integration test passes.

- [ ] **Step 4: Run compile and lint gates**

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo check -p alife_core --all-targets`

Run: `$env:CARGO_TARGET_DIR='C:\Users\PC\AppData\Local\Temp\codex-alife-ei0-target'; cargo clippy -p alife_core --all-targets -- -D warnings`

Expected: both commands exit zero with no warnings.

- [ ] **Step 5: Re-read the approved spec and audit each objective**

Confirm direct evidence for two alleles, bounded crossover, bounded mutation, continuous blending, discrete dominance/codominance, causal family mappings, foundation identity, lineage provenance, seeded determinism, serialization, body-drive-hormone-neural links, critical periods, sleep, reproduction, development, ordinary-birth memory exclusion, invalid bounds, incompatible classes, mutation overflow, and no hidden action authority.

- [ ] **Step 6: Commit any verification-only corrections and report readiness**

```powershell
git status --short --branch
git log --oneline --decorate main..HEAD
```

Do not merge to `main`. Send the supervisor the branch, commits, focused/full verification, and shortest integration path.
