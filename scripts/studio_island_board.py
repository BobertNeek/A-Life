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
elif subject == "forest-fern":
    from build_island_copse_floor import export

    fern = out / "artist-fern"
    floor = export(out / "artist-floor", creeping=True)
    blender("studio_forest_fern.py", fern, "--author-from", d / "Fern.glb")
elif subject == "cove-water":
    from island_cove_optics import world_paint

    chart = out / "artist-water"
    original_water = root / "studies/island-art-week/water-inputs"
    world_paint(SURFACE, original_water, chart)
else:
    raise ValueError(f"Unknown studio subject: {subject}")
if paired and subject in ("canopy", "forest-floor"):
    # Lock the left half across future branch installs. The steel-blue
    # source bough and the subsequent feathered spruce do not use the
    # same material files, so fetch only the inputs of that precise recipe.
    old = out / "old-source"
    base = (
        "c6a3e18b36a8460307f525a0a69993da5c4a1cc9"
        if subject == "canopy"
        else "5d6181e662ddd9e6cf8daf16f4c9d39a5ef1fe9c"
    )
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", base], check=True
    )
    names = ["grove-crowns.glb", "grove-crowns.json"]
    if subject == "forest-floor":
        names.extend(
            "canopy/" + name
            for name in ("needles.png", "needles-normal.png", "needles-orm.png")
        )
    for name in names:
        native = d / name
        dest = old / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open("wb") as output:
            subprocess.run(
                ["git", "show", f"{base}:{native.relative_to(root).as_posix()}"],
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
    "shelf-eye": ["--cove-showcase", "--cove-framing", "shelf-edge"],
    "shelf-guardian": ["--guardian-review", "--grove-framing", "shelf"],
    "copse-eye": ["--copse-eye"],
    "fern-hollow": ["--copse-eye", "--copse-framing", "fern-hollow"],
    "sunlit-margin": ["--copse-eye", "--copse-framing", "sunlit-margin"],
    "stone-eye": ["--rock-eye"],
    "pine-eye": ["--pine-eye"],
    "guardian-woods": ["--guardian-review", "--grove-framing", "dense"],
}
subject_label = {
    "canopy": "Maritime spruce",
    "forest-floor": "Rooted woodland floor",
    "cove-water": "Sheltered tidal shelf",
    "forest-fern": "Feather moss and divided fern",
}[subject]
base_water = root / "studies/island-art-week/water-inputs"
for tag in board["angles"]:
    for side in ("before", "after") if paired else ("after",):
        pack = fresh if side == "after" else old
        water = chart if subject == "cove-water" and side == "after" else base_water
        dressing = None
        if subject in ("forest-floor", "forest-fern") and side == "after":
            dressing = floor
        elif (
            subject in ("cove-water", "forest-fern")
            and (d / "copse-floor.glb").is_file()
        ):
            dressing = d / "copse-floor.glb"
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
            water,
            "--habitats",
            d / "habitat-ground.glb",
            "--grove-forms",
            pack / "grove-crowns.glb",
            *(["--copse-floor", dressing] if dressing is not None else []),
            *(
                ["--fern-library", fern]
                if subject == "forest-fern" and side == "after"
                else []
            ),
        )
    if paired:
        make(
            out / "before" / f"{tag}.png",
            out / "after" / f"{tag}.png",
            f"{subject_label} | {tag} | Same V1 terrain, lens and daylight | CPU Blender",
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
