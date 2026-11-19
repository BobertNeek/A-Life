"""An artist's broken sward on the genuine lee slope, with no path, new land or collider.

The native slope is smooth here. It does not contain an exposed rock step.
Instead of fabricating rocky relief, identify a broad living meadow attached
to the *actual* 22–30 degree rain-catching inner face. Unequal swaths of
short grass are interrupted by old plants, loss of shelter, and real bare soil.
"""

import csv, json, math, sys
from pathlib import Path
import numpy as np
from scipy.spatial import cKDTree

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parent))
from community import Exposure, ease, nz

# Major masses, NOT individual root positions. Long side follows the sea breeze
# or turns as the meadow opens to the crest. Values are metres and radians.
DRIFTS = (
    (349.15, -84.30, 3.5, 1.42, 0.52),
    (354.00, -77.48, 2.8, 1.65, -0.16),
    (359.30, -91.92, 4.9, 1.80, 0.72),
    (360.70, -81.55, 4.3, 2.22, 0.29),
    (366.75, -85.92, 4.5, 1.45, -0.21),
    (371.38, -84.98, 2.8, 1.61, 0.68),
    (375.54, -78.43, 4.7, 2.28, -0.57),
    (380.30, -87.21, 3.3, 1.55, 0.73),
    (378.72, -95.00, 3.2, 1.79, 0.50),
    (369.85, -70.85, 4.5, 1.66, -0.13),
    (383.32, -74.89, 2.7, 1.80, 0.49),
    (358.91, -74.15, 2.8, 1.19, -0.78),
)


def plan(native, hollow, rise, out):
    out.mkdir(parents=True, exist_ok=True)
    e = Exposure(native, hollow)
    prior = list(csv.DictReader((rise / "lee-roots.csv").open()))
    previous = cKDTree([(float(r["x"]), float(r["z"])) for r in prior])
    rng = np.random.default_rng(451211)
    selected = []
    # Each island of grass is a tangle of small oblique three-to-nine-crown
    # rhizomes. Blanks in and BETWEEN the communities stay open ground.
    for k, (cx, cz, lu, lv, ang) in enumerate(DRIFTS):
        co, si = math.cos(ang), math.sin(ang)
        used = []
        for j in range(round(lu * lv * 4)):
            u, v = rng.uniform(-1, 1, 2)
            if (u * u + v * v) > 1:
                continue
            x = cx + co * u * lu - si * v * lv
            z = cz + si * u * lu + co * v * lv
            y, gx, gz, ob, lee, salt, duff, edge = e.field(np.array(x), np.array(z))
            grade = math.hypot(gx, gz)
            if not (0.18 < grade < 0.72) or ob < 0.17 or duff > 0.45:
                continue
            if nz(x * 0.90, z, 1.24, 753) < 0.25 or e.anchors.query([x, z])[0] < 1.0:
                continue
            if any(math.hypot(xx - x, zz - z) < 0.63 for xx, zz in used):
                continue
            used.append((x, z))
            angle = ang + rng.normal(0, 0.52)
            lc = math.cos(angle)
            ls = math.sin(angle)
            axis = rng.uniform(0.30, 0.91)
            stem = rng.uniform(0.12, 0.29)
            n = int(rng.integers(3, 9))
            phase = rng.uniform(-math.pi, math.pi)
            for i in range(n):
                t = i / max(1, n - 1)
                p = (t - 0.5) * 2 * axis + rng.normal(0, 0.04)
                q = stem * math.sin(phase + 3.2 * t) + rng.normal(0, 0.12)
                ax = x + lc * p - ls * q
                az = z + ls * p + lc * q
                if (
                    previous.query([ax, az])[0] < 0.33
                    or e.original.query([ax, az])[0] < 0.36
                ):
                    continue
                if any(math.hypot(o[3] - ax, o[5] - az) < 0.19 for o in selected):
                    continue
                ay, dx, dz = map(float, e.b.floor(ax, az))
                # Blades grow from a precise native root. Mature meadow
                # lapels have soft rolled points, not the earlier tall rods.
                size = float(rng.uniform(0.67, 1.18)) * (
                    0.66 + 0.35 * math.sin(math.pi * (0.10 + 0.81 * t))
                )
                wind = rng.normal(0, 0.44) + ang * 0.35
                selected.append(
                    (k, j, i, ax, ay, az, dx, dz, int(rng.integers(0, 4)), wind, size)
                )
    with (out / "stitches.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            (
                "drift",
                "rhizome",
                "shoot",
                "x",
                "native_y",
                "z",
                "dx",
                "dz",
                "form",
                "wind_yaw",
                "size",
            )
        )
        w.writerows(
            [
                [
                    round(float(a), 6) if y in (3, 4, 5, 6, 7, 9, 10) else a
                    for y, a in enumerate(q)
                ]
                for q in selected
            ]
        )
    clusters = len({(r[0], r[1]) for r in selected})
    counts = [sum(r[0] == j for r in selected) for j in range(len(DRIFTS))]
    meta = dict(
        art="Stitchgrass / broken lee-slope brows",
        community_centres=DRIFTS,
        individual_shoots=len(selected),
        separate_rhizomes=clusters,
        shoots_by_drift=counts,
        held_rise_roots=181,
        minimum_clearance_existing_woody_hollow_m=0.36,
        newly_allocated_creature_target=0,
        world_displacement_m=0,
    )
    (out / "planting.json").write_text(json.dumps(meta, indent=2) + "\n")
    return meta


if __name__ == "__main__":
    print(plan(*(Path(p) for p in sys.argv[1:])))
