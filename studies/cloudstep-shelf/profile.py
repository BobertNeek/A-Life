"""Measure the actual high forest shelf, its two gates and its seaward lip.
No route/pathfinding claim. A connected gentle patch is a terrain surveying
fact, not a clearance check against original trees or native creatures.
"""

import csv, hashlib, json, math, sys
from pathlib import Path
import numpy as np
from scipy.ndimage import label

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[1] / "scripts"))
from island_inlet_ground import Inlet

BASE = "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
BOX = (48.0, 127.0, -447.0, -369.0)


def cast(b, eye, aim):
    x, y, z = eye
    tx, ty, tz = aim
    ts = np.linspace(0.004, 0.996, 230)
    xx = x + (tx - x) * ts
    zz = z + (tz - z) * ts
    yy = y + (ty - y) * ts
    diff = b.floor(xx, zz)[0] - yy
    i = int(np.argmax(diff))
    return dict(
        blocked_over_ray_m=round(float(diff[i]), 3),
        max_blocker_xz=[round(float(xx[i]), 2), round(float(zz[i]), 2)],
    )


def survey(native, out):
    out.mkdir(parents=True, exist_ok=True)
    world = native / "island-v1.bin"
    if hashlib.sha256(world.read_bytes()).hexdigest() != BASE:
        raise ValueError("Not the frozen V1 island")
    b = Inlet(world)
    p = json.loads((native / "props.json").read_text())
    a, c, e, f = BOX
    x, z = np.meshgrid(np.arange(a, c + 1, 1.0), np.arange(e, f + 1, 1.0))
    y, dx, dz = b.floor(x, z)
    gr = np.hypot(dx, dz)
    gentle = (y >= 57.0) & (y <= 73.0) & (gr < 0.44)
    component, n = label(gentle)
    ix = int(-406 - e)
    iz = int(82 - a)
    connected = component == component[ix, iz]
    if not gentle[ix, iz]:
        raise ValueError("Selected anchor is not on the sheltered incline")
    records = []
    for i, r in enumerate(p):
        px, py, pz = r["position"]
        if a <= px <= c and e <= pz <= f:
            yy, gx, gz = map(float, b.floor(px, pz))
            qx, qz = min(int(c - a), round(px - a)), min(int(f - e), round(pz - e))
            records.append(
                dict(
                    id=i,
                    kind=r["kind"],
                    original_xyz=r["position"],
                    scale=r["scale"],
                    yaw=r["yaw"],
                    original_v1_y=round(yy, 4),
                    native_grade=round(math.hypot(gx, gz), 4),
                    on_gentle_connected_shelf=bool(connected[qz, qx]),
                )
            )
    # Two real unobstructed-by-ground viewing directions might be composition
    # choices, but the original forest can still hide them. Measure, don't
    # claim visibility until the mapped meshes are composed.
    px, pz = 82.0, -408.0
    ey = [px, float(b.floor(px, pz)[0]) + 1.04, pz]
    mountain = [28.125, 240.0, -275.0]
    sea = [118.0, 0.0, -461.0]
    witnesses = {
        name: dict(xyz=q, ground_profile=cast(b, ey, q))
        for name, q in [("real_summit", mountain), ("seaward_reference", sea)]
    }
    tr = []
    for xi in np.arange(52.0, 122.0, 2.0):
        for zi in np.arange(-445.0, -370.0, 2.0):
            yy, gx, gz = map(float, b.floor(xi, zi))
            tr.append(
                [
                    xi,
                    zi,
                    round(yy, 5),
                    round(gx, 5),
                    round(gz, 5),
                    int(connected[round(zi - e), round(xi - a)]),
                ]
            )
    with (out / "shelf-stations.csv").open("w", newline="") as h:
        w = csv.writer(h)
        w.writerow(
            (
                "x",
                "z",
                "native_y",
                "native_dx",
                "native_dz",
                "connected_gentle_ground_only",
            )
        )
        w.writerows(tr)
    close = (x >= 54) & (x <= 114) & (z >= -444) & (z <= -384)
    stats = dict(
        name="Cloudstep Shelf",
        original_V1_sha256=BASE,
        bounds_xz=BOX,
        original_ground_range_m=[round(float(y.min()), 3), round(float(y.max()), 3)],
        relief_in_named_60m_square_m=round(float(np.ptp(y[close])), 3),
        survey_resolution_m=1,
        one_metre_centres_connected=int(connected.sum()),
        gentle_survey_grade_max=0.44,
        gentle_survey_height_m=[57, 73],
        original_pines_on_connected_centres=sum(
            q["kind"] == "Conifer" and q["on_gentle_connected_shelf"] for q in records
        ),
        real_native_props_in_window=records,
        registered_eye_xyz=ey,
        physical_sight_witnesses=witnesses,
        sampled_stations=len(tr),
        clearance_or_navigation_checked=False,
        changed_native_bytes=0,
    )
    (out / "shelf-register.json").write_text(json.dumps(stats, indent=2) + "\n")
    return {k: v for k, v in stats.items() if k != "real_native_props_in_window"}


if __name__ == "__main__":
    print(survey(*(Path(x) for x in sys.argv[1:])))
