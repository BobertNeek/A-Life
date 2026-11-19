"""Cloudstep shoulder study: three real Cliff placements, not new landmarks.
Retain each original pivot, yaw, scale, oriented footprint and highest rock.
Seat the *visible* stone on the actual incline instead of hanging 10 m up.
This artist source is not registered as game geometry, proxies or navigation.
"""

import json, math, sys
from pathlib import Path
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[1] / "scripts"), str(H.parent / "lanternback-rim")]
from island_inlet_ground import Inlet, ease
from finished_island_glb import save
from island_erratic_glb import streams
from strata import nz

IDS = (1335, 1695, 1847)
AREA = (-40.0, 105.0, -453.0, -324.0)


def local(x, z, q):
    ca, sa = np.cos(q["yaw"]), np.sin(q["yaw"])
    sc = q["scale"]
    return q["position"][0] + sc * (ca * x + sa * z), q["position"][2] + sc * (
        -sa * x + ca * z
    )


def angular(x, z, sx, sz, ang, x0, z0):
    x = x - x0
    z = z - z0
    c, s = np.cos(ang), np.sin(ang)
    a = (c * x + s * z) / sx
    b = (-s * x + c * z) / sz
    return np.maximum.reduce(
        (abs(a), abs(b), 0.66 * abs(a + 0.86 * b), 0.71 * abs(a - 0.96 * b))
    )


def shoulder(land, q, idx, detail):
    # All body vertices stay inside the original transformed 8 x 4 m envelope.
    # The back/upslope ledge reaches the genuine old 12 m top, not a new spire.
    rng = np.random.default_rng(9751 + idx)
    N, M = (27, 17) if detail else (15, 9)
    x, z = np.meshgrid(np.linspace(-4, 4, N), np.linspace(-2, 2, M))
    x[1:-1, 1:-1] += rng.uniform(-0.12, 0.12, (M - 2, N - 2))
    z[1:-1, 1:-1] += rng.uniform(-0.085, 0.085, (M - 2, N - 2))
    X, Z = local(x, z, q)
    y, gx, gz = land.floor(X, Z)
    s = q["scale"]
    base = q["position"][1]
    ca, sa = np.cos(q["yaw"]), np.sin(q["yaw"])
    u, v = (np.mean(gx) * ca - np.mean(gz) * sa, np.mean(gx) * sa + np.mean(gz) * ca)
    d = np.hypot(u, v)
    u, v = u / max(0.001, d), v / max(0.001, d)
    k = IDS.index(idx)
    direction = [0.19, -0.22, 0.31][k]
    # One broad asymmetric upthrust, intersected by two lower and differently
    # truncated sister beds. No replicated finger cylinders or palisade tops.
    xx = x * u + z * v
    lead = np.clip(1.5 * u, -2.2, 2.2)
    cross = 0.63 * v
    a = angular(x, z, 2.46, 1.13, direction, lead, cross)
    b = angular(x, z, 2.17, 0.85, direction - 0.21, -1.28 * u - 0.28, -0.54 * v + 0.20)
    c = angular(x, z, 1.46, 0.63, direction + 0.18, 2.1 * np.sign(u or 1), -0.68 * v)
    p = np.maximum.reduce(
        (
            0.98 - 0.105 * a - 0.96 * np.maximum(0, a - 0.44),
            0.63 - 0.17 * b - 0.81 * np.maximum(0, b - 0.61),
            0.47 - 0.13 * c - 0.86 * np.maximum(0, c - 0.52),
        )
    )
    edge = ease(0, 0.53, np.minimum(4 - abs(x), 2 - abs(z)))
    dry = (nz(X, Z, 0.75, 191 + idx) - 0.5) * 0.075 + (
        nz(X, Z, 2.3, 229 + idx) - 0.5
    ) * 0.085
    p = np.maximum(0, p + dry) * edge
    # There is no arbitrary level base. Every perimeter sample underlaps its
    # exact native triangle by 0.12 m. Positive mesh height is visual only.
    low = (y - base) / s - 0.12 / s
    target = 12.0
    # Absolute sloping benches keep the old hard upper bound without
    # scaling a radial tent on top of a hill (the first concept made teeth).
    # Each site has its own master block, lower buttress and broken side.
    mainx, mainz = [(0.10, 0.24), (0.12, -0.05), (-0.34, 0.23)][k]
    A = angular(
        x, z, [3.10, 3.26, 2.55][k], [1.30, 1.24, 1.31][k], direction, mainx, mainz
    )
    B = angular(
        x,
        z,
        [2.25, 2.01, 2.72][k],
        [0.77, 0.92, 0.83][k],
        direction + 0.15,
        -1.22,
        0.67,
    )
    D = angular(x, z, [1.84, 1.54, 1.40][k], 0.77, direction - 0.13, 1.69, -0.77)
    # 'ease' is upward only, so describe falling side profiles explicitly.
    main = 1 - ease(0.55, 1.17, A)
    neck = 0.83 * (1 - ease(0.42, 1.10, B))
    spur = 0.69 * (1 - ease(0.49, 1.15, D))
    geoline = (
        x * 0.53
        + z * 0.44
        + (nz(X, Z, 1.60, 311 + idx) - 0.5) * 0.48
        + [0.76, -0.22, 0.19][k]
    )
    fault = 1 - ease(0.15, 0.46, abs(geoline))
    plateau = np.minimum(11.87, 10.45 + 0.20 * x * u + 0.32 * z * v + dry * 3.2)
    plateau += [0.1, 0.00, 0.17][k] - 1.3 * fault
    bed = np.maximum.reduce(
        (
            low + (plateau - low) * main,
            low + (6.1 + 0.34 * x + dry * 2.3 - low) * neck,
            low + (5.3 - 0.37 * x + 0.23 * z + dry * 2.8 - low) * spur,
        )
    )
    top = low + np.maximum(0, bed - low) * edge
    # Lift the whole genuine high bench to the old protected ceiling, not
    # just one vertex; bare outer contacts stay 12 cm below native ground.
    raised = np.maximum(0, top - low)
    mult = np.min((target - low)[raised > 0.01] / raised[raised > 0.01])
    top = low + mult * raised
    seam = geoline
    C = np.stack((x, top, z), axis=-1).reshape(-1, 3).astype("f")
    ii = []
    for j in range(M - 1):
        for i in range(N - 1):
            a = j * N + i
            if (i + j) % 2:
                ii.extend(((a, a + N, a + 1), (a + 1, a + N, a + N + 1)))
            else:
                ii.extend(((a, a + N + 1, a + 1), (a, a + N, a + N + 1)))
    ii = np.array(ii, "<u2")
    n = np.zeros_like(C)
    nf = np.cross(C[ii[:, 1]] - C[ii[:, 0]], C[ii[:, 2]] - C[ii[:, 0]])
    for j in range(3):
        np.add.at(n, ii[:, j], nf)
    n /= np.maximum(1e-8, np.linalg.norm(n, axis=1, keepdims=True))
    worldY = base + s * top
    grade = (1 - n[:, 1]).reshape(x.shape)
    fleck = nz(X, Z, 0.72, 990) - 0.5
    block = nz(X, Z, 2.5, 171) - 0.5
    pale = ease(0.14, 0.65, p) * (0.49 + 0.34 * nz(X, Z, 2.3, 131)) * (1 - 0.55 * grade)
    rust = (
        (1 - ease(0.07, 0.46, abs(seam - 0.33)))
        * (0.34 + 0.50 * nz(X, Z, 1.2, 672))
        * edge
    )
    dark = 1 - ease(0.08, 0.29, abs(seam))
    # Petrological structure passes diagonally through the thick broken
    # face. The lower foot darkens locally, not a repeated painterly stripe.
    beds = top * 0.55 + x * 0.77 - z * 0.19 + block * 0.67
    seamlight = (1 - ease(0.17, 0.84, abs(beds - [3.25, 3.90, 2.48][k]))) * (
        0.31 + 0.56 * nz(X, Z, 1.69, 751)
    )
    seamw = (1 - ease(0.08, 0.51, abs(beds - [1.44, 6.24, 5.85][k]))) * (
        0.30 + 0.48 * nz(X, Z, 1.23, 777)
    )
    rind = ease(0.09, 0.38, top - low) * (1 - ease(2.4, 5.4, top - low))
    strata = (
        block * 0.21 + fleck * 0.12 + 0.13 * seamlight - 0.060 * seamw - 0.055 * rind
    )
    color = np.stack(
        (
            0.225 + 0.15 * pale + 0.15 * rust + strata,
            0.212 + 0.15 * pale + 0.052 * rust + strata,
            0.189 + 0.135 * pale - 0.025 * rust + strata,
        ),
        -1,
    )
    color *= 1 - (0.20 * dark * edge)[..., None]
    mat = np.clip(color, 0.045, 0.55).reshape(-1, 3).astype("f")
    ring = np.zeros((M, N), bool)
    ring[[0, -1]] = True
    ring[:, [0, -1]] = True
    return (
        C,
        n,
        mat,
        ii,
        dict(
            seats=int(ring.sum()),
            lowest_perimeter_relative_to_v1_m=round(float((worldY - y)[ring].min()), 5),
            highest_perimeter_relative_to_v1_m=round(
                float((worldY - y)[ring].max()), 5
            ),
            world_y_max=round(float(worldY.max()), 4),
            same_original_y_max=round(base + 12 * s, 4),
            native_oriented_bound_m=[8 * s, 4 * s],
        ),
    )


def fans(land, props, folder):
    a, b, c, d = AREA
    hh, ww = 486, 544
    X, Z = np.meshgrid(
        a + (np.arange(ww) + 0.5) * (b - a) / ww,
        c + (np.arange(hh) + 0.5) * (d - c) / hh,
    )
    Y, gx, gz = land.floor(X, Z)
    rng = np.random.default_rng(5713)
    body = np.zeros_like(X)
    iron = body.copy()
    steps = {}
    for i in IDS:
        q = props[i]
        x0, _, z0 = q["position"]
        _, u, v = land.floor(x0, z0)
        grad = np.hypot(u, v)
        u, v = -u / grad, -v / grad
        path = []
        ids = []
        for arm, off in enumerate((-0.92, 0.58, 1.63)):
            xp = x0 + v * off
            zp = z0 - u * off
            length = [32, 38, 24][arm] * [1.18, 1.00, 0.87][IDS.index(i)]
            pts = []
            for k in range(int(length / 0.82)):
                t = k * 0.82
                yp, dx, dz = land.floor(xp, zp)
                g = np.hypot(dx, dz)
                if k > 7 and g < 0.23:
                    break
                pts.append([float(xp), float(zp)])
                if g < 0.06:
                    break
                bend = 0.20 * np.sin(t * 0.14 + arm) * np.sin(
                    min(1, t / 8) * math.pi * 0.5
                ) + (0.14 * off * np.exp(-t / 13))
                dx, dz = dx / g, dz / g
                xp -= 0.82 * (dx - dz * bend)
                zp -= 0.82 * (dz + dx * bend)
            pts = np.array(pts)
            dist = np.full_like(X, 1e6)
            along = np.zeros_like(X)
            for k, (ax, az) in enumerate(pts):
                ds = (X - ax) ** 2 + (Z - az) ** 2
                take = ds < dist
                dist[take] = ds[take]
                along[take] = k * 0.82
            width = 1.05 + 0.08 * along + 0.62 * (nz(X, Z, 3.8, i + arm) - 0.5)
            extent = (1 - ease(length * 0.57, length + 0.5, along)) * ease(
                -2.0, 1.3, along + 1.3
            )
            # World-coordinate variation splits the fan into coarse broken
            # planes and intermittent tails, not several parallel paint lines.
            p = (1 - ease(width * 0.12, width * 2.35, np.sqrt(dist))) * extent
            if arm == 0:
                path = pts.round(3).tolist()
            body = np.maximum(body, p)
            iron = np.maximum(
                iron,
                p
                * (0.46 + 0.45 * (nz(X, Z, 5.3, 413 + arm) - 0.5))
                * (1 - ease(0.35, 1.0, along / length)),
            )
        steps[str(i)] = path
    mask = ease(0.3, 0.82, np.hypot(gx, gz))
    a0 = nz(X, Z, 3.1, 999) - 0.5
    f = nz(X, Z, 0.66, 902) - 0.5
    fractured = ease(0.12, 0.68, 0.41 + 0.83 * a0 + 0.42 * f)
    rock = body * (0.35 + 0.65 * fractured) * mask
    red = iron * mask
    g = np.stack(
        [
            1 + 0.68 * rock + 0.20 * red,
            1 + 0.48 * rock + 0.04 * red,
            1 + 0.30 * rock - 0.14 * red,
        ],
        -1,
    )
    Image.fromarray(np.uint8(np.clip(np.rint(g * 128), 0, 255))).save(
        folder / "shoulder-talus.png", optimize=True
    )
    return {
        "native_plan_xz": AREA,
        "main_fall_lines_xz": steps,
        "source": "Downslope against original V1 triangle gradients at three real cliff placements; pigment only beyond original envelopes.",
    }


def main(native, folder):
    land = Inlet(native / "island-v1.bin")
    props = json.loads((native / "props.json").read_text())
    notes = {}
    for lod in (0, 1):
        doc = dict(
            asset=dict(
                version="2.0", generator="Cloudstep / three registered parent outcrops"
            ),
            scene=0,
            scenes=[dict(nodes=[])],
            nodes=[],
            meshes=[],
            accessors=[],
            bufferViews=[],
            materials=[
                dict(
                    name="Warm gneiss crowns and cool footstone",
                    doubleSided=True,
                    pbrMetallicRoughness=dict(metallicFactor=0.0, roughnessFactor=0.86),
                )
            ],
        )
        doc["images"] = [dict(uri="jointed-rock-face.png")]
        doc["textures"] = [dict(source=0, sampler=0)]
        doc["samplers"] = [
            dict(magFilter=9729, minFilter=9987, wrapS=10497, wrapT=10497)
        ]
        doc["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] = dict(index=0)
        buf = bytearray()
        emit = streams(doc, buf)
        for index in IDS:
            q = props[index]
            v, n, c, ii, r = shoulder(land, q, index, lod == 0)
            yaw = q["yaw"]
            # Long standing faces need real vertical material coordinates:
            # xz-only colour smears the stone into featureless fins. Bake a
            # discontinuity-aware rock-face atlas onto the actual triangles.
            fc = np.cross(v[ii[:, 1]] - v[ii[:, 0]], v[ii[:, 2]] - v[ii[:, 0]])
            fc /= np.maximum(1.0e-8, np.linalg.norm(fc, axis=1, keepdims=True))
            group = np.argmax(np.abs(fc), axis=1)
            ids, inv = np.unique(
                np.column_stack((ii.ravel(), np.repeat(group, 3))),
                axis=0,
                return_inverse=True,
            )
            old = ids[:, 0]
            axes = ids[:, 1]
            vv = v[old]
            # Repeat on an 8m face with variable bractstone/bright granite.
            U = np.where(axes == 0, vv[:, 2], vv[:, 0])
            V = np.where(axes == 1, vv[:, 2], -vv[:, 1])
            uu = np.column_stack(((U + 0.32 * V) / 8.9, (V - 0.10 * U) / 8.9)).astype(
                "<f4"
            )
            v, n, c = vv, n[old], c[old]
            from mountainskin import field

            xx, zz = local(v[:, 0], v[:, 2], q)
            native_granite = field(land, xx, zz)[0]
            # A standing face belongs to the same gneiss as the region but is
            # not coated in downhill fan deposit or needlefall. Bake its
            # mineral family once; keep the other optical layers ground-only.
            c = np.clip(
                c
                * np.array([1.52, 1.46, 1.40], "f")
                * (1 + 0.48 * (native_granite - 1)),
                0,
                1,
            ).astype("<f4")
            ii = inv.reshape(-1, 3).astype("<u2")
            att = dict(
                POSITION=emit(v, 5126, "VEC3", bounds=True),
                NORMAL=emit(n, 5126, "VEC3"),
                COLOR_0=emit(c, 5126, "VEC3"),
                TEXCOORD_0=emit(uu, 5126, "VEC2"),
            )
            j = len(doc["nodes"])
            doc["nodes"].append(
                dict(
                    name="Shoulder_" + str(index),
                    mesh=j,
                    translation=q["position"],
                    rotation=[0, math.sin(yaw / 2), 0, math.cos(yaw / 2)],
                    scale=[q["scale"]] * 3,
                    extras=dict(original_prop_index=index, visual_only=True),
                )
            )
            doc["meshes"].append(
                dict(
                    primitives=[
                        dict(
                            material=0,
                            attributes=att,
                            indices=emit(ii.ravel(), 5123, "SCALAR", indices=True),
                        )
                    ]
                )
            )
            doc["scenes"][0]["nodes"].append(j)
            notes.setdefault(str(index), dict(original_transform=q, levels={}))
            notes[str(index)]["levels"][str(lod)] = dict(**r, triangles=len(ii))
        doc["buffers"] = [dict(byteLength=len(buf))]
        save(doc, buf, folder / ("slope-shoulders-L" + str(lod) + ".glb"))
    from mountainskin import plates

    # Seam-compatible texture: use translated repeats in structural lattice
    # for the few exposed square metres, with disturbed scale and dark clefts.
    z, x = np.mgrid[0:256, 0:256]
    x = (x + 0.5) / 256
    z = (z + 0.5) / 256

    def art(u, v):
        U = 4 * u + 0.80 * v
        V = 6 * v - 1.75 * u
        q = nz(U, V, 0.83, 643) - 0.5
        t = nz(U, V, 0.33, 812) - 0.5
        f = nz(U, V, 0.081, 92) - 0.5
        joint, cell = plates(U * 0.90 + 0.2 * q, V * 0.87 + 0.1 * t)
        fiss = (1 - ease(0.042, 0.17, joint)) * (
            0.35 + 0.65 * ease(0.1, 0.65, q + 0.45 + t * 0.65)
        )
        quartz = (1 - ease(0.045, 0.19, abs(0.31 * U + 0.71 * V - 2.5 + 0.29 * q))) * (
            0.37 + 0.51 * ease(0.1, 0.65, 0.49 + t)
        )
        mineral = (
            0.65
            + 0.22 * cell
            + 0.17 * q
            + 0.081 * t
            + 0.052 * f
            - 0.34 * fiss
            + 0.23 * quartz
        )
        return np.stack(
            (
                mineral + 0.05 * quartz + 0.039 * q,
                mineral + 0.007 * quartz,
                mineral - 0.032 * quartz - 0.025 * q,
            ),
            -1,
        )

    # Edge-to-edge value/slope continuous quilting, not a repeated hard seam.
    w = ease(0.15, 0.88, x)[..., None]
    t = ease(0.15, 0.88, z)[..., None]
    image = (1 - t) * ((1 - w) * art(x, z) + w * art(x - 1, z)) + t * (
        (1 - w) * art(x, z - 1) + w * art(x - 1, z - 1)
    )
    pix = np.uint8(
        255 * np.clip(1.055 * np.maximum(0.0, image) ** (1 / 2.4) - 0.055, 0, 1)
    )
    Image.fromarray(pix).save(folder / "jointed-rock-face.png", optimize=True)
    note = dict(
        placements=notes,
        talus=fans(land, props, folder),
        no_new_colliders=True,
        runtime_registered=False,
    )
    (folder / "shoulders.json").write_text(json.dumps(note, indent=2) + "\n")


if __name__ == "__main__":
    main(*(Path(p) for p in sys.argv[1:]))
