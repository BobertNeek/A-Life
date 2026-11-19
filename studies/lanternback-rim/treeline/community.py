"""A sparse wind-sheltered understory between an existing herb colony and four true spruce."""

from pathlib import Path
import csv, json, math, sys
import numpy as np
from scipy.spatial.transform import Rotation
from PIL import Image

HERE = Path(__file__).resolve().parent
sys.path[:0] = [
    str(HERE),
    str(HERE.parent),
    str(HERE.parents[2] / "scripts"),
    str(HERE.parent / "windcups"),
]
from survey import Bed, BOX, DIGEST, EYE
from strata import nz
from island_inlet_ground import ease
from specimen import unit, pol, grid, tube, Y
from fissures import Piece, export
from island_erratic_glb import streams
from finished_island_glb import parse, save, values, set_values

# Geometric lee lobes. Unequal lengths, interrupted in real native open stone.
# (x, z, extent along/across, orientation radians, relative crown strength).
LOBES = (
    (248, -153.4, 2.36, 1.11, -0.66, 0.52),
    (250.5, -157.5, 3.24, 1.48, -0.86, 0.69),
    (245, -161.6, 2.85, 1.36, 0.73, 0.57),
    (252.5, -164.5, 3.15, 1.01, 0.29, 0.49),
    (246.4, -170.2, 3.45, 1.75, -0.73, 0.87),
    (243.7, -174.7, 3.25, 1.11, -1.11, 0.94),
    (239.7, -181.4, 4.22, 1.74, -0.64, 1.0),
    (233.9, -186.6, 4.8, 2.03, 0.54, 0.82),
    (251.3, -178.9, 3.84, 1.32, 0.89, 0.74),
    (233, -178.6, 1.91, 0.91, 0.24, 0.58),
)


def masks(x, z):
    x, z = np.broadcast_arrays(x, z)
    xs = x + 0.65 * (nz(x, z, 1.47, 11) - 0.5)
    zs = z + 0.77 * (nz(x, z, 1.88, 57) - 0.5)
    best = np.zeros_like(xs)
    shelter = np.zeros_like(xs)
    for cx, cz, a, b, theta, h in LOBES:
        u = (xs - cx) * math.cos(theta) + (zs - cz) * math.sin(theta)
        v = -(xs - cx) * math.sin(theta) + (zs - cz) * math.cos(theta)
        q = np.hypot(u / a, v / b)
        t = np.maximum(0, 1 - ease(0.18, 1.14, q + 0.19 * (nz(x, z, 0.55, 22) - 0.5)))
        best = np.maximum(best, t * h)
        shelter = np.maximum(shelter, (1 - ease(0.55, 1.82, q)) * h)
    return best, shelter


def make_model(kind, variant, middle):
    rng = np.random.default_rng(22107 + variant * 83 + (12 if kind == "sweep" else 39))
    g = Piece()
    g.middle = middle
    base = np.array([0.0, -0.018, 0.0])

    def leaf(p, d, width, size, shade, twist):
        lat = unit(np.cross(d, Y))
        axis = unit(d + Y * 0.14)
        if np.linalg.norm(lat) < 0.3:
            lat = np.array([1.0, 0, 0])
        rows = []
        cols = []
        for t in (0, 0.24, 0.54, 0.82, 1.0):
            center = (
                p
                + axis * size * t
                + Y * size * (0.21 * math.sin(math.pi * t) - 0.095 * t * t)
                + lat * size * twist * t * t
            )
            span = width * (0.017 + math.sin(math.pi * (0.025 + 0.965 * t)) ** 0.72)
            rows.append(
                [
                    center + lat * span * c + Y * span * (0.26 - 0.21 * abs(c))
                    for c in (-1, 0, 1)
                ]
            )
            cols.append([shade * v * (0.78 + 0.22 * t) for v in (0.93, 1.1, 0.98)])
        if middle:
            # Keep the real length, widest pair and ridge. At this range the
            # finer twists should not spend sixteen faces on a single leaflet.
            base = len(g.p)
            for i, k in [(0, 1), (2, 0), (2, 1), (2, 2), (4, 1)]:
                g.p.append(rows[i][k])
                g.c.append(cols[i][k])
            g.t.extend(
                (base + a, base + b, base + c)
                for a, b, c in ((0, 1, 2), (0, 2, 3), (1, 4, 2), (2, 4, 3))
            )
        else:
            grid(g, rows, cols)

    if kind == "sweep":
        # Low common woody base; wind-kneed first limbs branch to leaf tufts.
        # Holly-like wax cups appear only past each two-year stem's crook.
        n = int(rng.integers(6, 9))
        wind = np.array([0.72, 0, -0.29])
        for j in range(n):
            D = pol(variant * 0.77 + j * 2.39996 + rng.normal(0, 0.18))
            L = pol(variant * 0.77 + j * 2.39996 + 1.57)
            reach = rng.uniform(0.27, 0.65)
            height = rng.uniform(0.23, 0.53)
            push = wind * rng.uniform(0.18, 0.44)
            a = base + D * 0.032
            b = base + D * reach * 0.26 + Y * height * 0.56
            c = base + D * reach * 0.59 + push * 0.43 + Y * height * 0.88
            e = base + D * reach + push + Y * height * (1 + rng.uniform(-0.25, 0.1))
            tube(
                g,
                [a, b, c, e],
                [0.019, 0.013, 0.009, 0.004],
                np.array([106.0, 97.0, 74.0]),
                5,
            )
            for t in (0.48, 0.76, 1.0):
                root = c + (e - c) * t
                for sign in (-1, 1) if t < 1 else (1,):
                    Dr = unit(D * 0.52 + L * sign * 0.63 + wind * 0.37)
                    fork = (
                        root
                        + Dr * rng.uniform(0.086, 0.19)
                        + Y * rng.uniform(0.038, 0.092)
                    )
                    tube(
                        g,
                        [root, fork],
                        [0.0055, 0.0025],
                        np.array([126.0, 111.0, 80.0]),
                        4,
                    )
                    for k in range(5):
                        d = unit(
                            Dr * 0.38
                            + pol(2.39996 * k + j + variant) * 0.8
                            + Y * rng.uniform(-0.24, 0.62)
                        )
                        sz = rng.uniform(0.061, 0.111) * (1.14 if t > 0.8 else 1)
                        shade = (
                            np.array([109, 130, 96])
                            + rng.uniform(-7, 7)
                            + np.array([28, 19, 9]) * float(t)
                        )
                        leaf(
                            fork - Dr * 0.022,
                            d,
                            sz * rng.uniform(0.22, 0.34),
                            sz,
                            shade,
                            rng.uniform(-0.13, 0.13),
                        )
                    if (j + sign + variant) % 4 == 0:
                        leaf(
                            fork,
                            unit(Dr * 0.42 + Y * 0.82),
                            0.013,
                            0.048,
                            np.array([169.0, 136.0, 85.0]),
                            0.15,
                        )
    else:
        # Dusty woolly cushion. Separate radiating shoots, with earth showing
        # through on one side, not a smooth ellipsoid or ground-facing decal.
        for j in range(13):
            d = pol(j * 2.39996 + variant * 0.94 + rng.normal(0, 0.12))
            l = rng.uniform(0.19, 0.40)
            h = rng.uniform(0.06, 0.15)
            root = base + d * 0.048
            mid = root + d * l * 0.5 + Y * h
            end = root + d * l + Y * h * rng.uniform(0.34, 0.72)
            tube(
                g,
                [root, mid, end],
                [0.008, 0.005, 0.0023],
                np.array([118.0, 107.0, 78.0]),
                4,
            )
            for k in range(4):
                t = 0.14 + 0.23 * k
                p = mid + (end - mid) * t
                for side in [-1, 1]:
                    dr = unit(
                        0.51 * d
                        + pol(j * 2.39996 + variant * 0.94 + side * 1.03) * 0.7
                        + Y * (0.2 + 0.09 * k)
                    )
                    size = rng.uniform(0.055, 0.095)
                    col = (
                        np.array([138.0, 148.0, 114.0])
                        + np.array([16.0, 12.0, 4.0]) * t
                    )
                    leaf(p, dr, size * 0.30, size, col, 0.08 * side)
    return g


def seeded(native, rock):
    bed = Bed(native)
    rng = np.random.default_rng(71106)
    positions = []
    # Explicitly preserve existing real highland and earlier plant pockets.
    with (rock / "heath-roots.csv").open() as f:
        held = np.array([[float(r["x"]), float(r["z"])] for r in csv.DictReader(f)])
    ax = rng.uniform(BOX[0] + 0.8, BOX[1] - 0.8, 9700)
    az = rng.uniform(BOX[2] + 0.8, BOX[3] - 0.8, 9700)
    mass, shelter = masks(ax, az)
    high = bed.visible(ax, az, 0.48)
    n = rng.random(len(ax))
    for i in np.argsort(-(mass + 0.38 * n)):
        x, z = ax[i], az[i]
        v = mass[i]
        if v < 0.14 or high[i] < -0.29:
            continue
        if (
            bed.keep_out(x, z, 0.71)
            or ((held[:, 0] - x) ** 2 + (held[:, 1] - z) ** 2).min() < 0.54**2
        ):
            continue
        _, gx, gz = bed.floor(x, z)
        grade = np.hypot(gx, gz)
        if not 0.19 < grade < 0.95:
            continue
        onward = np.clip((-z - 150) / 36.0, 0, 1)
        kind = "sweep" if rng.random() < 0.23 + 0.51 * onward else "cushion"
        sc = (0.63 + 0.57 * onward) * rng.uniform(0.68, 1.2) * (0.78 + 0.22 * v)
        gap = (0.95 if kind == "sweep" else 0.62) * sc * (1.25 + 0.4 * n[i])
        if any(math.hypot(x - p[1], z - p[3]) < gap + p[5] for p in positions):
            continue
        if n[i] > 0.61 + 0.24 * v:
            continue
        h = float(bed.floor(x, z)[0])
        normal = unit(np.array([-gx * 0.69, 1.0, -gz * 0.69]))
        axis = np.cross(Y, normal)
        tilt = Rotation.from_rotvec(unit(axis) * math.acos(normal[1]))
        rot = (
            tilt * Rotation.from_rotvec([0.0, rng.normal(-0.06, 0.31), 0.0])
        ).as_quat()
        positions.append([kind + str(int(rng.integers(1, 4))), x, h, z, sc, gap, *rot])
    return positions


def emit_asset(pos, out, mid):
    names = sorted(set(r[0] for r in pos))
    obj = {k: make_model(k[:-1], int(k[-1]), mid) for k in names}
    dst = out / ("lee-middle.glb" if mid else "lee-near.glb")
    specs = export(obj, dst, "Wind-shaped spruce verge | willow and woolly cushion")
    doc, raw = parse(dst)
    emit = streams(doc, raw)
    drawn = 0
    for n in doc["nodes"]:
        k = n["name"]
        mesh = doc["meshes"][n["mesh"]]
        a = mesh["primitives"][0]["attributes"]["POSITION"]
        points = np.array(values(doc, raw, a), "<f4") + np.array(
            n.pop("translation"), "<f4"
        )
        set_values(doc, raw, a, points)
        doc["accessors"][a].update(
            min=points.min(0).tolist(), max=points.max(0).tolist()
        )
        pts = np.array([r[1:] for r in pos if k == r[0]], "<f4")
        scale = pts[:, 3]
        att = dict(
            TRANSLATION=emit(pts[:, :3], 5126, "VEC3", bounds=True),
            SCALE=emit(np.repeat(scale[:, None], 3, axis=1), 5126, "VEC3"),
            ROTATION=emit(pts[:, 5:], 5126, "VEC4"),
        )
        n["extensions"] = {"EXT_mesh_gpu_instancing": {"attributes": att}}
        drawn += len(pts) * len(obj[k].t)
    doc["extensionsUsed"] = doc["extensionsRequired"] = ["EXT_mesh_gpu_instancing"]
    doc["buffers"] = [dict(byteLength=len(raw))]
    save(doc, raw, dst)
    return dict(**specs, instances=len(pos), instanced_triangles=int(drawn))


def paint(out):
    w, h = 630, 918
    x0, x1, z0, z1 = BOX
    x, z = np.meshgrid(
        x0 + (np.arange(w) + 0.5) * (x1 - x0) / w,
        z0 + (np.arange(h) + 0.5) * (z1 - z0) / h,
    )
    mass, outer = masks(x, z)
    var = nz(x, z, 0.63, 14)
    grain = nz(x, z, 0.12, 28)
    moss = ease(0.51, 0.83, var + 0.15 * nz(x, z, 1.86, 37))
    soil = (
        np.array([103.0, 95.0, 71.0])
        + ((grain - 0.5) * 22 + 10 * (var - 0.5))[..., None]
    )
    surface = (
        soil * (1 - moss[..., None])
        + (np.array([136.0, 142.0, 98.0]) + 16 * (grain[..., None] - 0.5))
        * moss[..., None]
    )
    amt = (0.23 * outer + 0.42 * mass) * (0.68 + 0.32 * var)
    Image.fromarray(np.uint8(np.clip(np.dstack((surface, amt * 255)), 0, 255))).save(
        out / "lee-soil.png", optimize=True
    )


def author(native, rock, out):
    out.mkdir(exist_ok=True, parents=True)
    pos = seeded(native, rock)
    paint(out)
    with (out / "lee-roots.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            ("kind", "x", "y", "z", "scale", "keepout_radius_m", "qx", "qy", "qz", "qw")
        )
        for k, *v in pos:
            w.writerow([k, *[round(float(a), 6) for a in v]])
    n = emit_asset(pos, out, False)
    m = emit_asset(pos, out, True)
    (out / "lee.json").write_text(
        json.dumps(
            dict(
                name="Spruce Lee",
                bounds=BOX,
                native_hash=DIGEST,
                near=n,
                middle=m,
                roots=len(pos),
                unchanged_wilderness=True,
                original_tree_ids=[643, 1095, 0, 979],
                not_registered=True,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    author(*(Path(a) for a in sys.argv[1:4]))
