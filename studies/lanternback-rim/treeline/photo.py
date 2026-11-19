"""The actual 39 m sightline. The seven windcups and their chaff are held on both sides."""

import json, sys, runpy
from pathlib import Path

here = Path(__file__).resolve().parent
native, scene, rock, hollow, detail, lee, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:7]
)
out.mkdir(exist_ok=True, parents=True)
args = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(p) for p in (native, scene, rock, hollow, detail, out)],
    "setup",
]
try:
    s = runpy.run_path(str(here.parent / "underfoot/capture.py"), run_name="frozen_rim")
finally:
    sys.argv = args
p = s["parent"]
cameras = s["cameras"]
pose = ["spruce-lee", [275.0, 73.0, -149.0], [242.5, 50.0, -170.9], 58.0, 40, 1]
cameras[pose[0]] = p["cam"](pose[0], pose)


def snap(name, side):
    s["choose"](name)
    s["shot"](name, side)


if args[-1] != "final":
    snap(pose[0], "before")
ground = p["ground"]
binder = p["load_at"](here.parents[1] / "leeward-passage/material.py", "soil")
binder._bind_layer(ground, lee, (), "lee.json", "lee-soil.png", 30.0, 0.0025, 0.20)
p["grab"](lee / "lee-middle.glb")
snap("stone-eye", "after")
snap(pose[0], "after")
(out / "station-receipt.json").write_text(
    json.dumps(
        dict(
            method="Blender Cycles; original V1 ground, trees and existing seven windcups held; 24 samples, 840x500, same sky/lamps",
            guardian=pose,
            eye=(
                s["site"]["stations"]["stone-eye"]
                if "site" in s
                else p["site"]["stations"]["stone-eye"]
            ),
            first_eye_control="Final coastal eye from the 28859c3f Windcups archive",
            near_or_middle="Middle forms at both camera distances; near palette lives in the editable",
            native=False,
        ),
        indent=2,
    )
    + "\n"
)
