# Modular Creature Mesh Assembly Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace whole-animal creature rendering with an extensible, heritable assembly of deterministically sliced head, torso, limb, tail, and hidden-join parts.

**Architecture:** `alife_world` owns only stable renderer-neutral part-family genes and deterministic inheritance/migration. `alife_game_app` owns the versioned part catalog, generated part-pack loader, Bevy handle cache, assembly recipes, and child-entity projection. `alife_tools` consumes licensed source OBJs plus catalog cut profiles and emits deterministic named-part OBJs, socket metadata, previews, and production manifest records; runtime never performs geometric slicing.

**Tech Stack:** Rust 2024 workspace, serde/serde_json, Bevy `=0.18.0`, existing OBJ/PNG production assets, deterministic offline mesh processing in `alife_tools`, PowerShell validation on Windows 10.

## Global Constraints

- Preserve the approved design in `docs/superpowers/specs/2026-07-12-modular-creature-mesh-assembly-design.md`.
- Use `CreaturePartFamilyId(u16)` values that are append-only and never derived from array order or filenames.
- Keep Bevy, wgpu, mesh, material, socket-transform, and renderer-resource types out of `alife_core` and `alife_world`.
- Keep creature part genes renderer-neutral, saved, inherited, mutated, and schema-versioned in `alife_world`.
- Runtime visuals remain display-only and cannot authorize actions, cognition, rewards, weights, locomotion, sensing, or physics outcomes.
- Use the existing real save, population, backend selection, GPU/fallback, and production launch paths; no mock simulation or fake backend.
- Founders and schema-v1 migrations are coherent; normal mutation is compatibility-constrained; rare mutation may replace at most one incompatible part.
- Every assembly remains bipedal with one head, one torso, paired arms, paired grounded legs, optional tail/back feature, finite bounded transforms, and hidden joins.
- Preserve original source UVs and normals through slicing; do not copy a crossing triangle into two visible parts.
- Source assets may remain in a developer-only licensed source directory, but production packaging includes only generated part packs and required textures/catalog metadata.
- Do not commit archives, Blender caches, generated previews, screenshots, temporary meshes, or target artifacts.
- Keep every production asset at most 512 KiB and the total production manifest at most 8 MiB unless a separately reviewed plan changes those bounds.
- Use `-j 1` for Bevy-heavy builds and tests on this machine.
- Do not claim visual completion without fresh inspected 1920x1080 minimum and comfort screenshots.

---

## File Responsibility Map

| File | Responsibility |
|---|---|
| `crates/alife_world/src/appearance.rs` | Schema-v2 part-family IDs, migration, inheritance, mutation, and validation. |
| `crates/alife_world/src/persistence.rs` | Deserialize/migrate appearance schema without renderer dependencies. |
| `crates/alife_world/tests/save_load_roundtrip.rs` | Schema-v1 migration and schema-v2 roundtrip evidence. |
| `crates/alife_game_app/src/creature_part_catalog.rs` | Shared serde catalog contract, stable IDs, compatibility queries, paths, and socket scalar data. |
| `crates/alife_game_app/src/creature_part_assets.rs` | Named-part OBJ parsing, runtime mesh conversion, texture lookup, and handle cache. |
| `crates/alife_game_app/src/creature_assembly.rs` | Pure assembly recipe resolution, bounds checks, fallback, join-cover recipes, and Bevy child spawning. |
| `crates/alife_game_app/src/production_voxel_renderer.rs` | Remove whole-animal path and delegate creature spawning to assembly modules. |
| `crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json` | Append-only family registry, compatibility, source/output paths, LODs, cut profiles, and sockets. |
| `crates/alife_game_app/assets/source_creature_meshes/` | Developer-only licensed whole source OBJs; excluded from production packaging. |
| `crates/alife_game_app/assets/production_voxel_v1/creature_parts/generated/` | Deterministic named-part runtime OBJs and socket manifests. |
| `crates/alife_tools/src/creature_part_builder.rs` | OBJ model, clipping, slot ownership, output generation, determinism, and validation. |
| `crates/alife_tools/src/bin/creature_part_builder.rs` | `analyze`, `build`, `validate`, `preview`, and `manifest` CLI. |
| `crates/alife_game_app/tests/fvr03_voxel_renderer.rs` | Runtime hierarchy, handle reuse, LOD, display-only, and visual-contract integration tests. |
| `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md` | Final modular-creature strategy, screenshots, receipts, and remaining limitations. |

---

### Task 1: Add Stable Schema-V2 Part Genes And Migration

**Files:**

- Modify: `crates/alife_world/src/appearance.rs:9-261`
- Modify: `crates/alife_world/src/persistence.rs:740-790`
- Modify: `crates/alife_world/tests/save_load_roundtrip.rs`
- Modify: `crates/alife_game_app/src/lifecycle_lineage.rs`
- Modify: `crates/alife_game_app/tests/app_shell.rs`

**Interfaces:**

- Consumes: existing `CreatureAppearanceGenome`, `offspring_from_parents`, serde save loading, and deterministic mutation seeds.
- Produces: `CreaturePartFamilyId`, `CreaturePartSources`, schema-v2 `CreatureAppearanceGenome`, `migrate_v1_part_sources`, and per-slot parental inheritance.

- [ ] **Step 1: Write failing schema and migration tests**

Add tests that deserialize a literal schema-v1 appearance record and assert coherent schema-v2 family IDs, then roundtrip a mixed schema-v2 record:

```rust
#[test]
fn schema_v1_appearance_migrates_to_coherent_schema_v2_parts() {
    let json = r#"{
        "schema_version":1,"species_archetype":5,"palette_family":3,
        "fur_pattern":4,"marking_density":8,"accessory_trait":2,
        "ear_muzzle_trait":6,"tail_trait":7,"body_mass_trait":9,
        "mutation_count":0,"bipedal_caveman_furry":true
    }"#;
    let appearance: CreatureAppearanceGenome = serde_json::from_str(json).unwrap();
    assert_eq!(appearance.schema_version, CREATURE_APPEARANCE_SCHEMA_VERSION);
    assert_eq!(appearance.part_sources, CreaturePartSources::coherent(CreaturePartFamilyId(5)));
    assert_eq!(appearance.palette_family, 3);
}

#[test]
fn schema_v2_mixed_parts_roundtrip_without_renderer_types() {
    let mut appearance = CreatureAppearanceGenome::founder_for_species(2, 99);
    appearance.part_sources = CreaturePartSources {
        head: CreaturePartFamilyId(2), torso: CreaturePartFamilyId(2),
        arms: CreaturePartFamilyId(6), legs: CreaturePartFamilyId(2),
        tail: CreaturePartFamilyId(7),
    };
    let json = serde_json::to_string(&appearance).unwrap();
    assert_eq!(serde_json::from_str::<CreatureAppearanceGenome>(&json).unwrap(), appearance);
    assert!(!json.to_ascii_lowercase().contains("bevy"));
    assert!(!json.to_ascii_lowercase().contains("mesh"));
}
```

- [ ] **Step 2: Run the tests and verify the intended red state**

Run:

```powershell
cargo test -p alife_world appearance -- --nocapture
cargo test -p alife_world --test save_load_roundtrip -- --nocapture
```

Expected: compilation fails because `CreaturePartFamilyId`, `CreaturePartSources`, and schema-v2 deserialization do not exist.

- [ ] **Step 3: Add renderer-neutral ID and source contracts**

Implement in `appearance.rs`:

```rust
pub const CREATURE_APPEARANCE_SCHEMA_VERSION: u16 = 2;
pub const INITIAL_CREATURE_PART_FAMILY_COUNT: u16 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CreaturePartFamilyId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreaturePartSources {
    pub head: CreaturePartFamilyId,
    pub torso: CreaturePartFamilyId,
    pub arms: CreaturePartFamilyId,
    pub legs: CreaturePartFamilyId,
    pub tail: CreaturePartFamilyId,
}

impl CreaturePartSources {
    pub const fn coherent(family: CreaturePartFamilyId) -> Self {
        Self { head: family, torso: family, arms: family, legs: family, tail: family }
    }

    pub fn distinct_family_count(self) -> usize {
        [self.head, self.torso, self.arms, self.legs, self.tail]
            .into_iter().collect::<std::collections::BTreeSet<_>>().len()
    }

    pub fn iter_slots(self) -> [(CreaturePartSlotKey, CreaturePartFamilyId); 5] {
        [
            (CreaturePartSlotKey::Head, self.head),
            (CreaturePartSlotKey::Torso, self.torso),
            (CreaturePartSlotKey::Arms, self.arms),
            (CreaturePartSlotKey::Legs, self.legs),
            (CreaturePartSlotKey::Tail, self.tail),
        ]
    }
}
```

Define `CreaturePartSlotKey` beside `CreaturePartSources` as a renderer-neutral
five-value enum used only for genetics iteration. It must not contain asset
paths, sockets, or renderer concepts.

Add `part_sources: CreaturePartSources` to `CreatureAppearanceGenome`. Use a custom `Deserialize` implementation or an untagged private wire enum so schema-v1 records migrate deterministically from `species_archetype % 8`, while malformed schema-v2 records fail validation.

- [ ] **Step 4: Extend founder, validation, signatures, and inheritance**

Founders use:

```rust
let source_family = CreaturePartFamilyId(u16::from(species_archetype % 8));
part_sources: CreaturePartSources::coherent(source_family),
```

Add all five IDs to `signature_line`. Validate IDs as non-sentinel `u16` values without imposing a current-family-count upper bound; catalog membership is app-owned. Update `inherited_from` to require each child source to equal a parent source or be marked by the deterministic mutation path.

- [ ] **Step 5: Implement deterministic per-slot parental inheritance**

Add a private helper that chooses each `u16` source without using the old `u8` gene bucket modulus:

```rust
fn choose_family(parent_a: CreaturePartFamilyId, parent_b: CreaturePartFamilyId, seed: u64, salt: u64) -> CreaturePartFamilyId {
    if mix_seed(seed, salt, u64::from(parent_a.0), u64::from(parent_b.0)) & 1 == 0 {
        parent_a
    } else {
        parent_b
    }
}
```

`offspring_from_parents` inherits all five slots independently. Do not perform compatibility substitution in `alife_world`; catalog-aware mutation is Task 3 and is invoked by the app lifecycle before the birth save is finalized.

- [ ] **Step 6: Update lifecycle tests**

Strengthen `lifecycle_lineage_birth_inherits_and_mutates_appearance_genes` to assert every child slot is parental before catalog mutation and that save/load preserves the resulting sources.

- [ ] **Step 7: Run focused and boundary validation**

```powershell
cargo test -p alife_world appearance -- --nocapture
cargo test -p alife_world --test save_load_roundtrip -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
```

Expected: all pass; no renderer strings or types appear in serialized world contracts.

- [ ] **Step 8: Commit**

```powershell
git add crates/alife_world crates/alife_game_app/src/lifecycle_lineage.rs crates/alife_game_app/tests/app_shell.rs
git commit -m "Add heritable modular creature part genes"
```

---

### Task 2: Define The Append-Only Part Catalog

**Files:**

- Create: `crates/alife_game_app/src/creature_part_catalog.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Create: `crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json`
- Test: `crates/alife_game_app/src/creature_part_catalog.rs`

**Interfaces:**

- Consumes: `CreaturePartFamilyId` from Task 1 and package-relative production asset paths.
- Produces: `CreaturePartCatalog`, `CreaturePartFamilyDefinition`, `CreaturePartSlot`, `CreaturePartLod`, `SocketFrame`, `CutProfile`, `CompatibilityPolicy`, and `load_production_creature_part_catalog`.

- [ ] **Step 1: Write failing catalog tests**

```rust
#[test]
fn production_catalog_has_append_only_unique_ids_and_all_required_lods() {
    let catalog = load_production_creature_part_catalog().unwrap();
    assert_eq!(catalog.schema, CREATURE_PART_CATALOG_SCHEMA);
    assert_eq!(catalog.families.len(), 8);
    assert_eq!(catalog.families.iter().map(|f| f.id).collect::<BTreeSet<_>>().len(), 8);
    for family in &catalog.families {
        assert_eq!(family.lods.len(), 3);
        assert!(family.compatibility_tags.len() >= 2);
        family.validate().unwrap();
    }
}

#[test]
fn synthetic_ninth_family_requires_no_rust_match_arm() {
    let mut catalog = load_production_creature_part_catalog().unwrap();
    let mut ninth = catalog.families[0].clone();
    ninth.id = CreaturePartFamilyId(100);
    ninth.label = "future-family".into();
    catalog.families.push(ninth);
    catalog.validate().unwrap();
    assert_eq!(catalog.family(CreaturePartFamilyId(100)).unwrap().label, "future-family");
}
```

- [ ] **Step 2: Run the catalog tests red**

```powershell
cargo test -p alife_game_app creature_part_catalog -j 1 -- --nocapture
```

Expected: fail because the module and catalog do not exist.

- [ ] **Step 3: Implement catalog types**

Use renderer-independent scalar arrays in the serialized contract, converting to Bevy types only in Task 6:

```rust
pub const CREATURE_PART_CATALOG_SCHEMA: &str = "alife.creature_part_catalog.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CreaturePartSlot { Head, Torso, LeftArm, RightArm, LeftLeg, RightLeg, TailBack }

impl CreaturePartSlot {
    pub const REQUIRED_RUNTIME_SLOTS: [Self; 6] = [
        Self::Head, Self::Torso, Self::LeftArm, Self::RightArm,
        Self::LeftLeg, Self::RightLeg,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CreaturePartLodId { Full, Compact, Impostor }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocketFrame { pub translation: [f32; 3], pub rotation_xyzw: [f32; 4], pub scale: [f32; 3] }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartLod {
    pub lod: String,
    pub source_obj: String,
    pub generated_obj: String,
    pub socket_manifest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreaturePartFamilyDefinition {
    pub id: CreaturePartFamilyId,
    pub label: String,
    pub texture_asset: String,
    pub compatibility_tags: BTreeSet<String>,
    pub ordinary_substitutions: BTreeMap<CreaturePartSlot, BTreeSet<String>>,
    pub source_to_canonical: SocketFrame,
    pub cuts: BTreeMap<CreaturePartSlot, CutVolume>,
    pub sockets: BTreeMap<String, SocketFrame>,
    pub join_covers: Vec<JoinCoverDefinition>,
    pub lods: Vec<CreaturePartLod>,
}
```

Validate finite values, normalized quaternions within `1e-3`, scale components in `0.25..=4.0`, exactly three unique LOD labels, package-relative output paths, developer-source paths outside the production root, all required slots, paired socket names, and append-only unique IDs. Reject unknown slot tags instead of ignoring them.

- [ ] **Step 4: Add the initial eight-family catalog**

Assign immutable IDs:

| ID | Label | Base tags |
|---:|---|---|
| 0 | `colobus` | `mammalian`, `long-arm`, `plume-tail` |
| 1 | `gecko` | `compact`, `scaled`, `long-tail` |
| 2 | `herring` | `aquatic`, `fin-arm`, `tailless` |
| 3 | `inkfish` | `aquatic`, `tentacle-arm`, `soft-body` |
| 4 | `muskrat` | `mammalian`, `compact`, `aquatic` |
| 5 | `pudu` | `mammalian`, `heavy-torso`, `short-tail` |
| 6 | `sparrow` | `compact`, `wing-arm`, `plume-tail` |
| 7 | `taipan` | `scaled`, `long-body`, `tailless` |

Use canonical normalized socket names `neck`, `left-shoulder`, `right-shoulder`, `left-hip`, `right-hip`, and `tail-base`. Use the same output naming template for every family and LOD:

```text
production_voxel_v1/creature_parts/generated/<family>_<lod>_parts.obj
production_voxel_v1/creature_parts/generated/<family>_<lod>_sockets.json
```

Cut profiles use normalized source coordinates after `source_to_canonical`. Each slot owns a convex set of half-spaces; the profile explicitly records mirrored left/right volumes. Do not infer anatomy from labels at runtime.

- [ ] **Step 5: Implement compatibility queries**

```rust
pub fn ordinarily_compatible(&self, torso: CreaturePartFamilyId, slot: CreaturePartSlot, candidate: CreaturePartFamilyId) -> bool;
pub fn coherent_fallback(&self, requested: CreaturePartFamilyId) -> CreaturePartFamilyId;
pub fn texture_path(&self, family: CreaturePartFamilyId) -> Result<&str, CreaturePartCatalogError>;
```

Compatibility succeeds for the same family or when the candidate contains every tag listed by the torso's `ordinary_substitutions[slot]`. Rare mutation bypasses this query but not assembly validation.

- [ ] **Step 6: Run focused tests and commit**

```powershell
cargo test -p alife_game_app creature_part_catalog -j 1 -- --nocapture
git add crates/alife_game_app/src/creature_part_catalog.rs crates/alife_game_app/src/lib.rs crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json
git commit -m "Define extensible creature part catalog"
```

---

### Task 3: Add Catalog-Aware Compatible And Rare Mutation

**Files:**

- Create: `crates/alife_game_app/src/creature_part_genetics.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/lifecycle_lineage.rs`
- Test: `crates/alife_game_app/src/creature_part_genetics.rs`
- Test: `crates/alife_game_app/tests/app_shell.rs`

**Interfaces:**

- Consumes: schema-v2 parental inheritance and `CreaturePartCatalog::ordinarily_compatible`.
- Produces: `mutate_creature_part_sources`, `CreaturePartMutationResult`, and lifecycle integration that mutates at most one incompatible slot.

- [ ] **Step 1: Write failing deterministic mutation tests**

```rust
#[test]
fn ordinary_mutation_never_violates_catalog_compatibility() {
    let catalog = test_catalog();
    let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(0));
    for seed in 0..10_000 {
        let result = mutate_creature_part_sources(inherited, 3, seed, &catalog).unwrap();
        if !result.rare_cross_family {
            assert!(part_sources_are_ordinary_compatible(&result.sources, &catalog));
        }
    }
}

#[test]
fn rare_mutation_replaces_at_most_one_incompatible_slot() {
    let catalog = test_catalog();
    let inherited = CreaturePartSources::coherent(CreaturePartFamilyId(0));
    let result = mutate_creature_part_sources(inherited, RARE_PART_MUTATION_THRESHOLD, 0xfeed, &catalog).unwrap();
    assert!(result.rare_cross_family);
    assert!(result.incompatible_slot_count <= 1);
}
```

- [ ] **Step 2: Run red tests**

```powershell
cargo test -p alife_game_app creature_part_genetics -j 1 -- --nocapture
```

Expected: missing module and functions.

- [ ] **Step 3: Implement mutation policy**

Define:

```rust
pub const RARE_PART_MUTATION_THRESHOLD: u16 = 8;

pub struct CreaturePartMutationResult {
    pub sources: CreaturePartSources,
    pub changed_slot: Option<CreaturePartSlot>,
    pub rare_cross_family: bool,
    pub incompatible_slot_count: u8,
}

pub fn mutate_creature_part_sources(
    inherited: CreaturePartSources,
    mutation_count: u16,
    mutation_seed: u64,
    catalog: &CreaturePartCatalog,
) -> Result<CreaturePartMutationResult, CreaturePartCatalogError>;

pub fn part_sources_are_ordinary_compatible(
    sources: &CreaturePartSources,
    catalog: &CreaturePartCatalog,
) -> bool;
```

Derive exactly one candidate slot and family from the deterministic seed. Below the threshold, choose only catalog-compatible candidates. At or above the threshold, use one seed bucket out of eight for rare mutation; replace at most one slot. Never replace the torso with an incompatible family because torso sockets define the assembly frame.
Map renderer-neutral `CreaturePartSlotKey::Arms` to both runtime arm slots and
`Legs` to both runtime leg slots inside this app-owned module; do not move the
seven-slot runtime mesh enum into `alife_world`.

- [ ] **Step 4: Integrate birth finalization**

After `offspring_from_parents`, invoke the catalog-aware function in `lifecycle_lineage.rs` using the existing deterministic birth mutation seed. Store only the resulting renderer-neutral sources. Do not pass Bevy types or asset handles through lifecycle code.

- [ ] **Step 5: Run lifecycle and boundary tests**

```powershell
cargo test -p alife_game_app creature_part_genetics -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
```

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_game_app/src/creature_part_genetics.rs crates/alife_game_app/src/lib.rs crates/alife_game_app/src/lifecycle_lineage.rs crates/alife_game_app/tests/app_shell.rs
git commit -m "Add compatible modular creature mutations"
```

---

### Task 4: Build Deterministic OBJ Slicing And Validation

**Files:**

- Create: `crates/alife_tools/src/creature_part_builder.rs`
- Modify: `crates/alife_tools/src/lib.rs`
- Modify: `crates/alife_tools/Cargo.toml`
- Test: `crates/alife_tools/src/creature_part_builder.rs`

**Interfaces:**

- Consumes: catalog definitions from `alife_game_app::creature_part_catalog`, licensed source OBJ bytes, and normalized cut half-spaces.
- Produces: `SourceObjMesh`, `SlicedCreaturePartPack`, `slice_creature_mesh`, `validate_sliced_pack`, and deterministic named-group OBJ/socket JSON bytes.

- [ ] **Step 1: Add the default-feature dependency needed to share catalog types**

In `alife_tools/Cargo.toml`:

```toml
alife_game_app = { path = "../alife_game_app", default-features = false }
```

Verify there is no reverse `alife_game_app -> alife_tools` dependency.

- [ ] **Step 2: Write failing parser, ownership, and determinism tests**

Use an in-memory triangulated humanoid fixture with UVs and normals:

```rust
#[test]
fn slicing_assigns_every_source_triangle_exactly_once() {
    let source = SourceObjMesh::parse(TEST_BIPED_OBJ).unwrap();
    let pack = slice_creature_mesh(&source, &test_family(), CreaturePartLodId::Compact).unwrap();
    assert_eq!(pack.source_triangle_count, source.triangles.len());
    assert_eq!(pack.source_triangle_owners.len(), source.triangles.len());
    assert!(pack.source_triangle_owners.values().all(|owners| owners.len() == 1));
    assert!(pack.parts.values().all(|part| part.indices.len() % 3 == 0));
}

#[test]
fn generated_bytes_are_deterministic() {
    let first = build_test_pack();
    let second = build_test_pack();
    assert_eq!(first.obj_bytes, second.obj_bytes);
    assert_eq!(first.socket_json_bytes, second.socket_json_bytes);
}
```

- [ ] **Step 3: Run tests red**

```powershell
cargo test -p alife_tools creature_part_builder -- --nocapture
```

Expected: missing builder module and dependency interfaces.

- [ ] **Step 4: Implement a structured OBJ parser**

Parse `v`, `vt`, `vn`, and polygonal `f` records into indexed triangles while preserving the original position/UV/normal tuple. Reject zero indices, out-of-range indices, non-finite values, faces with fewer than three vertices, and files with no triangles. Triangulate polygons with a stable fan order.

Use these core records:

```rust
pub struct ObjVertex { pub position: [f64; 3], pub uv: [f64; 2], pub normal: [f64; 3] }
pub struct ObjTriangle { pub vertices: [ObjVertex; 3], pub source_index: usize }
pub struct SourceObjMesh { pub triangles: Vec<ObjTriangle> }
pub struct SlicedCreaturePartPack {
    pub parts: BTreeMap<CreaturePartSlot, GeneratedPartMesh>,
    pub source_triangle_owners: BTreeMap<usize, BTreeSet<CreaturePartSlot>>,
    pub obj_bytes: Vec<u8>,
    pub socket_json_bytes: Vec<u8>,
}
```

- [ ] **Step 5: Implement deterministic convex-volume clipping**

For each source triangle, transform positions through `source_to_canonical`. Determine the owning slot by centroid against ordered cut volumes. Clip the triangle against that slot's convex half-spaces using Sutherland-Hodgman polygon clipping. Interpolate UVs and normals at cut intersections, normalize interpolated normals, triangulate the clipped polygon, and retain `source_index` ownership once.

If a centroid matches no volume or multiple equal-priority volumes, return an error containing family, LOD, and source triangle index. Slot priority is fixed and serialized as:

```text
Head, LeftArm, RightArm, LeftLeg, RightLeg, TailBack, Torso
```

Torso is the final catch volume but may not accept a centroid outside its declared bounds.

- [ ] **Step 6: Recenter parts on sockets and emit named OBJ groups**

Emit groups in stable slot order:

```text
o part_head
o part_torso
o part_left_arm
o part_right_arm
o part_left_leg
o part_right_leg
o part_tail_back
```

Each part's vertices are transformed into its attachment socket's local frame. Deduplicate vertices using exact generated f64 bit tuples after clipping, then serialize f32 with nine decimal places. Preserve UV orientation. Output uses LF line endings through `.gitattributes`.

- [ ] **Step 7: Validate sliced packs**

`validate_sliced_pack` rejects missing required groups, unpaired limbs, empty required parts, NaN/Inf, invalid normals, UVs outside `-0.001..=1.001`, indices outside vertex bounds, sockets outside canonical source bounds, feet whose lowest point differs by more than `0.04`, join overlap below `0.015`, and generated files over 512 KiB.

- [ ] **Step 8: Run tests and commit**

```powershell
cargo test -p alife_tools creature_part_builder -- --nocapture
cargo check --workspace --all-targets -j 1
git add Cargo.lock crates/alife_tools
git commit -m "Add deterministic creature mesh slicing"
```

---

### Task 5: Add Builder CLI, Generate Initial Part Packs, And Update Licensing

**Files:**

- Create: `crates/alife_tools/src/bin/creature_part_builder.rs`
- Create: `crates/alife_game_app/assets/source_creature_meshes/README.md`
- Move: `crates/alife_game_app/assets/production_voxel_v1/models/*_LOD*.obj` to `crates/alife_game_app/assets/source_creature_meshes/`
- Create: `crates/alife_game_app/assets/production_voxel_v1/creature_parts/generated/*`
- Modify: `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json`
- Modify: `crates/alife_game_app/assets/production_voxel_v1/models/ATTRIBUTION.md`
- Modify: `.gitattributes`
- Test: `crates/alife_tools/src/bin/creature_part_builder.rs`

**Interfaces:**

- Consumes: Task 4 builder library and Task 2 catalog.
- Produces: reproducible CLI, 24 named-part OBJ packs, 24 socket manifests, preview receipts, and complete production asset records.

- [ ] **Step 1: Write failing CLI contract tests**

Test argument parsing without spawning subprocesses:

```rust
#[test]
fn cli_supports_all_required_commands() {
    for command in ["analyze", "build", "validate", "preview", "manifest"] {
        assert!(CreaturePartBuilderCommand::parse_for_test([command]).is_ok());
    }
}
```

- [ ] **Step 2: Implement exact CLI command behavior**

```text
analyze  --catalog <path> --family <u16> --lod <full|compact|impostor> --json <target path>
build    --catalog <path> [--family <u16>] --staging <target path>
validate --catalog <path>
preview  --catalog <path> --family <u16> --lod <...> --output <target png>
manifest --catalog <path> --manifest <production manifest path>
```

`build` writes to `target/generated_art/creature_parts/staging`, validates every requested family, then copies successful outputs to their catalog paths. A failure leaves production paths unchanged. `preview` writes only under `target/artifacts/creature_parts/` and rejects workspace source paths.

- [ ] **Step 3: Move whole source OBJs to the developer-only directory**

Preserve filenames and LF normalization. Add a README recording that these files are licensed Omabuarts sources, are excluded from production packaging, and feed the deterministic builder. Keep textures in the production pack because generated parts retain source UVs.

- [ ] **Step 4: Calibrate and commit all eight cut profiles**

Run `analyze` for each family and LOD. Profiles operate in normalized canonical coordinates and must define all required slots. Use the blueprint as the visual target. Store final profile data in the catalog; do not put family-specific cut constants in Rust.

Required review loop for each family:

```powershell
cargo run -p alife_tools --bin creature_part_builder -- analyze --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json --family <id> --lod compact --json target/artifacts/creature_parts/<label>_analysis.json
cargo run -p alife_tools --bin creature_part_builder -- build --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json --family <id> --staging target/generated_art/creature_parts/staging
cargo run -p alife_tools --bin creature_part_builder -- preview --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json --family <id> --lod compact --output target/artifacts/creature_parts/<label>_compact.png
```

Inspect every compact preview before moving to the next family. Reject exposed holes outside overlap zones, missing recognizable source features, unpaired anatomy, or cuts through the face/hands/feet.

- [ ] **Step 5: Generate all production LOD packs twice**

Build all families, hash outputs, remove only the staging directory, build again, and assert identical path/digest pairs. Do not delete committed source or production directories during this check.

- [ ] **Step 6: Replace whole-OBJ manifest entries with generated part entries**

Each generated OBJ and socket manifest records source family, builder version, catalog path, source digest, CC-BY-4.0 attribution, output digest, size, generated=true, external=false, final_art=true, and replacement policy. Source OBJs receive developer-source attribution but are excluded from the runtime production manifest.

- [ ] **Step 7: Run builder and production asset validation**

```powershell
cargo test -p alife_tools --bin creature_part_builder -- --nocapture
cargo run -p alife_tools --bin creature_part_builder -- validate --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
```

Expected: 24 generated part OBJs and 24 socket manifests validate; no whole LOD OBJ is referenced by the production manifest; unknown-license, rejected, placeholder-final, and missing counts are zero; size bounds pass.

- [ ] **Step 8: Commit**

```powershell
git add .gitattributes crates/alife_tools/src/bin/creature_part_builder.rs crates/alife_game_app/assets/source_creature_meshes crates/alife_game_app/assets/production_voxel_v1
git status --short
git commit -m "Generate modular creature part asset packs"
```

---

### Task 6: Load Named Part Assets And Resolve Assembly Recipes

**Files:**

- Create: `crates/alife_game_app/src/creature_part_assets.rs`
- Create: `crates/alife_game_app/src/creature_assembly.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Test: `crates/alife_game_app/src/creature_part_assets.rs`
- Test: `crates/alife_game_app/src/creature_assembly.rs`

**Interfaces:**

- Consumes: generated named-part OBJs, socket manifests, catalog, schema-v2 part sources, and existing appearance colors.
- Produces: `CreaturePartAssetLibrary`, `CreatureAssemblyRecipe`, `resolve_creature_assembly`, `CreatureAssemblyWarning`, and cached Bevy mesh/material handles.

- [ ] **Step 1: Write failing named-group loader tests**

```rust
#[test]
fn generated_obj_loader_returns_all_required_named_parts() {
    let pack = parse_generated_part_obj(TEST_NAMED_PART_OBJ).unwrap();
    for slot in CreaturePartSlot::REQUIRED_RUNTIME_SLOTS {
        assert!(pack.parts.contains_key(&slot), "missing {slot:?}");
    }
    assert!(pack.parts.values().all(|mesh| !mesh.positions.is_empty()));
}
```

- [ ] **Step 2: Write failing assembly and fallback tests**

```rust
#[test]
fn mixed_recipe_uses_saved_sources_and_torso_sockets() {
    let sources = CreaturePartSources { head: id(1), torso: id(0), arms: id(6), legs: id(0), tail: id(7) };
    let recipe = resolve_creature_assembly(sources, CreaturePartLodId::Compact, &test_catalog()).unwrap();
    assert_eq!(recipe.root_family, id(0));
    assert_eq!(recipe.parts[&CreaturePartSlot::Head].family, id(1));
    assert!(recipe.join_covers.len() >= 5);
}

#[test]
fn unknown_family_uses_coherent_visible_fallback() {
    let recipe = resolve_creature_assembly(CreaturePartSources::coherent(id(999)), CreaturePartLodId::Compact, &test_catalog()).unwrap();
    assert!(recipe.warning.is_some());
    assert_eq!(recipe.parts.values().map(|p| p.family).collect::<BTreeSet<_>>().len(), 1);
}
```

- [ ] **Step 3: Implement the named-group parser**

Move generic OBJ parsing out of `production_voxel_renderer.rs`. Require exact `o part_*` group names and build one `PartMeshData` per group while retaining source UVs/normals. Reject unknown duplicate groups, faces before a group, and required empty groups.

- [ ] **Step 4: Implement pure assembly resolution**

Define:

```rust
pub struct CreatureAssemblyPartRecipe {
    pub family: CreaturePartFamilyId,
    pub slot: CreaturePartSlot,
    pub mesh_asset_path: String,
    pub texture_asset_path: String,
    pub socket: SocketFrame,
    pub local_scale: [f32; 3],
}

pub struct CreatureAssemblyRecipe {
    pub root_family: CreaturePartFamilyId,
    pub parts: BTreeMap<CreaturePartSlot, CreatureAssemblyPartRecipe>,
    pub join_covers: Vec<ResolvedJoinCover>,
    pub warning: Option<CreatureAssemblyWarning>,
    pub display_only: bool,
}
```

Resolve torso first, then attach other source families through torso socket names. Clamp inherited scale to the intersection of source and torso socket bounds. Reject non-finite transforms, missing paired slots, foot-height mismatch over `0.04`, or joins with no cover.

- [ ] **Step 5: Implement Bevy handle caching**

`CreaturePartAssetLibrary` keys meshes by `(family, lod, slot)` and materials by `(family, palette_family, fur_pattern, expression_bucket)`. Load and convert each generated OBJ once per LOD. Tests must prove 30 creatures reuse bounded handles rather than creating 30 copies per part.

- [ ] **Step 6: Implement join-cover mesh recipes**

Use shared app-local low-poly ruff/cuff/tuft meshes selected by catalog cover kind. Covers overlap adjacent parts by the catalog depth and use a blended tint from both source families. Keep covers below 12 per creature and omit hidden-detail covers in impostor LOD.

- [ ] **Step 7: Run focused tests and commit**

```powershell
cargo test -p alife_game_app creature_part_assets -j 1 -- --nocapture
cargo test -p alife_game_app creature_assembly -j 1 -- --nocapture
git add crates/alife_game_app/src/creature_part_assets.rs crates/alife_game_app/src/creature_assembly.rs crates/alife_game_app/src/lib.rs
git commit -m "Load and resolve modular creature assemblies"
```

---

### Task 7: Replace Whole-Mesh Rendering With Bevy Child Assemblies

**Files:**

- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs:2935-3520`
- Modify: `crates/alife_game_app/src/creature_assembly.rs`
- Modify: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

**Interfaces:**

- Consumes: `CreaturePartAssetLibrary`, `resolve_creature_assembly`, existing creature visual records, LOD profiles, expressions, and product camera composition.
- Produces: one creature root per visible creature, part child markers, join-cover markers, unchanged selection/animation roots, and updated diagnostics.

- [ ] **Step 1: Replace old integration assertions with failing modular assertions**

Add markers:

```rust
#[derive(Component)]
pub struct ProductionCreatureAssemblyRoot { pub stable_id: WorldEntityId, pub display_only: bool }

#[derive(Component)]
pub struct ProductionCreaturePartMarker { pub stable_id: WorldEntityId, pub family: CreaturePartFamilyId, pub slot: CreaturePartSlot }

#[derive(Component)]
pub struct ProductionCreatureJoinCoverMarker { pub stable_id: WorldEntityId, pub cover_kind: String }
```

Test 30 roots, required child slots per root, family diversity, shared handles, hidden joins, no whole-mesh marker, and display-only authority:

```rust
assert_eq!(root_count, 30);
assert!(parts_by_root.values().all(|parts| REQUIRED_PART_SLOTS.iter().all(|slot| parts.contains(slot))));
assert!(join_cover_count >= 30 * 5);
assert!(all_roots_display_only);
assert!(unique_mesh_handles < total_part_entities / 3);
```

- [ ] **Step 2: Run the modular renderer tests red**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer modular -j 1 -- --nocapture
```

Expected: old renderer spawns one whole mesh and no part hierarchy.

- [ ] **Step 3: Remove hardcoded family-name and whole-mesh paths**

Delete `Fvr10CreatureSpeciesMeshes`, `fvr10_creature_species_meshes`, the renderer-local synchronous OBJ parser, `fvr10_readable_cute_biped_mesh`, and both `match species % 8` blocks. Catalog lookup becomes the only source of mesh and texture paths.

- [ ] **Step 4: Spawn assembly roots and children**

Keep selection, stable IDs, base translation, root animation phase, and creature-level scale on the root entity. Spawn torso, head, paired limbs, optional tail/back feature, eyes/face details, and join covers as children with local transforms. Eye and face placement derives from the resolved head socket manifest, not fixed world offsets.

- [ ] **Step 5: Preserve animation and expression behavior**

Move bob, turn, sleep, fear, pain, and selection motion to the root transform. Apply arm/leg secondary motion only to part children through marker queries. Expression material changes remain display-only and use the correct source-family texture for each part.

- [ ] **Step 6: Update diagnostics**

Record:

```text
creature_visual_profile=modular-heritable-part-assembly-v1
creature_root_count
creature_part_entity_count
creature_join_cover_count
creature_part_family_count
creature_mixed_assembly_count
creature_shared_mesh_handle_count
production_visuals_display_only=true
```

- [ ] **Step 7: Run complete renderer and lineage suites**

```powershell
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
```

- [ ] **Step 8: Commit**

```powershell
git add crates/alife_game_app/src/production_voxel_renderer.rs crates/alife_game_app/src/creature_assembly.rs crates/alife_game_app/tests/fvr03_voxel_renderer.rs
git commit -m "Render creatures from heritable mesh parts"
```

---

### Task 8: Run The Visual Loop, Final Validation, And Integration

**Files:**

- Modify as screenshot evidence requires: `crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json`
- Modify as screenshot evidence requires: `crates/alife_game_app/src/creature_assembly.rs`
- Modify: `docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md`
- Generate only under ignored paths: `target/artifacts/creature_parts/` and `target/artifacts/fvr03/`

**Interfaces:**

- Consumes: final production release path, real saves, GPU/backend selection, approved modular blueprint, and accepted FVR11 terrain.
- Produces: inspected minimum/comfort screenshots, runtime receipts, final docs, reviewed commits, feature push, and safe merge.

- [ ] **Step 1: Run focused validation before the release build**

```powershell
cargo fmt --all -- --check
cargo test -p alife_world appearance -- --nocapture
cargo test -p alife_world --test save_load_roundtrip -- --nocapture
cargo test -p alife_tools creature_part_builder -- --nocapture
cargo test -p alife_game_app creature_part_catalog -j 1 -- --nocapture
cargo test -p alife_game_app creature_part_genetics -j 1 -- --nocapture
cargo test -p alife_game_app creature_assembly -j 1 -- --nocapture
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
```

Expected: all pass before visual iteration.

- [ ] **Step 2: Build the production release executable**

```powershell
cargo build -p alife_game_app --release --features "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi" --bin alife_game_app -j 1
```

- [ ] **Step 3: Capture minimum and comfort profiles**

```powershell
target\release\alife_game_app.exe production-voxel --profile MinimumSettings30x30 --population 30 --resolution 1920x1080 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance
target\release\alife_game_app.exe production-voxel --profile MinSpecComfort1080p --resolution 1920x1080 --gpu-mode auto-with-cpu-fallback --graphics-backend vulkan --record-performance
```

Inspect at original resolution:

```text
D:\A life\target\artifacts\fvr03\MinimumSettings30x30_runtime_screenshot.png
D:\A life\target\artifacts\fvr03\MinSpecComfort1080p_runtime_screenshot.png
```

- [ ] **Step 4: Compare against the blueprint and reject visible defects**

Require coherent founders plus visible inherited/mutated structural combinations. Reject recognizable untouched source animals, exposed holes, toy seams, floating parts, non-bipedal stance, feet above terrain, extreme scale mismatch, covers obscuring faces/hands/feet, broken source textures, debug overlays, or terrain regression.

Iterate one variable group at a time: socket translation, socket scale, cut profile, join overlap, cover silhouette, then material blending. Rebuild and recapture both profiles after any production visual change.

- [ ] **Step 5: Run the complete gate**

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo test -p alife_game_app --features "bevy-app voxel-backend" --test fvr03_voxel_renderer -j 1 -- --nocapture
cargo test -p alife_game_app --test app_shell lifecycle_lineage_birth_inherits_and_mutates_appearance_genes -j 1 -- --nocapture
cargo run -p alife_tools --bin creature_part_builder -- validate --catalog crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json
cargo run -p alife_game_app --bin alife_game_app -- validate-production-assets
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
```

- [ ] **Step 6: Audit runtime evidence**

Both receipts must report `real_save_loaded=true`, `mock_data_source=false`, `voxel_roundtrip=true`, actual backend/adapter/API/fallback values, `production_visuals_display_only=true`, bounded part/join counts, and measured local smoke FPS. Empty stderr and no missing-part or missing-texture warnings are required.

- [ ] **Step 7: Update the handoff**

Document schema migration, append-only catalog, current family IDs, source/license handling, builder commands, part-pack sizes/digests, runtime hierarchy, screenshots, GPU/fallback receipts, validations, and explicit renderer authority boundaries. State that no ADR changed because ownership boundaries remain intact.

- [ ] **Step 8: Refresh Graphify and request review**

```powershell
C:\Users\PC\.local\bin\graphify.exe update .
```

Use `superpowers:requesting-code-review`. Fix all Critical and Important findings and rerun the complete gate. Review must explicitly inspect schema migration, family-ID stability, geometric ownership, source licensing, runtime handle reuse, and screenshot evidence.

- [ ] **Step 9: Commit the accepted pass**

```powershell
git add crates/alife_world crates/alife_game_app crates/alife_tools Cargo.lock docs/productization_s_plans/fullstack_bevy_voxel_frontend_replacement/FVR10_VISUAL_GAME_LAYER_REDO_HANDOFF.md docs/superpowers
git status --short
git commit -m "Complete modular creature mesh assembly"
```

Unstage target artifacts, previews, screenshots, archives, caches, and unrelated files before committing.

- [ ] **Step 10: Push and integrate safely**

```powershell
git fetch origin
git rebase origin/main
git push -u origin codex/modular-creature-mesh-assembly
git switch main
git pull --ff-only origin main
git merge --no-ff codex/modular-creature-mesh-assembly -m "Merge modular creature mesh assembly"
```

Run the complete validation gate on merged `main`, then:

```powershell
git push origin main
```

Never force-push, erase parallel work, or remove a host-owned worktree.

---

## Completion Audit

The feature is complete only when all of these are evidenced together:

- schema-v1 appearances migrate coherently to schema v2;
- schema-v2 saves preserve five stable part-family IDs;
- founders use coherent families and offspring inherit per slot;
- normal mutation is catalog-compatible and rare mutation changes at most one incompatible non-torso slot;
- the catalog accepts a synthetic ninth family without renderer code changes;
- builder output is deterministic and every source triangle has exactly one owner;
- generated parts preserve valid UVs, normals, indices, bounds, LODs, and sockets;
- current licensed source assets retain complete attribution and generated outputs have manifest metadata;
- whole source OBJs are not packaged as production runtime meshes;
- Bevy renders child-part assemblies with shared handles and hidden joins;
- renderer diagnostics and markers remain display-only;
- no Bevy/wgpu/renderer types leak into `alife_core` or `alife_world`;
- minimum and comfort screenshots are fresh, inspected, bipedal, coherent, structurally varied, and terrain-safe;
- all commands in the complete gate pass or exact failures are reported;
- no previews, screenshots, archives, caches, source sheets, or large temporary artifacts are committed;
- reviewed commits are pushed and merged without overwriting parallel work.
