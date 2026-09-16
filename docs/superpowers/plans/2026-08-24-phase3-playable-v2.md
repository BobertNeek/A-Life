# Phase 3 Playable v2.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and prove the first six-creature, persistent, GPU-authoritative A-Life v2.0 New Game and playable loop.

**Architecture:** `alife_world` creates and admits canonical founders and world objects. `GpuLiveBrainRuntime` gives those founders production GPU cognition, personal memory, local learning, sleep, and exact checkpoints. `alife_game_app` coordinates launch and projects canonical state into the existing Bevy voxel UI without owning creature truth.

**Tech Stack:** Rust 2021, Bevy, wgpu/Vulkan, WGSL, serde, existing content-addressed GPU checkpoint storage.

**Spec:** `docs/superpowers/specs/2026-08-24-phase3-playable-v2-design.md`

## Global Constraints

- A-Life v2.0 and stable `AOA-*` requirements are controlling.
- Founder count accepts 4 through 8 and defaults to exactly 6.
- Phase 3 uses the existing promoted Nano512 foundation and GroundedObjectSlotsV1 sensor profile.
- Production neural execution remains GPU-authoritative WGSL with no CPU neural fallback.
- The world owns legality and physical outcomes. It emits no reward or semantic utility score.
- Do not add host-authored food seeking, hazard avoidance, or scripted survival behavior.
- Do not train, pretrain, fine-tune, evolve, rank, or deliberately optimize any neural weights.
- Run Cargo serially with `-j 1` and keep GPU work serialized.
- Preserve the unrelated dirty `D:\A life\AGENTS.md` outside this worktree.
- Commit and push each independently useful checkpoint.

---

## File structure

- `crates/alife_world/src/new_game.rs`: versioned New Game configuration, canonical world construction, founder admission, and bootstrap receipt.
- `crates/alife_world/tests/canonical_new_game.rs`: founder-count, ownership, deterministic identity, ecology, and failure atomicity contracts.
- `crates/alife_game_app/src/new_game_lifecycle.rs`: runtime configuration, initial save construction, GPU residency, archive attachment, exact checkpoint publication, and rollback.
- `crates/alife_game_app/tests/canonical_new_game_lifecycle.rs`: CPU-stage lifecycle contracts and GPU-gated durable completion smoke.
- `crates/alife_game_app/src/bin/alife_game_app.rs`: `--new-game`, `--seed`, and exact population CLI parsing.
- `crates/alife_game_app/src/production_voxel_frontend.rs`: explicit load-existing versus New Game launch source.
- `crates/alife_game_app/src/bevy_shell.rs`: calls the lifecycle coordinator before renderer setup and keeps the exact runtime save path.
- `crates/alife_game_app/src/gpu_live_runtime.rs`: player world-edit transaction and richer read-only cognitive projection.
- `crates/alife_game_app/src/production_voxel_renderer.rs`: `E` resource placement input and canonical inspector text.
- `crates/alife_game_app/tests/phase3_playable_loop.rs`: focused causal loop and exact save/load contracts.
- `docs/DEVELOPMENT.md`: exact Phase 3 New Game and relaunch commands.

---

### Task 1: Canonical world-owned New Game bootstrap

**Requirements:** AOA-INV-011, AOA-AUTH-001, AOA-WORLD-001..004, AOA-BODY-001, AOA-FOUND-002, AOA-PERSIST-001.

**Files:**
- Create: `crates/alife_world/src/new_game.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Test: `crates/alife_world/tests/canonical_new_game.rs`

**Interfaces:**
- Consumes: `FoundationWeightAsset`, `CreatureGenome::early_mammal_founder`, `WorldOrganismRecord::newborn`, `HeadlessWorld`, and `CreatureSaveState`.
- Produces: `CanonicalNewGameConfig`, `CanonicalNewGame`, `CanonicalNewGameReceipt`, and `create_canonical_new_game`.

- [ ] **Step 1: Write the failing configuration and population tests**

```rust
#[test]
fn canonical_new_game_creates_exact_requested_population() {
    let foundation = FoundationWeightAsset::builtin_nano512_v1(
        SensorProfile::GroundedObjectSlotsV1,
    ).unwrap();
    for population in [4, 6, 8] {
        let game = create_canonical_new_game(
            &CanonicalNewGameConfig::phase3(240_824, population).unwrap(),
            &foundation,
        ).unwrap();
        assert_eq!(game.receipt.requested_population, population);
        assert_eq!(game.world.organism_registry().len(), usize::from(population));
        assert_eq!(game.creatures.len(), usize::from(population));
        assert_eq!(game.receipt.founders.len(), usize::from(population));
    }
}

#[test]
fn canonical_new_game_rejects_population_outside_phase3_bounds() {
    assert!(CanonicalNewGameConfig::phase3(1, 3).is_err());
    assert!(CanonicalNewGameConfig::phase3(1, 9).is_err());
}
```

- [ ] **Step 2: Run the RED gate**

Run: `cargo test -p alife_world --test canonical_new_game -j 1`

Expected: compile failure because the New Game module and types do not exist.

- [ ] **Step 3: Implement the versioned configuration and receipt**

```rust
pub const PHASE3_NEW_GAME_SCHEMA_VERSION: u16 = 1;
pub const PHASE3_DEFAULT_POPULATION: u16 = 6;

pub struct CanonicalNewGameConfig {
    pub schema_version: u16,
    pub world_seed: u64,
    pub founder_count: u16,
    pub brain_class: BrainScaleTier,
    pub sensor_profile: SensorProfile,
}

impl CanonicalNewGameConfig {
    pub fn phase3(world_seed: u64, founder_count: u16)
        -> Result<Self, ScaffoldContractError>;
}

pub fn create_canonical_new_game(
    config: &CanonicalNewGameConfig,
    foundation: &FoundationWeightAsset,
) -> Result<CanonicalNewGame, ScaffoldContractError>;
```

Validate nonzero seed, 4..=8 founders, Nano512 class, GroundedObjectSlotsV1, and exact foundation manifest identity before constructing the candidate world.

- [ ] **Step 4: Construct founders through the ordinary path**

For each deterministic founder seed, call `CreatureGenome::early_mammal_founder`, `express`, create one agent object, call `WorldOrganismRecord::newborn`, register the record, register the default Wild habitat membership, and build `CreatureSaveState` from the resulting canonical record. Give every founder a deterministic `CreatureAppearanceGenome::founder_for_species` value and stable world label.

- [ ] **Step 5: Add ordinary resources, hazards, obstacles, and ecology**

Create at least eight food objects, two tracked renewable resources, two hazards, and two obstacles. Use `HeadlessWorld` or `HeadlessScenarioBuilder` APIs only. Do not add action scores.

- [ ] **Step 6: Add deterministic and failure-atomicity tests**

```rust
#[test]
fn canonical_new_game_binds_every_subsystem_before_admission() {
    let game = phase3_game(6);
    for founder in &game.receipt.founders {
        let record = game.world.organism_registry().get(founder.organism_id).unwrap();
        record.validate_contract().unwrap();
        assert_eq!(record.genome().id, founder.genome_id);
        assert_eq!(record.phenotype().phenotype_hash, founder.phenotype_hash);
        assert_eq!(record.state_graph().organism_id, founder.organism_id);
        assert!(game.world.habitat_authority().membership(founder.organism_id).is_some());
    }
}
```

- [ ] **Step 7: Run GREEN and commit**

Run: `cargo test -p alife_world --test canonical_new_game -j 1`

Commit: `feat(world): add canonical Phase 3 new game bootstrap`

Push the branch.

---

### Task 2: Exact GPU-owned New Game lifecycle

**Requirements:** AOA-BRAIN-005, AOA-BRAIN-007, AOA-FOUND-002, AOA-ART-003, AOA-PERSIST-001..004, AOA-FAIL-002.

**Files:**
- Create: `crates/alife_game_app/src/new_game_lifecycle.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Test: `crates/alife_game_app/tests/canonical_new_game_lifecycle.rs`

**Interfaces:**
- Consumes: `CanonicalNewGame`, validated `RuntimeConfig`, `AssetManifest`, `GpuClosedLoopBackend`, `GpuLiveBrainRuntime::new_profiled`, `attach_lineage_archive`, `attach_durable_checkpoint_boundary`, and `capture_portable_checkpoint`.
- Produces: `CanonicalNewGameLaunchRequest`, `CanonicalNewGameLaunchResult`, and `create_canonical_new_game_runtime`.

- [ ] **Step 1: Write the failing CPU-stage lifecycle test**

```rust
#[test]
fn new_game_base_save_matches_every_canonical_founder() {
    let staged = stage_phase3_new_game(phase3_request(temp_root(), 6)).unwrap();
    assert_eq!(staged.save.creatures.len(), 6);
    assert_eq!(staged.save.world.organism_records.as_ref().unwrap().len(), 6);
    assert!(staged.save.creatures.iter().all(|creature| creature.gpu_brain.is_none()));
    staged.save.validate_with_asset_root(&staged.asset_root).unwrap();
}
```

- [ ] **Step 2: Run the RED gate**

Run: `cargo test -p alife_game_app --test canonical_new_game_lifecycle -j 1`

Expected: compile failure because the lifecycle API does not exist.

- [ ] **Step 3: Implement staging without frontend synthesis**

```rust
pub struct CanonicalNewGameLaunchRequest {
    pub world_seed: u64,
    pub population: u16,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
    pub config: RuntimeConfig,
    pub assets: AssetManifest,
}

pub struct CanonicalNewGameLaunchResult {
    pub runtime: GpuLiveBrainRuntime,
    pub exact_save: PortableSaveFile,
    pub save_path: PathBuf,
    pub receipt: CanonicalNewGameReceipt,
}
```

Reject an existing target path. Build the canonical world, create the base save, validate requested versus created population, and do not mutate the production fixture.

- [ ] **Step 4: Complete actual GPU admission and exact publication**

Construct `GpuLiveBrainRuntime::new_profiled` with the real backend. Require one handle, memory sidecar, topology sidecar, and resident record per founder. Attach the lineage archive. Attach durability at a hidden staging save path, capture the exact checkpoint, publish it, reopen it, require `gpu_brain.is_some()` for all creatures, then atomically rename the validated save into the requested player path. Remove only the newly created staging manifest when the transaction fails.

- [ ] **Step 5: Add the GPU-gated lifecycle smoke**

```rust
#[test]
fn gpu_new_game_publishes_only_exact_resident_state() {
    let result = run_gpu_phase3_new_game(4).unwrap();
    assert_eq!(result.runtime.world_snapshot().organism_registry().len(), 4);
    assert!(result.exact_save.creatures.iter().all(|creature| creature.gpu_brain.is_some()));
    let reopened = PortableSaveFile::from_json_file(&result.save_path).unwrap();
    assert_eq!(reopened, result.exact_save);
}
```

- [ ] **Step 6: Run GREEN and commit**

Run: `cargo test -p alife_game_app --test canonical_new_game_lifecycle -j 1`

Commit: `feat(game): create exact GPU-owned new games`

Push the branch.

---

### Task 3: One-command production New Game launch

**Requirements:** AOA-PORT-004, AOA-GAME-001, AOA-PERSIST-004, AOA-ART-005.

**Files:**
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`
- Modify: `crates/alife_game_app/src/production_voxel_frontend.rs`
- Modify: `crates/alife_game_app/src/bevy_shell.rs`
- Test: unit tests in the three modules.

**Interfaces:**
- Consumes: `CanonicalNewGameLaunchRequest` and `create_canonical_new_game_runtime`.
- Produces: `ProductionWorldSource::{LoadExisting, NewGame}`, `--new-game`, and `--seed`.

- [ ] **Step 1: Write failing CLI and preflight tests**

```rust
#[test]
fn production_cli_parses_canonical_new_game() {
    let launch = parse_launch(&args(&[
        "--new-game", "--population", "6", "--seed", "240824"
    ]), false).unwrap();
    assert!(matches!(launch.world_source, ProductionWorldSource::NewGame { seed: 240824 }));
    assert_eq!(launch.effective_population(), 6);
}
```

Also assert that `--new-game` without `--seed` fails, load-existing keeps its current behavior, and mismatched population never scales a save.

- [ ] **Step 2: Run the RED gate**

Run: `cargo test -p alife_game_app production_cli_parses_canonical_new_game --features production-voxel-frontend -j 1`

- [ ] **Step 3: Implement explicit world-source launch state**

```rust
pub enum ProductionWorldSource {
    LoadExisting,
    NewGame { seed: u64 },
}
```

New Game defaults to population 6. Existing-save launch derives or checks its exact canonical population. The Bevy shell runs the lifecycle transaction before scene construction and stores the resulting exact save path in the launch summary.

- [ ] **Step 4: Run focused GREEN tests and compile the product feature**

Run: `cargo test -p alife_game_app production_voxel_frontend --features production-voxel-frontend -j 1`

Run: `cargo check -p alife_game_app --features production-voxel-frontend -j 1`

- [ ] **Step 5: Commit and push**

Commit: `feat(game): launch canonical new games from production CLI`

---

### Task 4: Player resource placement through world authority

**Requirements:** AOA-WORLD-001, AOA-WORLD-003, AOA-AUTH-004, AOA-INV-009.

**Files:**
- Modify: `crates/alife_world/src/headless.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs`
- Test: `crates/alife_game_app/tests/phase3_playable_loop.rs`

**Interfaces:**
- Consumes: selected terrain position and `WorldEditorSpawnSpec`.
- Produces: `PlayerResourcePlacementRequest`, `PlayerResourcePlacementReceipt`, and `GpuLiveBrainRuntime::place_player_food`.

- [ ] **Step 1: Write the failing world-authority test**

```rust
#[test]
fn player_food_placement_changes_canonical_world_only_after_validation() {
    let mut runtime = phase3_test_runtime(4);
    let before = runtime.world_snapshot().object_snapshots();
    let receipt = runtime.place_player_food(Vec3f::new(2.0, 1.0, 0.0)).unwrap();
    let object = runtime.world_snapshot().entity(receipt.world_entity_id).unwrap();
    assert_eq!(object.kind, WorldObjectKind::Food);
    assert!(object.nutrition > 0.0);
    assert_eq!(runtime.world_snapshot().object_snapshots().len(), before.len() + 1);
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p alife_game_app --test phase3_playable_loop player_food_placement -j 1`

- [ ] **Step 3: Implement the validated transaction and `E` control**

The runtime clones the world, validates and inserts one food object with a deterministic stable label and spawn identity, optionally registers resource lifecycle, validates organism bindings, then commits the candidate. Bevy only supplies the selected terrain position and displays the receipt.

- [ ] **Step 4: Run GREEN, compile the product, commit, and push**

Run: `cargo test -p alife_game_app --test phase3_playable_loop player_food_placement -j 1`

Run: `cargo check -p alife_game_app --features production-voxel-frontend -j 1`

Commit: `feat(game): place food through canonical world authority`

---

### Task 5: Canonical creature inspector

**Requirements:** AOA-GAME-007, AOA-GAME-008, AOA-OBS-001..003, AOA-GOAL-007.

**Files:**
- Modify: `crates/alife_game_app/src/live_brain_bridge.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs`
- Test: unit tests in `production_voxel_renderer.rs` and `gpu_live_runtime.rs`.

**Interfaces:**
- Consumes: `WorldOrganismPresentationRow`, resident cognitive context, memory sidecar, topology sidecar, and subsystem state graph.
- Produces: expanded `LiveCognitivePresentationSnapshot` and `phase3_creature_inspector_text`.

- [ ] **Step 1: Write the failing inspector projection test**

```rust
#[test]
fn inspector_reports_identity_biology_cognition_memory_and_consistency() {
    let text = phase3_creature_inspector_text(&world_row(), &cognitive_snapshot());
    for required in [
        "organism", "genome", "phenotype", "age", "development",
        "organs", "energy", "hunger", "pain", "attention", "sleep",
        "memory", "learning", "state graph", "consistent",
    ] {
        assert!(text.to_ascii_lowercase().contains(required));
    }
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test -p alife_game_app inspector_reports_identity_biology_cognition_memory_and_consistency -j 1`

- [ ] **Step 3: Expand the read-only runtime snapshot**

Add stable attention target, selected action, memory counts, learning activity, sleep state, foundation and phenotype identity, typed organ values, and state-graph revisions. Never copy these into independently advancing UI state.

- [ ] **Step 4: Render unavailable evidence honestly**

The formatter prints `unavailable` for absent cognitive evidence and `inconsistent` only when canonical validation fails. It does not create default values.

- [ ] **Step 5: Run GREEN, commit, and push**

Run: `cargo test -p alife_game_app inspector_reports_identity_biology_cognition_memory_and_consistency -j 1`

Run: `cargo check -p alife_game_app --features production-voxel-frontend -j 1`

Commit: `feat(game): inspect canonical live organism state`

---

### Task 6: Causal resource, hazard, sleep, and exact continuity proof

**Requirements:** AOA-WORLD-004, AOA-BIO-011, AOA-BIO-024, AOA-SLEEP-001..007, AOA-PERSIST-001..004, AOA-OBS-002.

**Files:**
- Modify: `crates/alife_game_app/tests/phase3_playable_loop.rs`
- Modify production files only for defects reproduced by a failing test.

**Interfaces:**
- Consumes: completed New Game lifecycle and production GPU tick.
- Produces: one bounded causal smoke receipt and one exact roundtrip receipt.

- [ ] **Step 1: Add focused food and hazard causal tests**

Assert that an ordinary registered food interaction changes resource material state, stomach or metabolic organ state, biochemistry, and homeostasis. Assert that ordinary hazard contact changes physical injury, pain/threat chemistry, biological value, and prediction residual. Require sealed production patches and no world reward field.

- [ ] **Step 2: Run RED, fix only reproduced gaps, and run GREEN**

Run: `cargo test -p alife_game_app --test phase3_playable_loop causal_resource_and_hazard_consequences -j 1`

- [ ] **Step 3: Add bounded sleep/wake test**

Drive one organism to natural or explicit recovery sleep through the production scheduler. Require entering sleep, one atomic consolidation receipt, restored fatigue or energy, and waking. Do not repeat trials to improve behavior.

- [ ] **Step 4: Add exact save/load continuation test**

Capture and compare organism IDs, genome IDs, phenotype hashes, body and organ state, biochemistry, embodiment revision, state-graph revisions, GPU checkpoint identities, memory digest, sleep state, lifecycle state, and world digest. Restore into a staging backend and require the next tick to preserve identity and causal sequence.

- [ ] **Step 5: Run all focused Phase 3 tests, commit, and push**

Run: `cargo test -p alife_world --test canonical_new_game -j 1`

Run: `cargo test -p alife_game_app --test canonical_new_game_lifecycle -j 1`

Run: `cargo test -p alife_game_app --test phase3_playable_loop -j 1`

Commit: `test(game): prove Phase 3 causal loop and continuity`

---

### Task 7: Product launch, visual receipt, and final documentation

**Requirements:** Phase 3 graphical launch, player actions, visual identity, exact instructions, and final receipt.

**Files:**
- Modify: `docs/DEVELOPMENT.md`
- Create: `docs/evidence/phase3-playable-v2/README.md`
- Create: bounded screenshots under `docs/evidence/phase3-playable-v2/` only if repository policy permits tracked visual receipts; otherwise record their external artifact paths.

**Interfaces:**
- Consumes: complete product feature and exact durable save.
- Produces: reproducible launch instructions and final evidence ledger.

- [ ] **Step 1: Run formatting, static boundaries, docs, and targeted build**

Run: `cargo fmt --all -- --check`

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`

Run: `cargo check -p alife_core -p alife_world -p alife_runtime -p alife_game_app --features alife_game_app/production-voxel-frontend -j 1`

- [ ] **Step 2: Run one short headless causal smoke**

Run the focused Phase 3 test binary once. Record exact test counts and hardware-independent causal receipts.

- [ ] **Step 3: Run one bounded Vulkan graphical smoke**

```powershell
cargo run -p alife_game_app --features production-voxel-frontend -- `
  production-voxel --new-game --population 6 --seed 240824 `
  --graphics-backend vulkan --require-gpu --smoke-seconds 30
```

Verify a physical Vulkan adapter, six distinct creatures, autonomous GPU-selected activity, selection and inspector, pause/resume, speed, `E` food placement, save, load, and sleep state visibility. Capture screenshots after real runtime ticks.

- [ ] **Step 4: Relaunch the exact save**

Run the documented existing-save command against the created durable save and verify the same six organism, genome, phenotype, body, brain, memory, sleep, and lifecycle identities.

- [ ] **Step 5: Document exact commands and evidence**

Record merged-main SHA, Phase 3 SHA, adapter/backend, player actions, visible autonomous behavior, causal systems, save/load result, screenshot paths, blockers, teacher status, breeding status, and explicit no-training confirmation.

- [ ] **Step 6: Final review, commit, push, and stop**

Run `git diff --check`, inspect `git status --short`, compare `origin/main...HEAD`, stage only Phase 3 files, commit `docs: record Phase 3 playable v2 evidence`, push, and stop. Do not begin teacher interaction, reproduction, scaling, or training.
