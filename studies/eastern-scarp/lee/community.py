"""One contiguous wind/soil field between the real inner hollow and the two original pine roots.

Well-marked landmarks remain native. The patch palette exists in original
habitat coordinates and never creates an edible node, route, collider or
repeat planting band on a contour.
"""

from pathlib import Path
import math, json, csv, sys
import numpy as np
from scipy.spatial import cKDTree
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[2] / "scripts"), str(H.parents[1] / "lanternback-rim")]
from island_inlet_ground import Inlet, ease
from strata import nz

BOX = (341.0, 403.0, -107.0, -59.0)


class Exposure:
    def __init__(self, native, hollow):
        self.b = Inlet(native / "island-v1.bin")
        ps = json.loads((native / "props.json").read_text())
        self.twins = np.array([ps[i]["position"] for i in (113, 638)])
        rr = list(csv.DictReader((hollow / "root-register.csv").open()))
        self.original = cKDTree([[float(v["x"]), float(v["z"])] for v in rr])
        self.anchors = cKDTree(
            [
                [p["position"][0], p["position"][2]]
                for p in ps
                if p["kind"]
                in ("Conifer", "Broadleaf", "Cliff", "BoulderA", "BoulderB")
            ]
        )

    def field(self, x, z):
        b = self.b
        y, gx, gz = b.floor(x, z)
        a0, a1, c0, c1 = BOX
        edge = ease(0, 4.1, np.minimum(x - a0, a1 - x)) * ease(
            0, 5.2, np.minimum(z - c0, c1 - z)
        )
        broad = nz(x * 0.76, z, 6.8, 341) - 0.5
        f = nz(x, z, 2.1, 351) - 0.5
        m = nz(x, z, 0.70, 372) - 0.5
        # Never assume a flat, unbroken ridge: chart the original physical
        # horizon in the direction from which the sea breeze reaches the site.
        obstruction = np.zeros_like(y)
        for d in (2.5, 5.5, 10, 17, 25, 36):
            hy = b.floor(x + d, z + d * 0.16)[0]
            obstruction = np.maximum(obstruction, (hy - y) / d)
        gentle = ease(0.09, 0.46, obstruction)
        exposed = 1 - gentle
        dist = np.full_like(x, 500.0, dtype=float)
        for tx, ty, tz in self.twins:
            # The original trees break the wind on their *lee* side, not
            # inside a stamped circular soil disk at the crest.
            d = np.hypot((x - tx + 2.4) / 4.9, (z - tz + 1.0) / 2.65)
            dist = np.minimum(dist, d)
        pine = (1 - ease(0.30, 1.66, dist + f * 0.31)) * edge
        # A broken sheltered society that splits around the original turf, an
        # exposed salt/seed community, and a smaller dry pine-floor pocket.
        lee = (
            ease(0.38, 0.70, 0.54 + 1.15 * broad + 0.69 * f + 0.35 * m)
            * gentle
            * edge
            * (1 - 0.57 * pine)
        )
        salt = (
            ease(
                0.54,
                0.79,
                0.52 + 0.96 * broad + 0.66 * f + 0.65 * m + 0.15 * np.maximum(0, gx),
            )
            * exposed
            * edge
            * (1 - 0.73 * pine)
        )
        dry = pine * ease(0.22, 0.64, 0.54 + 0.45 * f + 0.22 * m)
        return y, gx, gz, obstruction, lee, salt, dry, edge


def plan(native, hollow, out):
    out.mkdir(exist_ok=True, parents=True)
    e = Exposure(native, hollow)
    rng = np.random.default_rng(94027)
    a, b, c, d = BOX
    W, H = 620, 480
    x, z = np.meshgrid(
        a + (np.arange(W) + 0.5) * (b - a) / W, c + (np.arange(H) + 0.5) * (d - c) / H
    )
    y, gx, gz, shade, lee, salt, dry, edge = e.field(x, z)
    # Store masks rather than paint a new opaque lawn over the whole ridge.
    eco = np.uint8(np.clip(255 * np.dstack([lee, salt, dry]), 0, 255))
    Image.fromarray(eco).save(out / "ground-communities.png", optimize=True)
    r = []
    # Choose from broad original grassland and a few extra candidates in the
    # genuine wind shadow. All later minimum distances still apply.
    X = np.r_[rng.uniform(a, b, 4400), rng.normal(383.7, 2.80, 210)]
    Z = np.r_[rng.uniform(c, d, 4400), rng.normal(-82.8, 1.92, 210)]
    HH, GG, BB, CC, LL, SS, DD, EE = e.field(X, Z)
    for x, z, y, gx, gz, cover, L, S, D, edge in zip(
        X, Z, HH, GG, BB, CC, LL, SS, DD, EE
    ):
        probs = np.array([L, S, D]) ** 1.3
        strength = max(probs)
        if strength < 0.175 or rng.random() > 0.16 * strength:
            continue
        if e.original.query([x, z])[0] < 0.67 or e.anchors.query([x, z])[0] < 0.82:
            continue
        t = int(np.argmax(probs))
        rad = rng.uniform(0.25, 0.54) * [1.0, 0.72, 0.65][t]
        if any(
            (x - q[2]) ** 2 + (z - q[4]) ** 2 < (0.61 + rad * 0.40 + q[5] * 0.33) ** 2
            for q in r
        ):
            continue
        r.append(
            [
                t + 1,
                int(rng.integers(1, 4)),
                x,
                y,
                z,
                rad,
                rng.uniform(-math.pi, math.pi),
                math.hypot(gx, gz),
                cover,
                strength,
            ]
        )
    with (out / "lee-roots.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            [
                "habitat",
                "form",
                "x",
                "native_y",
                "z",
                "canopy_m",
                "yaw",
                "grade",
                "sea_shelter_grade",
                "community_weight",
            ]
        )
        w.writerows(
            [[a if j < 2 else round(float(a), 6) for j, a in enumerate(q)] for q in r]
        )
    manifest = dict(
        bounds=BOX,
        communities=[
            "sheltered windwillow",
            "sea-facing copper rush",
            "twin-spruce duff and silver mat",
        ],
        new_root_counts=[sum(q[0] == j for q in r) for j in (1, 2, 3)],
        original_trees=[113, 638],
        no_rescatter_or_deletion=True,
        original_hollow_contacts_avoided_m=0.67,
    )
    (out / "communities.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


if __name__ == "__main__":
    print(plan(*[Path(p) for p in sys.argv[1:4]]))
