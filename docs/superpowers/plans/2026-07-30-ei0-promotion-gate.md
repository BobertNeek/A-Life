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

- [ ] Add red tests proving a Managed birth can only follow the production
  player command/runtime entrypoint and preserves that command receipt.
- [ ] Add red GPU tests proving a Wild birth requires an actual neural decision,
  legal mate target, world outcome, and sealed causal patch from the selected
  parent. A forged `HabitatActor::Organism` is rejected.
- [ ] Implement one production composite-population runtime that owns both
  entrypoints and calls `CreatureGenome::reproduce` only after their causal
  receipts validate.
- [ ] Verify wrong actor, wrong target, replayed decision, and mismatched parent
  identities are rejected.

### Task 5: Authoritative save, restore, and nonvacuous inheritance

- [ ] Add red persistence tests requiring each creature save to reference a
  content-addressed composite `CreatureGenome` and shipped foundation asset.
- [ ] Seed every founder with distinctive nonzero lifetime memory and lifetime
  weight assets, save them, discard the pre-save population, and restore the
  authoritative runtime population from validated assets.
- [ ] Advance the restored population through a bounded 128-tick lifecycle
  interval before any birth or test.
- [ ] Breed only restored parents. Prove each child starts with zero inherited
  lifetime memory/weights while both parents had nonzero distinct state.

### Task 6: Full archive and evidence-digest binding

- [ ] Add a content-addressed composite-genome asset to genetic birth archives,
  load it back, and verify complete conception/recombination/mutation,
  parent/lineage, and foundation provenance.
- [ ] Bind the final report to canonical source-genome, foundation payload,
  closed-loop WGSL bundle, portable-save, archive manifest, and archive
  composite-asset digests.
- [ ] Independently compile each expected phenotype and add negative tests for
  phenotype-hash and foundation mismatch.

### Task 7: Honest operational and artifact contract

- [ ] Replace boolean-only clauses with `Pass`, `Fail`, `Unknown`, or
  `Unavailable` evidence statuses.
- [ ] Make the CLI write a schema-valid partial report before returning nonzero
  for GPU, save, archive, or other operational failure.
- [ ] Validate the committed artifact against exact source/foundation/shader,
  save, archive, adapter, and causal birth receipts.
- [ ] Regenerate the report, run each focused gate once with serialized GPU/Cargo
  execution, commit coherent fixes, and keep Era 1 out of scope.
