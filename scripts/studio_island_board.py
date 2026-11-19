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
if subject == "sunspindle-hollow":
    sys.path.insert(0, str(root / "studies/sunspindle-hollow"))
    from atelier import photograph
    photograph(root, d, SURFACE, board, out, blender)
    raise SystemExit(0)
if subject == "weatherlip-seats":
    sys.path.insert(0, str(root / "studies/weatherlip-cove/seat"))
    from atelier_seat import photograph
    photograph(root, d, SURFACE, board, out, blender)
    raise SystemExit(0)
if subject == "weatherlip-cove":
    sys.path.insert(0, str(root / "studies/weatherlip-cove"))
    from atelier import photograph
    photograph(root, d, SURFACE, board, out, blender)
    raise SystemExit(0)
if subject == "leeward-passage":
    # An independent artwork; neither a game asset install nor a test lane.
    sys.path.insert(0, str(root / "studies/leeward-passage"))
    from atelier import photograph

    photograph(root, d, SURFACE, board, out, blender, make)
    raise SystemExit(0)
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
elif subject == "erratics":
    from studio_erratic_rock import carve

    # Native A/B/C each contain three separate stones. The old pencil cliff
    # and stock scree stay the same on both sides of these two photographs.
    frozen = "01d86f19569e67d41fcd2c6c8b5b689143e05175"
    erratic_old, erratic_new = out / "original-erratics", out / "artist-erratics"
    for folder in (erratic_old, erratic_new):
        folder.mkdir(exist_ok=True)
        (folder / "minerals").symlink_to(
            (d / "minerals").resolve(), target_is_directory=True
        )
        for kind in ("Cliff", "Scree"):
            if kind == "Scree" and board.get("talus_study"):
                continue
            (folder / f"{kind}.glb").symlink_to((d / f"{kind}.glb").resolve())
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", frozen], check=True
    )
    if board.get("talus_study"):
        # After eventual selection the active Scree GLB is no longer the
        # original. Both future rebakes and camera controls require the
        # versioned donor to avoid quietly sculpting a sculpture again.
        with (erratic_old / "Scree.glb").open("wb") as target:
            subprocess.run(
                [
                    "git",
                    "show",
                    f"{frozen}:{(d/'Scree.glb').relative_to(root).as_posix()}",
                ],
                stdout=target,
                check=True,
            )
    if board.get("mineral_study"):
        if board.get("weather_rind"):
            from island_erratic_rind import paint as minerals
        else:
            from island_erratic_minerals import granite as minerals
        minerals(erratic_new / "erratic")
    catalogue = {}
    for kind in ("BoulderA", "BoulderB", "BoulderC"):
        piece = kind + ".glb"
        with (erratic_old / piece).open("wb") as target:
            subprocess.run(
                ["git", "show", f"{frozen}:{(d/piece).relative_to(root).as_posix()}"],
                stdout=target,
                check=True,
            )
        catalogue[kind] = carve(
            erratic_old / piece,
            erratic_new / piece,
            crystalline=board.get("mineral_study", False),
            crown_rind=board.get("weather_rind", False),
            rolled_toe=board.get("rolled_toe", False),
        )
    if board.get("talus_study"):
        catalogue["Scree"] = carve(erratic_old / "Scree.glb", erratic_new / "Scree.glb")
        pre_talus = out / "before-cleaved-talus"
        pre_talus.mkdir(exist_ok=True)
        for name in (
            "minerals",
            "erratic",
            "BoulderA.glb",
            "BoulderB.glb",
            "BoulderC.glb",
            "Cliff.glb",
            "Scree.glb",
        ):
            donor = (erratic_old if name == "Scree.glb" else erratic_new) / name
            if donor.exists():
                (pre_talus / name).symlink_to(
                    donor.resolve(), target_is_directory=donor.is_dir()
                )
    if board.get("talus_contact"):
        from island_talus_contact import seated

        if not board.get("talus_study"):
            raise ValueError("Seat the same carved shingle on both sides.")
        # The before artist library keeps its own unchanged carved Scree.glb;
        # only after supplies one extra site-specific variant.
        old_scree = pre_talus / "Scree.glb"
        old_scree.unlink()
        old_scree.symlink_to((erratic_new / "Scree.glb").resolve())
        index = 1303
        catalogue["seated_study"] = seated(
            erratic_new / "Scree.glb",
            erratic_new / f"ScreeSeat{index}.glb",
            SURFACE,
            json.loads((d / "props.json").read_text())[index],
            index,
        )
    if board.get("woodland_edge"):
        from island_woodland_edge import build as woodland_study

        if not board.get("talus_contact"):
            raise ValueError("Both forest comparisons require the same seated stones")
        # The control also exposes the finished near scree. No stone movement
        # or model change at all between the woodland photographs.
        (pre_talus / "ScreeSeat1303.glb").symlink_to(
            (erratic_new / "ScreeSeat1303.glb").resolve()
        )
        edge_folder = out / "artist-woodland-edge"
        woodland_study(
            SURFACE,
            d / "props.json",
            edge_folder,
            succession=board.get("edge_succession", False),
        )
        # An immutable ordinary-V1 control already exists as completed
        # artwork, including the exact four-channel coverage, not a bare field.
        if board.get("edge_succession"):
            published = root / "studies/island-glacial-deposit/woodland-edge"
            prev_edge = out / "previous-woodland"
            prev_edge.mkdir(exist_ok=True)
            for name in ("woodland-edge.glb", "woodland-bed.png"):
                (prev_edge / name).symlink_to((published / name).resolve())
            (prev_edge / "woodland-source.json").write_text(
                json.dumps(
                    json.loads((published / "artist-source.json").read_text())["fields"]
                )
                + "\n"
            )
    if board.get("boulder_mount"):
        from island_boulder_bedjoint import mount

        if not board.get("woodland_edge"):
            raise ValueError(
                "Retain the same completed woodland and seated loose shingle."
            )
        index = 1383
        catalogue["socketed_rock"] = mount(
            erratic_new / "BoulderA.glb",
            SURFACE,
            json.loads((d / "props.json").read_text())[index],
            index,
            erratic_new / f"BoulderASeat{index}.glb",
        )
    if board.get("edge_succession") and not board.get("boulder_mount"):
        raise ValueError(
            "Both species portraits must retain the same selected bedjoint."
        )
    (erratic_new / "catalogue.json").write_text(json.dumps(catalogue, indent=2) + "\n")
    if board.get("glacial_train"):
        from island_erratic_train import bake

        moraine = erratic_new / "deposit-till.glb"
        if board.get("silverleaf"):
            previous_till = erratic_new / "deposit-before-runners.glb"
            bake(SURFACE, erratic_old, d / "props.json", previous_till)
        counts = bake(
            SURFACE,
            erratic_old,
            d / "props.json",
            moraine,
            silverleaf=board.get("silverleaf", False),
        )
        (erratic_new / "till-source.json").write_text(
            json.dumps(counts, indent=2) + "\n"
        )
    if board.get("hero_soil"):
        from island_erratic_soil import build as resolve_soil

        soil = out / "artist-moraine-soil"
        description = resolve_soil(d, soil)
        (soil / "source.json").write_text(json.dumps(description, indent=2) + "\n")
        (soil / "geology").symlink_to(
            (d / "geology").resolve(), target_is_directory=True
        )
    if board.get("shared_microfield"):
        from island_ground_microfield import build as microfield

        micro = out / "artist-microfield"
        microfield(SURFACE, d / "props.json", micro)
elif subject == "fault-scarp":
    from island_fault_scarp import sculpt

    # Keep the old pencil-shaped true native mesh even if a later artist
    # stores the new production candidate; matched pictures must not drift.
    original = "01d86f19569e67d41fcd2c6c8b5b689143e05175"
    scarp_base = out / "original-scarp"
    scarp = out / "artist-scarp"
    scarp_base.mkdir(exist_ok=True)
    scarp.mkdir(exist_ok=True)
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", original], check=True
    )
    with (scarp_base / "Cliff.glb").open("wb") as target:
        subprocess.run(
            [
                "git",
                "show",
                f"{original}:{(d/'Cliff.glb').relative_to(root).as_posix()}",
            ],
            stdout=target,
            check=True,
        )
    for folder in (scarp, scarp_base):
        (folder / "minerals").symlink_to(
            (d / "minerals").resolve(), target_is_directory=True
        )
    note = sculpt(scarp_base / "Cliff.glb", scarp / "Cliff.glb")
    if board.get("uphill_seating"):
        props = json.loads((d / "props.json").read_text())
        seats = {}
        for i in board["uphill_seating"]:
            key = f"CliffSeat{i}"
            if props[i]["kind"] != "Cliff":
                raise ValueError("Not a native cliff index")
            seats[str(i)] = sculpt(
                scarp_base / "Cliff.glb",
                scarp / (key + ".glb"),
                surface=SURFACE,
                landmark=props[i],
                key=key,
            )
        (scarp / "seated.json").write_text(json.dumps(seats, indent=2) + "\n")
        if board.get("wall_beds"):
            from island_scarp_beds import expose

            substrate = out / "artist-scarp-ground"
            substrate.mkdir(exist_ok=True)
            (substrate / "habitats").symlink_to(
                (d / "habitats").resolve(), target_is_directory=True
            )
            # Both views must share the selected macro map and its signed
            # far-relief tint. The renderer looks next to the habitat GLB.
            macro = substrate / "geology"
            macro.mkdir(exist_ok=True)
            for name in ("pigment.png", "relief.png"):
                original = (d / "geology" / name).resolve()
                if original.is_file():
                    (macro / name).symlink_to(original)
            wall = substrate / "habitat-ground.glb"
            bed_note = expose(
                d / "habitat-ground.glb",
                SURFACE,
                scarp_base / "Cliff.glb",
                d / "props.json",
                wall,
                board["uphill_seating"],
            )
            (substrate / "bed-source.json").write_text(
                json.dumps(bed_note, indent=2) + "\n"
            )
    (scarp / "scarp-source.json").write_text(json.dumps(note, indent=2) + "\n")
    blender(
        "studio_fault_scarp_editable.py",
        scarp / "Cliff.glb",
        scarp / "fault-scarp.blend",
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
elif subject == "inlet-underpaint":
    from island_inlet_underpaint import smooth_bank

    original_bank = out / "stock-inlet"
    selected = "0d2bfa94e2292fdce5d0b4b8abb04098d7e54385"
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", selected], check=True
    )
    for piece in (
        "inlet-bank.glb",
        "inlet/inlet-bed.png",
        "inlet/inlet-normal.png",
        "inlet/inlet-orm.png",
    ):
        path = original_bank / piece
        path.parent.mkdir(exist_ok=True, parents=True)
        with path.open("wb") as to:
            subprocess.run(
                ["git", "show", f"{selected}:{(d/piece).relative_to(root).as_posix()}"],
                stdout=to,
                check=True,
            )
    skin = smooth_bank(
        d / "habitat-ground.glb",
        SURFACE,
        original_bank,
        out / "artist-inlet-underpaint",
        opaque_finish=board.get("opaque_bank", False),
    )
    # Keep the old masked film as donor to bake the selected opaque coat.
    # A subsequent tidecast board must isolate *only* the actual new clasts.
    evidence_before = (
        d if board.get("tidecast_pockets") else original_bank
    ) / "inlet-bank.glb"
    if board.get("tidecast_pockets"):
        from build_island_inlet import V2
        from island_inlet_tidecast import embed

        pockets = embed(skin, SURFACE, V2)
        receipt = skin.parent / "bank-source.json"
        receipt.write_text(
            json.dumps({**json.loads(receipt.read_text()), **pockets}, indent=2) + "\n"
        )
    # Regenerate the far paint once over the last *unpainted* pre-bank
    # macro. A new V1 coat must not accumulate on the earlier one.
    from build_island_inlet import V2
    from island_inlet_distance import far_pigment

    previous = "ae225e4a572c50c99140146322b33eb30c7b0b65"
    frozen_pigment = out / "pre-bank.png"
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
    stats = far_pigment(
        SURFACE,
        V2,
        skin.parent
        / (
            "far-pigment-cut.png" if board.get("opaque_bank") else "inlet/inlet-bed.png"
        ),
        frozen_pigment,
        revised_pigment,
    )
    note = skin.parent / "bank-source.json"
    note.write_text(
        json.dumps({**json.loads(note.read_text()), **stats}, indent=2) + "\n"
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
    originals = ["Grass.glb", "Flowers.glb"]
    if board.get("young_saltwood"):
        originals.append("Sapling.glb")
    for name in originals:
        with (original_meadow / name).open("wb") as target:
            subprocess.run(
                ["git", "show", f"{original}:{(d/name).relative_to(root).as_posix()}"],
                stdout=target,
                check=True,
            )
    blender(
        "studio_meadow_flowers.py",
        meadow,
        "--author-from",
        original_meadow,
        *(["--wax-petals"] if board.get("wax_petals") else []),
    )
    if board.get("young_saltwood"):
        young = out / "artist-young-saltwood"
        blender(
            "studio_young_saltwood.py",
            young,
            "--author-from",
            original_meadow / "Sapling.glb",
        )
    if board.get("meadow_sward"):
        from build_island_meadow import export as meadow_floor

        native_field = meadow_floor(
            SURFACE,
            d / "habitat-ground.glb",
            d / "props.json",
            out / "artist-meadow-floor",
            inked=board.get("inked_swards", False),
            felted=board.get("felted_swards", False),
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
elif subject in ("inlet-survey", "meadow-survey", "estuary-optics"):
    # Photograph real terrain. The localized estuary variant is painted below
    # after the original shelf maps have been recovered.
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
    "sapling-eye": ["--copse-eye", "--copse-framing", "sapling-eye"],
    "scarp-eye": ["--copse-eye", "--copse-framing", "scarp-eye"],
    "scarp-spur": ["--guardian-review", "--grove-framing", "scarp-spur"],
    "rift-root": ["--guardian-review", "--grove-framing", "rift-root"],
    "meadow-bells": ["--copse-eye", "--copse-framing", "meadow-bells"],
    "meadow-eye": ["--copse-eye", "--copse-framing", "meadow-eye"],
    "meadow-reach": ["--guardian-review", "--grove-framing", "meadow-reach"],
    "alder-eye": ["--copse-eye", "--copse-framing", "alder-eye"],
    "alder-bank": ["--guardian-review", "--grove-framing", "alder-bank"],
    "inlet-eye": ["--cove-showcase", "--cove-framing", "inlet-eye"],
    "inlet-bar": ["--cove-showcase", "--cove-framing", "inlet-bar"],
    "estuary-mouth": ["--cove-showcase", "--cove-framing", "estuary-mouth"],
    "inlet-landing": ["--cove-showcase", "--cove-framing", "inlet-landing"],
    "inlet-bank": ["--guardian-review", "--grove-framing", "inlet-bank"],
    "shelf-eye": ["--cove-showcase", "--cove-framing", "shelf-edge"],
    "shelf-guardian": ["--guardian-review", "--grove-framing", "shelf"],
    "copse-eye": ["--copse-eye"],
    "fern-hollow": ["--copse-eye", "--copse-framing", "fern-hollow"],
    "sunlit-margin": ["--copse-eye", "--copse-framing", "sunlit-margin"],
    "stone-eye": ["--rock-eye"],
    "erratic-foot": ["--copse-eye", "--copse-framing", "erratic-foot"],
    "moraine-soil": ["--copse-eye", "--copse-framing", "moraine-soil"],
    "talus-eye": ["--copse-eye", "--copse-framing", "talus-eye"],
    "woodland-gap": ["--copse-eye", "--copse-framing", "woodland-gap"],
    "boulder-heel": ["--copse-eye", "--copse-framing", "boulder-heel"],
    "woodland-edge": ["--guardian-review", "--grove-framing", "woodland-edge"],
    "erratic-reach": ["--guardian-review", "--grove-framing", "erratic-reach"],
    "pine-eye": ["--pine-eye"],
    "guardian-woods": ["--guardian-review", "--grove-framing", "dense"],
}
subject_label = {
    "canopy": "Maritime spruce",
    "forest-floor": "Rooted woodland floor",
    "cove-water": "Sheltered tidal shelf",
    "forest-fern": "Feather moss and divided fern",
    "coastal-alder": "Branch-grown maritime alders",
    "erratics": "Glacial erratics on the real wooded slope",
    "fault-scarp": "Planted fractured scarp: original posts versus joined stone",
    "meadow-survey": "Actual starting-valley meadow at grass 3406",
    "wild-meadow": "Current meadow and coastal sedge / bellflower study",
    "inlet-survey": "The actual southern inlet before new bank art",
    "inlet-ground": "The native southern tidal cut",
    "estuary-optics": "Shallow jade creek and marine-blue mouth",
    "inlet-underpaint": "Original bank pigment and native-material feathering",
    "inlet-concept": "Shape study: actual V1 and proposed asymmetric inlet",
}[subject]
if subject == "erratics" and board.get("glacial_train"):
    subject_label = (
        "Same glacial rock: exposed till lenses and woodrush on the true slope"
    )
if subject == "erratics" and board.get("hero_soil"):
    subject_label = (
        "Sheltered till on the original soil, at twice the close pigment density"
    )
if subject == "erratics" and board.get("shared_microfield"):
    subject_label = (
        "Original habitat pigment, additional centimetre-scale island relief"
    )
if subject == "erratics" and board.get("silverleaf"):
    subject_label = (
        "Real moraine: the woodrush patch and its sheltered silverleaf runners"
    )
if subject == "erratics" and board.get("talus_study"):
    subject_label = "Woodland talus: the original 22 stones, flaked against the bedding"
if subject == "erratics" and board.get("talus_contact"):
    subject_label = (
        "Actual woodland shingle: the same 22 stones, seated on the individual slopes"
    )
if subject == "erratics" and board.get("woodland_edge"):
    subject_label = (
        "Needle bed, wind-combed rush and the open moraine between two real spruce"
    )
if subject == "erratics" and board.get("boulder_mount"):
    subject_label = "One solid glacial boulder, fitted inside its genuine rising bed"
if subject == "erratics" and board.get("edge_succession"):
    subject_label = (
        "Spruce shade, fern hollow and open shingle | A local woodland succession"
    )
if subject == "fault-scarp" and board.get("wall_beds"):
    subject_label = "Same seated scarp, with its exposed bed on the true hillside"
if subject == "inlet-underpaint" and board.get("tidecast_pockets"):
    subject_label = "Rooted V1 inlet and low tidecast"
base_water = root / "studies/island-art-week/water-inputs"
if subject in (
    "coastal-alder",
    "fault-scarp",
    "erratics",
    "inlet-survey",
    "estuary-optics",
    "inlet-ground",
    "inlet-underpaint",
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
if subject == "estuary-optics":
    from island_estuary_optics import author as estuary_chart

    # The selected strip itself may later install, so archive the original
    # shelf input by fixed SHA. This keeps the two photographed states distinct.
    original_water = out / "pre-estuary"
    original_water.mkdir(exist_ok=True)
    original = "bf017e222a6cdced3c9f91243c9fad76af96d8ad"
    subprocess.run(
        ["git", "fetch", "--no-tags", "--depth=1", "origin", original], check=True
    )
    for name in ("shore.png", "ripple.png"):
        with (original_water / name).open("wb") as target:
            subprocess.run(
                [
                    "git",
                    "show",
                    f"{original}:{(d/'cove-water'/name).relative_to(root).as_posix()}",
                ],
                stdout=target,
                check=True,
            )
    (original_water / "water.json").write_bytes(
        (base_water / "water.json").read_bytes()
    )
    base_water = original_water
    estuary = estuary_chart(SURFACE, base_water, out / "artist-estuary")
if subject == "inlet-concept":
    concept_water = out / "studied-water"
    paint_concept_water(SURFACE, base_water, concept_water)
for tag in board["angles"]:
    rock_portrait = tag in ("boulder-heel", "talus-eye") and board.get("boulder_mount")
    vegetation_portrait = board.get("edge_succession") and not rock_portrait
    sides = ("before", "after") if paired else ("after",)
    if (
        subject == "inlet-underpaint"
        and tag == "inlet-bar"
        and not board.get("tidecast_pockets")
    ):
        sides = ("bare", "before", "after")
    for side in sides:
        pack = fresh if side == "after" else old
        water = chart if subject == "cove-water" and side == "after" else base_water
        if subject == "estuary-optics" and side == "after":
            water = estuary
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
                "fault-scarp",
                "erratics",
                "inlet-survey",
                "estuary-optics",
                "inlet-ground",
                "inlet-underpaint",
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
                [
                    "--woodland-edge",
                    (
                        prev_edge
                        if board.get("edge_succession")
                        and (side == "before" or rock_portrait)
                        else edge_folder
                    ),
                ]
                if subject == "erratics"
                and board.get("woodland_edge")
                and (side == "after" or board.get("boulder_mount"))
                else []
            ),
            *(
                ["--ground-microfield", micro]
                if subject == "erratics"
                and board.get("shared_microfield")
                and side == "after"
                else []
            ),
            *(
                [
                    "--wild-sward",
                    (
                        previous_till
                        if board.get("silverleaf") and side == "before"
                        else moraine
                    ),
                ]
                if subject == "erratics"
                and board.get("glacial_train")
                and (
                    side == "after"
                    or board.get("silverleaf")
                    or board.get("hero_soil")
                    or board.get("shared_microfield")
                )
                else []
            ),
            *(
                [
                    "--stone-library",
                    (
                        pre_talus
                        if board.get("talus_study")
                        and side == "before"
                        and not vegetation_portrait
                        else (
                            erratic_new
                            if side == "after"
                            or board.get("glacial_train")
                            or board.get("talus_study")
                            else erratic_old
                        )
                    ),
                ]
                if subject == "erratics"
                else []
            ),
            *(
                [
                    "--cliff-library",
                    scarp if side == "after" or board.get("wall_beds") else scarp_base,
                ]
                if subject == "fault-scarp"
                else []
            ),
            *(
                ["--studio-samples", str(board["samples"])]
                if "samples" in board
                else []
            ),
            *(["--quick-proof"] if approx else []),
            *(
                [
                    "--inlet-cut",
                    skin if side == "after" else evidence_before,
                ]
                if subject == "inlet-underpaint" and side != "bare"
                else []
            ),
            *(
                ["--inlet-cut", d / "inlet-bank.glb"]
                if subject == "estuary-optics"
                else []
            ),
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
                if subject == "inlet-underpaint"
                and side != "before"
                and not board.get("tidecast_pockets")
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
                ["--young-library", young if side == "after" else original_meadow]
                if subject == "wild-meadow" and board.get("young_saltwood")
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
            (
                wall
                if subject == "fault-scarp"
                and board.get("wall_beds")
                and side == "after"
                else (
                    soil / "habitat-ground.glb"
                    if subject == "erratics"
                    and board.get("hero_soil")
                    and (
                        side == "after"
                        or board.get("shared_microfield")
                        or board.get("silverleaf")
                    )
                    else d / "habitat-ground.glb"
                )
            ),
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
    if (
        subject == "inlet-underpaint"
        and tag == "inlet-bar"
        and not board.get("tidecast_pockets")
    ):
        make(
            out / "bare" / f"{tag}.png",
            out / "before" / f"{tag}.png",
            "V1 original terrain only / existing V1 bank film | identical daylight and camera | Blender",
            out / "inlet-bar-source.jpg",
        )
    if paired:
        caption = (
            "Young saltwood at real original 1116"
            if tag == "sapling-eye"
            else (
                "Same solid boulder, original rounded sole versus keyed heel"
                if rock_portrait
                else subject_label
            )
        )
        make(
            out / "before" / f"{tag}.png",
            out / "after" / f"{tag}.png",
            (
                f"Editable inlet proposal | {tag} | V1 versus bank sculpt / same lens, daylight and water level | CPU Blender"
                if subject == "inlet-concept"
                else f"{caption} | {tag} | Same V1 terrain, lens and daylight | CPU Blender"
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
