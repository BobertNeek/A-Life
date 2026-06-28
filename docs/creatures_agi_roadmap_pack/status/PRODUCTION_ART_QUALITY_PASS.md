# Production Art Quality Pass

Goal: move A-Life graphical alpha assets toward a production-quality game look comparable in polish intent to commercial creature/ecosystem games, without copying any existing game and without changing simulation authority.

Branch: `codex/production-art-quality-pass`
Status: first production-alpha asset upgrade implemented

## Visual Target

The pass uses an original top-down organic ecosystem art direction:

- rounded readable creature silhouettes with expression and antenna pose cues;
- crystalline hazard silhouettes with glow and spike shapes;
- organic food sprout/flower silhouette;
- irregular rock cluster silhouettes;
- softened terrain tiles with layered grass, soil, grove, hazard-pressure, and stone details;
- small environmental dressing props with shadows.

The reference blueprint was generated as local, untracked visual guidance only. Product assets are deterministic repo-generated PNGs, not copied commercial artwork.

## Asset Upgrade

The committed `alpha_art_v1` pack now regenerates at 128x128 instead of 64x64 while remaining below the 64 KB per-PNG cap.

Updated assets:

- `creature_idle.png`
- `creature_hurt.png`
- `selection_ring.png`
- `food_sprout.png`
- `hazard_crystal.png`
- `rock_cluster.png`
- `terrain_safe_grass.png`
- `terrain_soil_path.png`
- `terrain_resource_grove.png`
- `terrain_hazard_pressure.png`
- `terrain_stone_rough.png`
- `prop_grass_tuft.png`
- `prop_pebble_cluster.png`
- `prop_warning_shard.png`
- `prop_leaf_patch.png`

The generator remains deterministic and standard-library only:

```powershell
python scripts/generate_alpha_art_v1.py
```

## Manifest And Validation Changes

The alpha art manifest now records:

- `art_direction: production-alpha-organic-topdown-v2`
- 128x128 dimensions for each asset
- current file sizes

The validator now rejects required art below `CA44A_MIN_PRODUCTION_ART_DIMENSION` of 96 pixels, preventing regression to tiny placeholder icons.

## Evidence

Local untracked contact sheets were generated under:

```text
target/art_review/
```

They are not tracked. They are used only for visual review.

## Boundaries

- No gameplay or simulation semantics changed.
- No action authority changed.
- CPU fallback and CPU shadow parity remain unchanged.
- No `alife_core` dependency leak.
- No release tag.
- No screenshots, target artifacts, logs, model files, or caches are tracked.

## Remaining Work

This pass is not a claim that the entire product now matches a finished commercial game. It improves the current committed pack and raises the validation floor. Remaining production-art work includes animation frames, larger coherent biome sets, better composition in the live scene, lighting/VFX polish, UI skinning, and external visual review.
