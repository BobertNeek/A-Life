"""Lock the review camera and light for terrain GLBs (Blender, CPU; not a gameplay capture)."""

import argparse
import json
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
cli.add_argument(
    "--ground-microfield",
    type=Path,
    help="Optional real-coordinate close substrate for DCC art only",
)
cli.add_argument("--surface", type=Path)
cli.add_argument("--placements", type=Path)
cli.add_argument(
    "--inlet-cut",
    type=Path,
    help="Local fitted surface artwork on the two real southern cut banks",
)
cli.add_argument(
    "--inlet-sculpt-study",
    type=Path,
    help="DCC-only shape alternative; never serialized to game terrain",
)
cli.add_argument("--prop-library", type=Path)
cli.add_argument(
    "--cliff-library",
    type=Path,
    help="Only the actual solid Cliff model at its planted positions",
)
cli.add_argument(
    "--young-library",
    type=Path,
    help="Only the original juvenile tree, never new placements.",
)
cli.add_argument(
    "--wild-sward",
    type=Path,
    help="Near fitted stool and plant communities of the wild valley",
)
cli.add_argument(
    "--meadow-library",
    type=Path,
    help="Only Grass and Flowers on existing world anchors",
)
cli.add_argument(
    "--meadow-habits",
    action="store_true",
    help="Read three named sedge families from the pictured native study pack",
)
cli.add_argument(
    "--broadleaf-library", type=Path, help="Only the native Broadleaf model"
)
cli.add_argument(
    "--fern-library", type=Path, help="Only the separably authored ordinary Fern model"
)
cli.add_argument(
    "--stone-library",
    type=Path,
    help="Actual candidate mineral skins for existing solid stones only",
)
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
    choices=(
        "open-bank",
        "grove-edge",
        "grove-gate",
        "shelf-edge",
        "inlet-eye",
        "inlet-bar",
        "estuary-mouth",
        "inlet-landing",
    ),
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
cli.add_argument(
    "--marine-sky",
    type=Path,
    help="Actual checksummed marine-layer backdrop; not an invented world gradient",
)
cli.add_argument(
    "--guardian-review",
    action="store_true",
    help="Fixed orthographic grove and ridge shoulder",
)
cli.add_argument(
    "--rock-eye", action="store_true", help="1.03 m diagnostic at a real boulder foot"
)
cli.add_argument(
    "--pine-eye",
    action="store_true",
    help="Creature among the lower sprays of actual pine 434",
)
cli.add_argument(
    "--grove-framing",
    choices=(
        "shoulder",
        "dense",
        "shelf",
        "alder-bank",
        "inlet-bank",
        "meadow-reach",
        "scarp-spur",
        "rift-root",
        "erratic-reach",
    ),
    default="shoulder",
    help="Select one of the two fixed guardian cameras",
)
cli.add_argument(
    "--grove-forms", type=Path, help="Variants of existing upper crowns only"
)
cli.add_argument(
    "--studio-samples",
    type=int,
    choices=(12, 48, 80, 96),
    help="Explicit art sample count for both sides of a paired still",
)
cli.add_argument(
    "--copse-eye",
    action="store_true",
    help="Near actual pine 803 and its sheltered forest floor",
)
cli.add_argument(
    "--copse-framing",
    choices=(
        "slope",
        "fern-hollow",
        "sunlit-margin",
        "alder-eye",
        "meadow-eye",
        "meadow-bells",
        "sapling-eye",
        "scarp-eye",
        "erratic-foot",
        "moraine-soil",
        "talus-eye",
    ),
    default="slope",
)
cli.add_argument(
    "--copse-floor", type=Path, help="Optional close-only rooted woodland community art"
)
args = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
args.out.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.device = "CPU"
scene.cycles.samples = (
    48
    if args.cove_showcase
    or args.guardian_review
    or args.rock_eye
    or args.pine_eye
    or args.copse_eye
    else 20
)
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
if args.studio_samples:
    scene.cycles.samples = args.studio_samples
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
if args.habitats and (args.habitats.parent / "geology/relief.png").is_file():
    old_normal = next(
        i
        for i in bpy.data.images
        if i.name in ("Island geological relief", "relief.png")
        or Path(bpy.path.abspath(i.filepath)).name == "relief.png"
    )
    new_normal = bpy.data.images.load(
        str((args.habitats.parent / "geology/relief.png").resolve())
    )
    new_normal.colorspace_settings.name = "Non-Color"
    bound = 0
    for mat in bpy.data.materials:
        if mat.use_nodes:
            for node in mat.node_tree.nodes:
                if getattr(node, "image", None) == old_normal:
                    node.image = new_normal
                    bound += 1
    assert bound > 0, "Far normal texture not changed by mineral art"
if args.habitats:
    previous = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.habitats.resolve()))
    for mesh in set(bpy.data.objects) - previous:
        if mesh.type == "MESH":
            _, ix, iz = mesh.name.split("_")
            habitats[int(ix), int(iz)] = mesh
if args.ground_microfield:
    from studio_ground_microfield import bind as apply_microfield

    apply_microfield(habitats.values(), args.ground_microfield)
if args.marine_sky:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.marine_sky.resolve()))
    envelope = next(i for i in set(bpy.data.objects) - old if i.type == "MESH")
    for property_name in (
        "visible_shadow",
        "visible_diffuse",
        "visible_glossy",
        "visible_transmission",
        "visible_volume_scatter",
    ):
        setattr(envelope, property_name, False)
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


rigid_plants = []
shots = [
    ("wide", (705, 795, 975), (0, 27, -12), 1120, 36, 2),
    ("estuary", (190, 54, 440), (4, 20, 236), 0, 35, 0),
    ("headland", (-480, 51, -72), (-265, 30, -212), 0, 36, 0),
]
if (
    args.creature_height
    or args.tidal_review
    or args.cove_showcase
    or args.guardian_review
    or args.rock_eye
    or args.pine_eye
    or args.copse_eye
):
    root = Path(__file__).resolve().parents[1]
    library = (
        args.prop_library or root / "crates/alife_game_app/assets/landscape/island"
    )
    from island_review_grid import Grid

    grid = Grid.read(args.surface or root / "crates/alife_world/assets/island-v1.bin")
    ground = grid.height
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
            "inlet-eye": (90.0, 451.0, 24.8, -19.6, -0.52, 33),
            "inlet-landing": (94.7, 454.0, -1.6, -4.0, -1.10, 36),
            "estuary-mouth": (128.8, 458.2, -21.4, 4.7, -3.35, 33),
            "inlet-bar": (94.15, 453.9, 2.45, -29.2, -1.20, 29),
            "shelf-edge": (345.20, 116.70, 13.7, -27.4, -1.36, 29),
            "grove-edge": (319.85, 143.85, 4.0, -66, 3.6, 22),
            "grove-gate": (320.80, 145.90, 9.0, -66, 3.2, 23),
        }[args.cove_framing]
        eye = ground(x, z) + 1.12
        shots = [
            (
                {
                    "shelf-edge": "shelf-eye",
                    "inlet-eye": "inlet-eye",
                    "inlet-bar": "inlet-bar",
                    "estuary-mouth": "estuary-mouth",
                    "inlet-landing": "inlet-landing",
                }.get(args.cove_framing, "cove-showcase"),
                (x, eye, z),
                (x + dx, eye + rise, z + dz),
                0,
                lens,
                0,
            )
        ]
    if args.guardian_review:
        dense = args.grove_framing == "dense"
        x, z = (-161.7, -38.25) if dense else (247.0, -184.0)
        eye = ground(x, z) + 2.0
        ortho_width = 156 if dense else 240
        shots = [
            (
                "guardian-woods" if dense else "guardian-grove",
                (x + 230, eye + 176, z + 280),
                (x, eye, z),
                ortho_width,
                35,
                1,
            )
        ]
        if args.grove_framing == "shelf":
            x, z = 350.6, 105.1
            # The sea plane is unmodified. Frame the immersed bank as well
            # as open water rather than manufacturing a new rocky headland.
            shots = [
                ("shelf-guardian", (x - 132, 108, z + 177), (x, 0.64, z), 173, 35, 1)
            ]
        if args.grove_framing == "alder-bank":
            tx, tz = 294.8, 148.0
            ty = ground(tx, tz) + 3.4
            shots = [
                ("alder-bank", (tx + 103, ty + 68, tz + 122), (tx, ty, tz), 146, 35, 1)
            ]
        if args.grove_framing == "inlet-bank":
            x, z = 98.0, 419.0
            shots = [("inlet-bank", (x + 47, 62, z + 96), (x, 2.3, z), 116, 35, 1)]
        if args.grove_framing == "rift-root":
            # Actual original 1395 extends out over the steep highland notch.
            x, z = 16.12, -189.17
            shots = [
                (
                    "rift-root",
                    (x + 68, ground(x, z) + 26, z + 31),
                    (x, ground(x, z) + 6, z),
                    43,
                    36,
                    0,
                )
            ]
        if args.grove_framing == "erratic-reach":
            # Use the actual deposit, not a tabletop arrangement: 1570, 1383
            # and 1675, with unaltered scree 1303 and the original treeline.
            tx, tz = 257.65, -184.20
            shots = [
                (
                    "erratic-reach",
                    (286.0, ground(tx, tz) + 22.5, -149.0),
                    (tx, ground(tx, tz) + 1.0, tz),
                    43.0,
                    37,
                    0,
                )
            ]
        if args.grove_framing == "scarp-spur":
            # Different native cliff: foothill 1731, including the real trees.
            x, z = 260.3, -282.2
            shots = [
                (
                    "scarp-spur",
                    (x + 64, ground(x, z) + 56, z + 70),
                    (x, ground(x, z) + 4.4, z),
                    72,
                    36,
                    0,
                )
            ]
        if args.grove_framing == "meadow-reach":
            x, z = 72.5, 299.0
            shots = [
                (
                    "meadow-reach",
                    (x + 24, ground(x, z) + 19, z + 32),
                    (x, ground(x, z) + 0.30, z),
                    39,
                    38,
                    0,
                )
            ]
    if args.rock_eye:
        x, z = 258.40, -170.00
        tx, tz = 254.68, -177.68
        shots = [
            (
                "stone-eye",
                (x, ground(x, z) + 1.03, z),
                (tx, ground(tx, tz) + 0.31, tz),
                0,
                26,
                0,
            )
        ]
    if args.pine_eye:
        # 1.02 m uphill of actual pine 434, looking through its outer
        # boughs into 21 trees on the opposite edge of the guardian stand.
        x, z, tx, tz = -149.04, -38.54, -151.27, -32.78
        shots = [
            (
                "pine-eye",
                (x, ground(x, z) + 1.02, z),
                (tx, ground(-152.35, -33.405) + 3.2, tz),
                0,
                29,
                0,
            )
        ]
    if args.copse_eye:
        x, z = 253.40, -207.05
        tx, tz = 250.40, -210.00
        shot = "copse-eye"
        if args.copse_framing == "fern-hollow":
            # East of the two original planted pines (2, 934). No foreground
            # models were moved or duplicated to compose the shared understory.
            x, z, tx, tz = 195.1, -246.15, 192.37, -245.96
            shot = "fern-hollow"
        elif args.copse_framing == "sunlit-margin":
            # Actual west-facing, sunlit grove edge, looking toward a
            # natural cluster of ferns around original planted tree 952.
            x, z, tx, tz = -149.85, 6.18, -146.4, 4.88
            shot = "sunlit-margin"
        elif args.copse_framing == "alder-eye":
            x, z, tx, tz = 321.6, 148.5, 316.1, 140.1
            shot = "alder-eye"
        elif args.copse_framing == "meadow-bells":
            # The existing Flowers anchor 3394; field turns into the open water.
            x, z, tx, tz = 76.74, 299.2, 75.8, 297.2
            shot = "meadow-bells"
        elif args.copse_framing == "sapling-eye":
            # Original #1116, one of three stock balloon-shaped juvenile trees.
            x, z, tx, tz = 50.1, 244.3, 45.447, 237.543
            shot = "sapling-eye"
        elif args.copse_framing == "talus-eye":
            # At original 1303 in the same woodland, with 1383 unchanged
            # beyond. Near side is reached from the open downhill flank.
            shot = "talus-eye"
            x, z, tx, tz = 269.35, -188.52, 266.00, -190.70
        elif args.copse_framing == "moraine-soil":
            # Across the real lee face from one animal height, so the native
            # 3.125m soil cell can be judged without a hovering spectator lens.
            shot = "moraine-soil"
            x, z, tx, tz = 254.93, -174.49, 255.29, -176.73
        elif args.copse_framing == "erratic-foot":
            # 1.04 m actual animal eye, three metres down and across the
            # slope from original 1570, looking back toward true pine 643.
            shot = "erratic-foot"
            x, z, tx, tz = 257.48, -176.11, 254.68, -177.67
        elif args.copse_framing == "scarp-eye":
            # Native highland cliff 1549. A 1.04m eye on the actual V1
            # northwest bench, with a clear 28m approach to the front.
            x, z, tx, tz = 91.69, -187.58, 112.9, -205.87
            shot = "scarp-eye"
        elif args.copse_framing == "meadow-eye":
            # Twenty existing grass anchors surround 3406. Share the real
            # turf with nine flower clumps in the established lower meadow.
            x, z, tx, tz = 76.4, 303.65, 70.1, 295.25
            shot = "meadow-eye"
        shots = [
            (
                shot,
                (x, ground(x, z) + 1.04, z),
                (
                    tx,
                    ground(tx, tz)
                    + {
                        "alder-eye": 3.75,
                        "sapling-eye": 2.7,
                        "meadow-bells": 0.28,
                        "scarp-eye": 4.46,
                        "erratic-foot": 0.74,
                        "moraine-soil": 0.055,
                        "talus-eye": 0.35,
                    }.get(shot, 0.38),
                    tz,
                ),
                0,
                31,
                0,
            )
        ]
    props = json.loads((args.placements or library / "props.json").read_text())
    models = {}
    from island_grove_review import sources, form

    forms = sources(args.grove_forms, models)
    review_span = shots[0][3] * 0.60 if args.guardian_review else 45.0
    cove_focus = None
    if args.cove_showcase:
        eye, aim = native(shots[0][1]), native(shots[0][2])
        cove_focus = eye + (aim - eye).normalized() * 15.0
    if args.guardian_review or args.rock_eye or args.pine_eye or args.copse_eye:
        cove_focus = native(shots[0][2])
    for index, prop in enumerate(props):
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
        level = (
            2
            if dist > 240
            else int(dist > 100 or (args.guardian_review and review_span > 80))
        )
        model_key = form(forms, index, kind, px, pz)
        seated = (
            kind == "Cliff"
            and args.cliff_library
            and (args.cliff_library / f"CliffSeat{index}.glb").is_file()
        )
        loose_seat = (
            kind == "Scree"
            and args.stone_library
            and (args.stone_library / f"ScreeSeat{index}.glb").is_file()
        )
        if seated or loose_seat:
            model_key = f"{kind}Seat{index}"
        if kind == "Grass" and args.meadow_habits:
            from island_meadow_variation import family

            model_key = family(index, px, ground(px, pz), pz)
        if (model_key, level) not in models:
            old = set(bpy.data.objects)
            stone = kind in ("Cliff", "Scree", "BoulderA", "BoulderB", "BoulderC")
            where = args.stone_library if stone and args.stone_library else library
            if kind == "Cliff" and args.cliff_library:
                where = (
                    args.cliff_library
                    if seated or not (args.cliff_library / "seated.json").is_file()
                    else library
                )
            if kind == "Sapling" and args.young_library:
                where = args.young_library
            if kind == "Broadleaf" and args.broadleaf_library:
                where = args.broadleaf_library
            if kind == "Fern" and args.fern_library:
                where = args.fern_library
            if kind in ("Grass", "Flowers") and args.meadow_library:
                where = args.meadow_library
            bpy.ops.import_scene.gltf(
                filepath=str(
                    where / f"{model_key if seated or loose_seat else kind}.glb"
                )
            )
            items = set(bpy.data.objects) - old
            names = (
                ["Grass", "GrassSilver", "GrassAmber"]
                if kind == "Grass" and args.meadow_habits
                else [model_key if seated or loose_seat else kind]
            )
            for name in names:
                for lod_index in (0, 1, 2):
                    item = next(
                        (o for o in items if o.name == name + f"_L{lod_index}"), None
                    )
                    if item is not None:
                        models[name, lod_index] = item
            for o in items:
                o.hide_render = True
        instance = models[model_key, level].copy()
        scene.collection.objects.link(instance)
        instance.hide_render = False
        if kind in ("Grass", "Fern", "Flowers", "BerryBush", "CoastalShrub"):
            py = ground(px, pz)
        instance.location = native((px, py, pz))
        instance.rotation_mode = "XYZ"
        instance.rotation_euler = (0, 0, prop["yaw"])
        instance.scale = (prop["scale"],) * 3
        rigid_plants.append(instance)
decor = []
if args.strandline:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.strandline.resolve()))
    decor = [o for o in set(bpy.data.objects) - old if o.type == "MESH"]
    for o in decor:
        o.visible_shadow = o.name.endswith("_headland")
if args.inlet_cut:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.inlet_cut.resolve()))
    skins = [o for o in set(bpy.data.objects) - old if o.type == "MESH"]
    for o in skins:
        o.visible_shadow = False
    decor.extend(skins)
sward = []
if args.wild_sward:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.wild_sward.resolve()))
    sward = [o for o in set(bpy.data.objects) - old if o.type == "MESH"]
    for o in sward:
        o.visible_shadow = False  # Ground-hugging seedbank; hero plants still cast.
copse = []
if args.copse_floor:
    old = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(args.copse_floor.resolve()))
    copse = [o for o in set(bpy.data.objects) - old if o.type == "MESH"]
    for ob in copse:
        ob.visible_shadow = False
if args.inlet_sculpt_study:
    here = Path(__file__).resolve().parents[1] / "studies/island-inlet-proposal"
    sys.path.insert(0, str(here))
    from form import Reach
    from sculpt import finish

    bpy.context.view_layer.update()
    from deposit import attach as inner_sand

    sculpt = Reach(args.surface)
    sediment = inner_sand(sculpt, args.inlet_sculpt_study)
    bpy.context.view_layer.update()
    finish(
        sculpt, [*chunks, *habitats.values(), *decor, *copse, sediment], rigid_plants
    )
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
            15.0
            if name
            in ("cove-showcase", "shelf-eye", "inlet-eye", "inlet-bar", "estuary-mouth")
            else (native(aim) - native(pos)).length
        )
        span = (
            review_span
            if name
            in (
                "cove-showcase",
                "guardian-grove",
                "guardian-woods",
                "shelf-eye",
                "shelf-guardian",
            )
            else (45.0 if name == "strand-reach" else 8.0)
        )
        scene.world.mist_settings.start = focus_m + mood["air_start_beyond_focus_m"]
        scene.world.mist_settings.depth = mood["air_span_min_m"] + 0.62 * span
    if decor:
        # Same 100 m terrain gate as the Bevy binding. The eye-level study
        # uses a 15 m focal subject; diagnostics use their original target.
        a, b = native(pos), native(aim)
        focus = (
            a + (b - a).normalized() * 15.0
            if name
            in ("cove-showcase", "shelf-eye", "inlet-eye", "inlet-bar", "estuary-mouth")
            else b
        )
        for o in decor:
            cx = math.floor(o.location.x / 100) * 100 + 50
            cz = math.floor(-o.location.y / 100) * 100 + 50
            o.hide_render = (args.guardian_review and review_span >= 100) or math.hypot(
                cx - focus.x, cz + focus.y
            ) >= 150
    if sward:
        b = native(aim)
        for ob in sward:
            ob.hide_render = (args.guardian_review and review_span >= 60) or math.hypot(
                ob.location.x - b.x, ob.location.y - b.y
            ) >= 55
    if copse:
        b = native(aim)
        for ob in copse:
            ob.hide_render = (args.guardian_review and review_span >= 60) or math.hypot(
                ob.location.x - b.x, ob.location.y - b.y
            ) >= 65
    for mesh in habitats.values():
        mesh.hide_render = True
    for ob in chunks:
        _, ix, iz, level = ob.name.split("_")
        key = int(ix), int(iz)
        if (
            args.cove_showcase
            or args.guardian_review
            or args.rock_eye
            or args.pine_eye
            or args.copse_eye
        ):
            target = grid.level(key, (cove_focus.x, -cove_focus.y), review_span)
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
