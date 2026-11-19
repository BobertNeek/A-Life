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
    fern_base = d
    if paired:
        # The native installation replaces both old inputs. Pin them so later
        # artists never get two pictures of the new plant mislabeled as A/B.
        fern_base = out / "pre-lace"
        base = "fc5d22dcdf61c789d9f25c9b833be588d366c14d"
        subprocess.run(
            ["git", "fetch", "--no-tags", "--depth=1", "origin", base], check=True
        )
        for name in [
            "Fern.glb",
            "copse-floor.glb",
            *(
                "copse/" + n
                for n in ("duff-bark.png", "duff-bark-normal.png", "duff-bark-orm.png")
            ),
        ]:
            dest = fern_base / name
            dest.parent.mkdir(parents=True, exist_ok=True)
            with dest.open("wb") as output:
                subprocess.run(
                    ["git", "show", f"{base}:{(d/name).relative_to(root).as_posix()}"],
                    stdout=output,
                    check=True,
                )

    from island_fern_ink import paint

    floor = export(out / "artist-floor", creeping=True)
    fern.mkdir(exist_ok=True)
    paint(fern / "lady-fronds.png")
    blender("studio_forest_fern.py", fern, "--author-from", fern_base / "Fern.glb")
    if board.get("botanical_portrait"):
        blender(
            "studio_fern_portrait.py",
            fern_base / "Fern.glb",
            fern / "Fern.glb",
            out / "fern-atelier.png",
        )
elif subject == "coastal-alder":
    from island_alder_ink import paint

    alder = out / "artist-alder"
    alder.mkdir(exist_ok=True)
    old_tree = out / "stock-alder"
    old_tree.mkdir(exist_ok=True)
    base = "9450218b29944a3409079d6ca948ddbf238dd6cc"
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", base], check=True
    )
    with (old_tree / "Broadleaf.glb").open("wb") as output:
        subprocess.run(
            [
                "git",
                "show",
                f"{base}:{(d/'Broadleaf.glb').relative_to(root).as_posix()}",
            ],
            stdout=output,
            check=True,
        )
    paint(alder / "alder-ink.png")
    blender(
        "studio_coastal_alder.py", alder, "--author-from", old_tree / "Broadleaf.glb"
    )
elif subject == "inlet-ground":
    from build_island_inlet import export

    skin = export(out / "artist-inlet", rooted=board.get("rooted_cutbank", False))
    if board.get("distance_paint"):
        from build_island_inlet import V2
        from island_inlet_distance import far_pigment

        # Left must remain the last approved picture after the maps are installed.
        previous = "ae225e4a572c50c99140146322b33eb30c7b0b65"
        frozen_pigment = out / "pre-inlet-pigment.png"
        subprocess.run(
            ["git", "fetch", "--no-tags", "--depth=1", "origin", previous], check=True
        )
        with frozen_pigment.open("wb") as into:
            subprocess.run(
                [
                    "git",
                    "show",
                    f"{previous}:{(d/'geology/pigment.png').relative_to(root).as_posix()}",
                ],
                stdout=into,
                check=True,
            )
        revised_pigment = skin.parent / "geology/pigment.png"
        statistics = far_pigment(
            SURFACE,
            V2,
            skin.parent / "inlet/inlet-bed.png",
            frozen_pigment,
            revised_pigment,
        )
        note = skin.parent / "inlet-painting.json"
        note.write_text(
            json.dumps({**json.loads(note.read_text()), **statistics}, indent=2) + "\n"
        )

elif subject == "wild-meadow":
    # Freeze the real pre-meadow source. Later artists must not photograph
    # the new native plants on the nominally "old" side of their comparison.
    meadow = out / "artist-meadow"
    original_meadow = out / "old-meadow"
    original_meadow.mkdir(exist_ok=True)
    original = "507a2cbdb8645d12249cf99269725cced1fa9381"
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", original], check=True
    )
    for name in ("Grass.glb", "Flowers.glb"):
        with (original_meadow / name).open("wb") as target:
            subprocess.run(
                ["git", "show", f"{original}:{(d/name).relative_to(root).as_posix()}"],
                stdout=target,
                check=True,
            )
    blender("studio_meadow_flowers.py", meadow, "--author-from", original_meadow)
    if board.get("meadow_sward"):
        from build_island_meadow import export as meadow_floor

        native_field = meadow_floor(
            SURFACE,
            d / "habitat-ground.glb",
            d / "props.json",
            out / "artist-meadow-floor",
            inked=board.get("inked_swards", False),
        )
        # Temporarily place the generated GLB by the original shared habitat
        # maps so Blender can follow its portable URIs. The art archive only
        # contains the new GLB, not a second copy of the original textures.
        filed = d / ".weekly-meadow-sward.glb"
        filed.write_bytes(native_field.read_bytes())
        if board.get("inked_swards"):
            ink = d / "meadow/sedge-ink.png"
            ink.parent.mkdir(exist_ok=True)
            ink.write_bytes((native_field.parent / "meadow/sedge-ink.png").read_bytes())
elif subject == "inlet-concept":
    # This subject has no native export, no generated world and no save version.
    study = root / "studies/island-inlet-proposal"
    sys.path.insert(0, str(study))
    from water import water as paint_concept_water

    from deposit import paint as inner_sand

    skin = d / "inlet-bank.glb"
    bar_ink = out / "study-ink/inner-bar.png"
    inner_sand(SURFACE, bar_ink)
    blender(
        study / "editable.py",
        SURFACE,
        skin,
        bar_ink,
        out / "southern-inlet-proposal.blend",
    )
elif subject in ("inlet-survey", "meadow-survey"):
    # Two working photographs of real terrain; no candidate is invented here.
    pass
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
    "meadow-eye": ["--copse-eye", "--copse-framing", "meadow-eye"],
    "meadow-reach": ["--guardian-review", "--grove-framing", "meadow-reach"],
    "alder-eye": ["--copse-eye", "--copse-framing", "alder-eye"],
    "alder-bank": ["--guardian-review", "--grove-framing", "alder-bank"],
    "inlet-eye": ["--cove-showcase", "--cove-framing", "inlet-eye"],
    "inlet-bar": ["--cove-showcase", "--cove-framing", "inlet-bar"],
    "inlet-bank": ["--guardian-review", "--grove-framing", "inlet-bank"],
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
    "coastal-alder": "Branch-grown maritime alders",
    "meadow-survey": "Actual starting-valley meadow at grass 3406",
    "wild-meadow": "Current meadow and coastal sedge / bellflower study",
    "inlet-survey": "The actual southern inlet before new bank art",
    "inlet-ground": "The native southern tidal cut",
    "inlet-concept": "Shape study: actual V1 and proposed asymmetric inlet",
}[subject]
base_water = root / "studies/island-art-week/water-inputs"
if subject in (
    "coastal-alder",
    "inlet-survey",
    "inlet-ground",
    "inlet-concept",
    "meadow-survey",
    "wild-meadow",
):
    base_water = out / "current-shelf"
    base_water.mkdir(exist_ok=True)
    for n in ("shore.png", "ripple.png"):
        (base_water / n).symlink_to((d / "cove-water" / n).resolve())
    (base_water / "water.json").write_text(
        (root / "studies/island-art-week/water-selected.json").read_text()
    )
if subject == "inlet-concept":
    concept_water = out / "studied-water"
    paint_concept_water(SURFACE, base_water, concept_water)
for tag in board["angles"]:
    for side in ("before", "after") if paired else ("after",):
        pack = fresh if side == "after" else old
        water = chart if subject == "cove-water" and side == "after" else base_water
        if subject == "inlet-concept" and side == "after":
            water = concept_water
        dressing = None
        if subject == "forest-fern" and side == "before":
            dressing = fern_base / "copse-floor.glb"
        elif subject in ("forest-floor", "forest-fern") and side == "after":
            dressing = floor
        elif (
            subject
            in (
                "cove-water",
                "forest-fern",
                "coastal-alder",
                "inlet-survey",
                "inlet-ground",
                "inlet-concept",
                "meadow-survey",
                "wild-meadow",
            )
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
            *(
                ["--inlet-cut", skin]
                if subject == "inlet-concept"
                or (subject == "inlet-ground" and side == "after")
                else []
            ),
            *(
                [
                    "--original-macro",
                    revised_pigment if side == "after" else frozen_pigment,
                ]
                if subject == "inlet-ground" and board.get("distance_paint")
                else []
            ),
            *(
                ["--inlet-sculpt-study", bar_ink]
                if subject == "inlet-concept" and side == "after"
                else []
            ),
            *(
                ["--meadow-library", meadow if side == "after" else original_meadow]
                if subject == "wild-meadow"
                else []
            ),
            *(
                ["--meadow-habits"]
                if subject == "wild-meadow" and side == "after"
                else []
            ),
            *(
                ["--wild-sward", filed]
                if subject == "wild-meadow"
                and board.get("meadow_sward")
                and side == "after"
                else []
            ),
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
            *(
                ["--broadleaf-library", alder if side == "after" else old_tree]
                if subject == "coastal-alder"
                else []
            ),
            "--grove-forms",
            pack / "grove-crowns.glb",
            *(["--copse-floor", dressing] if dressing is not None else []),
            *(
                ["--fern-library", fern if side == "after" else fern_base]
                if subject == "forest-fern"
                else []
            ),
        )
    if paired:
        make(
            out / "before" / f"{tag}.png",
            out / "after" / f"{tag}.png",
            (
                f"Editable inlet proposal | {tag} | V1 versus bank sculpt / same lens, daylight and water level | CPU Blender"
                if subject == "inlet-concept"
                else f"{subject_label} | {tag} | Same V1 terrain, lens and daylight | CPU Blender"
            ),
            out / f"{tag}.jpg",
        )
(out / "studio-note.json").write_text(
    json.dumps(
        {
            "board": board,
            "terrain": (
                "Original V1 at left; Blender-only inlet sculpt at right. No native binding."
                if subject == "inlet-concept"
                else "unchanged playable V1"
            ),
            "tool": "locked CPU Blender art render; not gameplay",
        },
        indent=2,
    )
)
