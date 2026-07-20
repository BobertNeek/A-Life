# GeneForge Frankenstein Creature Roster Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` task by task. Follow strict TDD:
> observe each focused test fail for the intended reason before production code.

**Goal:** Replace the rejected eight procedural animal packs with twelve
GeneForge-derived Frankenstein creature families that use compatible modular
attachments and one cohesive inherited coat per creature.

**Architecture:** Preserve `CreatureAppearanceGenome` schema v2 and its five
stable part-family genes. Separate saved family recipes from shared generated
part assets, normalize GeneForge source geometry offline through Blender, and
generate one cached coat atlas/material from the complete inherited appearance.
All render assets and code remain app/tool-owned and display-only.

**Tech stack:** Rust, Bevy 0.18, `StandardMaterial`, `image`, deterministic OBJ
and PNG production assets, Blender 5.1 background Python, PowerShell, WGSL GPU
neural runtime unchanged.

**Design source:**
`docs/superpowers/specs/2026-07-17-geneforge-frankenstein-creature-roster-design.md`

## Global Execution Rules

- Work in `D:\A life\.worktrees\modular-creature-mesh-assembly`.
- Treat the 75 pre-plan dirty/untracked paths plus this plan, revised spec, and
  blueprint as intentional work. Do not reset, overwrite blindly, or regenerate
  over them without diff review; track the current count in the SDD ledger.
- Keep `CARGO_TARGET_DIR=D:\A life\target`, `CARGO_BUILD_JOBS=1`, and
  `RUST_TEST_THREADS=1`; compile Bevy-heavy targets once and reuse the cache.
- Keep schema-v1 migration modulo eight forever. New founders use a separate
  twelve-family mapping.
- Do not add Bevy, wgpu, Blender, mesh, material, image, or renderer types to
  `alife_core` or `alife_world`.
- Do not add a mock creature, primitive fallback, fake backend, CPU neural
  fallback, or renderer authority.
- Do not commit `.blend`, archives, source texture trees, bake caches, previews,
  screenshots, or `target/` artifacts.
- After each task, review the diff against the design and update
  `.superpowers/sdd/progress.md` before starting the next task.

## Task 1: Stabilize And Preserve The Existing Remediation Baseline

**Files:**

- Review all current dirty paths; do not edit generated assets yet.
- Create: `.superpowers/sdd/progress.md`

1. Record branch, HEAD, dirty path count, diff stat, source hashes, blueprint
   path, and required gates in the SDD ledger.
2. Run the cheapest current focused checks before changing behavior:

   ```powershell
   $env:CARGO_TARGET_DIR='D:\A life\target'
   $env:CARGO_BUILD_JOBS='1'
   $env:RUST_TEST_THREADS='1'
   cargo fmt --all -- --check
   cargo test -p alife_world appearance --lib -j 1 --offline
   cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline
   ```

3. Record exact baseline failures. Fix only formatting or compile blockers that
   belong to the existing remediation; do not change the new design yet.
4. Review the 69-file generated-asset diff for archives, source assets, and
   accidental unrelated files. Preserve intentional work and record what the
   new pipeline will replace.
5. Commit a baseline checkpoint only after the focused checks pass, so later
   Blender generation cannot erase unreviewed work.

## Task 2: Freeze Legacy Migration And Add Twelve New Founder Families

**Files:**

- Modify: `crates/alife_world/src/appearance.rs`
- Modify: `crates/alife_world/tests/save_load_roundtrip.rs`
- Modify: `crates/alife_game_app/src/schema.rs` only if its current count
  assertion needs the production constant

1. Add failing tests proving:
   - all schema-v1 species still migrate with modulo eight;
   - new founders use `[0,1,2,3,4,5,6,7,8,9,10,11,3,6,9,0]`;
   - schema-v2 values `8..=11` serialize and deserialize unchanged;
   - offspring can inherit IDs `8..=11` per slot.
2. Run and observe RED:

   ```powershell
   cargo test -p alife_world appearance --lib -j 1 --offline -- --nocapture
   cargo test -p alife_world --test save_load_roundtrip -j 1 --offline -- --nocapture
   ```

3. Rename the historical constant to
   `LEGACY_CREATURE_PART_FAMILY_COUNT = 8`, add
   `PRODUCTION_CREATURE_PART_FAMILY_COUNT = 12`, and add a fixed founder mapping
   function. Do not change `CREATURE_APPEARANCE_SCHEMA_VERSION`.
4. Re-run the focused tests to GREEN and commit the renderer-neutral migration
   change independently.

## Task 3: Define Catalog V2 And The Shared Part Registry

**Files:**

- Modify: `crates/alife_game_app/src/creature_part_catalog.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Create: `crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json`

1. Add failing unit tests for catalog schema v2:
   - exact family IDs `0..=11` in append-only order;
   - every family references all three donors across its slots;
   - every family has at least one nonzero authored tweak;
   - no family is a complete stock Norn, Ettin, or Grendel body;
   - every referenced `CreaturePartAssetId` exists for all three LODs;
   - source attribution and SHA-256 values match the design;
   - a synthetic ID 12 family validates without renderer code changes;
   - duplicate asset IDs, invalid slot types, invalid sockets, Ettin tails,
     missing semantic regions, and non-finite fits fail clearly.
2. Run and observe RED:

   ```powershell
   cargo test -p alife_game_app creature_part_catalog --lib -j 1 --offline -- --nocapture
   ```

3. Add app-local serializable types for catalog v2 alongside the current v1
   production loader. The v1 catalog remains the live production catalog until
   Task 8; v2 tests load `geneforge_recipes.json` directly.
4. Add types for:
   - `CreaturePartAssetId`;
   - donor source and exact object selectors;
   - generated mesh group, LOD, bounds, attachment frame, semantic mask, and
     face/detail landmarks;
   - per-family slot recipes and bounded authored fits;
   - source digests, importer version, recipe digest, and output digest.
5. Author the exact twelve recipes from the design. Include explicit head
   expression, upper/lower limbs, feet, tail root/tip, eyes, lids, teeth,
   tongue, hair, ears, whiskers, and extras where present.
6. Keep catalog validation pure Rust and independent of Blender availability.
7. Re-run to GREEN and commit catalog contracts plus recipes. Prove the current
   v1 production catalog and manifest still validate unchanged.

## Task 4: Build The Deterministic GeneForge Blender Importer

**Files:**

- Create: `scripts/build_geneforge_creature_parts.py`
- Create: `scripts/test_geneforge_creature_recipes.py`
- Create: `scripts/create_geneforge_import_fixture.py`
- Modify: `crates/alife_tools/src/creature_part_builder.rs`
- Modify: `crates/alife_tools/src/bin/creature_part_builder.rs`
- Create: `crates/alife_tools/tests/creature_part_visual_contract.rs`

1. Write failing pure recipe tests before the importer:
   - the three input SHA-256 values are exact;
   - marker IDs `1..=14` map to stable semantic parts;
   - all selectors are explicit and deterministic;
   - each recipe uses all three donors and has a tweak;
   - no Ettin tail is selected;
   - all output paths stay under the staging/production roots.
2. Run and observe RED:

   ```powershell
   python scripts/test_geneforge_creature_recipes.py
   ```

3. Generate a tiny disposable `.blend` fixture under `target/artifacts/` with
   image relinking, a constraint, a mirrored geometry-node object, an armature,
   a non-manifold edge, marker properties, and both UV channel conventions. Add
   subprocess tests that observe RED for `inventory`, `validate-sources`,
   `build`, and `preview`, including wrong Blender version, broken texture,
   invalid marker, topology repair, and deterministic rerun cases.
4. Implement Blender subcommands `inventory`, `validate-sources`, `build`, and
   `preview`. Pin Blender `5.1.0`; discover its executable through `BLENDER_EXE`,
   PATH, or the existing project discovery helper, then reject every other
   reported version with an exact error.
5. Implement deterministic source processing:
   - relink Norn, Ettin, and Grendel texture roots by declared basename;
   - parse `kc3dsbpy_visscript` and custom selector properties;
   - reconstruct sockets from marker IDs rather than object order;
   - evaluate constraints, mirrored geometry nodes, modifiers, and armatures;
   - normalize each donor independently; never use PPU as scale;
   - triangulate, remove degenerate faces, merge exact duplicate vertices,
     repair declared non-manifold arm edges, and generate finite smooth normals;
   - apply authored body/head bridge recipes and seam offsets;
   - preserve source UV sampling while baking semantic masks and microdetail;
   - generate Full, Compact, and Impostor outputs deterministically.
6. Re-run every subprocess fixture test to GREEN. Extend the Rust builder
   validator to reject invalid OBJ indices, UV regions,
   normals, bounds, sockets, face landmarks, ground contacts, detached parts,
   missing masks, digest drift, or budget overruns.
7. First build to ignored staging:

   ```powershell
   python scripts/build_geneforge_creature_parts.py validate-sources --source-root "E:\Creatures Reborn\resources for gpt\Geneforge4"
   python scripts/build_geneforge_creature_parts.py build --source-root "E:\Creatures Reborn\resources for gpt\Geneforge4" --recipes crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json --staging target/artifacts/creature_parts/geneforge-staging
   cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline -- --nocapture
   ```

8. Inspect staged previews, then leave all generated output in ignored staging.
   Do not alter production asset paths or delete the existing packs in this
   task. Runtime tests in Tasks 5-7 use a temporary staging catalog.
9. Re-run the importer twice and prove byte-identical staged outputs. Commit
   importer, validators, fixture generator, and recipes without production
   asset cutover.

## Task 5: Implement Donor-Independent Cohesive Coat Baking

**Files:**

- Create: `crates/alife_game_app/src/creature_coat.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/creature_part_assets.rs`

1. Add failing pure tests for:
   - `CreatureCoatKey` includes five selected family IDs plus palette, pattern,
     and marking density;
   - donor geometry identity does not select coat colors;
   - controlled gene changes alter deterministic atlas bytes;
   - identical keys produce byte-identical 256x256 RGBA8 atlases;
   - primary/secondary value contrast is at least `0.28`;
   - all seven slot regions are populated and join-cover regions match;
   - cache reuse returns one material identity for an entire assembly.
   - 1,000 generations of unique keys stay inside profile-specific limits and
     evict only unreferenced image/material pairs.
2. Run and observe RED:

   ```powershell
   cargo test -p alife_game_app creature_coat --lib -j 1 --offline -- --nocapture
   ```

3. Implement a pure deterministic coat resolver and CPU atlas baker using the
   committed semantic masks and microdetail. Keep it app-local and bounded.
4. Extend `CreaturePartAssetLibrary` with shared generated mesh lookup and a
   separate coat material/image cache. Add deterministic LRU/refcount eviction
   with limits of 48/12 MiB for minimum, 96/24 MiB for comfort, and 256/64 MiB
   for future scale-up. Do not make mesh caching depend on coat.
5. Re-run to GREEN and commit coat baking independently.

## Task 6: Resolve Shared Assets And One Material Per Assembly

**Files:**

- Modify: `crates/alife_game_app/src/creature_assembly.rs`
- Modify: `crates/alife_game_app/src/creature_part_assets.rs`
- Modify: `crates/alife_game_app/src/creature_surface_details.rs`
- Modify: `crates/alife_game_app/src/creature_visual_geometry.rs`
- Modify: `crates/alife_game_app/src/creature_part_genetics.rs`
- Modify: `crates/alife_game_app/src/lifecycle_lineage.rs`
- Modify: `crates/alife_game_app/tests/app_shell.rs`

1. Add failing tests proving:
   - a family recipe resolves slot assets rather than one family OBJ;
   - a mixed offspring resolves the correct slot asset for every saved gene;
   - every part and cover receives one `CreatureCoatKey`;
   - material keys no longer contain donor family identity;
   - no `% 8` or eight-entry morphology table controls current production;
   - source-derived head landmarks replace hard-coded face offsets;
   - assembly transforms remain finite, grounded, and attachment-safe.
   - all twelve torso frames accept compatible inherited IDs `8..=11` through
     ordinary and rare mutation paths;
   - lineage save round-trips retain original inherited IDs, including unknown
     IDs normalized only for display.
2. Run and observe RED with focused library tests.
3. Change `CreatureAssemblyPartRecipe` to carry `CreaturePartAssetId`, generated
   mesh group, attachment transform, bounds, and landmarks. Remove
   `texture_asset_path` from parts.
4. Replace `CreaturePartMaterialKey` with `CreatureCoatKey`. Preserve mesh-handle
   reuse across families that reference the same asset.
5. Replace fixed eight-family surface arrays with catalog landmarks and inherited
   trait math. Remove generic hat, floating muzzle, and duplicate face geometry
   already present in generated heads.
6. Update lifecycle birth normalization to consume catalog v2 compatibility
   without rewriting saved parental genes. Add an app diagnostic for visible
   unknown-ID fallback and a save-after-render test proving the original unknown
   ID remains serialized.
7. Re-run to GREEN and commit the resolver/cache/lineage change.

## Task 7: Render Source-Derived Heads, Eyes, And Articulated Bodies

**Files:**

- Modify: `crates/alife_game_app/src/production_voxel_renderer.rs`
- Modify: `crates/alife_game_app/src/creature_part_pose.rs`
- Modify: `crates/alife_game_app/tests/fvr03_voxel_renderer.rs`

1. Add failing Bevy ECS tests for all 12 families at Full and Compact LOD:
   - the expected shared mesh asset children exist;
   - all body parts and covers use the same material handle;
   - two embedded eyes contain warm sclera, saturated iris, bounded pupil,
     glint, and lids from generated head landmarks;
   - no procedural hat/muzzle marker remains;
   - upright, walking, resting, sleeping, hurt, and dead states preserve sockets
     and ground contacts;
   - every marker remains display-only and declares no renderer authority.
   - a before/after authoritative snapshot proves renderer startup, coat baking,
     spawning, animation, and capture do not change world state, candidates,
     actions, outcomes, rewards, or cognition.
2. Compile once and observe RED:

   ```powershell
   cargo test -p alife_game_app --features bevy-app --test fvr03_voxel_renderer -j 1 --offline --no-run
   cargo test -p alife_game_app --features bevy-app --test fvr03_voxel_renderer -j 1 --offline -- --nocapture
   ```

3. Load the validated shared part assets at startup and resolve one cached coat
   before spawning an assembly. Bind its material to every part and cover.
4. Spawn source-derived head details and animate only declared eyelid/expression
   groups. Remove black cuboid/bead eyes and generic face patches.
5. Preserve the existing articulated pose and exact-grounding remediation,
   adapting it to generated landmarks rather than reintroducing root squash.
6. Re-run the already-built test target to GREEN and commit renderer integration.

## Task 8: Atomic Production Asset, Catalog, Loader, And Manifest Cutover

**Files:**

- Modify: `crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json`
- Modify: `crates/alife_game_app/assets/production_voxel_v1/models/ATTRIBUTION.md`
- Modify: `crates/alife_game_app/src/production_assets.rs`
- Modify: `crates/alife_game_app/src/creature_part_catalog.rs`
- Modify: `crates/alife_game_app/src/creature_part_assets.rs`
- Create: `crates/alife_game_app/assets/production_voxel_v1/models/GENEFORGE_LICENSE_RECEIPT.md`
- Delete only superseded files under
  `crates/alife_game_app/assets/production_voxel_v1/creature_parts/generated/`
  and obsolete `T_*.png` surfaces after confirming no remaining references

1. Prove the current production manifest validates before cutover. Add failing
   manifest tests for exact generated paths, digests, sizes, source
   attribution, recipe digest, importer version, and replacement policy.
2. Assert no `.blend`, archive, source texture, preview, screenshot, stale Quirky
   creature entry, or duplicate whole-body pack is present.
3. Write a transparent license receipt recording the project owner's explicit
   MIT statement, the absence of embedded/upstream-page license text, and any
   immutable permission evidence supplied. Do not invent an upstream copyright
   notice. Point `license_ref` to this receipt and update attribution with source
   URL, modifications, Blender 5.1.0, and source SHA-256 values.
4. Keep the complete production creature pack at or below 8 MiB. If it exceeds
   the cap, improve mesh reuse/LOD/PNG compression; do not silently raise it.
5. In one working-tree operation, promote staged assets, switch catalog/loader
   paths, refresh manifest/attribution, and remove only superseded packs. Do not
   leave an intermediate committed or tested production state.
6. Run catalog, loader, manifest, renderer, and repository invariant tests to
   GREEN before committing the atomic cutover. Confirm production manifest
   validation passes after cutover.

## Task 9: Deterministic Visual Audit And Iteration

**Files:**

- Modify: `crates/alife_tools/src/bin/creature_part_builder.rs`
- Modify tests as needed; keep captures under ignored `target/artifacts/`

1. Add failing command tests for a deterministic `audit-atlas` command that
   renders labeled all-12-family sheets at front, three-quarter, and back views
   for Full, Compact, and Impostor LOD, plus upright/resting/sleeping atlases.
2. Emit capture metadata with family, source donors, selected asset IDs, coat
   key, projected bounds, eye bounds, socket error, foot-ground error, triangle
   count, and detached-part count.
3. Implement `audit-atlas` with fixed orthographic camera, neutral gray
   background, fixed three-point lighting, fixed cell scale, stable ID/recipe
   labels, and measurable pixel/eye occupancy plus silhouette distance. Run the
   command tests to GREEN.
4. Generate atlases and compare them at high visual reasoning against:
   `docs/superpowers/specs/assets/geneforge-frankenstein-roster-blueprint.png`.
5. Reject and iterate any hat, slab, fused limb, bead eye, exposed socket,
   patchwork coat, pastel-only palette, stock donor body, color-only variant,
   detached part, unreadable face, or dinosaur-first silhouette.
6. Do not proceed on marker-only assertions. The visible atlas is the gate.

## Task 10: Shipping Captures, Full Validation, And Live Playtest

1. Run focused gates first, then compile the workspace once:

   ```powershell
   $env:CARGO_TARGET_DIR='D:\A life\target'
   $env:CARGO_BUILD_JOBS='1'
   $env:RUST_TEST_THREADS='1'
   cargo fmt --all -- --check
   cargo test -p alife_world appearance --lib -j 1 --offline
   cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline
   cargo test -p alife_game_app --features bevy-app --test fvr03_voxel_renderer -j 1 --offline
   cargo check -p alife_game_app --features production-voxel-frontend --all-targets -j 1 --offline
   cargo check --workspace --all-targets -j 1 --offline
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
   ```

2. Build the exact shipping feature set once and reuse that binary for both
   profiles:

   ```powershell
   cargo build -p alife_game_app --release --features production-voxel-frontend -j 1 --offline
   ```
3. Capture fresh `MinimumSettings30x30` and `MinSpecComfort1080p` screenshots
   from the real GPU-authoritative backend with overlays closed.
4. Require runtime telemetry to report `GpuAuthoritative`, the expected GPU
   adapter, and no unavailable/fallback mode. Use Computer Use for a live
   camera, selection, pause, save/load, and pose
   smoke. Confirm no disconnect, crash, missing asset, mock backend, primitive
   fallback, or simulation behavior regression.
5. Compare both screenshots to the blueprint and accepted terrain composition.
   Iterate until the visible acceptance criteria pass.
6. Record exact command outputs, artifact paths, executable path, PID/profile,
   backend/GPU mode, and visual review result in the SDD ledger.

## Task 11: Review, Commit, Push, And Careful Merge

1. Request independent code review and independent visual review. Resolve every
   correctness, save, authority, asset, license, performance, and visible-quality
   finding.
2. Review `git status`, `git diff --check`, asset sizes, ignored artifacts, and
   commit history. Exclude all source/bake/capture artifacts.
3. Fetch current `origin/main`. Integrate concurrent work into the feature branch
   without resetting either side. Re-run affected focused tests after conflicts.
4. Re-run the full Task 10 validation on the exact final feature tree.
5. Push `codex/modular-creature-mesh-assembly`, merge through a temporary
   integration worktree or reviewed non-destructive merge, and verify the merged
   tree contains both this work and all concurrent brain/backend work.
6. Push `main`, verify `origin/main` matches local `main`, and record commit IDs
   plus exact tree equality for all intentional paths.

## Completion Standard

The work is complete only when twelve geometry-distinct Frankenstein families
use normalized GeneForge-derived parts, every mixed offspring receives one
cohesive inherited coat, source-derived faces replace hats and bead eyes, all
automated/visual/live gates pass, production assets remain bounded and licensed,
and the exact validated tree is committed, pushed, and safely merged without
losing concurrent work.
