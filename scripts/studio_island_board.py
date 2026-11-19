"""Actual artwork, using the ordinary V1 island. No gameplay or acceptance suites."""

import json
from pathlib import Path
import subprocess
import sys
from build_island_strandline import SURFACE, ASSET
from compare_island_art import make

root = Path(__file__).resolve().parents[1]
board = json.loads(Path(sys.argv[1]).read_text())
out = Path(sys.argv[2]).resolve()
out.mkdir(exist_ok=True, parents=True)
d = ASSET.parent


def blender(script, *args):
    subprocess.run(
        [
            "blender",
            "-b",
            "-t",
            "2",
            "--python-exit-code",
            "1",
            "-P",
            str(root / "scripts" / script),
            "--",
            *(str(x) for x in args),
        ],
        check=True,
    )


# The GLB exporter is the production operation. Do not call its separate
# test/verification CLI in a studio workweek.
subject = board["subject"]
approx = board.get("quick", False)
paired = board.get("paired", False)
fresh = d
old = d
if subject == "canopy":
    from build_island_boughs import build

    fresh = out / "artist-crowns"
    build(d / "Conifer.glb", d / "props.json", fresh)
elif subject == "forest-floor":
    from build_island_copse_floor import export

    floor = export(out / "artist-floor")
    if board.get("sculpted_bark"):
        from build_island_boughs import build

        fresh = out / "artist-crowns"
        build(d / "Conifer.glb", d / "props.json", fresh, sculpted_bark=True)
else:
    raise ValueError(f"Unknown studio subject: {subject}")
if paired and subject == "canopy":
    old = out / "old-source"
    base = "c6a3e18b36a8460307f525a0a69993da5c4a1cc9"
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", base], check=True
    )
    old.mkdir(exist_ok=True)
    for ext in ("glb", "json"):
        name = f"grove-crowns.{ext}"
        with (old / name).open("wb") as output:
            subprocess.run(
                ["git", "show", f"{base}:{(d/name).relative_to(root).as_posix()}"],
                stdout=output,
                check=True,
            )

if board.get("editable_canopy"):
    blender(
        "studio_island_canopy_blend.py",
        fresh / "grove-crowns.glb",
        out / "grove-boughs.blend",
    )
views = {
    "copse-eye": ["--copse-eye"],
    "fern-hollow": ["--copse-eye", "--copse-framing", "fern-hollow"],
    "stone-eye": ["--rock-eye"],
    "pine-eye": ["--pine-eye"],
    "guardian-woods": ["--guardian-review", "--grove-framing", "dense"],
}
for tag in board["angles"]:
    for side in ("before", "after") if paired else ("after",):
        pack = fresh if side == "after" else old
        blender(
            "render_island_geology.py",
            d / "terrain-chunks.glb",
            out / side,
            *views[tag],
            *(
                ["--studio-samples", str(board["samples"])]
                if "samples" in board
                else []
            ),
            *(["--quick-proof"] if approx else []),
            "--coastal-light",
            d / "coastal-daylight.json",
            "--marine-sky",
            d / "marine-day.glb",
            "--strandline",
            d / "strandline.glb",
            "--surface",
            SURFACE,
            "--placements",
            d / "props.json",
            "--water-inputs",
            root / "studies/island-art-week/water-inputs",
            "--habitats",
            d / "habitat-ground.glb",
            "--grove-forms",
            pack / "grove-crowns.glb",
            *(
                ["--copse-floor", floor]
                if subject == "forest-floor" and side == "after"
                else []
            ),
        )
    if paired:
        make(
            out / "before" / f"{tag}.png",
            out / "after" / f"{tag}.png",
            f"{'Maritime spruce' if subject=='canopy' else 'Rooted woodland floor'} | {tag} | Identical tree anchors, V1 terrain and light | CPU Blender",
            out / f"{tag}.jpg",
        )
(out / "studio-note.json").write_text(
    json.dumps(
        {
            "board": board,
            "terrain": "unchanged playable V1",
            "tool": "locked CPU Blender art render; not gameplay",
        },
        indent=2,
    )
)
