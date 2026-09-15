# Mountainous landscape revision 2

Authored with Blender 5.2.1 LTS. The landscape library retains the existing oak
and berry assets and replaces the three rounded boulders with fractured,
stratified rocks. Grass now uses curved folded blades. New reusable assets are
`Cliff_outcrop`, `Fern_patch`, and `Wildflower_patch`. All use solid geometry and
ordinary glTF PBR materials; vegetation and rocks carry vertex colors.

`highlands/mountain-valley.blend` is the editable 800 x 1050 metre landscape.
`highlands/Mountain_Valley.glb` contains continuous terrain, a river, layered
mountain ridges, exposed rock slopes, high-altitude snow patches, scree, groves,
and ground cover. Packed seamless grain and tangent normal textures add ground
relief without procedural shader dependencies. The broad river valley stays
open. Repeated props share mesh data. The studio cameras and lights stay in the
Blender file, outside the GLB.

Reproduce from the repository root with the explicit Blender 5.2.1 executable:

1. `blender --background --factory-startup --python-exit-code 1 --python scripts/export_landscape.py`
2. `blender --background --factory-startup --python-exit-code 1 --python scripts/build_highlands.py`
3. `python scripts/register_approved_art.py`
4. `python scripts/check_landscape_assets.py`
5. `blender --background --factory-startup --python-exit-code 1 --python scripts/render_highlands.py`

The review reimports the GLB and renders wide, ground-level, and clay views.
The existing graphics branch consumes the replacement boulder and grass asset
paths. The full highland mesh and additional dressing are an art delivery:
world heights, collision, navigation, streaming/LOD, and main-branch integration
are not implemented by these authoring scripts. Do not substitute this visual
height function for authoritative simulation state. Runtime performance and
in-game presentation remain unverified.
