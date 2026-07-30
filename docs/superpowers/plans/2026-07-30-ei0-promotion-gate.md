# Era 0 Promotion Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans inline with test-driven development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce one reproducible receipt proving the player-facing Era 0 population loop across run, observe, save, breed, test, archive, and compare without hidden policy control.

**Architecture:** Add an offline evidence orchestrator in `alife_tools` that composes existing production contracts instead of creating new simulation authority. It will breed authoritative composite `CreatureGenome` populations under Wild and Managed habitat permissions, round-trip the real portable save, archive expressed genetic births in `LineageLibrary`, compare lineage/provenance records, and run the actual final-generation N2048 offspring through the existing shared-session WGSL active battery. The committed heuristic fixture remains an honest pre-Era-1 tooling baseline and never controls this Era 0 lifecycle gate.

**Tech Stack:** Rust 2021, serde JSON, `alife_core`, `alife_world`, `alife_archive`, `alife_training`, `alife_runtime`, wgpu/WGSL, Vulkan.

## Global Constraints

- Production cognition remains GPU-authoritative WGSL with no CPU neural shadow or fallback.
- Wild breeding is creature-chosen. Managed breeding is explicit player authority.
- Offspring use `CreatureGenome::reproduce` and `CreatureGenome::express`; no scripted policy or copied lifetime state.
- Archives, save receipts, tests, and comparisons retain exact organism, genome, parent, lineage, foundation, exposure, assistance, and backend provenance.
- Missing intelligence evidence remains `UNKNOWN`. `ei0_real_fixture_report.json` stays `HeuristicBaseline`, zero hidden trials, and `promotion_eligible=false`.
- This closes only the Era 0 lifecycle gate. It does not start or promote Era 1.

---

### Task 1: Integrated multi-generation lifecycle receipt

**Files:**

- Create: `crates/alife_tools/src/ei0_exit_gate.rs`
- Modify: `crates/alife_tools/src/lib.rs`
- Modify: `crates/alife_tools/Cargo.toml`
- Test: `crates/alife_tools/tests/ei0_exit_gate.rs`

**Interfaces:**

- Produces: `Ei0ExitGateReport`, typed clause receipts, and `run_ei0_lifecycle_gate`.
- Consumes: `CreatureGenome::{early_mammal_founder,reproduce,express}`, `HabitatAuthority`, the portable world save codec, and `LineageLibrary`.

- [x] **Step 1: Write a failing integrated test.** Require eight founders, two independent Generation 1 births and one Generation 2 birth in each of Wild and Managed lanes. Assert Wild requests use `CreatureChosen` with an organism actor, Managed requests use `Explicit` with the player actor, and every receipt reports `NeuralClosedLoopGpu`.
- [x] **Step 2: Verify RED.** Run `cargo test -p alife_tools --test ei0_exit_gate lifecycle_receipt --no-fail-fast -j 1`; expect failure because the gate API does not exist.
- [x] **Step 3: Implement the minimal lifecycle path.** Register all births in habitat authority, preserve full `GeneticLineageProvenance`, serialize and restore the real portable population save, archive each expressed genetic birth before comparison, and compare Generation 2 parent/ancestor/foundation identities from restored content.
- [x] **Step 4: Verify GREEN and negative boundaries.** Prove player-directed Wild breeding is rejected, creature-directed Managed breeding is rejected, tampered save/provenance is rejected, and no offspring contains inherited lifetime state.
- [x] **Step 5: Commit the coherent non-GPU slice.**

### Task 2: Exact offspring GPU test execution

**Files:**

- Modify: `crates/alife_training/src/active_battery.rs`
- Modify: `crates/alife_training/tests/active_battery.rs`
- Modify: `crates/alife_tools/src/ei0_exit_gate.rs`
- Test: `crates/alife_tools/tests/ei0_exit_gate.rs`

**Interfaces:**

- Produces: `N2048ActiveBatteryRunner::run_creature_genome` and per-offspring GPU battery receipts.
- Consumes: the Generation 2 `CreatureGenome`, its expressed `BrainGenome`, the built-in N2048 foundation, `GpuAuthoritativeSession`, grounded challenge worlds, world legality/outcomes, sealed patches, and WGSL sleep consolidation.

- [x] **Step 1: Write a failing exact-identity GPU test.** Require the runner to preserve source genome ID, parent IDs, lineage ID, phenotype hash, N2048 class, adapter/API, 15 completed challenges, matching GPU dispatch/sealed-outcome counts, and at least one committed sleep consolidation.
- [x] **Step 2: Verify RED.** Run the focused `alife_training` GPU test serially; expect failure because only seed-built genetic founders are accepted.
- [x] **Step 3: Implement minimal exact-genome execution.** Compile the supplied offspring’s expressed `BrainGenome` against the shipped foundation and execute the existing battery unchanged. Do not create a CPU scorer or alternate policy.
- [x] **Step 4: Verify GREEN.** Run both final-generation lane offspring through the real GPU battery and retain the adapter, backend, phenotype, dispatch, outcome, sleep, and challenge receipts.
- [x] **Step 5: Commit the GPU-bound slice.**

### Task 3: Reproducible gate artifact and truthful verdict

**Files:**

- Create: `crates/alife_tools/src/bin/ei0_exit_gate.rs`
- Create: `crates/alife_tools/reports/ei0_exit_gate_report.json`
- Modify: `docs/architecture/evolution_genome_lab.md`

**Interfaces:**

- Produces: `cargo run -p alife_tools --bin ei0_exit_gate -- --out <path>` and a schema-versioned report mapping every Era 0 exit clause to evidence.
- Keeps the separate heuristic fixture report non-promotional and preserves all `UNKNOWN` measures.

- [x] **Step 1: Write a failing CLI integration test.** Parse the emitted report and require PASS for run, observe, save/load, Wild breed, Managed breed, archive, compare, stability, real test execution, and GPU policy identity. Require the heuristic baseline classification and all unsupported measures to remain `UNKNOWN`.
- [x] **Step 2: Implement the CLI and deterministic JSON artifact.** A failed clause makes the command nonzero and remains explicit in the report.
- [x] **Step 3: Run serialized verification.** Run focused crate tests, the real GPU gate, `cargo fmt --all -- --check`, `scripts/check_core_boundaries.ps1`, `scripts/docs_check.ps1`, `git diff --check`, and inspect the final report and Git status.
- [x] **Step 4: Commit docs and the gate artifact.** Report exact PASS/FAIL/UNKNOWN receipts. Declare Era 0 passed only if every lifecycle clause passes; never infer Era 1 eligibility.

## Plan Self-Review

- Spec coverage: every exit-gate verb maps to a runtime or persisted receipt; both breeding authorities and GPU identity have negative tests.
- Evidence boundary: the heuristic fixture is explicitly separate from the Era 0 lifecycle verdict and remains non-promotional with `UNKNOWN` measures.
- Authority boundary: only existing world, genome, archive, save, and shared GPU-session owners mutate state; `alife_tools` only orchestrates evidence.
- Placeholder scan: no deferred steps, fabricated scores, copied fixtures, lowered thresholds, or unlabeled fallback paths.
- Scope: no Era 1 targets, hidden promotion trials, foundation promotion, or brain-class scaling are included.

---

## Review Remediation Wave

The first receipt is withdrawn as promotion evidence until every item below is
implemented and reverified. Review found that habitat permission labels were
being treated as causal reproduction receipts, and that save/load did not own
the composite genetic state used after restore.

### Task 4: Production causal reproduction authority

**Files:**

- Modify: `crates/alife_game_app/src/production_conversation_lineage_ui.rs`
- Modify: `crates/alife_game_app/src/composite_population_runtime.rs`
- Modify: `crates/alife_training/src/active_battery.rs`
- Test: `crates/alife_game_app/tests/composite_population_runtime.rs`
- Test: `crates/alife_training/tests/active_battery.rs`

- [ ] Add RED tests requiring Managed births to consume the exact
  `HabitatBreedingReceipt` produced by the production habitat-lab
  `ExplicitBreed` command. Mutated actor, parent, habitat, or tick receipts must
  fail before `CreatureGenome::reproduce` runs.
- [ ] Add a versioned canonical `HeadlessWorld` pre-action signature digest to
  `GpuReproductionIntentReceipt`. Add RED tests rejecting same-seed worlds with
  changed objects or later ticks.
- [ ] Replace `apply_player_breed_command` with receipt consumption. Export one
  production `produce_habitat_lab_explicit_breed_receipt` function and call it
  from both real app input and the Era 0 gate.
- [ ] Require exact runtime/pre-action world digest equality for Wild births,
  retain the sealed GPU patch, and verify wrong target, replay, and phenotype
  identity failures.

### Task 5: Authoritative save, restore, and nonvacuous inheritance

**Files:**

- Modify: `crates/alife_game_app/src/composite_population_runtime.rs`
- Modify: `crates/alife_tools/src/ei0_exit_gate.rs`
- Test: `crates/alife_game_app/tests/composite_population_runtime.rs`
- Test: `crates/alife_tools/tests/ei0_exit_gate.rs`

- [ ] Add RED offspring-restore coverage. Restore founders plus ordinary
  offspring from composite assets and require generations `0, 0, 1`, derived
  from persisted parent genome IDs. Missing parents and ancestry cycles fail.
- [ ] Add a bounded stability receipt recording exact start/end world digests,
  ecology metrics, resident IDs, and 128 elapsed ticks. Use only
  `HeadlessWorld::advance_tick`, which already advances ecology.
- [ ] Configure the gate world with a real bounded ecology spawn policy. Require
  world/ecology evolution, unchanged restored residents, and equality between
  the stability end digest and the first subsequent GPU pre-action digest.
- [ ] Keep child lifetime memory and weights empty while retaining distinct,
  nonzero restored parent lifetime receipts.

### Task 6: Full archive and evidence-digest binding

- [ ] Add RED hostile coverage with a valid preexisting manifest from another
  run. Count and validate only the 14 manifest digests emitted for
  `ei0-exit-gate-v2`; never use the shared library total.
- [ ] Independently compile every final `CreatureGenome` with the shipped N2048
  foundation and compare its expected `PhenotypeHash` to the exact GPU battery
  receipt. Add a mutated receipt negative.
- [ ] Preserve complete archive composite genome, conception, recombination,
  mutation, lineage, parent, and foundation validation for the 14 current-run
  receipts.

### Task 7: Honest operational and artifact contract

- [ ] Add RED artifact validation for a producing Git commit/tree, a BLAKE3
  digest over the exact relevant source paths, exact adapter/API, and digests of
  the complete causal birth and GPU receipts.
- [ ] Make `validate_committed_ei0_exit_gate_report` recompute source, genome,
  foundation, shader, causal receipt, clause, and final phenotype evidence.
  Reject any relevant source diff from the producing commit. Exclude only the
  generated report to avoid a self-hash cycle.
- [ ] Commit the focused RED/GREEN implementation checkpoint. Regenerate the
  report from that clean source commit with one serialized GPU run.
- [ ] Re-run the committed-artifact validator, focused lifecycle/GPU gates,
  formatting, boundary/docs checks, and diff checks. Only then mark Tasks 4-7
  complete and commit the report lock. Do not push, merge, or start Era 1.
