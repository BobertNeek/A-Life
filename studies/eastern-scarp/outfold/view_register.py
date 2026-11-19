"""Find which true original shore coordinates occupy the held creature camera, before placing detail.

These are light-weight art sightline samples on the archived V1 heightfield, not
gameplay sensor/creature rays, physics checks or an occlusion-rendering claim.
"""

import csv, json, sys, math
from pathlib import Path
import numpy as np

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[2] / "scripts"))
from island_inlet_ground import Inlet


def normal(v):
    return v / np.linalg.norm(v)


def mapview(native, photo, out):
    b = Inlet(native / "island-v1.bin")
    meta = json.loads((photo / "photography.json").read_text())
    eye, aim, _, lens = meta["cameras"]["wave-eye"]
    eye = np.array(eye)
    v = normal(np.array(aim) - eye)
    right = normal(np.cross(v, [0.0, 1.0, 0.0]))
    up = np.cross(right, v)
    dx = 36 / (2 * lens)
    dy = dx * 500 / 840
    result = []
    closest = []
    sea = 0
    for iy in range(17):
        for ix in range(27):
            sx = (ix + 0.5) / 27
            sy = (iy + 0.5) / 17
            d = normal(v + right * dx * (2 * sx - 1) + up * dy * (1 - 2 * sy))
            ds = np.r_[np.linspace(0.02, 15, 170), np.linspace(15.12, 90, 120)]
            p = eye + d * ds[:, None]
            hh = b.floor(p[:, 0], p[:, 2])[0]
            inter = np.nonzero(p[:, 1] <= np.maximum(0, hh))[0]
            if not len(inter):
                continue
            i = inter[0]
            lo = ds[max(0, i - 1)]
            hi = ds[i]
            for j in range(12):
                mid = (lo + hi) / 2
                x, y, z = eye + d * mid
                floor = float(b.floor(x, z)[0])
                if y <= max(0, floor):
                    hi = mid
                else:
                    lo = mid
            dist = (lo + hi) / 2
            x, y, z = eye + d * dist
            ground, gx, gz = map(float, b.floor(x, z))
            if ground <= 0:
                sea += 1
                continue
            result.append([sx, sy, x, ground, z, math.hypot(gx, gz), dist])
    out.mkdir(exist_ok=True, parents=True)
    with (out / "sighted-face.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            ["screen_u", "screen_v", "x", "native_y", "z", "grade", "distance_m"]
        )
        w.writerows([[round(float(a), 5) for a in row] for row in result])
    ar = np.array(result)
    quant = lambda col: [
        round(float(k), 3) for k in np.percentile(ar[:, col], [5, 25, 50, 75, 95])
    ]
    summary = dict(
        grid=[27, 17],
        land_hits=len(ar),
        water_hits=sea,
        sky_or_not_within_90m=27 * 17 - len(ar) - sea,
        height_5_25_50_75_95_m=quant(3),
        z_5_25_50_75_95=quant(4),
        distance_percentiles_m=quant(6),
        note="Original held optics; cut off at 90 m. Not a native game or organism camera.",
    )
    (out / "sighted-face.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


if __name__ == "__main__":
    print(json.dumps(mapview(*map(Path, sys.argv[1:4]))))
