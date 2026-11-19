"""Render-only crown forms for the real, immutable island pine population.

Exposed crowns form low, asymmetric groups. Some intermediate pines keep a
broader club-shaped crown; sheltered leaders retain the source silhouette.
All vertices up through 3.2 m, original index order, and every tree anchor stay
untouched. The actual solid trunk proxy ends at 3 m before placement scale.
"""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import struct

import numpy as np
from scipy.spatial import cKDTree

from finished_island_glb import parse, values
from island_geology_paint import grain

ROOT = Path(__file__).resolve().parents[1]
D = ROOT / "crates/alife_game_app/assets/landscape/island"
SURFACE = ROOT / "crates/alife_world/assets/island-v1.bin"
GUARD = 3.2


def crown(xyz, profile, quarter):
    # One smooth bend and radial field for all LODs of a given tree. Preserve
    # the authoritative low stem exactly, rather than scaling a whole prop.
    p = np.array(xyz, dtype="f8", copy=True)
    angle = math.pi * 0.5 * quarter
    f = np.array((math.cos(angle), math.sin(angle)))
    n = np.array((-f[1], f[0]))
    y = p[:, 1]
    u = np.clip((y - GUARD) / (12 - GUARD), 0, 1)
    blend = u * u * (3 - 2 * u)
    r = p[:, [0, 2]]
    along = r @ f
    cross = r @ n
    if profile == 1:
        # Short, broad shore fir. Its windward side stops early; the lee has
        # two uneven shoulders. The central leader leans rather than vanishing.
        k = 0.50
        wide = 1 + 0.79 * np.sin(math.pi * u * 0.93) ** 1.1
        compress = 1 + 0.25 * np.sin(math.pi * u)
        bend = 1.60 * (0.48 * blend + 0.52 * u * u * u)
        skirt = 0.19 * np.sin(u * math.tau * 1.4 + cross * 0.91) * blend
        outward = np.maximum(0, along) * 0.19 * np.sin(u * math.pi)
    else:
        # Bed-side broad crown leaves a taller, rounded leader with open sky
        # beneath its lower branches; it does not turn into a second coast fir.
        k = 0.21
        wide = 1 + 0.71 * np.sin(math.pi * u * 0.80) ** 1.8
        compress = 1 + 0.38 * np.sin(math.pi * u * 0.86)
        bend = 0.63 * blend
        skirt = 0.22 * np.sin(4.4 * u + along * 0.87) * blend
        outward = np.maximum(0, along) * 0.08 * np.sin(u * math.pi)
    h = GUARD + (12 - GUARD) * (u - k * u * u * (1.5 - 0.5 * u)) + skirt
    q = (along * wide + outward + bend)[:, None] * f + (
        cross * compress + 0.12 * blend * np.sin(u * 5.7)
    )[:, None] * n
    active = y > GUARD
    p[active, 1] = h[active]
    p[np.ix_(active, [0, 2])] = q[active]
    return p


def derive(pos, norm, color, profile, quarter):
    new = crown(pos, profile, quarter)
    jac = []
    eps = 0.0009
    for axis in range(3):
        d = np.eye(3)[axis] * eps
        jac.append(
            (crown(pos + d, profile, quarter) - crown(pos - d, profile, quarter))
            / (2 * eps)
        )
    # Take a bounded one-sided derivative at a source's cap, whose input spans
    # no higher than 12 m. The transformed tangent basis has nonzero stretch.
    jac = np.stack(jac, axis=-1)
    nn = np.linalg.solve(jac.transpose(0, 2, 1), norm[..., None])[..., 0]
    nn /= np.maximum(1e-12, np.linalg.norm(nn, axis=1)[:, None])
    base = pos[:, 1] <= GUARD
    nn[base] = norm[base]
    new[base] = pos[base]
    c = color.copy()
    foliage = (c[:, 1] > c[:, 0] * 1.16) & (c[:, 1] > c[:, 2] * 1.3) & ~base
    t = np.clip((pos[:, 1] - GUARD) / 6, 0, 1)
    # Sun-bleached silver tips on windbreaks vs. a warm fern interior, in
    # linear vertex colour. Bark and the near contact zone remain original.
    palette = np.array([1.23, 1.15, 1.37] if profile == 1 else [1.16, 1.055, 0.93])
    c[foliage, :3] = np.clip(
        c[foliage, :3] * (1 + t[foliage, None] * (palette - 1)), 0, 1
    )
    return new.astype("<f4"), nn.astype("<f4"), np.rint(c * 255).astype("u1")


def layout(path):
    props = json.loads(path.read_text())
    pine = [(i, p) for i, p in enumerate(props) if p["kind"] == "Conifer"]
    trees = np.array(
        [
            np.array(p["position"])[[0, 2]]
            for p in props
            if p["kind"] in ("Conifer", "Broadleaf", "Sapling")
        ]
    )
    fixture = np.array([p["position"] for _, p in pine])
    x, h, z = fixture.T
    nearest = cKDTree(trees).query(fixture[:, [0, 2]], k=7)[0]
    edge = np.clip((nearest[:, 6] - 15) / 32, 0, 1)
    grove = grain(x + 3 * grain(x, z, 33, 17), z, 51, 590)
    local = grain(x, z, 12, 741)
    coastal = np.clip((np.hypot(x / 445, z / 535) - 0.39) / 0.58, 0, 1)
    exposure = (
        0.34 * edge
        + 0.21 * np.clip(h / 120, 0, 1)
        + 0.15 * coastal
        + 0.28 * grove
        + 0.09 * local
    )
    lo, hi = np.quantile(exposure, (0.32, 0.68))
    result = {}
    for rank, (idx, prop) in enumerate(pine):
        profile = 1 if exposure[rank] >= hi else 2 if exposure[rank] >= lo else 0
        if profile == 0:
            continue
        # Runtime yaw rotates Bevy +X toward -Z. Select the closest of four
        # prebuilt bends to the same prevailing world horizontal direction.
        yaw = prop["yaw"]
        quarter = round((math.atan2(-0.30, 0.69) + yaw) * 2 / math.pi) % 4
        code = (profile - 1) * 4 + quarter + 1
        result[str(idx)] = {"form": code, "xz": [float(x[rank]), float(z[rank])]}
    return result


def build(source, placement, out):
    out.mkdir(parents=True, exist_ok=True)
    src, bin = parse(source)
    doc = dict(
        asset=dict(
            version="2.0", generator="Qualia ecology-graded pine crown variants"
        ),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        accessors=[],
        bufferViews=[],
        materials=src["materials"],
    )
    binary = bytearray()
    stats = {}

    def attr(rows, dtype, kind, shader, normalized=False, bounds=False):
        a = np.ascontiguousarray(rows, dtype=dtype)
        binary.extend(bytes(-len(binary) % 4))
        v = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=len(binary),
                byteLength=a.nbytes,
                target=34963 if shader == "SCALAR" else 34962,
            )
        )
        binary.extend(a.tobytes())
        rec = dict(bufferView=v, componentType=kind, type=shader, count=len(a))
        if normalized:
            rec["normalized"] = True
        if bounds:
            rec.update(min=a.min(0).tolist(), max=a.max(0).tolist())
        j = len(doc["accessors"])
        doc["accessors"].append(rec)
        return j

    for m in src["meshes"]:
        level = m["name"].split("_L")[1]
        prim = m["primitives"][0]
        a = prim["attributes"]
        pos = np.array(values(src, bin, a["POSITION"]), dtype="f4")
        norm = np.array(values(src, bin, a["NORMAL"]), dtype="f4")
        col = np.array(values(src, bin, a["COLOR_0"]), dtype="f8")
        ix = np.array(values(src, bin, prim["indices"]), dtype="<u2").ravel()
        original_indices = attr(ix, "<u2", 5123, "SCALAR")
        for profile in (1, 2):
            for quarter in range(4):
                code = (profile - 1) * 4 + quarter + 1
                name = f"PineForm{code}_L{level}"
                pp, nn, cc = derive(pos, norm, col, profile, quarter)
                base = pos[:, 1] <= GUARD
                assert np.array_equal(pp[base], pos[base]) and np.array_equal(
                    nn[base], norm[base]
                )
                assert np.allclose(cc[base] / 255.0, col[base], atol=0.5 / 255)
                assert (
                    pp[~base, 1].min() > GUARD
                    and np.isfinite(pp).all()
                    and np.isfinite(nn).all()
                )
                assert np.max(np.abs(np.linalg.norm(nn, axis=1) - 1)) < 0.0002
                attrs = dict(
                    POSITION=attr(pp, "<f4", 5126, "VEC3", bounds=True),
                    NORMAL=attr(nn, "<f4", 5126, "VEC3"),
                    COLOR_0=attr(
                        cc, "u1", 5121, "VEC" + str(cc.shape[1]), normalized=True
                    ),
                )
                i = len(doc["meshes"])
                doc["meshes"].append(
                    dict(
                        name=name,
                        primitives=[
                            dict(indices=original_indices, attributes=attrs, material=0)
                        ],
                    )
                )
                doc["nodes"].append(dict(name=name, mesh=i))
                doc["scenes"][0]["nodes"].append(i)
                stats[name] = dict(
                    triangles=int(len(ix) // 3),
                    frozen=int(base.sum()),
                    height=float(pp[:, 1].max()),
                )
    doc["buffers"] = [dict(byteLength=len(binary))]
    from finished_island_glb import save

    save(doc, binary, out / "grove-crowns.glb")
    mapping = layout(placement)
    (out / "grove-crowns.json").write_text(
        json.dumps(mapping, separators=(",", ":")) + "\n"
    )
    receipt = dict(
        source="Conifer.glb",
        root_guard_m=GUARD,
        protected_proxy_height_m=3,
        forms=stats,
        tree_count_by_new_form=dict(Counter(v["form"] for v in mapping.values())),
        unchanged_pines=421 - len(mapping),
    )
    (out / "grove-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--conifer", type=Path, default=D / "Conifer.glb")
    cli.add_argument("--placements", type=Path, default=D / "props.json")
    cli.add_argument("--out", type=Path, required=True)
    a = cli.parse_args()
    build(a.conifer, a.placements, a.out)
