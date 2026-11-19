"""Optional full habitat Blender photography. Not a game, CI job, or desk frame.
The earlier art stage loads untouched native land/sky/props, stock water, and
previous Hollow/shore pigment. Its full real surroundings remain on both sides.
Use only in an authorized art window with sufficient DCC resources.
"""

from pathlib import Path
import json, runpy, sys
import bpy

H = Path(__file__).resolve().parent
argv = sys.argv[sys.argv.index("--") + 1 :]
if len(argv) != 13:
    raise ValueError(
        "NATIVE SCENE SHELF HOLLOW HEADLAND CUPS UPPER TOE WASH LACE RISE WOVEN OUTPUT"
    )
(
    native,
    scene,
    shelf,
    hollow,
    headland,
    cups,
    upper,
    toe,
    wash,
    lace,
    rise,
    woven,
    out,
) = map(Path, argv)
out.mkdir(parents=True, exist_ok=True)
old = sys.argv
sys.argv = [
    "blender",
    "--",
    *[
        str(x)
        for x in (
            native,
            scene,
            shelf,
            hollow,
            headland,
            cups,
            upper,
            toe,
            wash,
            lace,
            rise,
            out,
        )
    ],
    "setup",
]
try:
    s = runpy.run_path(
        str(H.parent / "photo.py"), run_name="frozen_full_gullglass_rise"
    )
finally:
    sys.argv = old

sc = s["sc"]
cv = s["cv"]
draw = s["s"]["grab"]
cameras = dict(s["cams"])
physical = json.loads((woven / "kit.json").read_text())["matched_second_eye"]
cam = bpy.data.cameras.new("Near stitchgrass, same planted native 1.04m eye")
cam.lens = physical["lens"]
cam.clip_end = 3600
obj = bpy.data.objects.new(cam.name, cam)
sc.collection.objects.link(obj)
obj.location = cv(physical["eye"])
obj.rotation_euler = (
    (cv(physical["aim"]) - obj.location).to_track_quat("-Z", "Y").to_euler()
)
cameras["stitchgrass-glance"] = obj

for side in ("before", "after"):
    (out / side).mkdir(exist_ok=True)
    if side == "after":
        runpy.run_path(str(H / "material.py"))["paint"](s["ground"], s["surf"], woven)
    for lod, names in [
        ("eye", ["holding-grove", "stitchgrass-glance"]),
        ("guardian", ["east-section"]),
    ]:
        objects = draw(rise / ("rise-" + lod + ".glb"))
        if side == "after":
            objects += draw(woven / ("stitchgrass-" + lod + ".glb"))
        for name in names:
            sc.camera = cameras[name]
            sc.render.filepath = str(out / side / (name + ".png"))
            bpy.ops.render.render(write_still=True)
        for o in objects:
            o.hide_render = True
(out / "capture-source.json").write_text(
    json.dumps(
        dict(
            plate="matched full-authored habitat in Blender only",
            native_gameplay=False,
            near=physical,
            baseline="Same Gullglass native sky, water, land, six local pines and surrounding source props, earlier Hollow/Rise",
            only_change="Opt-in new stitchgrass plus flat pigment transfer and true-ground dead blades/seed scales",
        ),
        indent=2,
    )
    + "\n"
)
