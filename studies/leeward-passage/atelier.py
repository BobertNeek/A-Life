"""Produce optional artist media only. Neither side starts the game."""

import json, sys
from pathlib import Path
from reach import Reach
from ground import paint
from flora import approach


def assets(surface, props, out):
    out.mkdir(parents=True, exist_ok=True)
    reach = Reach(surface)
    stats = reach.profile(props)
    if (
        stats["min_conservative_obstacle_clearance_m"] <= 0.2
        or stats["max_triangle_rise_run"] > 0.85
        or stats["height_min_max_m"][0] < 0
    ):
        raise ValueError(
            "The studio map is not this original dry passage; do not mount the artwork."
        )
    stats.update(paint(reach, out))
    tris, counts = approach(reach, out)
    stats.update(
        art_triangles=tris,
        artist_groups=counts,
        range="This one corridor. No native binding, no physical mesh, no tested play.",
    )
    (out / "route.json").write_text(json.dumps(stats, indent=2) + "\n")


def photograph(root, d, surface, board, out, blender, pair):
    geometry = out / "artists-reach"
    assets(surface, d / "props.json", geometry)
    water = out / "shelf"
    water.mkdir(exist_ok=True)
    for n in ("ripple.png", "shore.png"):
        (water / n).symlink_to((d / "cove-water" / n).resolve())
    (water / "water.json").write_bytes(
        (root / "studies/island-art-week/water-selected.json").read_bytes()
    )
    for tag in board["angles"]:
        for side in ("before", "after") if board.get("paired") else ("after",):
            blender(
                "render_island_geology.py",
                d / "terrain-chunks.glb",
                out / side,
                "--guardian-review" if tag == "reach-guardian" else "--copse-eye",
                "--leeward-view",
                tag,
                *(["--leeward-paint", geometry] if side == "after" else []),
                "--surface",
                surface,
                "--placements",
                d / "props.json",
                "--prop-library",
                d,
                "--coastal-light",
                d / "coastal-daylight.json",
                "--marine-sky",
                d / "marine-day.glb",
                "--grove-forms",
                d / "grove-crowns.glb",
                "--water-inputs",
                water,
                "--habitats",
                d / "habitat-ground.glb",
                "--wild-sward",
                d / "meadow-sward.glb",
                "--meadow-library",
                d,
                "--meadow-habits",
                "--inlet-cut",
                d / "inlet-bank.glb",
                "--strandline",
                d / "strandline.glb",
                "--copse-floor",
                d / "copse-floor.glb",
                "--studio-samples",
                board.get("samples", 48)
            )
        if board.get("paired"):
            pair(
                out / "before" / (tag + ".png"),
                out / "after" / (tag + ".png"),
                "Existing V1 on both sides | Shellback Reach | identical camera and daylight | offline Blender",
                out / (tag + ".jpg"),
            )
    (out / "studio-note.json").write_text(json.dumps(board, indent=2) + "\n")
