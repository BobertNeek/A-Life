# Highlands runtime

New Game uses the approved Blender valley as physical terrain. Existing saves
without a terrain binding retain their original world. The binding records
version 1 and the digest of the immutable bake; incompatible data fails loading.

`scripts/export_highlands_runtime.py` reads `mountain-valley.blend` and exports
all three products together:

- `crates/alife_world/assets/highlands-v1.bin`: 321 by 421 height samples at 2.5 m,
  and 1,288 static collision proxies. World coordinates are Y-up, X/Z horizontal.
- `terrain-chunks.glb`: 140 chunks with three mesh levels and boundary skirts.
- `props.json`: 3,441 placements using the shared landscape library meshes.

World movement sweeps the body footprint across the exact near-mesh triangles.
Deep water, slopes above a 0.85 gradient, trunks, boulders, cliffs, and map edges
block movement and produce the ordinary physical blocked outcome. New-game and
editor placement, live creature projection, camera clearance, selection rays,
and foot grounding use this same surface. Collision proxies are conservative;
this is a walking heightfield, without caves, swimming, jumping, or rigid-body
climbing on prop meshes.

Performance policy: terrain levels change at 150 and 350 m, evaluated five times
per second. Chunks occupied by creatures retain the exact near surface. Skirts
hide seams between levels. Total terrain triangles if every chunk used the same
level are 303,520 / 84,560 / 25,480. Props share mesh and material handles for
Bevy batching; grass draws within 45 m, ferns/flowers 38 m, rocks/shrubs 210 m,
and trees 360 m. Small vegetation does not cast shadows. The full authoring scene
`Mountain_Valley.glb` is retained for review and is not loaded into gameplay.
The runtime pack has a 12 MiB individual-asset cap and a 32 MiB total cap;
authoring scenes are excluded from this manifest.

Export with Blender 5.2.1 LTS, then run `python scripts/check_highlands_runtime.py`
and `python scripts/register_approved_art.py`. The check compares every exported
terrain vertex against the collision bake and verifies upward triangle winding,
LOD budgets, and reusable prop transforms.

Verification on source `0161fb546a329c6974a5339b2be611f5c4a32bc2`:
37 world tests passed; 96 in-game captures contained 576 creature samples with
zero horizontal projection error and 0.025–0.043 m foot clearance. The shipping
character loaded with all three clips and its painted eyes/coat intact.

A 60-second release run on an NVIDIA RTX 3050, Vulkan, 1920x1080,
MinSpecComfort1080p, eight founders (the current New Game maximum), averaged
41.47 FPS. Median frame time was 8.26 ms; p95 was 174.98 ms, with 144 frames over
100 ms. The worst 270.48 ms frame spent 261.52 ms inside the live GPU tick;
rendering/presentation and uninstrumented residual was 8.37 ms. Simulation
achieved 13.55 of the configured 20 ticks/second. Thus the graphics pass is
integrated, but the existing GPU runtime still causes significant stalls.
This is not a larger-population performance result or a smooth-gameplay claim.
The full receipt is `target/artifacts/phase31-performance/phase31-before-release-population-8.json`.
