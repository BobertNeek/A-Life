"""Design roots for a few coastal bladderlace fans on actual V1 intertidal stone.

These are noninteractive artist meshes, not spawning biology, edible food or
game entities. The huge gaps between rooted knuckles of rock are deliberate.
"""

from pathlib import Path
import csv, sys, json, math
import numpy as np
from scipy.spatial.transform import Rotation

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[2] / "scripts"))
from island_inlet_ground import Inlet

POCKETS = (
    (-146.42, 0.28, 0.41, 6),
    (-125.1, 0.63, 0.6, 5),
    (-113.7, 0.14, 0.8, 4),
    (-100.88, 0.84, 0.62, 8),
    (-98.45, 0.64, 0.44, 5),
    (-91.7, 1.02, 0.38, 3),
    (-79.2, 0.3, 0.45, 4),
)


def unit(a):
    return a / max(1e-10, np.linalg.norm(a))


def chart(native, shells, out):
    out.mkdir(parents=True, exist_ok=True)
    bed = Inlet(native / "island-v1.bin")
    shell = np.genfromtxt(shells / "shell-seat.csv", delimiter=",", names=True)
    rng = np.random.default_rng(14006)
    roots = []
    for group, (cz, cy, spread, n) in enumerate(POCKETS, 1):
        count = 0
        for k in range(220):
            if count >= n:
                break
            z = cz + rng.normal(0, spread)
            target = cy + rng.normal(0, 0.27 * spread)
            xx = np.arange(357, 433, 0.24)
            yy = bed.floor(xx, np.full_like(xx, z))[0]
            found = np.nonzero((yy[:-1] >= target) & (yy[1:] < target))[0]
            if not len(found):
                continue
            i = found[-1]
            lo = xx[i]
            hi = xx[i + 1]
            for j in range(15):
                v = (lo + hi) * 0.5
                if bed.floor(v, z)[0] >= target:
                    lo = v
                else:
                    hi = v
            x = (lo + hi) * 0.5
            y, gx, gz = map(float, bed.floor(x, z))
            nrm = unit(np.array([-gx, 1.0, -gz]))
            if nrm[0] < 0.15 or not -0.04 < y < 1.33:
                continue
            # Do not replace a previously photographed shell cast with brush.
            if (
                np.min(
                    (shell["z"] - z) ** 2
                    + (shell["x"] - x) ** 2
                    + (shell["native_y"] - y) ** 2
                )
                < 0.145**2
            ):
                continue
            if any(
                (z - r[4]) ** 2 + (y - r[3]) ** 2 + (x - r[2]) ** 2 < 0.18**2
                for r in roots
            ):
                continue
            roll = rng.uniform(-0.26, 0.26)
            down = unit(
                np.array([0.21, -1, roll]) - nrm * np.dot(nrm, [0.21, -1, roll])
            )
            tangent = unit(np.cross(nrm, down))
            down = unit(np.cross(tangent, nrm))
            R = Rotation.from_matrix(np.column_stack([tangent, nrm, down]))
            scale = rng.uniform(0.25, 0.48) * (1.05 if group in (4, 5) else 0.92)
            # Estimate local sheet deviation at the tips of a 70%-length fan.
            # Very sharp step changes can swallow the authored leaf; prefer a
            # neighbouring original face, never reshape it for the asset.
            d = np.array([x, y, z]) + down * scale * 0.70
            dh = float(bed.floor(d[0], d[2])[0]) - d[1]
            if abs(dh) > 0.095:
                continue
            roots.append(
                (group, int(rng.integers(1, 4)), x, y, z, scale, *R.as_quat(), *nrm)
            )
            count += 1
    with (out / "lace-roots.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            [
                "group",
                "form",
                "x",
                "native_y",
                "z",
                "span_m",
                "qx",
                "qy",
                "qz",
                "qw",
                "nx",
                "ny",
                "nz",
            ]
        )
        w.writerows(
            [
                [round(float(a), 7) if j > 1 else a for j, a in enumerate(p)]
                for p in roots
            ]
        )
    report = dict(
        distinct_pockets=len(set(p[0] for p in roots)),
        root_count=len(roots),
        close_8m=sum(math.hypot(p[2] - 409.6, p[4] + 97) < 8 for p in roots),
        minimum_native_y=min(r[3] for r in roots),
        maximum_native_y=max(r[3] for r in roots),
        not_game_entities=True,
        collision_changed=False,
        existing_tide_cast_contacts_avoided=True,
    )
    (out / "lace-sites.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    print(chart(*map(Path, sys.argv[1:4])))
