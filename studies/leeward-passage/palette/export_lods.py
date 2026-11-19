"""A real instanced glTF bank, never a Bevy change or flattened imitation.

The 1,578 original cosmetic stool positions stay on ordinary V1; the 12
specimens and three independent mesh levels share one vertex-colour material.
Fitted flakes and the unique bluff remain a separate, unchanged assembly.
"""

from pathlib import Path
from collections import defaultdict, Counter
from copy import deepcopy
import json, math, csv, hashlib
import numpy as np
from finished_island_glb import parse, save, payload
from island_erratic_glb import streams
from reach import Reach
from flora import approach
from selection import Collector, COUNT, representatives, fit, fertile, fitting_library
from forms import mesh, WOODY
from cameras import frame

EXT = "EXT_mesh_gpu_instancing"
STATIONS = ("reach-guardian", "reach-meadow", "reach-alder", "reach-strand")


def shell():
    return dict(
        asset={
            "version": "2.0",
            "generator": "Qualia Shellback placed wind-garden palette",
        },
        scene=0,
        scenes=[],
        nodes=[],
        meshes=[],
        materials=[
            dict(
                name="Original wind-grown local pigment",
                doubleSided=True,
                pbrMetallicRoughness={"metallicFactor": 0, "roughnessFactor": 0.87},
            )
        ],
        bufferViews=[],
        accessors=[],
        extensionsUsed=[EXT],
        extensionsRequired=[EXT],
    )


def geom(doc, binary, piece):
    p, col, tri, pins = piece
    p = p.astype("<f4")
    t = tri.astype("<u2")
    norm = np.zeros_like(p)
    face = np.cross(p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]])
    for k in range(3):
        np.add.at(norm, t[:, k], face)
    norm /= np.maximum(1e-10, np.linalg.norm(norm, axis=1, keepdims=True))
    sr = np.clip(col / 255, 0, 1)
    lin = np.where(sr < 0.04045, sr / 12.92, ((sr + 0.055) / 1.055) ** 2.4)
    c = np.column_stack((np.uint8(np.rint(lin * 255)), np.full(len(col), 255, "B")))
    emit = streams(doc, binary)
    att = dict(
        POSITION=emit(p, 5126, "VEC3", bounds=True),
        NORMAL=emit(norm, 5126, "VEC3"),
        COLOR_0=emit(c, 5121, "VEC4", normalized=True),
    )
    return dict(
        attributes=att,
        indices=emit(t.ravel(), 5123, "SCALAR", indices=True),
        material=0,
    )


def source_counts(folder):
    out = {}
    for fn in ("reach-detail.glb", "reach-scrub.glb", "bluff-foot.glb"):
        d, b = parse(folder / fn)
        out[fn] = {
            m["name"]: sum(
                d["accessors"][p["indices"]]["count"] // 3 for p in m["primitives"]
            )
            for m in d["meshes"]
        }
    return out


def copy_meshes(src, dest, keep):
    old, raw = parse(src)
    d = shell()
    del d["extensionsRequired"], d["extensionsUsed"]
    d["scenes"] = [dict(nodes=[])]
    d["materials"] = old["materials"]
    blob = bytearray()
    rel = {}

    def copy(acc):
        if acc not in rel:
            a = dict(old["accessors"][acc])
            v = old["bufferViews"][a["bufferView"]]
            blob.extend(b"\0" * (-len(blob) % 4))
            off = len(blob)
            content = payload(old, raw, acc)
            blob.extend(content)
            a.pop("byteOffset", None)
            a["bufferView"] = len(d["bufferViews"])
            d["bufferViews"].append(
                dict(
                    buffer=0,
                    byteOffset=off,
                    byteLength=len(content),
                    target=v["target"],
                )
            )
            rel[acc] = len(d["accessors"])
            d["accessors"].append(a)
        return rel[acc]

    count = 0
    allbounds = []
    nodedata = []
    for n in old["nodes"]:
        if "mesh" not in n or not keep(n):
            continue
        srcmesh = old["meshes"][n["mesh"]]
        prim = []
        ntri = 0
        for p in srcmesh["primitives"]:
            ntri += old["accessors"][p["indices"]]["count"] // 3
            prim.append(
                dict(
                    attributes={k: copy(v) for k, v in p["attributes"].items()},
                    indices=copy(p["indices"]),
                    material=p.get("material", 0),
                )
            )
        m = len(d["meshes"])
        d["meshes"].append(dict(name=srcmesh["name"], primitives=prim))
        node = dict(n)
        node["mesh"] = m
        d["scenes"][0]["nodes"].append(len(d["nodes"]))
        d["nodes"].append(node)
        count += ntri
        p = old["accessors"][srcmesh["primitives"][0]["attributes"]["POSITION"]]
        ax = n.get("translation", [0, 0, 0])
        nodedata.append(
            dict(
                name=n["name"],
                triangles=ntri,
                lo=[x + y for x, y in zip(p["min"], ax)],
                hi=[x + y for x, y in zip(p["max"], ax)],
            )
        )
    d["buffers"] = [dict(byteLength=len(blob))]
    if d["nodes"]:
        save(d, blob, dest)
    return nodedata


def node_attrs(doc, blob, rows):
    emit = streams(doc, blob)
    pos = np.array(
        [[r["root"][0], r["root"][1] + r["offset"], r["root"][2]] for r in rows], "<f4"
    )
    rot = np.array(
        [[0, math.sin(r["yaw"] / 2), 0, math.cos(r["yaw"] / 2)] for r in rows], "<f4"
    )
    scl = np.array([r["scale"] for r in rows], "<f4")
    return dict(
        TRANSLATION=emit(pos, 5126, "VEC3", bounds=True),
        ROTATION=emit(rot, 5126, "VEC4"),
        SCALE=emit(scl, 5126, "VEC3"),
    )


def write_host(doc, buf, label, groups, index, counts, cache):
    ids = []
    summary = Counter()
    tri = 0
    objects = 0
    for (sector, key, level), rows in sorted(groups.items()):
        cachekey = (sector, key, tuple(r["id"] for r in rows))
        if cachekey not in cache:
            cache[cachekey] = node_attrs(doc, buf, rows)
        node = dict(
            name=f"{label} / {sector:02d} / {key}",
            mesh=index[(key, level)],
            extensions={EXT: dict(attributes=cache[cachekey])},
            extras=dict(
                source_tiles_28m=sorted({r["zone"] for r in rows}),
                botanical_lod=level,
                stems=len(rows),
            ),
        )
        ids.append(len(doc["nodes"]))
        doc["nodes"].append(node)
        summary[f"L{level}"] += len(rows)
        objects += len(rows)
        tri += counts[key, level] * len(rows)
    doc["scenes"].append(dict(name=label, nodes=ids))
    return dict(
        instances=objects,
        instanced_primitive_groups=len(ids),
        expanded_plant_triangles=tri,
        levels=dict(summary),
        shared_meshes_referenced=len({(k, l) for _, k, l in groups}),
    )


def spatial(p, b):
    return math.hypot(
        max(b["lo"][0] - p[0], 0, p[0] - b["hi"][0]),
        max(b["lo"][2] - p[2], 0, p[2] - b["hi"][2]),
    )


def selection(rows, cam, tag):
    dct = defaultdict(list)
    guard = tag == "reach-guardian"
    for r in rows:
        dist = math.hypot(r["root"][0] - cam[0], r["root"][2] - cam[2])
        woody = r["family"] in WOODY
        if guard:
            if r["radius"] * 2 < 0.21 and not woody:
                continue
            level = 2
        else:
            if dist > (148 if woody else 105):
                continue
            if dist < 17:
                level = 0
            elif dist < 52:
                level = 1
            else:
                if 1000 * r["radius"] * 2 / max(1, dist) / (36 / 28) < 1.1:
                    continue
                level = 2
        dct[0, r["key"], level].append(r)
    return dct


def create(surface, full, folder):
    expected = json.loads((full / "route.json").read_text())["terrain_sha256"]
    if hashlib.sha256(surface.read_bytes()).hexdigest() != expected:
        raise ValueError(
            "Original V1 soil changed; do not mount a frozen placement bank."
        )
    r = Reach(surface)
    folder.mkdir(exist_ok=True, parents=True)
    collector = Collector(r)
    full_counts = approach(r, None, collector=collector)
    doc = shell()
    buf = bytearray()
    index = {}
    ct = {}
    rows = []
    specs = []
    for kind in sorted(collector.rows):
        originals = collector.rows[kind]
        specimen = (
            [
                representatives([o for o in originals if fertile(o) == st], 1)[0]
                for st in (False, True)
            ]
            if kind == "oat"
            else representatives(originals, COUNT[kind])
        )
        choices = []
        for j, o in enumerate(specimen):
            key = f"{kind}-{j+1}"
            levels = [mesh(o, l) for l in range(3)]
            choices.append((key, o, levels))
            for l, x in enumerate(levels):
                index[key, l] = len(doc["meshes"])
                ct[key, l] = len(x[2])
                doc["meshes"].append(
                    dict(name=f"{key}_L{l}", primitives=[geom(doc, buf, x)])
                )
            specs.append(
                dict(
                    key=key,
                    family=kind,
                    original_family_index=o.id,
                    original_native_xyz=o.anchor.tolist(),
                    old_triangles=len(o.tri),
                    tiers=[ct[key, l] for l in range(3)],
                )
            )
        candidates = fitting_library(choices)
        for o in originals:
            row = fit(o, candidates, r)
            row.update(
                family=kind, radius=float(np.linalg.norm(o.outline, axis=1).max())
            )
            rows.append(row)
    geo = deepcopy(doc)
    geom_buf = bytes(buf)
    full_scenes = []
    cache = {}
    for level, tag in enumerate(
        ("Creature-close artist bank", "Intermediate artist bank", "Guardian overview")
    ):
        groups = defaultdict(list)
        for row in rows:
            if level == 2 and row["family"] not in WOODY and row["radius"] * 2 < 0.21:
                continue
            groups[0, row["key"], level].append(row)
        full_scenes.append(write_host(doc, buf, tag, groups, index, ct, cache))
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, folder / "placed-palette.glb")
    with (folder / "roots.csv").open("w", newline="") as file:
        w = csv.writer(file)
        w.writerow(
            [
                "family",
                "original_stool",
                "source_tile",
                "specimen",
                "native_x",
                "native_ground_y",
                "native_z",
                "render_seat_y_offset_m",
                "yaw_rad",
                "scale_x",
                "scale_y",
                "scale_z",
                "mean_directional_fill",
            ]
        )
        for row in rows:
            w.writerow(
                [
                    row["family"],
                    row["id"],
                    row["zone"],
                    row["key"],
                    *[f"{v:.6f}" for v in row["root"]],
                    *[
                        f"{v:.6f}"
                        for v in (
                            row["offset"],
                            row["yaw"],
                            row["scale"][0],
                            row["scale"][1],
                            row["scale"][2],
                            row["support_fill"],
                        )
                    ],
                ]
            )
    literal = source_counts(full)
    n_total = sum(n for fn in literal.values() for n in fn.values())
    old_flakes = sum(
        n for k, n in literal["reach-detail.glb"].items() if k.endswith("_flakes")
    )
    root_n = sum(literal["bluff-foot.glb"].values())
    full_flakes = copy_meshes(
        full / "reach-detail.glb",
        folder / "fitted-flakes.glb",
        lambda n: n["name"].endswith("_flakes"),
    )
    stations = {}
    for tag in STATIONS:
        cam = frame(tag, lambda x, z: float(r.floor(x, z)[0]))[1]
        sub = folder / tag
        sub.mkdir(exist_ok=True)
        g = deepcopy(geo)
        bb = bytearray(geom_buf)
        groups = selection(rows, cam, tag)
        q = write_host(g, bb, tag, groups, index, ct, {})
        g["buffers"] = [dict(byteLength=len(bb))]
        save(g, bb, sub / "planted.glb")
        guardian = tag == "reach-guardian"
        keep = {f["name"] for f in full_flakes if not guardian and spatial(cam, f) < 36}
        flake_pieces = (
            copy_meshes(
                folder / "fitted-flakes.glb",
                sub / "fitted-flakes.glb",
                lambda n: n["name"] in keep,
            )
            if keep
            else []
        )
        foot = not guardian and math.hypot(cam[0] - 133.0, cam[2] - 444.8) < 50
        old_tri = n_total - (0 if foot else root_n) - (old_flakes if guardian else 0)
        old_nodes = (
            sum(len(fn) for fn in literal.values())
            - (0 if foot else len(literal["bluff-foot.glb"]))
            - (len(full_flakes) if guardian else 0)
        )
        ftri = sum(n["triangles"] for n in flake_pieces)
        stations[tag] = dict(
            camera_native_xyz=cam,
            old_visible_artist_triangles=old_tri,
            old_mesh_objects=old_nodes,
            **q,
            fitted_flake_tiles=len(keep),
            original_flake_triangles_loaded=ftri,
            original_bluff_triangles=root_n if foot else 0,
            selected_total_triangles=q["expanded_plant_triangles"]
            + ftri
            + (root_n if foot else 0),
            selected_total_primitive_groups=q["instanced_primitive_groups"]
            + len(keep)
            + int(foot),
        )
    for key in ("offset", "support_fill"):
        vals = np.asarray([row[key] for row in rows])
        external = (
            "visual_seat_offset_m"
            if key == "offset"
            else "mean_original_16_bearing_fill"
        )
        full_counts[external] = [
            float(v) for v in np.percentile(vals, [0, 10, 50, 90, 100])
        ]
    report = dict(
        source_full_quality="9d8e82b2ce0a39e3a41880a2b1c04fa8d1a72323",
        terrain_sha256=hashlib.sha256(surface.read_bytes()).hexdigest(),
        policy="Camera XZ range 17m high/52m middle; geometry outside 105m or 148m omitted. Guardian retains actual >.21m diameter or woody. No frustum, occlusion, performance, draw-call or runtime measurement.",
        full_source_triangles=n_total,
        old_plant_triangles=n_total - root_n - old_flakes,
        exact_fitted_flake_triangles=old_flakes,
        exact_bluff_mesh_triangles=root_n,
        number_unique_specimens=len(specs),
        unique_triangles_all_lods=sum(ct.values()),
        total_original_stools=len(rows),
        original_counts=full_counts,
        glb_bytes=(folder / "placed-palette.glb").stat().st_size,
        source_geometry_bytes=sum((full / p).stat().st_size for p in literal),
        separate_flake_bytes=(folder / "fitted-flakes.glb").stat().st_size,
        source_specimens=specs,
        all_fixed_level_scenes=full_scenes,
        stations=stations,
    )
    (folder / "palette.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    import argparse

    ap = argparse.ArgumentParser(description=__doc__)
    for n in ("surface", "full", "folder"):
        ap.add_argument(n, type=Path)
    a = ap.parse_args()
    create(a.surface, a.full, a.folder)
