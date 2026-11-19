"""Artist's real-ground transect from Sunspindle Hollow to the paired Gullglass spruce.

Use the original surface and actual pre-existing prop roots as compositional
anchors. A continuous sample of ground is not a gameplay navigation proof.
The notable feature is that the coast disappears behind a *real* dry ridge.
"""

from pathlib import Path
import sys, csv, json, math, hashlib
import numpy as np

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[2] / "scripts"))
from island_inlet_ground import Inlet

# Chosen stations span the existing calm hollow, a previously blank
# windward shoulder, the original twin grove at 113/638, and finally its real lip.
ANCHORS = (
    (344.0, -75.0),
    (354.0, -79.6),
    (364.0, -83.2),
    (372.5, -82.4),
    (381.0, -82.8),
    (386.0, -84.7),
    (395.0, -84.7),
    (402.2, -85.0),
    (406.5, -85.0),
)
WORLD_HASH = "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"


def inspect(native, hollow, out):
    out.mkdir(exist_ok=True, parents=True)
    p = native / "island-v1.bin"
    sha = hashlib.sha256(p.read_bytes()).hexdigest()
    if sha != WORLD_HASH:
        raise ValueError("Requires frozen original V1 terrain")
    s = Inlet(p)
    props = json.loads((native / "props.json").read_text())
    crowns = json.loads((native / "grove-crowns.json").read_text())
    herb = list(csv.DictReader((hollow / "root-register.csv").open()))
    close = [
        a for a in herb if 347 <= float(a["x"]) <= 390 and -101 <= float(a["z"]) <= -66
    ]

    def floor(x, z):
        return float(s.floor(x, z)[0])

    result = []
    total = 0

    def masked(x, h, z, target):
        tx, ty, tz = target
        t = np.linspace(0.025, 0.985, 111)
        X = x + (tx - x) * t
        Z = z + (tz - z) * t
        line = h + (ty - h) * t
        over = s.floor(X, Z)[0] - line
        i = int(np.argmax(over))
        return [round(float(over[i]), 3), round(float(X[i]), 3), round(float(Z[i]), 3)]

    sea = [448.0, 0.0, -85.0]
    for a, b in zip(ANCHORS, ANCHORS[1:]):
        a = np.asarray(a)
        b = np.asarray(b)
        length = np.linalg.norm(b - a)
        n = math.ceil(length / 0.42)
        for k in range(n):
            t = k / n
            x, z = a + t * (b - a)
            y, gx, gz = map(float, s.floor(x, z))
            d = total + t * length
            cover = masked(x, y + 1.04, z, sea)
            grade = math.hypot(gx, gz)
            result.append([d, x, y, z, grade, gx, gz, *cover])
        total += length
    with (out / "rise-section.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            [
                "along_m",
                "x",
                "native_y",
                "z",
                "surface_grade",
                "dx",
                "dz",
                "original_cover_over_seaward_ray_m",
                "cover_x",
                "cover_z",
            ]
        )
        w.writerows([[round(float(a), 5) for a in r] for r in result])
    y = np.array([r[2] for r in result])
    peak = int(np.argmax(y))
    i = next((i for i, r in enumerate(result) if r[7] < -0.02), None)
    focus = []
    for ix in [0, round(len(result) * 0.40), peak, len(result) - 1]:
        r = result[ix]
        focus.append(
            dict(xz=[r[1], r[3]], native_y=r[2], eye_y=r[2] + 1.04, sea_blockage_m=r[7])
        )
    r = dict(
        name="Twin Needle Rise / real lee cross-section",
        sha=sha,
        profile_length_m=round(float(total), 3),
        samples=len(result),
        low_lee_y=round(float(y[0]), 3),
        true_highpoint_y=round(float(y[peak]), 3),
        height_change_to_crest_m=round(float(y[peak] - y[0]), 3),
        provisional_first_clear_waterline_along_m=(
            None if i is None else round(float(result[i][0]), 3)
        ),
        ocean_sight_reference=sea,
        original_tree_records={
            str(k): dict(**props[k], crown_form=crowns[str(k)]["form"])
            for k in (113, 638)
        },
        earlier_hollow_roots_in_overlap=len(close),
        native_terrain_unchanged=True,
        actor_navigation_not_inferred=True,
        stations=focus,
    )
    (out / "rise.json").write_text(json.dumps(r, indent=2) + "\n")
    return r


if __name__ == "__main__":
    print(json.dumps(inspect(*(Path(a) for a in sys.argv[1:4])), indent=2))
