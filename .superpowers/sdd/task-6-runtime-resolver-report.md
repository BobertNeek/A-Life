# Task 6 Runtime Resolver Report

## Scope

- Branch: `codex/modular-creature-mesh-assembly`
- Worktree: `D:\A life\.worktrees\modular-creature-mesh-assembly`
- Runtime input: explicit `GeneForgeGroupTransform` values supplied by the caller
- No ignored `target` artifact loading, generated-file promotion, matrix synthesis,
  Blender, Python asset suite, release build, renderer edit, cognition edit, or
  authority edit
- `production_voxel_renderer.rs` was not modified

## RED Evidence

Environment for every Rust command:

```powershell
$env:CARGO_TARGET_DIR='D:\A life\target'
$env:CARGO_BUILD_JOBS='1'
$env:RUST_TEST_THREADS='1'
```

### Resolver And Cache

```powershell
cargo test -p alife_game_app creature_assembly --lib -j 1 --offline -- --nocapture
cargo test -p alife_game_app creature_part_assets --lib -j 1 --offline -- --nocapture
```

Expected RED observed: compile failed with missing
`GeneForgeAssemblyPreparationIndex`, `resolve_geneforge_creature_assembly`,
asset-key cache methods, evidence fields, and preparation rejection variants.

### Genetics And Persistence

```powershell
cargo test -p alife_game_app creature_part_genetics --lib -j 1 --offline -- --nocapture
cargo test -p alife_game_app lifecycle_lineage --lib -j 1 --offline -- --nocapture
```

Expected RED observed: genetics accepted only `CreaturePartCatalog`, not
`GeneForgeCreaturePartCatalog`; lifecycle then failed on the missing
`resolve_creature_part_display_sources` API. A later diagnostic test failed on
the missing `visible_message` method.

### Surface And Geometry

```powershell
cargo test -p alife_game_app creature_surface_details --lib -j 1 --offline -- --nocapture
cargo test -p alife_game_app creature_visual_geometry --lib -j 1 --offline -- --nocapture
```

Expected RED observed: compile failed on missing
`creature_face_style_from_landmarks`, `CreatureSurfaceDetailError`, and
`grounded_root_height_from_transformed_feet`.

### Evidence Hardening

The forged socket-evidence test initially failed because the index accepted a
claimed transformed anchor that did not match the stored matrix. The index now
reapplies the authored matrix and checks the measured residual before accepting
the record.

## GREEN Evidence

Fresh combined focused run:

| Command | Result |
| --- | --- |
| `cargo test -p alife_game_app creature_assembly --lib -j 1 --offline -- --nocapture` | 8 passed, 0 failed |
| `cargo test -p alife_game_app creature_part_assets --lib -j 1 --offline -- --nocapture` | 8 passed, 0 failed |
| `cargo test -p alife_game_app creature_part_genetics --lib -j 1 --offline -- --nocapture` | 7 passed, 0 failed |
| `cargo test -p alife_game_app lifecycle_lineage --lib -j 1 --offline -- --nocapture` | 2 passed, 0 failed |
| `cargo test -p alife_game_app --test app_shell -j 1 --offline -- --nocapture` | 6 passed, 0 failed |
| `cargo test -p alife_game_app creature_surface_details --lib -j 1 --offline -- --nocapture` | 4 passed, 0 failed |
| `cargo test -p alife_game_app creature_visual_geometry --lib -j 1 --offline -- --nocapture` | 4 passed, 0 failed |

Combined focused result: 39 passed, 0 failed.

## Twelve-Family Resolution

Each row resolved at Full and Compact LOD. Every runtime part selected the exact
asset from the saved family recipe and then used the exact target torso, LOD,
runtime group, and socket preparation key.

| ID | Family | Head | Torso | Arms | Legs | Tail |
| ---: | --- | --- | --- | --- | --- | --- |
| 0 | tuftback | norn-head | ettin-torso | grendel-arms | norn-legs | grendel-tail |
| 1 | tideclimber | ettin-head | norn-torso | norn-arms | grendel-legs | grendel-tail |
| 2 | mossknuckle | grendel-head | ettin-torso | norn-arms | ettin-legs | norn-tail |
| 3 | emberloper | norn-head | grendel-torso | ettin-arms | grendel-legs | norn-tail |
| 4 | duskmane | ettin-head | grendel-torso | norn-arms | norn-legs | grendel-tail |
| 5 | reefburrower | grendel-head | norn-torso | ettin-arms | ettin-legs | grendel-tail |
| 6 | velvetreed | norn-head | ettin-torso | norn-arms | grendel-legs | grendel-tail |
| 7 | copperskipper | ettin-head | norn-torso | grendel-arms | ettin-legs | norn-tail |
| 8 | slateprowler | grendel-head | grendel-torso | norn-arms | ettin-legs | norn-tail |
| 9 | cobaltbramble | norn-head | grendel-torso | grendel-arms | ettin-legs | norn-tail |
| 10 | orchidstout | ettin-head | ettin-torso | grendel-arms | norn-legs | grendel-tail |
| 11 | amberlongstep | grendel-head | norn-torso | ettin-arms | grendel-legs | grendel-tail |

## Mixed Assembly

Saved sources `{ head: 11, torso: 9, arms: 8, legs: 10, tail: 6 }`
resolved to:

| Runtime groups | Asset | Target torso |
| --- | --- | --- |
| head | grendel-head | grendel-torso |
| torso | grendel-torso | grendel-torso |
| left-arm, right-arm | norn-arms | grendel-torso |
| left-leg, right-leg | norn-legs | grendel-torso |
| tail-back | grendel-tail | grendel-torso |

The saved sources are retained separately from display fallback sources.

## Matrix And Residual Evidence

- Preparation fixture count: 684 exact keys: 252 canonical plus 432 cross-torso.
- Full/Compact coherent-family assertions: 168 resolved runtime parts.
- Matrix layout: exact stored row-major affine matrix; bottom row
  `[0, 0, 0, 1]`.
- Fixture residual: `0.001`, below the catalog limit `0.025`.
- Index rejection covers duplicate keys, invalid target identity, missing exact
  target/group lookup, non-finite matrices, forged transformed anchors, and
  residual mismatch.
- Canonical and transformed bounds are finite and positive on every axis.
- Foot grounding translates the root to the transformed minimum and leaves both
  foot bounds unchanged; no root squash is used.

## Cache And Coat Identity

- Four coherent Compact assemblies requested 28 part meshes.
- Asset/LOD/runtime-group deduplication produced 17 mesh keys.
- The same asset referenced by different families shares a mesh key.
- Four assemblies produced four coat keys total.
- Every part and every join cover in one assembly receives the same
  `CreatureCoatKey`.
- `CreaturePartMaterialKey` is absent from the Task 6 path; donor/family identity
  is not material identity.

## Genetics And Save Evidence

- Ordinary compatibility accepts IDs `8..=11` under every torso ID `0..=11`.
- Rare mutation selected each ID `8`, `9`, `10`, and `11` under every torso
  frame without normalization.
- Birth retained inherited torso ID `11`.
- Save JSON round-tripped IDs `8..=11` unchanged.
- Unknown ID `999` produced a display-only fallback and visible diagnostic.
- Serialization before and after display resolution was byte-for-byte equal;
  deserialization retained ID `999`.

## Source Scans

Scoped Task 6 scans covered the seven modified code/test files.

- No `% 8` selection in the new path.
- No eight-entry morphology tables in the new path.
- No `CreaturePartMaterialKey` in the new path.
- No ignored `target/artifacts`, staging, or compatibility-tag runtime lookup.
- No matrix constructor in production resolver code; matrices enter only through
  explicit validated records.
- `texture_asset_path` remains only in the clearly named deprecated legacy
  renderer adapter. The new `CreatureAssemblyPartRecipe` has no texture path.
- `production_voxel_renderer.rs` is absent from `git diff --name-only`.
- No `__pycache__` directory was created.

## Diff And Self-Review

Intentional code paths:

- `creature_assembly.rs`: validated preparation index, exact resolver, evidence,
  transformed bounds, display fallback diagnostic, asset/coat key cache, and
  isolated legacy renderer adapter.
- `creature_part_assets.rs`: asset/LOD/group mesh identity and coat-only material
  cache ownership.
- `creature_part_genetics.rs` and `lifecycle_lineage.rs`: v2 roster mutation,
  inherited-ID preservation, and display-only unknown fallback persistence.
- `creature_surface_details.rs` and `creature_visual_geometry.rs`: catalog
  landmarks, continuous trait math, duplicate procedural anatomy removal, and
  no-squash grounding.
- `tests/app_shell.rs`: high-ID and unknown-ID persistence acceptance.

Self-review findings resolved:

1. Added visible diagnostic text that explicitly says save and lineage data are
   unchanged.
2. Added independent matrix/evidence residual verification to reject forged
   evidence.
3. Made cache counts and all-torso rare high-ID coverage exact assertions.

Remaining sequencing concern: the deprecated family OBJ/texture resolver is
still present solely so Task 7 can switch `production_voxel_renderer.rs` in its
owned change. New tests, genetics, lineage, cache identity, and surface logic do
not call it.
