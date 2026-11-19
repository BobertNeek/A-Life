"""Tiny visual anchors on actual exposed east-coast triangles, not a fake high-water line."""

from pathlib import Path
import csv, json, sys, math
import numpy as np
from scipy.spatial.transform import Rotation

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
from island_inlet_ground import Inlet, ease


def chart(native, out):
    s = Inlet(native / "island-v1.bin")
    r = np.random.default_rng(11887)
    out.mkdir(exist_ok=True, parents=True)
    # No biological entities or food spawns. These are contact-scale shell casts
    # in disconnected sheltered wave pockets, absent on swept dry inclines.
    centers = [
        (-154.8, 0.67, 1.11),
        (-142.6, 1.04, 1.55),
        (-122.4, 0.84, 1.76),
        (-111.8, 0.62, 0.95),
        (-102.6, 0.95, 1.42),
        (-99.3, 0.78, 0.82),
        (-94.5, 1.36, 0.66),
        (-83.6, 0.62, 1.37),
        (-77.8, 1.09, 0.61),
    ]
    anchors = []
    sol = []
    for cz, h0, spread in centers:
        for j in range(int(r.integers(11, 26))):
            z = cz + r.normal(0, 0.74 * spread)
            target = h0 + r.normal(0, 0.23 * spread)
            if not 0.18 < target < 1.76:
                continue
            x0, x1 = 357.0, 432.0
            # Exposed shore in these columns is monotone only on the final
            # thin landward toe. Bracket the outermost fall through desired y.
            xx = np.arange(x0, x1, 0.25)
            hs = s.floor(xx, np.full_like(xx, z))[0]
            k = np.nonzero((hs[:-1] >= target) & (hs[1:] < target))[0]
            if not len(k):
                continue
            k = k[-1]
            lo, hi = xx[k], xx[k + 1]
            for _ in range(16):
                xm = (lo + hi) * 0.5
                if s.floor(xm, z)[0] >= target:
                    lo = xm
                else:
                    hi = xm
            x = (lo + hi) * 0.5
            h, gx, gz = s.floor(x, z)
            normal = np.array([-gx, 1.0, -gz])
            normal /= np.linalg.norm(normal)
            if normal[0] < 0.13 or normal[1] < 0.17:
                continue
            radius = r.uniform(0.019, 0.056) * (1.15 if j < 3 else 0.86)
            if any(
                math.hypot(z - b[3], h - b[2]) < 0.03 + radius + b[4] for b in anchors
            ):
                continue
            axis = np.cross([0, 1.0, 0], normal)
            rotation = Rotation.from_rotvec(
                axis / max(np.linalg.norm(axis), 1e-7) * math.acos(normal[1])
            ) * Rotation.from_rotvec([0.0, r.uniform(-2.8, 2.8), 0])
            anchors.append(
                [
                    int(r.integers(1, 4)),
                    float(x),
                    float(h),
                    float(z),
                    float(radius),
                    *rotation.as_quat(),
                    *normal,
                ]
            )
    with (out / "shell-seat.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            (
                "form",
                "x",
                "native_y",
                "z",
                "radius_m",
                "qx",
                "qy",
                "qz",
                "qw",
                "nx",
                "ny",
                "nz",
            )
        )
        for i, *q in anchors:
            w.writerow([i, *[round(float(x), 7) for x in q]])
    near = [a for a in anchors if math.hypot(a[1] - 409.6, a[3] + 97) < 8.0]
    (out / "shell-contacts.json").write_text(
        json.dumps(
            dict(
                wave_pockets=len(centers),
                actual_surface_contacts=len(anchors),
                within_8m_of_held_eye=len(near),
                minimum_ground_m=min(a[2] for a in anchors),
                maximum_ground_m=max(a[2] for a in anchors),
                static_visual_only=True,
                changes_world=False,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    chart(*(Path(x) for x in sys.argv[1:3]))
