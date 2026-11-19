"""Carve one native cliff form inside its actual old three-dimensional support.

The older Cliff was a bouquet of narrow sharpened posts, most visible in
silhouette and at creature scale. Do not change the sixteen planted root
transforms, any obstacle, or the actual 12 m height. This artist starts from
the native rock itself. Its existing support directions constrain the form.
"""

from pathlib import Path
import math, json
import numpy as np
from scipy.spatial import ConvexHull, cKDTree
from finished_island_glb import parse, values, save


def unit(a):
    return a / np.maximum(1e-12, np.linalg.norm(a, axis=-1, keepdims=True))


def ease(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def draft(original, triangles, density):
    old = np.unique(original, axis=0)
    hull = ConvexHull(old)
    support = old[hull.vertices]
    if density < 70:
        # Overview needs the outline, not the old pencil cap interior points.
        sole = old[old[:, 1] < 0.01]
        support = np.unique(
            np.r_[
                old[ConvexHull(old[:, [0, 2]]).vertices],
                sole[ConvexHull(sole[:, [0, 2]]).vertices],
                old[[np.argmax(old[:, 1])]],
                support[
                    cKDTree(support).query(
                        np.array(
                            [[-2, 10, 1], [1, 10, 1], [2.5, 9, -1], [-0.5, 11, 1.3]]
                        )
                    )[1]
                ],
            ],
            axis=0,
        )
    # Work on an anisotropic sphere with the true aspect ratio, otherwise
    # a thin vertical escarpment receives too few samples on its broad face.
    origin = np.array([0, 5.75, 0.0])
    scale = np.array([4.1, 6.1, 2.07])
    N = hull.equations[:, :3]
    offset = hull.equations[:, 3]
    start = -(N @ origin + offset)
    dirs = unit((support - origin) / scale)
    # A Fibonacci globe, not fixed vertical ribs or horizontal contour rings.
    j = np.arange(density, dtype=float) + 0.5
    yy = 1 - 2 * j / density
    q = np.sqrt(1 - yy * yy)
    golden = math.pi * (3 - math.sqrt(5))
    rays = np.r_[
        dirs, np.stack([np.cos(j * golden) * q, yy, np.sin(j * golden) * q], -1)
    ]
    # Sample inclined bedding on both lips of a parting. The ribs are
    # localized in the sculpt below; not every plane goes around the rock.
    if density > 70:
        seam = []
        count = 24 if density > 400 else 10
        for lev in (2.4, 4.85, 7.25, 9.05):
            for side in (-0.32, 0, 0.32):
                roots = np.array([0, lev + side, 0])
                angles = np.arange(count) * (math.tau / count) + 0.37 * lev
                dx, dz = np.cos(angles), np.sin(angles)
                v = np.stack((dx, 0.34 * dx - 0.17 * dz, dz), -1)
                den = v @ N.T
                radius = np.min(
                    np.where(
                        den > 1e-8,
                        -(N @ roots + offset) / np.maximum(den, 1e-9),
                        np.inf,
                    ),
                    axis=1,
                )
                seam.extend(unit((roots + radius[:, None] * v - origin) / scale))
        rays = np.r_[rays, np.asarray(seam)]
    simp = ConvexHull(rays).simplices.copy()
    axis = rays * scale
    den = axis @ N.T
    t = np.min(np.where(den > 1e-9, start / np.maximum(den, 1e-10), np.inf), axis=1)
    points = origin + axis * t[:, None]
    points[: len(support)] = support
    x, y, z = points.T
    u = y - 0.34 * x + 0.17 * z

    # Occasional inclined partings die within different faces. Avoid the
    # original stack of regularly spaced rings (or a repeated wall of teeth).
    def bed(level, run, where, depth):
        band = np.exp(-(((u - level) / run) ** 2))
        return depth * band * where

    crest = ease(0.5, 2.6, np.abs(z + 0.27 * x))
    west = 1 - ease(-2.1, 1.9, x + 0.48 * z)
    east = ease(-1.2, 2.8, x - 0.45 * z)
    sway = np.sin(0.76 * y + 0.42 * x - 0.72 * z)
    lines = (
        bed(2.4, 0.29, west, 0.48)
        + bed(4.85, 0.30, east, 0.53)
        + bed(7.25, 0.38, ease(-0.9, 0.9, z + 0.21 * x), 0.43)
        + bed(9.05, 0.23, west * 0.88, 0.35)
    )
    # The central break is a shallow weathered step crossed by a second
    # thinner joint, not a tunnel where a collider could catch an animal.
    joint = (
        np.exp(-(((x + 0.11 * y - 0.53 * z - 0.48) / 0.49) ** 2))
        * ease(1.5, 3.3, y)
        * (1 - ease(6.8, 9.2, y))
    )
    road = np.exp(-(((z - 0.055 * y - 0.37 * x + 0.43) / 0.36) ** 2)) * ease(
        6.7, 8.0, y
    )
    carving = 0.038 + 0.032 * np.abs(sway) + lines + 0.49 * joint + 0.21 * road
    amplitude = 0.54 + 0.40 * ease(0.4, 2.7, np.abs(x) / 2 + np.abs(z))
    # The game baked yawed local-box corners. Preserve an even stronger
    # constraint: the actual projected footprint and true summit. A cap need
    # not preserve the heights of 26 former *internal* pencil tips. Keep
    # the original rotated footprint for any yaw and the genuine high point.
    lateral = old[ConvexHull(old[:, [0, 2]]).vertices]
    pinned = np.r_[lateral, old[[np.argmax(old[:, 1])]]]
    distance = cKDTree(pinned).query(points)[0]
    allowed = ease(0.035, 0.79, distance)
    carving *= amplitude * allowed * (0.30 + 0.7 * ease(0.17, 1.35, y))
    carved = points - carving[:, None] * unit(points - origin)
    # Cut two distinct bedrock shoulders inside the convex outer support.
    # Rays toward the old interior never project a new overhanging obstacle.
    axis = carved - origin
    for _ in range(6):
        xx, yy, zz = carved.T
        face = 11.96 - 0.16 * np.abs(xx - 0.26) - 0.065 * (zz + 1.05)
        cap = (
            face
            - 2.78 * ease(0.80, 1.22, xx - 0.24 * zz)
            - 1.35 * (1 - ease(-1.92, -1.45, xx + 0.26 * zz))
        )
        drop = np.maximum(0, yy - cap) * allowed
        carved -= (
            np.minimum(0.55, drop / np.maximum(axis[:, 1], 1e-2) * 0.72)[:, None] * axis
        )
    # Near the ground settle the whole fused upper scarp onto, and within,
    # the convex rim of the true sole. Keep the original foot triangles as
    # the only pieces actually touching the plane; no row of exposed stilts.
    feet = old[old[:, 1] < 0.01][:, [0, 2]]
    fp = ConvexHull(feet)
    hn = fp.equations[:, :2]
    off = fp.equations[:, 2]
    base = feet.mean(0)
    delta = carved[:, [0, 2]] - base
    den = delta @ hn.T
    ratio = np.minimum(
        1,
        np.min(
            np.where(den > 1e-9, -(hn @ base + off) / np.maximum(den, 1e-10), np.inf),
            axis=1,
        ),
    )
    low = 1 - ease(0.12, 0.83, y)
    carved[:, [0, 2]] = base + delta * (1 - low * (1 - ratio + 0.012))[:, None]
    carved[:, 1] = np.maximum(0.017, carved[:, 1])
    exact = cKDTree(pinned).query(support)[0] < 1e-7
    if density < 400:
        # Keep the touching original outer toe, not 200 hidden interior
        # triangles of individual columns reenacted at overview distance.
        exact |= support[:, 1] < 0.01
    carved[: len(support)][exact] = support[exact]
    # Face winding, rounded by local angle only when writing normals below.
    fn = np.cross(
        carved[simp[:, 1]] - carved[simp[:, 0]], carved[simp[:, 2]] - carved[simp[:, 0]]
    )
    wrong = np.einsum("ij,ij->i", fn, carved[simp].mean(1) - origin) < 0
    simp[wrong] = simp[wrong][:, [0, 2, 1]]
    return carved, simp, support


def clipped_foot(position, tri, top=0.75):
    verts = []
    faces = []
    for ids in tri:
        polygon = list(position[ids])
        clip = []
        for a, b in zip(polygon, polygon[1:] + polygon[:1]):
            inside = a[1] <= top
            outside = b[1] > top
            if inside:
                clip.append(a)
            if inside == outside:
                k = (top - a[1]) / (b[1] - a[1])
                clip.append(a + k * (b - a))
        if len(clip) < 3:
            continue
        i = len(verts)
        verts.extend(clip)
        for k in range(1, len(clip) - 1):
            faces.append((i, i + k, i + k + 1))
    return np.asarray(verts), np.asarray(faces)


def channels(p, tri, pigment_tree, pigment_source):
    pts = p[tri]
    fn = unit(np.cross(pts[:, 1] - pts[:, 0], pts[:, 2] - pts[:, 0]))
    # Shade real face planes. Entirely smooth vertex normals would make a
    # large wet potato and every triangle flat would make a low-poly boulder.
    incident = [[] for _ in range(len(p))]
    for i, t in enumerate(tri):
        for v in t:
            incident[v].append(i)
    outn = np.zeros_like(pts)
    for i, t in enumerate(tri):
        for j, v in enumerate(t):
            near = fn[incident[v]]
            w = ease(0.60, 0.91, near @ fn[i])
            outn[i, j] = unit(0.34 * fn[i] + 0.66 * unit((near * w[:, None]).sum(0)))
    # Scale world-length projection independently on each major face; UV
    # seams follow natural changes of plane rather than the long pencils.
    axis = np.argmax(np.abs(fn), axis=1)
    uv = np.empty((len(tri), 3, 2))
    for i, t in enumerate(pts):
        a = axis[i]
        uv[i] = t[:, ([2, 1] if a == 0 else [0, 2] if a == 1 else [0, 1])] * 0.74
    # Subdued, wide weathering by face, no dark painted outlines in the cap.
    x, y, z = pts.transpose(2, 0, 1)
    band = y - 0.34 * x + 0.17 * z
    oxid = np.clip(
        0.30
        + 0.16 * np.sin(0.77 * band + 0.25 * z)
        + 0.11 * np.cos(0.82 * x + 0.30 * z),
        0,
        1,
    )
    wet = (1 - ease(0.10, 2.2, y)) * 0.27
    c = np.empty((*y.shape, 4))
    # Actual old mineral darks, including the cool slate and iron rind; the
    # shared fracture map was authored against these specific linear factors.
    dist, near = pigment_tree.query(pts.reshape(-1, 3), k=10)
    weights = 1 / (0.17 + dist) ** 2
    sampled = (
        (pigment_source[near, :3] * weights[..., None]).sum(1) / weights.sum(1)[:, None]
    ).reshape(*y.shape, 3)
    c[..., :3] = np.clip(
        sampled
        * np.stack(
            (
                0.96 + 0.12 * oxid - 0.13 * wet,
                1.00 + 0.03 * oxid - 0.10 * wet,
                1.025 - 0.06 * oxid - 0.07 * wet,
            ),
            -1,
        ),
        0,
        1,
    )
    c[..., 3] = 1
    return (
        outn.reshape(-1, 3),
        uv.reshape(-1, 2),
        np.uint8(np.rint(255 * c.reshape(-1, 4))),
    )


def sculpt(source, dest):
    d, original = parse(Path(source))
    prod = dict(d)
    prod.update(accessors=[], bufferViews=[], meshes=[])
    raw = bytearray()

    def att(arr, fmt, kind, target=34962, norm=False, bounds=False):
        a = np.ascontiguousarray(arr)
        raw.extend(b"\0" * (-len(raw) % 4))
        off = len(raw)
        raw.extend(a.tobytes())
        vi = len(prod["bufferViews"])
        prod["bufferViews"].append(
            dict(buffer=0, byteOffset=off, byteLength=a.nbytes, target=target)
        )
        ac = dict(bufferView=vi, componentType=fmt, count=len(a), type=kind)
        if norm:
            ac["normalized"] = True
        if bounds:
            ac.update(min=a.min(axis=0).tolist(), max=a.max(axis=0).tolist())
        ix = len(prod["accessors"])
        prod["accessors"].append(ac)
        return ix

    rows = {}
    # Lower levels keep the same outer three-dimensional native L0 support,
    # not the old simplified L2 which actually expanded past the L0 rim.
    original_prim = d["meshes"][0]["primitives"][0]
    pos = np.asarray(values(d, original, original_prim["attributes"]["POSITION"]))
    f = np.asarray(values(d, original, original_prim["indices"])).reshape(-1, 3)
    pigment_source = np.asarray(
        values(d, original, original_prim["attributes"]["COLOR_0"])
    )
    pigment_tree = cKDTree(pos)
    for level, amount in enumerate((780, 140, 40)):
        p, t, s = draft(pos, f, amount)
        if level == 0:
            a, b = clipped_foot(pos, f, 0.23)
            allp = np.concatenate((p, a))
            allf = np.concatenate((t, b + len(p)))
        else:
            allp, allf = p, t
        n, uv, col = channels(allp, allf, pigment_tree, pigment_source)
        q = allp[allf].reshape(-1, 3)
        attrs = dict(
            POSITION=att(q.astype("<f4"), 5126, "VEC3", bounds=True),
            NORMAL=att(n.astype("<f4"), 5126, "VEC3"),
            TEXCOORD_0=att(uv.astype("<f4"), 5126, "VEC2"),
            COLOR_0=att(col, 5121, "VEC4", norm=True),
        )
        prim = dict(
            attributes=attrs,
            indices=att(np.arange(len(q), dtype="<u2"), 5123, "SCALAR", 34963),
            material=0,
        )
        name = f"Cliff_L{level}"
        prod["meshes"].append(dict(name=name, primitives=[prim]))
        rows[name] = len(allf)
    prod["buffers"] = [{"byteLength": len(raw)}]
    dest = Path(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    save(prod, raw, dest)
    return {
        "source": "Existing 8 x 4 x 12 m Cliff.glb, 16 native locations",
        "triangle_counts": rows,
        "physical": "No world or collision authoring. Old true foot, yawed footprints and summit retained; carving is inside original native convex support.",
        "surface": "Original mineral colour/normal/roughness images.",
    }
