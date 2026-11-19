"""The genuine shore eye bent toward an actual low cluster; immutable no-fronds controls."""

from pathlib import Path
import bpy, runpy, sys, json

H = Path(__file__).resolve().parent
args = sys.argv[sys.argv.index("--") + 1 :]
native, scene, shelf, hollow, base, cups, upper, toe, wash, lace, out = map(
    Path, args[:11]
)
old = sys.argv
sys.argv = [
    "blender",
    "--",
    *[
        str(x)
        for x in (native, scene, shelf, hollow, base, cups, upper, toe, wash, out)
    ],
    "setup-only",
]
try:
    stack = runpy.run_path(str(H.parent / "toe/compose.py"), run_name="reefside_setup")
finally:
    sys.argv = old
s = stack["s"]
sc = s["sc"]
eye = s["poses"]["wave-eye"][0]
cv = s["cv"]
# Aim across the oldest real, wet pocket. This time the lens points genuinely
# down: the recorded y sits ~0.6m below the 1.04m standing eye and 1.6m ahead.
x, z = 409.069, -98.45
aim = [x, s["floor"](x, z) + 0.055, z]
c = bpy.data.cameras.new("Low tide pocket")
c.type = "PERSP"
c.lens = 31
c.clip_end = 3200
o = bpy.data.objects.new(c.name, c)
sc.collection.objects.link(o)
o.location = cv(eye)
o.rotation_euler = (cv(aim) - o.location).to_track_quat("-Z", "Y").to_euler()


def photo(tag, cam, side):
    (out / side).mkdir(exist_ok=True, parents=True)
    sc.camera = cam
    sc.render.filepath = str(out / side / (tag + ".png"))
    bpy.ops.render.render(write_still=True)


if "after-only" not in args[11:]:
    photo("shelf-garden", o, "before")
runpy.run_path(str(H / "material.py"))["bind"](s["ground"], s["surf"], lace)
for n in ("standing-detail.glb", "standing-context.glb"):
    s["grab"](lace / n)
photo("shelf-garden", o, "after")
photo("wave-eye", s["cams"]["wave-eye"], "after")
(out / "lace-cameras.json").write_text(
    json.dumps(
        dict(
            eye=eye,
            aim=aim,
            camera_lens_mm=31,
            tide_casts_held=True,
            prior_wave_eye="toe/gallery/photos/after/wave-eye.png",
            new_camera_before="before/shelf-garden.png",
            native_gameplay=False,
        ),
        indent=2,
    )
    + "\n"
)
