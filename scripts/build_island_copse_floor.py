"""Bake a scatter of true conifer roots, moss, ferns, sorrel and fallen duff.

This is a closed set of render-only clumps, not new prop placements. Soft
plant forms do not add obstacles, edible items or persistent state.
"""

from pathlib import Path
from collections import Counter
import json
import math
import numpy as np
from scipy.spatial import cKDTree
from build_island_strandline import Beach, Patch, SURFACE, ASSET
from finished_island_glb import save
from island_copse_botany import collar, root, moss, fern, sorrel, duff
from island_copse_mulch import bed, atlas

TILE = 50.0
ALTERNATE = (
    Path(__file__).resolve().parents[1] / "studies/island-ridge-v2/island-ridge-v2.bin"
)


class WoodlandPatch(Patch):
    def __init__(self, land, creeping=False):
        super().__init__(land)
        self.creeping = creeping
        self.bole_normals = {}

    def surface(self, x, z, lift, color, uv):
        return self.paint(x, z, self.beach.surface(x, z)[0] + lift, color, uv)

    def paint(self, x, z, y, color, uv):
        index = super().put_at(x, z, y, color)
        self.uv.extend((uv[0], uv[1] * (2 / 3 if self.creeping else 1)))
        return index

    def lower(self, point, lift, uv, color):
        # The bottom 256-pixel spring-feather strip uses square PBR panels.
        x, z = point
        index = super().put_at(x, z, self.beach.surface(x, z)[0] + lift, color)
        self.uv.extend(uv)
        return index

    def put_at(self, x, z, y, color):
        # The unpainted sprouts and spore stems sample an opaque, neutral
        # padded patch. Vary coordinates so their tangent frames are defined.
        uv = (
            0.015 + 0.010 * math.sin(x * 2.1 + z),
            0.947 + 0.032 * math.sin(z * 1.7 - x + 4.1 * y),
        )
        return self.paint(x, z, y, color, uv)


def bake(surface, alternate, placements, creeping=False):
    land, other = Beach(surface), Beach(alternate)
    props = json.loads(placements.read_text())
    stems = [
        (i, q) for i, q in enumerate(props) if q["kind"] in ("Conifer", "Broadleaf")
    ]
    xy = np.array([q["position"][::2] for _, q in stems])
    counts = cKDTree(xy).query_ball_point(xy, 21, return_length=True)
    tiles = {}
    receipt = Counter()
    for (i, q), (x, z), n in zip(stems, xy, counts, strict=True):
        h, dx, dz = land.surface(x, z)
        offset = q["position"][1] - h
        if (
            h < 2.9
            or abs(offset) > 0.105
            or math.hypot(dx, dz) > 0.63
            or (n < 5 and q["kind"] != "Conifer")
        ):
            continue
        # A six-metre halo includes all tips in the clump. Omit sites where
        # the alternate physical terrain diverges anywhere in that footprint.
        x0, x1 = int((x - 6 - land.x0) / land.s), int((x + 6 - land.x0) / land.s) + 2
        z0, z1 = int((z - 6 - land.z0) / land.s), int((z + 6 - land.z0) / land.s) + 2
        if not np.array_equal(land.h[z0:z1, x0:x1], other.h[z0:z1, x0:x1]):
            continue
        r = np.random.default_rng(247112 + i * 9743)
        pine = q["kind"] == "Conifer"
        damp = (
            0.48
            + 0.19 * math.sin(x * 0.041 + z * 0.031)
            + 0.16 * math.sin(z * 0.081 - x * 0.052)
            + 0.16 * min(1, (n - 4) / 8)
            + 0.12 * max(0, -dz)
        )
        rooted = pine and r.random() < 0.83
        green = r.random() < max(0.11, 0.86 * (damp - 0.29)) and n >= 5
        if not (pine or green):
            continue
        key = math.floor(x / TILE), math.floor(z / TILE)
        if key not in tiles:
            tiles[key] = WoodlandPatch(land, creeping)
        p = tiles[key]
        wind = math.atan2(-dz, -dx) + r.uniform(-1.5, 1.5)
        if pine and not rooted:
            local = np.random.default_rng(272891 + i * 4891)
            headings = [
                wind + arm * math.pi + local.uniform(-0.2, 0.2) for arm in range(2)
            ]
            collar(p, x, z, q["scale"], q["yaw"], offset, headings)
            receipt["quiet_pine_collars"] += 1
        if rooted:
            # Broadleaves already have flares; only expose extra wood around
            # the original pine cylinder, starting *inside* it at the soil.
            number = int(r.integers(2, 5))
            headings = [
                wind + math.tau * k / number + r.uniform(-0.36, 0.36)
                for k in range(number)
            ]
            collar(p, x, z, q["scale"], q["yaw"], offset, headings)
            for direction in headings:
                root(p, r, x, z, q["scale"], direction, offset, q["yaw"])
            receipt["tapered_pine_feet"] += 1
        litter = np.random.default_rng(234682 + i * 8171)
        if green or (rooted and litter.random() < 0.82):
            drift = wind + litter.uniform(-0.40, 0.40)
            reach = litter.uniform(1.05, 2.40) * min(1.18, q["scale"] ** 0.3)
            spread = reach * litter.uniform(0.46, 0.78)
            bed(
                p,
                litter,
                x + 0.29 * reach * math.cos(drift),
                z + 0.29 * reach * math.sin(drift),
                drift,
                reach,
                spread,
                int(litter.integers(2)),
            )
            receipt["triangle_seated_duff_beds"] += 1
            # Irregular low triplets emerge through only the cooler moist
            # margins, leaving grazing lanes and the drier trees open.
            if damp > 0.55 and litter.random() < 0.49:
                for k in range(int(litter.integers(2, 4))):
                    up = litter.uniform(-0.52, 0.79) * reach
                    side = litter.choice((-1, 1)) * litter.uniform(0.37, 0.91) * spread
                    ox = x + math.cos(drift) * up - math.sin(drift) * side
                    oz = z + math.sin(drift) * up + math.cos(drift) * side
                    moss(
                        p,
                        litter,
                        ox,
                        oz,
                        litter.uniform(0.14, 0.29),
                        litter.uniform(0.09, 0.15),
                        drift,
                    )
                    sorrel(
                        p,
                        litter,
                        ox,
                        oz,
                        litter.uniform(0.105, 0.165),
                        litter.uniform(0, math.tau),
                    )
                    receipt["wood_sorrel_margin_clumps"] += 1
        if not green:
            continue
        receipt["irregular_green_pockets"] += 1
        a = wind + r.uniform(-0.61, 0.61)
        c, s = math.cos(a), math.sin(a)
        extent = r.uniform(0.70, 1.62)
        px, pz = x + c * r.uniform(0.85, 1.60), z + s * r.uniform(0.85, 1.60)
        if creeping:
            from island_feather_moss import field

            mh = np.random.default_rng(77161 + i * 1913)
            # One low, branching colony per sheltered receiving pocket. Do not
            # lay overlapping discs under every individual moss/flower shoot.
            field(
                p,
                mh,
                px,
                pz,
                a,
                extent * mh.uniform(0.86, 1.28),
                extent * mh.uniform(0.48, 0.76),
                int(mh.integers(4)),
            )
            receipt["ground_tracing_feather_colonies"] += 1
        for k in range(int(r.integers(3, 8))):
            u = r.uniform(-0.86, 1.15) * extent
            v = r.normal(0, 0.27)
            sz = r.uniform(0.15, 0.45) * extent
            moss(
                p,
                r,
                px + c * u - s * v,
                pz + s * u + c * v,
                sz,
                sz * r.uniform(0.34, 0.72),
                a + r.uniform(-0.65, 0.65),
            )
            receipt["feather_shoot_sites" if creeping else "moss_cushions"] += 1
        dry = np.random.default_rng(51943 + i * 7561)
        for k in range(int(dry.integers(2, 6))):
            v = dry.choice((-1, 1)) * dry.uniform(0.14, 0.45)
            u = dry.uniform(-0.84, 0.96) * extent
            duff(p, dry, px + c * u - s * v, pz + s * u + c * v, a, pine)
            receipt["needle_or_leaf_gathers"] += 1
        if damp > 0.55 and r.random() < 0.83:
            for k in range(int(r.integers(1, 4)) if damp > 0.67 else 1):
                u = r.uniform(-0.85, 1.18) * extent
                v = r.normal(0.13, 0.5)
                fern(
                    p,
                    r,
                    px + c * u - s * v,
                    pz + s * u + c * v,
                    r.uniform(0.32, 0.58),
                    r.uniform(0, math.tau),
                )
                receipt["wood_ferns"] += 1
        else:
            for k in range(int(r.integers(2, 5))):
                ox, oz = r.normal(0, 0.31, 2)
                sorrel(
                    p,
                    r,
                    px + ox,
                    pz + oz,
                    r.uniform(0.10, 0.22),
                    r.uniform(0, math.tau),
                )
                receipt["sorrel_sprigs"] += 1
    return tiles, receipt


def write(tiles, path):
    doc = dict(
        asset=dict(version="2.0", generator="Qualia sheltered woodland floor"),
        scene=0,
        scenes=[dict(nodes=[])],
        nodes=[],
        meshes=[],
        bufferViews=[],
        accessors=[],
        images=[
            dict(uri="copse/" + s)
            for s in ("duff-bark.png", "duff-bark-normal.png", "duff-bark-orm.png")
        ],
        samplers=[dict(magFilter=9729, minFilter=9987, wrapS=33071, wrapT=33071)],
        textures=[dict(sampler=0, source=i) for i in range(3)],
        materials=[
            dict(
                name="Exposed bark, broken leaf mould and sheltered greens",
                alphaMode="MASK",
                alphaCutoff=0.42,
                normalTexture=dict(index=1),
                occlusionTexture=dict(index=2, strength=0.72),
                doubleSided=True,
                pbrMetallicRoughness=dict(
                    baseColorTexture=dict(index=0),
                    metallicRoughnessTexture=dict(index=2),
                    roughnessFactor=1,
                    metallicFactor=0,
                ),
            )
        ],
    )
    buf = bytearray()

    def attribute(rows, dtype, component, kind, **extra):
        rows = np.ascontiguousarray(rows, dtype)
        buf.extend(bytes(-len(buf) % 4))
        view = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(
                buffer=0,
                byteOffset=len(buf),
                byteLength=rows.nbytes,
                target=34963 if kind == "SCALAR" else 34962,
            )
        )
        buf.extend(rows.tobytes())
        idx = len(doc["accessors"])
        doc["accessors"].append(
            dict(
                bufferView=view,
                componentType=component,
                type=kind,
                count=len(rows),
                **extra,
            )
        )
        return idx

    for (cx, cz), p in sorted(tiles.items()):
        o = np.array(((cx + 0.5) * TILE, 0, (cz + 0.5) * TILE), "f4")
        v = np.frombuffer(p.vertices, "f4").reshape(-1, 3) - o
        rgb = np.frombuffer(p.colors, "u1").reshape(-1, 4)
        tri = np.frombuffer(p.tris, "I").reshape(-1, 3).copy()
        a, b, c = tri.T
        n = np.cross(v[b] - v[a], v[c] - v[a])
        flip = n[:, 1] < 0
        tri[flip, 1], tri[flip, 2] = tri[flip, 2].copy(), tri[flip, 1].copy()
        a, b, c = tri.T
        n = np.cross(v[b] - v[a], v[c] - v[a])
        live = np.linalg.norm(n, axis=1) > 3e-9
        tri, n = tri[live], n[live]
        nn = np.zeros_like(v)
        for k in range(3):
            np.add.at(nn, tri[:, k], n)
        nn[np.linalg.norm(nn, axis=1) < 1e-8] = [0, 1, 0]
        nn /= np.linalg.norm(nn, axis=1)[:, None]
        for idx, (hint, weight) in p.bole_normals.items():
            vec = nn[idx] * (1 - weight) + np.array(hint) * weight
            nn[idx] = vec / np.linalg.norm(vec)
        attr = dict(
            # glTF's normalized uint16 UVs retain many samples per source
            # atlas texel; no extension or custom material decode is needed.
            TEXCOORD_0=attribute(
                np.rint(65535 * np.frombuffer(p.uv, "f4").reshape(-1, 2)).clip(
                    0, 65535
                ),
                "<u2",
                5123,
                "VEC2",
                normalized=True,
            ),
            POSITION=attribute(
                v, "<f4", 5126, "VEC3", min=v.min(0).tolist(), max=v.max(0).tolist()
            ),
            NORMAL=attribute(nn, "<f4", 5126, "VEC3"),
            COLOR_0=attribute(rgb, "u1", 5121, "VEC4", normalized=True),
        )
        size, comp = ("<u2", 5123) if len(v) < 65536 else ("<u4", 5125)
        ix = attribute(tri.ravel(), size, comp, "SCALAR")
        name = f"Copse_{cx}_{cz}"
        i = len(doc["nodes"])
        doc["meshes"].append(
            dict(name=name, primitives=[dict(attributes=attr, indices=ix, material=0)])
        )
        doc["nodes"].append(dict(name=name, mesh=i, translation=o.tolist()))
        doc["scenes"][0]["nodes"].append(i)
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, path)


def export(out, creeping=False):
    out.mkdir(exist_ok=True, parents=True)
    atlas(out / "copse", creeping)
    tiles, notes = bake(SURFACE, ALTERNATE, ASSET.parent / "props.json", creeping)
    asset = out / "copse-floor.glb"
    write(tiles, asset)
    (out / "woodland-composition.json").write_text(json.dumps(notes, indent=2) + "\n")
    return asset
