"""Produce a matched contemporary island scene, no native adoption or fake water."""

from pathlib import Path
from collections import Counter
import csv, json, math
from field import BOUNDS, Hollow, SHA, community
from specimens import write
from stations import frame, TAGS


def assets(src, props, out):
    out.mkdir(parents=True, exist_ok=True)
    b = Hollow(src)
    b.record(out)
    rows, markers = community(b, props)
    b.paint(out, rows)
    from seedbed import quilt

    quilt(out)
    cams = {tag: frame(tag, lambda x, z: float(b.floor(x, z)[0])) for tag in TAGS}
    counts = Counter(r[0].rsplit("-", 1)[0] for r in rows)
    with (out / "root-register.csv").open("w", newline="") as f:
        c = csv.writer(f)
        c.writerow(["specimen", "x", "v1_y", "z", "scale", "yaw"])
        c.writerows(rows)
    sx, _, sz = cams["slack-eye"][1]
    routed = []
    refined = 0
    for row in rows:
        near = math.hypot(row[1] - sx, row[3] - sz) < 23
        routed.append([row[0] + (" / near" if near else " / silhouette"), *row[1:]])
        refined += near
    art = dict(
        eye=write(out / "registered-close.glb", routed, "near"),
        guardian=write(out / "registered-guardian.glb", rows, "silhouette"),
    )
    site = dict(
        subject="The east dry slack, behind the old foredune",
        terrain_sha256=SHA,
        bounds=BOUNDS,
        stations=cams,
        landmarks=markers,
        counts=dict(counts),
        registered_roots=len(rows),
        fine_roots=refined,
        render_geometry=art,
        treatment="Sunspindles over the lower cup, dry copper rush along the fans, silver mat on drier couloirs, a few wing-leaf stools. No water surface or physical proxy added.",
        survey="66 west/east cross-sections (65 intervals) 2 m apart along -112..18 native z; actual low point from x 305..360 on original triangle terrain, no native valley cut. Floor is above sea, so it stays dry.",
        original_boxes="Xmin Zmin Xmax Zmax Ymin Ymax; four per-species crown clearance margins, sampled native planted roots only. No gameplay path claim.",
    )
    (out / "site.json").write_text(json.dumps(site, indent=2) + "\n")


def photograph(root, d, src, board, out, blender):
    field = out / "hollow"
    assets(src, d / "props.json", field)
    shelf = out / "shelf"
    shelf.mkdir(exist_ok=True)
    for n in ("ripple.png", "shore.png"):
        (shelf / n).symlink_to((d / "cove-water" / n).resolve())
    (shelf / "water.json").write_bytes(
        (root / "studies/island-art-week/water-selected.json").read_bytes()
    )
    for tag in TAGS:
        for side in ("after",) if board.get("held_before") else ("before", "after"):
            blender(
                "render_island_geology.py",
                d / "terrain-chunks.glb",
                out / side,
                "--guardian-review" if tag == "slack-guardian" else "--copse-eye",
                "--sunspindle-view",
                tag,
                *(["--sunspindle-field", field] if side == "after" else []),
                "--surface",
                src,
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
                shelf,
                "--habitats",
                d / "habitat-ground.glb",
                "--wild-sward",
                d / "meadow-sward.glb",
                "--meadow-library",
                d,
                "--meadow-habits",
                "--strandline",
                d / "strandline.glb",
                "--copse-floor",
                d / "copse-floor.glb",
                "--studio-samples",
                48
            )
    blender("studio_sunspindle_desk.py", src, field, out / "Sunspindle-Hollow.blend")
    (out / "studio-note.json").write_text(json.dumps(board, indent=2) + "\n")
