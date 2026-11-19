"""True inner/outer ridge habitat continuity at Gullglass. No promised sight of sea from shelter.

Previous full art (stock V1 coast, Gullglass source pigments/wash, existing Hollow
plantings) is held in both sides. New rooted palette is the only plant change.
No simulation invoked. Eye and guardian are genuinely different postures.
"""

from pathlib import Path
import bpy, sys, runpy, json
from mathutils import Vector

H = Path(__file__).resolve().parent
native, scene, shelf, hollow, base, cups, upper, toe, wash, lace, rise, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:12]
)
mode = sys.argv[
    sys.argv.index("--") + 13 :
]  # optional controls, otherwise both source pairs
save = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(a) for a in (native, scene, shelf, hollow, base, out)],
    "setup",
]
try:
    s = runpy.run_path(str(H.parent / "photo.py"), run_name="held_ridge_coast")
finally:
    sys.argv = save
sc = s["sc"]
ground = s["ground"]
surf = s["surf"]
water = sc.objects["Held native V1 sea level"]


def bind(p):
    return runpy.run_path(str(p))["bind"]


bind(H.parent / "outfold/material.py")(ground, surf, base, upper)
bind(H.parent / "toe/bind.py")(ground, surf, base, toe)
bind(H.parent / "tideline/contact_material.py")(ground, surf, cups)
bind(H.parent / "bladderlace/material.py")(ground, surf, lace)
bind(H.parent / "wash/material.py")(water, surf, wash)
# These shore stations are entirely outside the held footprint's 14 m near
# seaweed range. Do not burn geometry or make a fake ocean visible inland.
floor = s["floor"]
cv = s["cv"]
# Draw a real previously rooted windwillow in the lower foreground, and use
# the genuine paired pine crowns as the far landmarks. GPU perf not inferred.
eye = (366.2, floor(366.2, -83.7) + 1.04, -83.7)
poses = {
    "holding-grove": (eye, (385.36, 22.34, -82.1), 0.0, 28),
    "east-section": ((351.0, 82.0, 11.0), (372.1, 13.2, -82.4), 129.0, 40),
}
cams = {}
for n, (p, q, span, lens) in poses.items():
    cam = bpy.data.cameras.new(n)
    cam.type = "ORTHO" if span else "PERSP"
    cam.lens = lens
    cam.ortho_scale = max(0.001, span)
    cam.clip_end = 3600
    obj = bpy.data.objects.new(n, cam)
    sc.collection.objects.link(obj)
    obj.location = cv(p)
    obj.rotation_euler = (cv(q) - obj.location).to_track_quat("-Z", "Y").to_euler()
    cams[n] = obj


def shot(n, side):
    (out / side).mkdir(parents=True, exist_ok=True)
    sc.camera = cams[n]
    sc.render.filepath = str(out / side / (n + ".png"))
    bpy.ops.render.render(write_still=True)


if "after-only" not in mode:
    for n in poses:
        shot(n, "before")
runpy.run_path(str(H / "material.py"))["paint"](ground, surf, rise)
# Load one station at a time so there is no double rendering of the LODs. The
# target is matched before/after within station, not forcing the two stations
# to use identical viewing distances or foliage detail.
for name, lod in [("holding-grove", "eye"), ("east-section", "guardian")]:
    objs = s["grab"](rise / ("rise-" + lod + ".glb"))
    shot(name, "after")
    for obj in objs:
        obj.hide_render = True
(out / "rise-cameras.json").write_text(
    json.dumps(
        dict(
            cameras=poses,
            original_twin_props=[113, 638],
            original_terrain=True,
            existing_hollow_roots=1511,
            groundwater_unchanged=True,
            static_stations_only=True,
            no_native_render=True,
        ),
        indent=2,
    )
    + "\n"
)
