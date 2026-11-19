"""Before committing pigment, measure where the genuine body/guardian lenses see rock.
This writes numeric native-ground footprint data only, never placeholder images.
The dense register is ground geometry only; the separate landmark survey handles
real original crown triangle/alpha visibility on the three named look lines.
"""

from pathlib import Path
import csv, sys, json, math
import numpy as np

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[1] / "scripts"))
from island_inlet_ground import Inlet


def unit(v):
    return v / np.linalg.norm(v, axis=-1, keepdims=True)


def find(native, out):
    b = Inlet(native / "island-v1.bin")
    x, z = 82.0, -408.0
    ey = [x, float(b.floor(x, z)[0]) + 1.04, z]
    heading = unit(np.array([28.125 - x, -275 - z]))
    run = 80.0
    pitch = math.radians(26.2)
    aim = [x + heading[0] * run, ey[1] + math.tan(pitch) * run, z + heading[1] * run]
    x2, z2 = 80.0, -412.0
    # Separate attitudes: looking at the actual high summit and looking down
    # with the original #991 and nearby boulder #1330. Do not squash the mountain.
    poses = {
        "summit-window": dict(eye=ey, aim=aim, lens=22, span=0.0),
        "sheltered-boles": dict(eye=ey, aim=[77.62, 68.10, -414.43], lens=29, span=0.0),
        "shelf-section": dict(
            eye=[155.0, 176.0, -504.0], aim=[63.0, 85.0, -378.0], lens=39, span=173.0
        ),
    }
    W, H = 51, 31
    xx, yy = np.meshgrid(
        ((np.arange(W) + 0.5) / W - 0.5) * 2, (0.5 - (np.arange(H) + 0.5) / H) * 2
    )
    a = xx.ravel()
    c = yy.ravel()
    report = {}
    for name, scene in poses.items():
        eye = np.array(scene["eye"])
        fwd = unit(np.array(scene["aim"]) - eye)
        right = unit(np.cross(fwd, [0, 1, 0]))
        up = np.cross(right, fwd)
        N = len(a)
        if scene["span"]:
            span = scene["span"]
            origin = eye + (span / 2) * (
                a[:, None] * right + (500 / 840) * c[:, None] * up
            )
            D = np.broadcast_to(fwd, (N, 3))
        else:
            origin = np.broadcast_to(eye, (N, 3))
            F = 36 / (2 * scene["lens"])
            D = unit(fwd + F * (a[:, None] * right + 500 / 840 * c[:, None] * up))
        alive = np.ones(N, bool)
        hits = np.full(N, np.nan)
        last = np.full(N, 0.06)
        for t in np.arange(0.10, 460.1, 0.90):
            idx = np.flatnonzero(alive)
            if not len(idx):
                break
            q = origin[idx] + D[idx] * t
            keep = (
                (q[:, 0] > -499) & (q[:, 0] < 499) & (q[:, 2] > -599) & (q[:, 2] < 599)
            )
            alive[idx[~keep]] = False
            idx = idx[keep]
            q = q[keep]
            if not len(q):
                continue
            below = q[:, 1] <= b.floor(q[:, 0], q[:, 2])[0]
            found = idx[below]
            if len(found):
                lo = last[found].copy()
                hi = np.full(len(found), t)
                for _ in range(11):
                    mid = 0.5 * (lo + hi)
                    q0 = origin[found] + D[found] * mid[:, None]
                    hit = q0[:, 1] <= b.floor(q0[:, 0], q0[:, 2])[0]
                    hi = np.where(hit, mid, hi)
                    lo = np.where(hit, lo, mid)
                hits[found] = (hi + lo) * 0.5
                alive[found] = False
            last[idx] = t
        ix = np.flatnonzero(np.isfinite(hits))
        p = origin[ix] + D[ix] * hits[ix, None]
        y, gx, gz = b.floor(p[:, 0], p[:, 2])
        grade = np.hypot(gx, gz)
        with (out / (name + "-footprint.csv")).open("w", newline="") as fp:
            w = csv.writer(fp)
            w.writerow(
                ("lens_col", "lens_row", "x", "v1_y", "z", "distance_m", "dx", "dz")
            )
            for i, row, hh, ds, dx, dz in zip(ix, p, y, hits[ix], gx, gz):
                w.writerow(
                    [
                        int(i % W),
                        int(i // W),
                        *[round(v, 4) for v in (row[0], hh, row[2], ds, dx, dz)],
                    ]
                )
        report[name] = dict(
            **scene,
            terrain_only=True,
            ray_grid=[W, H],
            native_hits=len(ix),
            unique_native_cells=len(
                {
                    (
                        math.floor((p0[0] + 500) / 3.125),
                        math.floor((p0[2] + 600) / 3.125),
                    )
                    for p0 in p
                }
            ),
            native_hit_height_m_10_50_90=np.quantile(y, [0.1, 0.5, 0.9])
            .round(3)
            .tolist(),
            distance_m_10_50_90=np.quantile(hits[ix], [0.1, 0.5, 0.9])
            .round(3)
            .tolist(),
            rock_grade_over_point75_percent=round(
                100 * float((grade > 0.75).mean()), 2
            ),
            native_hit_xz_10_90=np.quantile(p[:, [0, 2]], [0.1, 0.9], axis=0)
            .round(2)
            .tolist()
        )
    (out / "views.json").write_text(json.dumps(report, indent=2) + "\n")
    return {
        k: (
            v["native_hits"],
            v["native_hit_height_m_10_50_90"],
            v["distance_m_10_50_90"],
            v["native_hit_xz_10_90"],
        )
        for k, v in report.items()
    }


if __name__ == "__main__":
    print(find(*(Path(p) for p in sys.argv[1:])))
