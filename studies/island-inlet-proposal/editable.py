"""Create the local editable Blender terrain model, not a game map or art override."""

import sys, math
from pathlib import Path
import bpy, numpy as np
from mathutils import Vector

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here))
from form import Reach, CONTROLS
from sculpt import deform

surface, skin, deposit, out = map(Path, sys.argv[sys.argv.index("--") + 1 :])
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
r = Reach(surface)
ctrl = bpy.data.objects.new("START HERE | estuary shape controls", None)
scene.collection.objects.link(ctrl)
ctrl.empty_display_type = "CIRCLE"
ctrl.empty_display_size = 2
ctrl.location = (137, -418, 14)
for k in CONTROLS:
    ctrl[k] = 1.0
    ctrl.id_properties_ui(k).update(
        min=0,
        max=1,
        description="0 is exact V1, 1 is the proposed sculpt; no game runtime binding",
    )
j, i = np.mgrid[303:341, 174:215]
x = r.x0 + i * r.s
z = r.z0 + j * r.s
elev = r.h[j, i]
v = np.stack([x, -z, elev], axis=-1).reshape(-1, 3)
faces = []
for row in range(j.shape[0] - 1):
    for col in range(i.shape[1] - 1):
        a = row * i.shape[1] + col
        b = a + 1
        c = b + i.shape[1]
        d = a + i.shape[1]
        faces.extend(((a, c, b), (a, d, c)))
me = bpy.data.meshes.new("Measured V1 near triangles")
me.from_pydata(v.tolist(), [], faces)
me.update()
ground = bpy.data.objects.new("01 · exact V1 reach / three artist controls", me)
scene.collection.objects.link(ground)
# An opaque, editable clay palette clarifies the shape; the accompanying
# photographed board shows the actual landscape and materials, not this clay.
color = me.color_attributes.new(
    name="Coastal clay and moor", type="BYTE_COLOR", domain="POINT"
)
grad = np.hypot(*np.gradient(elev, r.s)).ravel()
h = elev.ravel()
sw = np.clip((h + 0.2) / 4.3, 0, 1)
moor = np.clip((h - 2.5) / 4.5, 0, 1) * np.clip(1.3 - grad, 0, 1)
sandy = np.array([0.49, 0.39, 0.26])
clay = np.array([0.35, 0.26, 0.18])
mo = np.array([0.19, 0.26, 0.13])
wash = np.array([0.21, 0.27, 0.24])
colors = (sandy * (1 - sw[:, None]) + clay * sw[:, None]) * (
    1 - moor[:, None]
) + mo * moor[:, None]
colors = np.where((h < 0)[:, None], wash, colors)
color.data.foreach_set(
    "color",
    np.ascontiguousarray(np.column_stack([colors, np.ones(len(h))]), "f").ravel(),
)
mat = bpy.data.materials.new("Working clay | original V1 grades")
mat.use_nodes = True
node = mat.node_tree.nodes.new("ShaderNodeVertexColor")
node.layer_name = color.name
mat.node_tree.links.new(
    node.outputs["Color"], mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"]
)
ground.data.materials.append(mat)
for p in me.polygons:
    p.use_smooth = True
bpy.context.view_layer.update()
deform(ground, r, ctrl)
before = set(bpy.data.objects)
bpy.ops.import_scene.gltf(filepath=str(skin.resolve()))
bpy.context.view_layer.update()
for ob in set(bpy.data.objects) - before:
    if ob.type == "MESH":
        deform(ob, r, ctrl)
from deposit import attach as inner_sand

sediment = inner_sand(r, deposit)
bpy.context.view_layer.update()
deform(sediment, r, ctrl)
# Hide the point-bar material along with its shape when the artist zeroes it.
for key in ("hide_viewport", "hide_render"):
    drv = sediment.driver_add(key).driver
    vv = drv.variables.new()
    vv.name = "bar"
    vv.targets[0].id = ctrl
    vv.targets[0].data_path = '["Tapered point bar"]'
    drv.expression = "bar < 0.15"
bpy.ops.mesh.primitive_plane_add(size=2, location=(108, -410, 0))
sea = bpy.context.object
sea.name = "03 · sea level (unchanged)"
sea.scale = (115, 85, 1)
sea_mat = bpy.data.materials.new("Reference sea level")
sea_mat.diffuse_color = (0.05, 0.25, 0.26, 1)
sea_mat.use_nodes = True
sea_mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (
    0.05,
    0.25,
    0.26,
    1,
)
sea.data.materials.append(sea_mat)
cam = bpy.data.cameras.new("Study lens")
ob = bpy.data.objects.new("04 · original guardian lens", cam)
scene.collection.objects.link(ob)
ob.location = (145, -515, 62)
ob.rotation_euler = (
    (Vector((98, -419, 2.3)) - ob.location).to_track_quat("-Z", "Y").to_euler()
)
cam.type = "ORTHO"
cam.ortho_scale = 116
scene.camera = ob
notes = bpy.data.texts.new("READ ME · concept only")
notes.write(
    "EDITABLE SHAPE STUDY · not a game terrain, save or fixture. Select START HERE and adjust the three object properties, individually or together. All zero = exact original near ground, all one = proposed natural mouth. No native exports. Working clay and water are for modelling; full-context Blender art is in the matched plates.\n"
)
for area in bpy.context.screen.areas:
    if area.type == "VIEW_3D":
        area.spaces.active.region_3d.view_perspective = "CAMERA"
        area.spaces.active.shading.color_type = "VERTEX"
bpy.ops.object.select_all(action="DESELECT")
ctrl.select_set(True)
bpy.context.view_layer.objects.active = ctrl
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out.resolve()), compress=True)
