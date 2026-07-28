# Era 0 Habitat Authority Implementation Plan

> **For agentic workers:** Execute inline with test-driven development. Keep all production changes inside `crates/alife_world`.

**Goal:** Add deterministic habitat authority and its save and presentation contracts without changing cognition, selection, or GPU execution.

**Architecture:** `HabitatAuthority` owns stable habitats, one membership per organism, permission checks, and append-only transfer provenance. Portable world saves persist that state and migrate older saves into one default Wild habitat. A separate pure projection reads authoritative speech, grounding, and relationship evidence without exposing mutation.

**Tech Stack:** Rust, serde, thiserror, Cargo tests.

## Global Constraints

- `docs/master_spec.md` and `docs/architecture_decisions.md` remain controlling.
- Every habitat mode reports `PolicyBackend::NeuralClosedLoopGpu`.
- Habitat code never accepts or emits actions, rewards, semantic answers, neural state, or policy scores.
- Use existing `alife_core` IDs and foundation/language evidence types.
- Do not modify `alife_tools`, GPU execution, player UI, or the evolutionary run.

### Task 1: Deterministic habitat authority

**Files:**

- Create: `crates/alife_world/src/habitat.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Test: `crates/alife_world/tests/habitat_authority.rs`

**Interfaces:**

- Produces `HabitatId`, `HabitatMode`, `HabitatAuthority`, membership and transfer provenance types, permission queries, and typed validation errors.
- Uses `OrganismId`, `FoundationId`, `PolicyBackend`, and `Tick` from `alife_core`.

- [ ] Write focused tests for four modes, one membership, deterministic transfers, provenance, mode permissions, and malformed state.
- [ ] Run `cargo test -p alife_world --test habitat_authority` and confirm the new contract is absent.
- [ ] Implement the smallest authority module that makes those tests pass.
- [ ] Re-run the focused test and commit the green slice.

### Task 2: Save compatibility

**Files:**

- Modify: `crates/alife_world/src/headless.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Test: `crates/alife_world/tests/save_load_roundtrip.rs`
- Test: `crates/alife_world/tests/habitat_authority.rs`

**Interfaces:**

- Persists `HabitatAuthority` inside `WorldSaveState`.
- Missing legacy habitat data migrates all saved creatures to the stable default Wild habitat with explicit legacy provenance.

- [ ] Write failing round-trip, legacy-default, duplicate membership, unknown ID, and stale transfer tests.
- [ ] Run only those tests and confirm expected failures.
- [ ] Add persistence wiring and cross-record validation.
- [ ] Re-run the focused save tests and commit the green slice.

### Task 3: Read-only presentation projection and authority isolation

**Files:**

- Create: `crates/alife_world/src/presentation.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Test: `crates/alife_world/tests/habitat_presentation.rs`

**Interfaces:**

- Produces a deterministic projection keyed by raw stable `OrganismId`.
- Reads creature utterances plus `LanguageGroundingLedger` evidence and typed relationship evidence.
- Represents absent utterance, affinity, trust, or fear evidence as `EvidenceValue::Unknown`.

- [ ] Write failing projection tests for latest grounded tokens, pairwise evidence, UNKNOWN fields, and stable ordering.
- [ ] Write failing isolation tests proving all modes share GPU policy identity and habitat mutations leave action, outcome reward, semantic receipt, and neural state unchanged.
- [ ] Implement the pure read-only projection and pass the focused tests.
- [ ] Commit the green slice.

### Task 4: Focused documentation and gate

**Files:**

- Modify: `crates/alife_world/README.md`

- [ ] Document the implemented public ownership boundary and no-policy rule in the crate README.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo test -p alife_world`.
- [ ] Run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`.
- [ ] Commit the final coherent slice and verify `git status --short` is empty.
