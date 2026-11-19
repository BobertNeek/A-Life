"""Editable collections for the original and every individually fitted V1 scarp."""

from pathlib import Path
import sys, tempfile
import bpy
from mathutils import Euler

args = sys.argv[sys.argv.index("--") + 1 :]
source, target = map(Path, args[:2])
sys.path.insert(0, str(Path(__file__).resolve().parent))
from finished_island_glb import parse, save

d, buf = parse(source)
d["scenes"] = [dict(name="Landmark sculpt library", nodes=list(range(len(d["nodes"]))))]
d["scene"] = 0
bpy.ops.wm.read_factory_settings(use_empty=True)
with tempfile.TemporaryDirectory() as tmp:
    stage = Path(tmp)
    (stage / "minerals").symlink_to(
        (source.parent / "minerals").resolve(), target_is_directory=True
    )
    save(d, buf, stage / "scarp-forms.glb")
    options = bpy.ops.import_scene.gltf.get_rna_type().properties.keys()
    extra = {"merge_vertices": True} if "merge_vertices" in options else {}
    bpy.ops.import_scene.gltf(filepath=str(stage / "scarp-forms.glb"), **extra)
    bpy.ops.file.pack_all()
scene = bpy.context.scene
objects = [ob for ob in bpy.data.objects if ob.type == "MESH"]
for ob in objects:
    root, level = ob.name.rsplit("_L", 1)
    idx = root.replace("CliffSeat", "")
    cname = "Original fallback" if root == "Cliff" else "V1 landmark " + idx
    coll = bpy.data.collections.get(cname)
    if not coll:
        coll = bpy.data.collections.new(cname)
        scene.collection.children.link(coll)
    for old in list(ob.users_collection):
        old.objects.unlink(ob)
    coll.objects.link(ob)
    show = ob.name == "CliffSeat1549_L0"
    ob.hide_set(not show)
    ob.hide_render = not show
    ob["authoring_subject"] = cname
    ob["physical_authority"] = (
        "Display geometry only. Native V1 obstacle bounds/placed roots remain unchanged."
    )
active = bpy.data.objects.get("CliffSeat1549_L0")
if active:
    bpy.ops.object.select_all(action="DESELECT")
    active.select_set(True)
    bpy.context.view_layer.objects.active = active
for area in bpy.context.screen.areas if bpy.context.screen else []:
    if area.type == "VIEW_3D":
        sp = area.spaces.active
        sp.region_3d.view_location = (0, 0, 5.4)
        sp.region_3d.view_distance = 17
        sp.region_3d.view_rotation = Euler((1.30, 0, -0.48)).to_quaternion()
        sp.shading.type = "MATERIAL"
scene["guide"] = (
    "Choose a V1 landmark collection and its L0/L1/L2 in the outliner. Geometry at local planted origin. The original fallback is in its own collection."
)
bpy.ops.wm.save_as_mainfile(filepath=str(target.resolve()), compress=True)
