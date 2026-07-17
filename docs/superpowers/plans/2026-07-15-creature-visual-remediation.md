# Creature Visual Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make all eight production creature families render as coherent, readable, textured bipedal creatures in upright, moving, resting, and sleeping states at the shipping camera distances.

**Architecture:** Keep appearance genes and simulation state renderer-neutral. Extend the app-owned catalog with canonical per-family fitting, face, grounding, and pose landmarks; rebuild generated part packs offline; and make the Bevy renderer consume those authored contracts without changing action, cognition, physics, or save authority. Automated geometry audits and deterministic family/state capture matrices are the acceptance gate, while fresh release screenshots remain the final visual proof.

**Tech Stack:** Rust, Bevy 0.18, OBJ/PNG production assets, deterministic `alife_tools` offline generation, PowerShell validation, GPU-authoritative A-Life runtime.

## Global Constraints

- Rust + Bevy + wgpu/WebGPU + WGSL only.
- Production neural execution remains GPU-authoritative; no CPU neural shadow, parity gate, or automatic fallback.
- No Bevy, wgpu, mesh, material, renderer, or OS types enter `alife_core` or `alife_world`.
- The renderer is display-only and never authorizes actions, cognition, rewards, learning, mutation, or world outcomes.
- Preserve stable `CreaturePartFamilyId` values and schema-v2 save compatibility.
- Future source meshes remain addable through catalog data plus offline tooling, without family-specific renderer match statements.
- Do not commit archives, Blender caches, target previews, screenshots, or duplicate whole-mesh runtime packs.
- Production visuals must match `docs/superpowers/specs/assets/creature-visual-remediation-blueprint.jpg` in camera-scale readability and finish, without copying its exact geometry.
- Windows Rust validation uses `RUST_TEST_THREADS=1`, `CARGO_BUILD_JOBS=1`, and `-j 1` for Bevy-heavy targets.

---

## File Map

| Path | Responsibility |
|---|---|
| `crates/alife_game_app/src/creature_part_catalog.rs` | Versioned app-local family morphology, face, grounding, pose, and material contracts. |
| `crates/alife_game_app/src/creature_part_assets.rs` | Load already-canonical generated geometry without destructive XYZ stretching; compute mesh bounds used by audits. |
| `crates/alife_game_app/src/creature_assembly.rs` | Resolve saved part genes into bounded transforms, anchors, covers, and family visual metadata. |
| `crates/alife_game_app/src/creature_visual_geometry.rs` | New pure app-local geometry audit and pose evaluation module. No Bevy world authority. |
| `crates/alife_game_app/src/production_voxel_renderer.rs` | Spawn real detail geometry/materials, integrated faces, grounded roots, and articulated state poses. |
| `crates/alife_tools/src/creature_part_builder.rs` | Deterministic canonicalization and validation of family part packs. |
| `crates/alife_tools/src/bin/creature_part_builder.rs` | Family/state audit atlas and deterministic preview commands. |
| `crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json` | Eight complete authored family profiles; no inherited empty cut/socket profiles. |
| `crates/alife_game_app/assets/production_voxel_v1/creature_parts/generated/` | Rebuilt canonical part packs and socket manifests. |
| `crates/alife_game_app/assets/production_voxel_v1/models/` | Bounded production texture maps and attribution metadata. |
| `crates/alife_game_app/tests/fvr03_voxel_renderer.rs` | ECS/render contract and deterministic family/state coverage. |
| `crates/alife_tools/tests/creature_part_visual_contract.rs` | Offline all-family geometry, socket, ground, and silhouette validation. |

### Task 1: Make Broken Geometry Measurable

**Interfaces:**
- Produces: `CreaturePartBounds`, `CreatureAssemblyAudit`, `audit_creature_assembly(...)`, and deterministic failure reasons.
- Consumes: parsed `PartMeshData`, `CreatureAssemblyRecipe`, catalog landmarks, and animation state.

- [ ] **Step 1: Write failing tests** in `crates/alife_tools/tests/creature_part_visual_contract.rs` for all eight families and all three LODs. Assert finite non-empty bounds, head above torso, shoulders above hips, two laterally separated legs, foot minima within `0.025` world units of ground, no detached part farther than the declared overlap tolerance, and no axis fit ratio outside `[0.65, 1.55]`.
- [ ] **Step 2: Run the focused test and verify RED.**

  ```powershell
  $env:RUST_TEST_THREADS='1'; $env:CARGO_BUILD_JOBS='1'; cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline -- --nocapture
  ```

  Expected: failures identify the inherited colobus profile, destructive per-axis fitting, missing biped legs, or invalid grounding for multiple families.

- [ ] **Step 3: Add** `creature_visual_geometry.rs` and bounds accessors in `creature_part_assets.rs`. Keep calculations pure and deterministic; return typed audit failures rather than marker booleans.
- [ ] **Step 4: Re-run the focused test.** It may still fail on bad assets, but every failure must now identify family, LOD, slot, measured value, and limit.
- [ ] **Step 5: Commit** the audit contract independently.

### Task 2: Replace Generic Stretching With Authored Canonical Fits

**Interfaces:**
- Produces: catalog schema v2 `canonical_fit`, `face_anchor`, `ground_anchor`, `pose_anchors`, and `surface_profile` per family.
- Consumes: stable family IDs and current generated part paths.

- [ ] **Step 1: Add failing catalog tests** proving every family owns complete values and that `template_family`, empty cuts, empty sockets, and implicit colobus cloning are rejected in production catalogs.
- [ ] **Step 2: Verify RED** with the focused catalog tests.
- [ ] **Step 3: Extend the Rust catalog schema** with serializable app-local types for bounded uniform fit, per-slot pivots, face origin/scale, foot contacts, joint pivots, pattern scale, roughness, and optional wetness/fur accents.
- [ ] **Step 4: Replace `fit_part_to_biped_envelope`** with catalog-authored canonical transforms. Generated parts must retain aspect ratios; runtime may apply only bounded uniform scale plus the saved body-mass variation.
- [ ] **Step 5: Author complete profiles for IDs 0-7.** Mammalian families use broad shoulders, short planted legs, hands, muzzle, and ears; aquatic/scaled families retain a biped torso and feet while using fins, crests, tail, gill, or scale silhouettes as accents instead of becoming fish/snake slabs.
- [ ] **Step 6: Rebuild all 24 generated packs** through the offline builder and validate deterministic digests.
- [ ] **Step 7: Run Task 1 audits** until all family/LOD geometry contracts pass.
- [ ] **Step 8: Commit** catalog, loader, builder, and generated pack changes together.

### Task 3: Render Real Details, Integrated Faces, And Heritable Surface Variation

**Interfaces:**
- Produces: rendered `Mesh3d`/`MeshMaterial3d` detail children, per-family face placement, and visible consumers for `marking_density`, `ear_muzzle_trait`, and `tail_trait`.
- Consumes: catalog `face_anchor`/`surface_profile` and saved `CreatureAppearanceGenome`.

- [ ] **Step 1: Add failing ECS tests** that query every detail entity for `Mesh3d`, `MeshMaterial3d<StandardMaterial>`, `Transform`, and `ChildOf`; assert each creature has two readable eyes, off-white sclera, colored iris, bounded dark pupil, glint, brow/muzzle geometry, hands, and feet.
- [ ] **Step 2: Add failing inheritance visibility tests** proving controlled changes to `marking_density`, `ear_muzzle_trait`, and `tail_trait` alter deterministic rendered detail recipes without changing simulation state.
- [ ] **Step 3: Verify RED** with only the focused `fvr03_voxel_renderer` tests.
- [ ] **Step 4: Fix `fvr10_spawn_creature_surface_details`** so it spawns the selected meshes and materials as children. Replace black cuboid bead eyes with layered low-poly ovoids, warm sclera, saturated irises, smaller pupils, glints, brows, and a family-anchored muzzle/mouth.
- [ ] **Step 5: Replace palette-strip-only materials** with bounded 256x256 or smaller production maps carrying high-contrast fur/scale/skin markings. Keep roughness and tint mutation compatible with shared material caching.
- [ ] **Step 6: Add real trait consumers** for stripe/spot density, muzzle/ear proportion, tail/crest extent, and accessory accents. Changes remain display-only.
- [ ] **Step 7: Run focused ECS and asset-manifest tests** until green, then commit.

### Task 4: Replace Root Squashing With Articulated Poses And Exact Grounding

**Interfaces:**
- Produces: `CreaturePartPose` transforms for idle, walking, eating, social, carrying, resting, sleeping, hurt, and dead states at deterministic phases.
- Consumes: catalog joint pivots, actual part bounds, authoritative visual animation state, and tile surface height.

- [ ] **Step 1: Add failing state-matrix tests** for eight families across every production animation state. Assert no root scale axis drops below `0.82`, feet remain grounded while upright, resting creatures crouch by rotating hips/knees/torso, sleeping creatures lie on a side with face clear of terrain, and no part detaches.
- [ ] **Step 2: Verify RED** against the current `Y * 0.52` root squash.
- [ ] **Step 3: Implement app-local articulated poses** in `creature_visual_geometry.rs` and consume them in `animate_fvr04_creature_parts`. Root scale remains morphology-only; state motion uses part rotations/translations around authored pivots.
- [ ] **Step 4: Ground roots from actual transformed foot minima** and sampled tile height, not a fixed LOD constant. Add deterministic pose phase overrides for capture/test mode.
- [ ] **Step 5: Run the full state matrix** and focused renderer tests until green, then commit.

### Task 5: Deterministic Visual Proof And Iteration

**Interfaces:**
- Produces: ignored all-family upright/rest audit atlases and fresh release screenshots.
- Consumes: shipping renderer, real save/backend, fixed animation phase, and both production profiles.

- [ ] **Step 1: Extend `creature_part_builder preview`** with `audit-atlas --lod <lod> --state <state> --output <target path>` covering IDs 0-7 at front and three-quarter views with identical camera/framing.
- [ ] **Step 2: Add deterministic capture metadata** containing family ID, chosen part sources, state, LOD, projected bounds, face projected bounds, foot-ground error, and detached-part count.
- [ ] **Step 3: Generate upright, resting, and sleeping atlases** under `target/artifacts/creature_parts/`. Reject any family that reads as a hat, slab, cone, quadruped, pasted face, fused limb mass, or color-only variant.
- [ ] **Step 4: Iterate meshes/materials** until every family is distinct at thumbnail scale and matches the benchmark's integrated face, planted biped anatomy, bold contrast, and surface detail.
- [ ] **Step 5: Build release once** and reuse the binary for both production captures to avoid repeated 20-minute links.
- [ ] **Step 6: Capture fresh** `MinimumSettings30x30` and `MinSpecComfort1080p` runtime screenshots from real GPU-authoritative gameplay with default overlays closed.
- [ ] **Step 7: Compare the screenshots** against the blueprint and accepted terrain composition using high-reasoning visual inspection. Record concrete discrepancies and iterate until none of the design rejection criteria remain.
- [ ] **Step 8: Commit only source, production assets, manifests, tests, and docs.** Keep all previews and screenshots ignored.

### Task 6: Full Validation, Live Playtest, And Integration

- [ ] **Step 1: Run formatting and focused gates.**

  ```powershell
  cargo fmt --all -- --check
  $env:RUST_TEST_THREADS='1'; $env:CARGO_BUILD_JOBS='1'; cargo test -p alife_tools --test creature_part_visual_contract -j 1 --offline
  $env:RUST_TEST_THREADS='1'; $env:CARGO_BUILD_JOBS='1'; cargo test -p alife_game_app --features bevy-app --test fvr03_voxel_renderer -j 1 --offline
  ```

- [ ] **Step 2: Run workspace and architecture gates.**

  ```powershell
  $env:CARGO_BUILD_JOBS='1'; cargo check --workspace --all-targets -j 1 --offline
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
  ```

- [ ] **Step 3: Launch the release binary** with `MinSpecComfort1080p`, use Computer Use for a live camera/state smoke, and confirm no crash, disconnect, debug overlay default, missing asset, or visually broken family.
- [ ] **Step 4: Review git status and asset sizes.** Reject archives, target artifacts, duplicate runtime packs, source-image generations, or unrelated changes.
- [ ] **Step 5: Request code and visual review, address findings, commit, push, and merge carefully into current `main` without deleting concurrent work.** Verify the merged tree matches the validated feature tree for all intentional paths.

## Completion Standard

The task is complete only when all eight families pass geometry and state matrices, every heritable visual gene has a visible bounded consumer, fresh shipping-profile screenshots show coherent textured bipeds without hats/black-bead eyes/fused torsos, the live game passes Computer Use inspection, all required commands pass, and the exact validated tree is merged and pushed.
