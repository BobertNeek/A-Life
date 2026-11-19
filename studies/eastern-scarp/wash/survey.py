"""Outer native waterline, bathymetric exposure and discontinuous breaker eligibility.

An actual map, not an SVG curve wrapped decoratively around the island. This is
a design input for small optical accents; it neither simulates tides nor
suggests that the player or creatures can walk offshore.
"""

from pathlib import Path
import sys, csv, json, math
import numpy as np

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parents[2] / "scripts"))
from island_inlet_ground import Inlet

RANGE = (-163.5, -67.0)


def curtain(b):
    z = np.arange(RANGE[0], RANGE[1] + 0.0001, 0.25)
    x = np.arange(343, 462.001, 0.25)
    gx, gz = np.meshgrid(x, z)
    y = b.floor(gx, gz)[0]

    def cross(yy, level):
        ii = np.flatnonzero((yy[:-1] >= level) & (yy[1:] < level))
        if not len(ii):
            return math.nan
        i = ii[-1]
        return x[i] + (x[i + 1] - x[i]) * (yy[i] - level) / (yy[i] - yy[i + 1])

    draws = np.array([[cross(r, d) for d in [0, -0.18, -0.65, -1.3, -2.5]] for r in y])
    return z, draws


def write(native, out):
    out.mkdir(parents=True, exist_ok=True)
    b = Inlet(native / "island-v1.bin")
    z, q = curtain(b)
    valid = np.all(np.isfinite(q), axis=1)
    if not valid.all():
        raise ValueError(
            "Outer contour left the authored envelope; do not interpolate a made-up seabed"
        )
    cx, v018, v065, v13, v25 = q.T
    # Outward normal of this gently winding east shoreline in true world XZ.
    dz = np.gradient(cx, 0.25)
    n = np.vstack([np.ones_like(dz), -dz]).T
    n /= np.sqrt((n * n).sum(1))[:, None]
    # Signed turn relative to the same real coast six metres either side.
    broad = np.convolve(np.pad(cx, 24, mode="edge"), np.ones(49) / 49, mode="valid")
    prominence = cx - broad
    with (out / "waterline.csv").open("w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(
            (
                "z",
                "coast_x",
                "depth018_x",
                "depth065_x",
                "depth13_x",
                "depth25_x",
                "normal_x",
                "normal_z",
                "local_prominence_m",
            )
        )
        for row in zip(z, cx, v018, v065, v13, v25, n[:, 0], n[:, 1], prominence):
            w.writerow([round(float(f), 5) for f in row])
    data = dict(
        interval_z=RANGE,
        samples=int(len(z)),
        coast_distance_m=round(float(np.hypot(np.diff(cx), np.diff(z)).sum()), 3),
        xz_endpoints=[[float(cx[0]), float(z[0])], [float(cx[-1]), float(z[-1])]],
        range_065_below_x_metres=[
            round(float(np.min(v065 - cx)), 3),
            round(float(np.max(v065 - cx)), 3),
        ],
        mean_water_y=0,
        art_only=True,
        existing_coast_held=True,
        new_land_or_tide_simulation=False,
    )
    (out / "reach.json").write_text(json.dumps(data, indent=2) + "\n")
    return data


if __name__ == "__main__":
    print(json.dumps(write(*(Path(s) for s in sys.argv[1:3])), indent=2))
