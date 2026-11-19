"""Reuse the Pennon Drift cameras and materials, substituting only seven crowns."""

import json, runpy, sys
from pathlib import Path

here = Path(__file__).resolve().parent
native, scene, rock, hollow, detail, live, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:7]
)
mode = sys.argv[-1]
# Independent scratch staging only; sources, frozen images and original art are
# never rewritten by this recipe. Only the alternate heath is a new local GLB.
out.mkdir(parents=True, exist_ok=True)
combined = out / "study-registered"
combined.mkdir(exist_ok=True)
for src in rock.iterdir():
    if not src.is_file():
        continue
    p = combined / src.name
    actual = live / "wind-heath.glb" if src.name == "wind-heath.glb" else src
    if not p.exists():
        p.symlink_to(actual.resolve())
args = sys.argv
dest = out / ("editable" if mode == "desk" else "pictures")
dest.mkdir(exist_ok=True)
sys.argv = [
    "blender",
    "--",
    *[str(q) for q in (native, scene, combined, hollow, detail, dest)],
    "desk" if mode == "desk" else "setup",
]
try:
    stage = runpy.run_path(
        str(here.parent / "underfoot/capture.py"), run_name="pennon_drift_stage"
    )
finally:
    sys.argv = args
if mode == "desk":
    old = dest / "Pennon-Drift.blend"
    new = out / "Folded-Windcups.blend"
    if new.exists():
        raise ValueError(
            "Choose a new output rather than overwrite the archived editable."
        )
    old.rename(new)
else:
    for station in ("stone-eye", "pennon-eye"):
        stage["choose"](station)
        stage["shot"](station, "after")
    (dest / "crown-receipt.json").write_text(
        json.dumps(
            dict(
                source="Seven original pennon roots, new near forms only. Artist photographs, not native gameplay.",
                roots=json.loads((live / "seven-roots.json").read_text()),
                visible_loose_detail=stage["visible"],
                held_stations=["stone-eye", "pennon-eye"],
                unchanged_parent_station="upper-rim",
            ),
            indent=2,
        )
        + "\n"
    )
