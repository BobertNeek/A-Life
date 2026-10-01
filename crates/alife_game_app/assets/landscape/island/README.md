# Approved island asset pack

Blender 5.2.1 LTS sources use metres, Z-up, and ground-centred prop origins.
`island-assets.blend` contains 13 reusable assets, three mesh LODs, dimension
guides, and the approved image blueprints. `island-terrain.blend` contains the
1000 x 1200 metre island, 240 metre peaks, ocean, river, and placed shared props.
The `Measured_Blueprint_Guides` collection is for authoring, not export.

Runtime terrain has 120 chunks with 20 metre skirts. All LODs use the same
vertex samples as the authoritative `alife_world/assets/island-v1.bin`.
Near/middle/overview triangle totals are 276480 / 76800 / 23040. The 3716 prop
instances share meshes and materials. Collision uses 1716 world-owned proxies.
Ground cover culls at 45 metres; LOD uses camera focus and viewport coverage.
Terrain zoom spans a 9.8 to 1600 metre vertical view on every graphics profile.
Camera clearance preserves the ground focus, and the two-triangle ocean extends
past the widest view. Overview shadows switch off above a 240 metre view.
Normalized byte colors reduce file size without changing heights or normals.
Existing Highlands saves retain their original dataset and assets.

Rebuild from the repository root, using the explicit Blender 5.2.1 executable:

1. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_island_assets.py -- --no-review`
2. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_island_terrain.py -- --no-review`
3. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/build_wizard_hand.py -- --no-review`
4. `blender --background --threads 4 --factory-startup --python-exit-code 1 --python scripts/finalize_island_blueprints.py`
5. `python scripts/register_approved_art.py`
6. `python scripts/check_island_art.py`

Omit `--no-review` to render material and actual polygon-edge reviews. Original
approved concepts must be present under the dated `target/artifacts/art-review`
folders when rebuilding from scratch; copies are embedded in the delivered
Blender files. Measurements and budgets are also in the specification JSON.
