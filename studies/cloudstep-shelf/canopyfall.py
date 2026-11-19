"""A real native bough-driven forest floor, distinct from the exposed cliff pigment.
Sample original mapped spruce needle triangles/alpha; let their litter fall
to authentic V1 faces, slide locally down the actual grade, and leave all
previous native copse material geometry exactly in place. Authoring only.
"""

from pathlib import Path
import csv, json, math, sys
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, distance_transform_edt

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[1] / "scripts"), str(H.parent / "lanternback-rim")]
from finished_island_glb import parse, values
from island_inlet_ground import Inlet, ease
from strata import nz


def field(native, folder):
    proj = json.loads((folder / "held-copse.json").read_text())
    a, b, c, d = proj["box_xz"]
    keep = np.asarray(Image.open(folder / "held-copse-coverage.png")) > 0
    Hh, W = keep.shape
    im = np.zeros((Hh, W), "f")
    reg = json.loads((folder / "shelf-register.json").read_text())[
        "real_native_props_in_window"
    ]
    props = json.loads((native / "props.json").read_text())
    forms = json.loads((native / "grove-crowns.json").read_text())
    g, raw = parse(native / "grove-crowns.glb")
    nodes = {n["name"]: n for n in g["nodes"]}
    img = np.asarray(Image.open(native / g["images"][0]["uri"]).convert("RGBA"))
    hh, ww = img.shape[:2]
    mask = g["materials"][0]["alphaCutoff"]
    land = Inlet(native / "island-v1.bin")
    groups = {}
    accepted = []
    cones = []
    parents = []
    for q in reg:
        i = q["id"]
        p = props[i]
        if p["kind"] != "Conifer" or str(i) not in forms:
            continue
        name = "PineForm" + str(forms[str(i)]["form"]) + "_L0"
        if name not in groups:
            op = g["meshes"][nodes[name]["mesh"]]["primitives"][0]
            att = op["attributes"]
            tri = np.array(values(g, raw, op["indices"]), int).reshape(-1, 3)
            P = np.array(values(g, raw, att["POSITION"]))[tri]
            U = np.array(values(g, raw, att["TEXCOORD_0"]))[tri]
            weights = 0.5 * np.linalg.norm(
                np.cross(P[:, 1] - P[:, 0], P[:, 2] - P[:, 0]), axis=1
            )
            radial = np.hypot(P[:, :, 0].mean(1), P[:, :, 2].mean(1))
            h = P[:, :, 1].mean(1)
            weights *= (radial > 0.33) & (h > 1.0)
            weights /= weights.sum()
            groups[name] = (P, U, weights)
        P, U, weights = groups[name]
        rr = np.random.default_rng(472910 + i)
        cnt = 1800
        ii = rr.choice(len(P), size=cnt, p=weights)
        u = rr.random(cnt)
        v = rr.random(cnt)
        fold = u + v > 1
        u[fold] = 1 - u[fold]
        v[fold] = 1 - v[fold]
        bary = np.stack([1 - u - v, u, v], axis=1)
        pt = np.einsum("ni,nij->nj", bary, P[ii])
        uv = np.einsum("ni,nij->nj", bary, U[ii])
        rgba = img[
            ((uv[:, 1] % 1) * hh).astype(int), ((uv[:, 0] % 1) * ww).astype(int)
        ].astype(float)
        # A bark/solid-trunk triangle cannot shed a seed. The source's clipped
        # foliage texture, not the crown bounding circle, defines the fall field.
        ok = (
            (rgba[:, 3] >= 255 * mask)
            & (rgba[:, 1] > rgba[:, 0] * 1.06)
            & (rgba[:, 1] > rgba[:, 2] * 1.04)
        )
        local = pt[ok] * p["scale"]
        xx, yy, zz = p["position"]
        co, si = math.cos(p["yaw"]), math.sin(p["yaw"])
        X = xx + local[:, 0] * co + local[:, 2] * si
        Z = zz - local[:, 0] * si + local[:, 2] * co
        _, dx, dz = land.floor(X, Z)
        grade = np.hypot(dx, dz)
        drift = np.minimum(0.67, 0.29 * grade) * rr.uniform(0.15, 1.0, len(X))
        X = X - drift * dx / np.maximum(grade, 1.0e-6)
        Z = Z - drift * dz / np.maximum(grade, 1.0e-6)
        inside = (X > a) & (X < b) & (Z > c) & (Z < d)
        X = X[inside]
        Z = Z[inside]
        ix = np.minimum(W - 1, ((X - a) / (b - a) * W).astype(int))
        iz = np.minimum(Hh - 1, ((Z - c) / (d - c) * Hh).astype(int))
        np.add.at(im, (iz, ix), 1)
        parents.append(i)
        accepted.append(len(X))
        # A few hand-sized spent cones, not one at every deposited sample.
        clean = np.flatnonzero(~keep[iz, ix])
        if len(clean):
            rr.shuffle(clean)
            added = 0
            for k in clean:
                if added >= int(3 + p["scale"] * 1.5):
                    break
                x, z = X[k], Z[k]
                if any(math.hypot(x - q0[1], z - q0[3]) < 0.47 for q0 in cones):
                    continue
                y, gx, gz = map(float, land.floor(x, z))
                cones.append(
                    [
                        i,
                        x,
                        y,
                        z,
                        rr.uniform(0.047, 0.083),
                        rr.uniform(-math.pi, math.pi),
                        gx,
                        gz,
                    ]
                )
                added += 1
    major = gaussian_filter(im, 4.70)
    minor = gaussian_filter(im, 1.55)
    xx, zz = np.meshgrid(
        a + (np.arange(W) + 0.5) * (b - a) / W, c + (np.arange(Hh) + 0.5) * (d - c) / Hh
    )
    mid = nz(xx, zz, 0.97, 906) - 0.5
    grain = nz(xx, zz, 0.268, 922) - 0.5
    cover = np.clip(
        0.56 * ease(0.19, 0.93, major) + 0.40 * ease(0.22, 1.9, minor), 0, 1
    )
    cover *= ease(0, 0.85, np.minimum(xx - a, b - xx)) * ease(
        0, 0.85, np.minimum(zz - c, d - zz)
    )
    # Preserve the nearby native floor: true geometry stays; glaze feathers
    # away beside its registered low polygon envelopes and into open soil.
    away = distance_transform_edt(~keep) * ((b - a) / W)
    cover *= 0.22 + 0.78 * ease(0.02, 0.48, away)
    gain = np.stack(
        [
            1 + cover * (0.065 + mid * 0.07 + grain * 0.06),
            1 - cover * (0.22 - 0.08 * mid - 0.06 * grain),
            1 - cover * (0.31 - 0.11 * mid - 0.07 * grain),
        ],
        axis=-1,
    )
    Image.fromarray(np.uint8(np.clip(np.rint(gain * 128), 0, 255))).save(
        folder / "forest-shed-gain.png", optimize=True
    )
    with (folder / "spent-cones.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(("source_pine", "x", "native_y", "z", "length_m", "yaw", "dx", "dz"))
        w.writerows(
            [[int(row[0]), *[round(float(v), 6) for v in row[1:]]] for row in cones]
        )
    note = dict(
        original_tree_ids=parents,
        original_opaque_green_bough_samples=sum(accepted),
        true_pine_count=len(parents),
        proposed_cone_seats=len(cones),
        minimum_old_copse_avoidance_m=0.2,
        chart_metres_per_texel=(b - a) / W,
        footprint_coverage_percent=round(
            100 * float((cover > 0.14).mean()),
            2,
        ),
        stock_props_moved=0,
        optical_skin_only=True,
        no_object_or_physics_added=True,
    )
    (folder / "forest-shed.json").write_text(json.dumps(note, indent=2) + "\n")
    return note


if __name__ == "__main__":
    print(field(*(Path(p) for p in sys.argv[1:])))
