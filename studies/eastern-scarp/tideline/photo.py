"""On-site coastal ground photograph, never a detached specimen on a tabletop."""

from pathlib import Path
import json, sys, runpy, bpy
from mathutils import Vector

here = Path(__file__).resolve().parent
native, scene, shelf, hollow, cliff, cups, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:7]
)
args = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(p) for p in (native, scene, shelf, hollow, cliff, out)],
    "setup",
]
try:
    s = runpy.run_path(str(here.parent / "photo.py"), run_name="held_gullglass")
finally:
    sys.argv = args
sc = s["sc"]
cv = s["cv"]
surf = s["surf"]
base_cam = s["cams"]["wave-eye"]
eye = s["poses"]["wave-eye"][0]
target = [408.50, s["floor"](408.50, -99.72) + 0.055, -99.72]
cam = bpy.data.cameras.new("Foothold")
cam.lens = 33
cam.clip_end = 3200
o = bpy.data.objects.new("Foothold", cam)
sc.collection.objects.link(o)
o.location = cv(eye)
o.rotation_euler = (cv(target) - o.location).to_track_quat("-Z", "Y").to_euler()


def shot(name, cam, side):
    (out / side).mkdir(exist_ok=True, parents=True)
    sc.camera = cam
    sc.render.filepath = str(out / side / (name + ".png"))
    bpy.ops.render.render(write_still=True)


shot("foothold", o, "before")
data = json.loads((cups / "patina.json").read_text())
u0, u1, v0, v1 = data["side_bounds"]
for mat in {
    q.material
    for g in s["ground"]
    for q in g.material_slots
    if q.material and q.material.use_nodes
}:
    nt = mat.node_tree
    bs = surf.bsdf(mat)
    uv = surf.coords(
        nt, (1, -1 / (u1 - u0), -u0 / (u1 - u0)), (2, 1 / (v1 - v0), -v0 / (v1 - v0))
    )
    tx = nt.nodes.new("ShaderNodeTexImage")
    tx.image = bpy.data.images.load(str(cups / "tide-patina.png"), check_existing=True)
    tx.image.alpha_mode = "CHANNEL_PACKED"
    tx.extension = "CLIP"
    nt.links.new(uv, tx.inputs["Vector"])
    p = nt.nodes.new("ShaderNodeNewGeometry")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    nt.links.new(p.outputs["Position"], sep.inputs[0])
    lo, hi = data["x_support"]
    ms = []
    for op, lim in [("GREATER_THAN", lo), ("LESS_THAN", hi)]:
        m = nt.nodes.new("ShaderNodeMath")
        m.operation = op
        m.inputs[1].default_value = lim
        nt.links.new(sep.outputs[0], m.inputs[0])
        ms.append(m.outputs[0])
    fac = tx.outputs["Alpha"]
    for gate in ms:
        mul = nt.nodes.new("ShaderNodeMath")
        mul.operation = "MULTIPLY"
        nt.links.new(fac, mul.inputs[0])
        nt.links.new(gate, mul.inputs[1])
        fac = mul.outputs[0]
    mix = nt.nodes.new("ShaderNodeMixRGB")
    col = bs.inputs["Base Color"]
    nt.links.new(col.links[0].from_socket, mix.inputs[1])
    nt.links.new(tx.outputs["Color"], mix.inputs[2])
    nt.links.new(fac, mix.inputs[0])
    nt.links.new(mix.outputs[0], col)
s["grab"](cups / "cups-near.glb")
shot("wave-eye", base_cam, "after")
shot("foothold", o, "after")
(out / "angles.json").write_text(
    json.dumps(
        dict(
            standing_native_eye=eye,
            close_target=target,
            close_lens_mm=33,
            hold="9b9999b592978e93d590609ec5daa402fe960494",
            real_terrain=True,
            sea_images_held=True,
            not_native=True,
        ),
        indent=2,
    )
    + "\n"
)
