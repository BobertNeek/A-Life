# Production Blender 2.5D Asset Pipeline

Plan context: continuing the active production-art/procedural-world goal after
CA44A. This is not roadmap continuation and does not start CA45.

Branch: `codex/production-blender-art-pipeline`

## Objective

Move the art workflow away from one-off generated PNG sheets toward a repeatable
Blender-to-Bevy 2.5D pipeline. The current game uses the committed v41
`alpha_art_v1` PNG pack, while future production-quality replacements now have
a validated source manifest, launcher script, and Blender Python renderer.

Superseded status: this document records a proposed Blender authoring lane, but
it is not the active Player View implementation. The current active Player View
correction uses committed native `.glb` assets under
`crates/alife_game_app/assets/true_25d_alpha_v1/` and a locked orthographic 3D
presentation. Blender remains a future authoring option once a local Blender
runtime is installed and validated.

## Blueprint

A new image-generation blueprint was created as local visual direction for this
slice. It targets a wide orthographic/isometric game view with:

- organic grass, path, water, sand, stone, resource, and hazard biomes;
- chunk-like world continuity without visible debug square slabs;
- fog-of-war at unexplored edges;
- readable creature, food, hazard, rock, and prop silhouettes;
- minimal HUD chips instead of debug dashboards.

The earlier blueprint is local evidence only under `.codex/generated_images/`
and is not committed. The latest user-provided direction has shifted the target
from 2D PNG sprites toward True 2.5D retro-futuristic biological geometry:
low-poly glTF assets, quantized toon-band shader contracts, Sobel-style outline
contracts, pixel-step filtering contracts, and procedural micro-ecology chunks.

## Pipeline Artifacts

Tracked source/control files:

- `crates/alife_game_app/assets/alpha_art_v1/blender_pipeline_manifest.json`
- `scripts/render_alpha_art_blender_sprites.ps1`
- `tools/blender/render_alpha_art_v1.py`
- `crates/alife_game_app/src/production_asset_pipeline.rs`

The manifest binds required production roles to the currently active PNGs and
to future Blender render targets under `target/generated_art/alpha_blender_v1/`.
The target directory is intentionally untracked.

Required roles include:

- creature idle and hurt;
- selection ring;
- food;
- hazard;
- rock/obstacle;
- grass, soil/path, resource/grove, hazard-pressure, stone, water, and sand
  terrain;
- at least three prop/dressing variants.

## Local Tool Status

Blender is installed locally and validated through the launcher script. On this
machine the executable is not on PATH, but the launcher discovers the standard
Windows install at:

```text
C:\Program Files\Blender Foundation\Blender 5.1\blender.exe
```

The smoke command reports Blender 5.1.0 and the full render command writes the
expected generated PNGs under `target/generated_art/alpha_blender_v1/`.

Current local app-smoke status:

```text
blender_on_path=false
blender_discovered=true
blender_executable=C:\Program Files\Blender Foundation\Blender 5.1\blender.exe
local_render_status=READY_TO_RENDER
user_action_required=false
```

The Windows launcher is:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/render_alpha_art_blender_sprites.ps1
```

With Blender installed, it runs:

```powershell
blender --background --python tools/blender/render_alpha_art_v1.py -- --manifest crates/alife_game_app/assets/alpha_art_v1/blender_pipeline_manifest.json --out-dir target/generated_art/alpha_blender_v1
```

## Boundary

This pipeline is dev/art tooling only.

- It is not a runtime game dependency.
- It does not enter `alife_core`.
- It does not emit actions.
- It does not rewrite weights.
- It does not change simulation semantics.
- It does not alter CPU fallback or CPU shadow parity.

## Evidence

Focused commands:

```powershell
cargo test -p alife_game_app production_asset_pipeline -- --nocapture
cargo test -p alife_game_app --test app_shell production_asset_pipeline_smoke_tracks_blender_art_without_runtime_authority -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- production-asset-pipeline-smoke
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/render_alpha_art_blender_sprites.ps1 -CheckOnly
```

Expected local result when Blender is not installed:

```text
local_render_status=USER_ACTION_REQUIRED
user_action_required=true
```

That result remains honest evidence of a missing local art tool on other
machines, not a failed game runtime.

## Known Limitations

- Blender-generated replacement PNGs are not active yet. Blender is installed
  and generation works locally, but generated renders still require human review
  before promotion into the committed app asset pack.
- The active Player View no longer treats the current committed v41
  `alpha_art_v1` PNG pack as the world-art target. The active visual lane is
  committed Blender-normalized GLB assets from `true_25d_alpha_v1`; the PNG pack remains for
  HUD/debug/fallback evidence.
- The active GLB lane now has a Blender normalization/calibration step. The
  original direct generator script remains historical seed-art tooling, not the
  final committed active asset contract.
- The procedural terrain system is seeded and chunk/creature anchored, but it is
  still presentation/context-only rather than an authoritative Minecraft-like
  sensory/navigation/ecology substrate.
- No release/tag status changed.

## Next Step

Review the generated `target/generated_art/alpha_blender_v1/` outputs, then
promote selected renders into the versioned app asset pack in a separate bounded
slice.
