"""True uphill-to-sea descent traces, used only as weathering reference for the artist.

There is no simulated runoff, added water, height change or generated gully.
The SVG/CSV-style polylines represent ordinary descending paths on the archived
original terrain. Rendering remains optional until the native reserve clears.
"""

from pathlib import Path
import json, sys, math
import numpy as np

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[2] / "scripts"))
from island_inlet_ground import Inlet


def coast_curve(bed, z):
    xx = np.arange(350.0, 435.0, 0.5)
    yy = bed.floor(xx, np.full_like(xx, z))[0]
    ids = np.nonzero((yy[:-1] >= 0) & (yy[1:] < 0))[0]
    if not len(ids):
        return None
    k = ids[-1]
    a, b = xx[k], xx[k + 1]
    for _ in range(12):
        c = (a + b) * 0.5
        if bed.floor(c, z)[0] > 0:
            a = c
        else:
            b = c
    return (a + b) * 0.5


def descend(bed, px, pz):
    pts = []
    distance = 0.0
    for _ in range(175):
        y, gx, gz = map(float, bed.floor(px, pz))
        pts.append([px, y, pz, distance])
        if y < 0.26:
            return pts if len(pts) > 18 else None
        grad = math.hypot(gx, gz)
        if grad < 0.008:
            return None
        direct = np.array([-gx, -gz]) / grad
        base = np.array([px, pz])
        accepted = None
        # At a true native diagonal a source plane can lead directly into a
        # concave crease. Search a small fan of still-descending steps rather
        # than climbing the opposite triangle or drawing a straight route.
        for length in (0.42, 0.22, 0.095):
            dirs = np.array(
                [
                    direct * np.cos(a) + np.array([-direct[1], direct[0]]) * np.sin(a)
                    for a in (0, 0.28, -0.28, 0.62, -0.62, 0.98, -0.98, 1.31, -1.31)
                ]
            )
            p = base + length * dirs
            ys = bed.floor(p[:, 0], p[:, 1])[0]
            # Prefer a true descent, softly penalise a sharp zigzag, but never
            # accept a step because of smoothing if its bed height goes up.
            loss = ys + np.linspace(0, 0.012, len(ys))
            idx = int(np.argmin(loss))
            if ys[idx] < y - 0.0015:
                accepted = p[idx], length
                break
        if accepted is None:
            return None
        (px, pz), step = accepted
        distance += step
        if not -174 < pz < -56 or not 350 < px < 435:
            return None
    return None


def pathbook(native, write_to=None):
    bed = Inlet(native / "island-v1.bin")
    starts = []
    # Neighbouring, deliberately non-random source points reveal channels
    # into which several true routes are focused by the actual ground.
    for z in np.arange(-158.0, -73.5, 1.6):
        cx = coast_curve(bed, z)
        if cx is None:
            continue
        for land in (5.4, 8.6, 12.2, 16.0):
            x = cx - land
            hh = float(bed.floor(x, z)[0])
            if not 8.0 < hh < 28:
                continue
            pts = descend(bed, x, z)
            if pts and hh - pts[-1][1] > 7.7:
                starts.append(pts)
    groups = []
    for pts in sorted(starts, key=lambda q: q[-1][2]):
        toe = pts[-1]
        choices = [g for g in groups if abs(g["center"] - toe[2]) < 1.8]
        if not choices:
            groups.append(dict(center=toe[2], paths=[pts]))
            continue
        group = min(choices, key=lambda g: abs(g["center"] - toe[2]))
        group["paths"].append(pts)
        group["center"] = np.mean([q[-1][2] for q in group["paths"]])
    selected = []
    for g in sorted(groups, key=lambda g: -len(g["paths"])):
        if len(g["paths"]) < 3 or any(
            abs(g["center"] - a["center"]) < 8.5 for a in selected
        ):
            continue
        selected.append(g)
        if len(selected) == 5:
            break
    work = []
    for g in selected:
        # A minor attached stain can exist adjacent to this path; the record
        # itself is unmodified except for retaining every fourth source sample.
        outline = max(g["paths"], key=lambda p: p[0][1])
        picked = outline[::4]
        if picked[-1] != outline[-1]:
            picked.append(outline[-1])
        work.append(
            dict(
                source_count=len(g["paths"]),
                toe_z=round(float(g["center"]), 3),
                fall_m=round(float(outline[0][1] - outline[-1][1]), 3),
                line=[[round(float(v), 4) for v in pt] for pt in picked],
            )
        )
    report = dict(
        name="Gullglass native exposure-flow references",
        source_candidates=len(starts),
        catchments=work,
        unchanged_heights=True,
        hydrology_simulated=False,
        material_only=True,
    )
    if write_to is not None:
        Path(write_to).write_text(json.dumps(report, separators=(",", ":")) + "\n")
    return report


if __name__ == "__main__":
    book = pathbook(Path(sys.argv[1]), sys.argv[2] if len(sys.argv) > 2 else None)
    print(
        book["source_candidates"],
        [(q["source_count"], q["toe_z"], q["fall_m"]) for q in book["catchments"]],
    )
