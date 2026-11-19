"""Lock the review camera and light for terrain GLBs (Blender, CPU; not a gameplay capture)."""

import argparse
import json
import struct
import math
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mathutils import Vector

cli = argparse.ArgumentParser(description=__doc__)
cli.add_argument("glb", type=Path)
cli.add_argument("out", type=Path)
cli.add_argument("--wide-only", action="store_true")
cli.add_argument("--creature-height", action="store_true")
cli.add_argument(
    "--tidal-review", action="store_true", help="Two fixed views of the eastern strand"
)
cli.add_argument(
    "--strandline", type=Path, help="Optional actual strand GLB for matched art review"
)
cli.add_argument(
    "--handoff",
    action="store_true",
    help="hold the camera at the river head and vary only surface LOD",
)
cli.add_argument(
    "--water-inputs", type=Path, help="Lossless maps from the native CPU water builder"
)
cli.add_argument(
    "--habitats", type=Path, help="Replace only the corresponding selected L0 chunks"
)
cli.add_argument("--surface", type=Path)
cli.add_argument("--placements", type=Path)
cli.add_argument("--prop-library", type=Path)
cli.add_argument(
    "--cove-showcase",
    action="store_true",
    help="Locked shore-to-massif habitat composition",
)
cli.add_argument(
    "--coastal-light",
    type=Path,
    help="Actual production palette and camera-relative air",
)
cli.add_argument(
    "--cove-framing",
    choices=("open-bank", "grove-edge", "grove-gate"),
    default="open-bank",
)
cli.add_argument(
    "--quick-proof",
    action="store_true",
    help="Private reduced framing scout, not an acceptance plate",
)
cli.add_argument(
    "--original-macro",
    type=Path,
    help="Frozen predecessor's shared far pigment for an exact A/B",
)
args = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
args.out.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.device = "CPU"
scene.cycles.samples = 48 if args.cove_showcase else 20
scene.cycles.use_denoising = (
    False  # OIDN erased metre-scale grass and swash ripples in the near field.
)
scene.cycles.seed = 27
scene.render.threads_mode = "FIXED"
scene.render.threads = 2
scene.render.resolution_x, scene.render.resolution_y = 1000, 600
scene.render.resolution_percentage = 52 if args.quick_proof else 100
if args.quick_proof:
    scene.cycles.samples = 12
scene.view_settings.view_transform = "AgX" if bpy.app.version >= (4, 0, 0) else "Filmic"
scene.world = bpy.data.worlds.new("Overcast blue skylight")
scene.world.use_nodes = True
bg = scene.world.node_tree.nodes["Background"]
bg.inputs["Color"].default_value = (0.51, 0.66, 0.77, 1)
bg.inputs["Strength"].default_value = 0.62
bpy.ops.import_scene.gltf(filepath=str(args.glb.resolve()))
chunks = [o for o in bpy.data.objects if o.type == "MESH"]
assert len(chunks) == 360
habitats = {}
if args.habitats or args.original_macro:
    # Far and middle use the literal replacement for the old global pigment;
    # normal/roughness, all mesh topology and the stock range gate stay intact.
    old = next(
        i
        for i in bpy.data.images
        if Path(bpy.path.abspath(i.filepath)).name == "pigment.png"
        or i.name in ("Island salt and stone", "pigment.png")
    )
    replacement = bpy.data.images.load(
        str(
            (
                args.original_macro or (args.habitats.parent / "geology/pigment.png")
            ).resolve()
        )
    )
    matches = 0
    for material in bpy.data.materials:
        if material.use_nodes:
            for node in material.node_tree.nodes:
                if getattr(node, "image", None) == old:
                    node.image = replacement
                    matches += 1
    assert (
        matches > 0
    ), "Real far terrain material must consume the actual replacement pigment"
if args.habitats:
    previous = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.habitats.resolve()))
    for mesh in set(bpy.data.objects) - previous:
        if mesh.type == "MESH":
            _, ix, iz = mesh.name.split("_")
            habitats[int(ix), int(iz)] = mesh
sun = bpy.data.lights.new("Low warm sun", "SUN")
sun.energy, sun.angle = 2.2, math.radians(8)
sun.color = (1.0, 0.89, 0.75)
light = bpy.data.objects.new("Sun", sun)
scene.collection.objects.link(light)
light.rotation_euler = Vector((0.90, 0.52, -0.71)).to_track_quat("-Z", "Y").to_euler()
# Neutral sea stand-in to expose the actual shoreline. No plants, nursery props,
# engine lighting or other scenery; the A/B is strictly the shipping terrain art.
bpy.ops.mesh.primitive_plane_add(size=8000)
ocean = bpy.context.object
mat = bpy.data.materials.new("Sea swatch")
mat.use_nodes = True
p = mat.node_tree.nodes["Principled BSDF"]
p.inputs["Base Color"].default_value = (0.025, 0.13, 0.18, 1)
p.inputs["Roughness"].default_value = 0.57
ocean.data.materials.append(mat)
if args.water_inputs:
    from island_review_water import bind

    bind(ocean, args.water_inputs)
cam = bpy.data.cameras.new("Locked lens")
cam.clip_end = 9000
obj = bpy.data.objects.new("Locked camera", cam)
scene.collection.objects.link(obj)
scene.camera = obj


# Coordinates below are in the game's Y-up space; Blender is Z-up.
def native(p):
    return Vector((p[0], -p[2], p[1]))


shots = [
    ("wide", (705, 795, 975), (0, 27, -12), 1120, 36, 2),
    ("estuary", (190, 54, 440), (4, 20, 236), 0, 35, 0),
    ("headland", (-480, 51, -72), (-265, 30, -212), 0, 36, 0),
]
if args.creature_height or args.tidal_review or args.cove_showcase:
    root = Path(__file__).resolve().parents[1]
    library = (
        args.prop_library or root / "crates/alife_game_app/assets/landscape/island"
    )
    raw = (
        args.surface or root / "crates/alife_world/assets/island-v1.bin"
    ).read_bytes()
    width, depth, ox, oz, spacing, _ = struct.unpack_from("<IIfffI", raw, 8)
    heights = struct.unpack_from(f"<{width*depth}f", raw, 32)

    def ground(x, z):
        u, v = (x - ox) / spacing, (z - oz) / spacing
        i, j = math.floor(u), math.floor(v)
        u, v = u - i, v - j
        a = j * width + i
        p, q, r, s = (
            heights[a],
            heights[a + 1],
            heights[a + width + 1],
            heights[a + width],
        )
        return (
            p + (q - p) * u + (r - q) * v if u >= v else p + (r - s) * u + (s - p) * v
        )

    x, z = 99.9, 256.9
    focus = (x, ground(x, z) + 0.88, z)
    aim = (98.3, ground(98.3, 253.1) + 0.045, 253.1)
    shots = [("creature-height", focus, aim, 0, 30, 0)]
    if args.tidal_review:
        x, z = 344.8, 114.8
        shots = [
            (
                "strand-eye",
                (x, ground(x, z) + 0.86, z),
                (347.05, ground(347.05, 110.86) + 0.16, 110.86),
                0,
                32,
                0,
            ),
            (
                "strand-reach",
                (334.4, ground(334.4, 139.3) + 4.2, 139.3),
                (348.2, 1.05, 104.8),
                0,
                43,
                0,
            ),
        ]
    if args.cove_showcase:
        # From the rooted windbreak inland of the strand: mountain left, bay
        # right; both remain visible from a human/creature-scale viewpoint.
        x, z, dx, dz, rise, lens = {
            "open-bank": (330.60, 125.65, -12.0, -66, 4.1, 21),
            "grove-edge": (319.85, 143.85, 4.0, -66, 3.6, 22),
            "grove-gate": (320.80, 145.90, 9.0, -66, 3.2, 23),
        }[args.cove_framing]
        eye = ground(x, z) + 1.12
        shots = [
            ("cove-showcase", (x, eye, z), (x + dx, eye + rise, z + dz), 0, lens, 0)
        ]
    props = json.loads((args.placements or library / "props.json").read_text())
    models = {}
    cove_focus = None
    if args.cove_showcase:
        eye, aim = native(shots[0][1]), native(shots[0][2])
        cove_focus = eye + (aim - eye).normalized() * 15.0
    for prop in props:
        px, py, pz = prop["position"]
        kind = prop["kind"]
        tree = kind in ("Broadleaf", "Conifer", "Sapling")
        if cove_focus is not None:
            dist = math.hypot(px - cove_focus.x, pz + cove_focus.y)
            limit = 45 if kind in ("Grass", "Fern", "Flowers") else 360 if tree else 210
        else:
            dist = math.hypot(px - x, pz - z)
            limit = 65
        if dist >= limit:
            continue
        level = 2 if dist > 240 else int(dist > 100)
        if (kind, level) not in models:
            old = set(bpy.data.objects)
            bpy.ops.import_scene.gltf(filepath=str(library / f"{kind}.glb"))
            items = set(bpy.data.objects) - old
            for lod_index in (0, 1, 2):
                item = next(
                    (o for o in items if o.name == kind + f"_L{lod_index}"), None
                )
                if item is not None:
                    models[kind, lod_index] = item
            for o in items:
                o.hide_render = True
        instance = models[kind, level].copy()
        scene.collection.objects.link(instance)
        instance.hide_render = False
        if kind in ("Grass", "Fern", "Flowers", "BerryBush", "CoastalShrub"):
            py = ground(px, pz)
        instance.location = native((px, py, pz))
        instance.rotation_mode = "XYZ"
        instance.rotation_euler = (0, 0, prop["yaw"])
        instance.scale = (prop["scale"],) * 3
decor = []
if args.strandline:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.strandline.resolve()))
    decor = [o for o in set(bpy.data.objects) - old if o.type == "MESH"]
    for o in decor:
        o.visible_shadow = o.name.endswith("_headland")
if args.wide_only:
    shots = shots[:1]
if args.handoff:
    shots = [
        (f"handoff-{lod}", (250, 245, 10), (33, 109, -198), 350, 36, lod)
        for lod in (0, 1, 2)
    ]
mood = None
if args.coastal_light:
    from island_review_light import apply

    mood = apply(scene, sun, light, args.coastal_light)
for name, pos, aim, ortho, lens, lod in shots:
    if mood:
        focus_m = (
            15.0 if name == "cove-showcase" else (native(aim) - native(pos)).length
        )
        span = 45.0 if name in ("cove-showcase", "strand-reach") else 8.0
        scene.world.mist_settings.start = focus_m + mood["air_start_beyond_focus_m"]
        scene.world.mist_settings.depth = mood["air_span_min_m"] + 0.62 * span
    if decor:
        # Same 100 m terrain gate as the Bevy binding. The eye-level study
        # uses a 15 m focal subject; diagnostics use their original target.
        a, b = native(pos), native(aim)
        focus = a + (b - a).normalized() * 15.0 if name == "cove-showcase" else b
        for o in decor:
            cx = math.floor(o.location.x / 100) * 100 + 50
            cz = math.floor(-o.location.y / 100) * 100 + 50
            o.hide_render = math.hypot(cx - focus.x, cz + focus.y) >= 150
    for mesh in habitats.values():
        mesh.hide_render = True
    for ob in chunks:
        _, ix, iz, level = ob.name.split("_")
        key = int(ix), int(iz)
        if args.cove_showcase:
            # The showcase represents the ordinary unoccupied 100 m cells.
            # Never draw a mountain with near-only geometry just to flatter it.
            center_x = ox + min(32 * int(ix) + 16, width - 1) * spacing
            center_z = oz + min(32 * int(iz) + 16, depth - 1) * spacing
            distance = math.hypot(center_x - cove_focus.x, center_z + cove_focus.y)
            target = 0 if distance < 150 else 1 if distance < 350 else 2
        else:
            target = lod
        drawn = level == f"L{target}"
        replace = drawn and target == 0 and key in habitats
        ob.hide_render = not drawn or replace
        if replace:
            habitats[key].hide_render = False
    obj.location = native(pos)
    obj.rotation_euler = (
        (native(aim) - obj.location).to_track_quat("-Z", "Y").to_euler()
    )
    cam.type = "ORTHO" if ortho else "PERSP"
    cam.ortho_scale, cam.lens = max(0.01, ortho), lens
    scene.render.filepath = str((args.out / f"{name}.png").resolve())
    bpy.ops.render.render(write_still=True)
