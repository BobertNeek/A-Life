"""Use the one truly unreferenced native habitat-atlas slot for hero till.

Two original V1 triangles at the foot of boulder 1570 keep their exact
positions, normals, vertex pigment and original UV orientation. Move only
their UV crop into the spare chart for twice the texels/metre. At the rim
the actual original albedo, normal, and roughness continue unaltered. No
new map, material, triangle, terrain authority or satellite foundation.
"""

from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, map_coordinates
from finished_island_glb import parse, values, save
from island_erratic_glb import streams

TILE = 336
PAD = 8
SLOT = (7, 6)
CENTRE = (254.682425, -177.670897)


def ease(lo, hi, x):
    t = np.clip((x - lo) / (hi - lo), 0, 1)
    return t * t * (3 - 2 * t)


def sample(im, uv):
    a = np.asarray(im)
    p = [uv[..., 1] * im.height - 0.5, uv[..., 0] * im.width - 0.5]
    return np.stack(
        [
            map_coordinates(a[:, :, k].astype("f"), p, order=1, mode="nearest")
            for k in range(3)
        ],
        -1,
    )


def field(r, n, sd):
    a = gaussian_filter(r.normal(size=(n, n)), sd, mode="reflect")
    return (a - a.mean()) / max(0.001, a.std())


def detail(uv, wx, wz, base, normal, orm):
    n = uv.shape[0]
    r = np.random.default_rng(117408)
    irregular = 0.64 * field(r, n, 14) + 0.36 * field(r, n, 5)
    litter = field(r, n, 1.25)
    grit = field(r, n, 0.63)
    rx = wx - CENTRE[0]
    rz = wz - CENTRE[1]
    # Existing pockets on opposite flanks, wider down the actual eastward dip.
    # Broken curved seams of soil between lenses; NOT a radial contact shadow.
    a = np.exp(
        -(((rz + 0.39 - 0.17 * rx - 0.13 * np.sin(3.1 * rx)) / 0.38) ** 2)
    ) * np.exp(-(((rx - 0.75) / 1.16) ** 2))
    b = np.exp(-(((rz - 0.45 + 0.27 * rx) / 0.28) ** 2)) * np.exp(
        -(((rx + 0.22) / 0.68) ** 2)
    )
    bed = ease(0.11, 0.82, 0.95 * a + 0.73 * b + 0.11 * irregular)
    rim = ease(
        0,
        0.095,
        np.minimum.reduce((uv[..., 0], uv[..., 1], 1 - uv[..., 0], 1 - uv[..., 1])),
    )
    m = bed * rim
    spall = np.zeros_like(m)
    cortex = np.zeros_like(m)
    hair = np.zeros_like(m)
    ij = np.mgrid[-13:14, -13:14]
    for j in range(1850):
        x, z = r.integers(13, n - 13, size=2)
        if r.uniform(0.01, 0.86) > m[z, x]:
            continue
        theta = r.normal(0.36, 0.87)
        along = ij[1] * np.cos(theta) + ij[0] * np.sin(theta)
        across = ij[0] * np.cos(theta) - ij[1] * np.sin(theta)
        if j % 8 == 0:
            # weathered fibrous litter, narrow and bent, not painted grass stamps
            L = r.uniform(3, 9)
            side = across - 0.032 * along**2 * np.sign(theta)
            mask = np.exp(-((side / 0.72) ** 2)) * np.clip(1 - abs(along) / L, 0, 1)
            buf = hair
        else:
            L = r.uniform(4.3, 9.7) if j % 11 == 1 else r.uniform(0.9, 3.8)
            w = L * r.uniform(0.26, 0.52)
            d = np.maximum(
                0.90 * abs(across) / w + 0.12 * abs(along) / L,
                0.82 * abs(along) / L + 0.24 * abs(across) / w,
            )
            mask = 1 - ease(0.45, 1.13, d)
            buf = spall if j % 3 else cortex
        patch = buf[z - 13 : z + 14, x - 13 : x + 14]
        np.maximum(patch, mask * r.uniform(0.45, 1), out=patch)
    rgb = base.copy()
    # A little dark oxidized soil is exposed between unaffected turf islands.
    # Leave macro terrain palette intact at the border, with lighter tiny feldspar
    # flecks conveying the fresh rock at the downhill lip.
    # Humic mineral soil fills blocked joints, not a uniform brown disc.
    pockets = ease(-0.38, 0.92, 0.47 * irregular + 0.32 * litter + 0.17 * grit)
    coef = m * (0.62 + 0.37 * pockets)
    rgb = base * (1 - 0.16 * coef[..., None])
    rgb += coef[..., None] * np.array([8.0, -11.0, -22.0])
    rgb += m[..., None] * (3.4 * litter + 2.4 * grit)[..., None]
    rgb += m[..., None] * (
        np.array([33, 35, 35]) * spall[..., None]
        - np.array([19, 20, 17]) * cortex[..., None]
        + np.array([16, 9, -3]) * hair[..., None]
    )
    # Heights are *shading* mm, consistent at one native 3.125 m cell.
    elev = m * (
        0.0013 * litter
        + 0.0007 * grit
        + 0.0080 * spall
        - 0.0023 * cortex
        + 0.0025 * hair
    )
    zv, xu = np.gradient(elev, 3.125 / (n - 1))
    nn = normal / 127.5 - 1
    nn[..., 0] -= xu
    nn[..., 1] += zv
    nn /= np.maximum(1e-9, np.linalg.norm(nn, axis=-1, keepdims=True))
    rough = orm.copy()
    rough[..., 1] = np.clip(rough[..., 1] + m * (4.9 - 10 * spall + 3 * hair), 0, 255)
    return rgb, (1 + nn) * 127.5, rough


def build(source, dest):
    source = Path(source)
    dest = Path(dest)
    dest.mkdir(exist_ok=True, parents=True)
    d, buf = parse(source / "habitat-ground.glb")
    record = None
    used = set()
    for mesh in d["meshes"]:
        pr = mesh["primitives"][0]
        att = pr["attributes"]
        uv = np.array(values(d, buf, att["TEXCOORD_0"]))
        faces = np.asarray(values(d, buf, pr["indices"]), dtype=int).reshape(-1, 3)
        used.update(map(tuple, np.floor(uv[faces].mean(1) * [8, 7]).astype(int)))
        if mesh["name"] != "Habitat_7_4":
            continue
        p = np.asarray(values(d, buf, att["POSITION"]))
        c = p[faces].mean(1)
        ids = np.flatnonzero(
            (c[:, 0] > 253.125)
            & (c[:, 0] < 256.25)
            & (c[:, 2] > -178.125)
            & (c[:, 2] < -175)
        )
        if len(ids) != 2:
            raise ValueError("Original hero cell no longer consists of two faces.")
        record = mesh, pr, faces, ids, p, uv
    if SLOT in used or record is None:
        raise ValueError("Required V1 atlas slot is no longer free.")
    mesh, pr, faces, ids, p, uv = record
    att = pr["attributes"]
    corners = np.unique(faces[ids])
    lo, hi = uv[corners].min(0), uv[corners].max(0)
    rel = (uv[corners] - lo) / (hi - lo)
    uv_to_ground = np.linalg.lstsq(
        np.column_stack((rel, np.ones(len(rel)))), p[corners][:, [0, 2]], rcond=None
    )[0]
    # Native atlas spans 8x7 slots. Preserve orientation exactly: local tangent
    # signs and the donor's D4 rotation are carried through an affine UV change.
    n = TILE - 2 * PAD
    v, u = np.mgrid[:n, :n] / (n - 1)
    local = np.stack((u, v), -1)
    site = np.stack((u, v, np.ones_like(v)), -1) @ uv_to_ground
    coords = lo + local * (hi - lo)
    names = ("substrates.png", "substrates-normal.png", "substrates-orm.png")
    imgs = [Image.open(source / "habitats" / k).convert("RGB") for k in names]
    if imgs[0].size != (2688, 2352):
        raise ValueError("Atlas layout changed; do not repaint.")
    orig = [sample(im, coords) for im in imgs]
    paint = detail(local, site[:, :, 0], site[:, :, 1], *orig)
    target = dest / "habitats"
    target.mkdir(exist_ok=True)
    for name, img, arr in zip(names, imgs, paint):
        tile = np.pad(
            np.rint(np.clip(arr, 0, 255)).astype("u1"),
            ((PAD, PAD), (PAD, PAD), (0, 0)),
            mode="edge",
        )
        scale = img.width // 8
        swatch = Image.fromarray(tile)
        if scale != TILE:
            swatch = swatch.resize((scale, scale), Image.Resampling.BOX)
        img.paste(swatch, (SLOT[0] * scale, SLOT[1] * scale))
        img.save(target / name)
    newuv = (np.array(SLOT) * TILE + PAD + 0.5 + rel * (n - 1)) / np.array(imgs[0].size)
    add = streams(d, buf)
    count = len(p)
    for key, idx in list(att.items()):
        acc = d["accessors"][idx]
        ct = acc["componentType"]
        a = np.asarray(values(d, buf, idx, normalize=False))
        subset = newuv if key == "TEXCOORD_0" else a[corners]
        dtype = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[ct]
        att[key] = add(
            np.concatenate((a, subset)).astype(dtype),
            ct,
            acc["type"],
            normalized=acc.get("normalized", False),
            bounds=key == "POSITION",
        )
    lut = {k: count + i for i, k in enumerate(corners)}
    faces[ids] = np.vectorize(lut.__getitem__)(faces[ids])
    pr["indices"] = add(faces.ravel().astype("<u2"), 5123, "SCALAR", indices=True)
    d["buffers"] = [dict(byteLength=len(buf))]
    save(d, buf, dest / "habitat-ground.glb")
    return dict(
        cell=[253.125, -178.125, 256.25, -175],
        new_uv_vertices=len(corners),
        new_triangles=0,
        old_texels_per_metre=159 / 3.125,
        new_texels_per_metre=319 / 3.125,
        old_and_new_atlas_size=imgs[0].size,
        existing_slots_used=len(used),
        new_slot=SLOT,
    )
