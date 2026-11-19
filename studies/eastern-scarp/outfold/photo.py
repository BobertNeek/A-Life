"""Two genuine matched views of upper exposures; neither side has the separate tide casts.

Using the frozen 9b9999 upper-coast cameras means only two new artist photographs
are required and the line of the sea stays comparable pixel by pixel.
"""

from pathlib import Path
import sys, runpy, bpy, json

h = Path(__file__).resolve().parent
native, scene, shelf, hollow, base, weather, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:7]
)
old = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(a) for a in (native, scene, shelf, hollow, base, out)],
    "setup",
]
try:
    s = runpy.run_path(str(h.parent / "photo.py"), run_name="gullglass_stage")
finally:
    sys.argv = old
sc = s["sc"]
s["module"]("optional_outfold", h / "material.py").bind(
    s["ground"], s["surf"], base, weather
)
(out / "after").mkdir(exist_ok=True, parents=True)
for name, cam in s["cams"].items():
    sc.camera = cam
    sc.render.filepath = str(out / "after" / (name + ".png"))
    bpy.ops.render.render(write_still=True)
(out / "outfold-angles.json").write_text(
    json.dumps(
        dict(
            cameras=s["poses"],
            prior="9b9999b592978e93d590609ec5daa402fe960494 / gallery/after",
            identical_native_ground=True,
            shell_casts_on_neither_side=True,
            altered_water=False,
        ),
        indent=2,
    )
    + "\n"
)
