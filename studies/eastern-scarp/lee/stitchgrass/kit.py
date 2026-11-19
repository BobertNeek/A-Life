"""Extend the two-state portable V1 edit. Same native vertices; meaningful near-field art is a third selectable state."""

import sys, math, csv, json, io, copy
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.spatial.transform import Rotation

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H), str(H.parent), str(H.parents[3] / "scripts")]
from finished_island_glb import parse, save, values
from island_erratic_glb import streams
from sculpt import shape
from surface import chart, BOX, CUT


def add(src, stage):
    records = list(csv.DictReader((stage / "stitches.csv").open()))
    d, raw = parse(src)
    old_len = d["buffers"][0]["byteLength"]
    binary = bytearray(raw[:old_len])
    emit = streams(d, binary)

    def node(name, pr, attrs=None):
        mi = len(d["meshes"])
        d["meshes"].append(dict(name=name, primitives=pr))
        ni = len(d["nodes"])
        n = dict(name=name, mesh=mi)
        if attrs:
            n["extensions"] = {"EXT_mesh_gpu_instancing": {"attributes": attrs}}
        d["nodes"].append(n)
        return ni

    def data(i):
        v = d["bufferViews"][d["images"][d["textures"][i]["source"]]["bufferView"]]
        off = v.get("byteOffset", 0)
        return np.asarray(
            Image.open(io.BytesIO(raw[off : off + v["byteLength"]])).convert("RGB")
        )

    base = d["scenes"][1]["nodes"][0]
    attrib = d["meshes"][d["nodes"][base]["mesh"]]["primitives"][0]
    points = np.array(values(d, raw, attrib["attributes"]["POSITION"]))
    source_tri = np.array(values(d, raw, attrib["indices"]), int).reshape(-1, 3)
    # Original streaming GLB contains vertical 20 m chunk-edge skirts.
    # Keep those OUT of this open-sided habitat desk. Only the 952 real
    # surface triangles have area on the V1 x/z lattice. No native vertices,
    # skirts, collider or native render meshes are edited in the game.
    spans = points[source_tri][:, :, [0, 2]]
    u = spans[:, 1] - spans[:, 0]
    v = spans[:, 2] - spans[:, 0]
    surface = np.abs(u[:, 0] * v[:, 1] - u[:, 1] * v[:, 0]) > 1.0e-6
    tri = source_tri[surface]
    surface_ix = emit(tri.astype("<u2").ravel(), 5123, "SCALAR", indices=True)
    for state in d["scenes"][:2]:
        idx = state["nodes"][0]
        d["meshes"][d["nodes"][idx]["mesh"]]["primitives"][0]["indices"] = surface_ix
    orig = d["materials"][attrib["material"]]
    c = orig["pbrMetallicRoughness"]["baseColorTexture"]["index"]
    png, size = chart(data(c), records, stage)
    binary.extend(b"\x00" * (-len(binary) % 4))
    vi = len(d["bufferViews"])
    d["bufferViews"].append(dict(buffer=0, byteOffset=len(binary), byteLength=len(png)))
    binary.extend(png)
    im = len(d["images"])
    d["images"].append(
        dict(
            name="Physical 1.95 cm stitchgrass soil",
            mimeType="image/png",
            bufferView=vi,
        )
    )
    tex = len(d["textures"])
    d["textures"].append(dict(source=im))
    ground_m = len(d["materials"])
    material = copy.deepcopy(orig)
    material["name"] = "Original ground pigment with native-root thatch"
    material["pbrMetallicRoughness"]["baseColorTexture"]["index"] = tex
    d["materials"].append(material)
    a, b, c, e = BOX
    uv = np.stack(((points[:, 0] - a) / (b - a), (points[:, 2] - c) / (e - c)), axis=1)
    inside = ((uv >= -0.00001) & (uv <= 1.00001)).all(1)[tri].all(1)

    def prim(ii, mat, attrs):
        return dict(
            attributes=attrs,
            material=mat,
            indices=emit(np.asarray(ii, dtype="<u2"), 5123, "SCALAR", indices=True),
        )

    new_ground = node(
        "Same native surface / broad + close pigments",
        [
            prim(tri[~inside].ravel(), attrib["material"], attrib["attributes"]),
            prim(
                tri[inside].ravel(),
                ground_m,
                dict(
                    attrib["attributes"],
                    TEXCOORD_0=emit(np.asarray(uv, "<f4"), 5126, "VEC2"),
                ),
            ),
        ],
    )
    # Remove the exposed rectangular *underside* of the crop's old full
    # y=0 sheet. No water on dry ground; submerge exactly on the real edge.
    sea_id = next(
        i for i, n in enumerate(d["nodes"]) if n["name"] == "True zero sea level"
    )
    sea_pr = d["meshes"][d["nodes"][sea_id]["mesh"]]["primitives"][0]
    sp = []
    sf = []
    for f in tri:
        poly = []
        for k, l in zip(f, np.roll(f, -1)):
            v, w = points[k], points[l]
            if v[1] <= 0:
                poly.append(v)
            if (v[1] < 0) != (w[1] < 0) and abs(w[1] - v[1]) > 1e-9:
                poly.append(v + (w - v) * (-v[1] / (w[1] - v[1])))
        if len(poly) < 3:
            continue
        off = len(sp)
        sp.extend([[t[0], 0.0, t[2]] for t in poly])
        sf.extend([[off, off + i, off + i + 1] for i in range(1, len(poly) - 1)])
    sp = np.asarray(sp, "<f4")
    A, B, C, D = CUT
    su = np.stack(((sp[:, 0] - A) / (B - A), (sp[:, 2] - C) / (D - C)), axis=1)
    water = prim(
        np.array(sf).ravel(),
        sea_pr["material"],
        dict(
            POSITION=emit(sp, 5126, "VEC3", bounds=True),
            NORMAL=emit(np.tile(np.float32([0, 1, 0]), (len(sp), 1)), 5126, "VEC3"),
            TEXCOORD_0=emit(su.astype("<f4"), 5126, "VEC2"),
        ),
    )
    d["meshes"][d["nodes"][sea_id]["mesh"]]["primitives"] = [water]

    stamp = len(d["materials"])
    d["materials"].append(
        dict(
            name="Folded stitchgrass and ochre sheaths",
            doubleSided=True,
            pbrMetallicRoughness=dict(metallicFactor=0.0, roughnessFactor=0.78),
        )
    )
    stats = {
        name: dict(roots=0, triangles=0)
        for name in ("folded", "silhouette", "guardian")
    }
    grouped = {}
    for r in records:
        x, z = float(r["x"]), float(r["z"])
        form = int(r["form"])
        close = math.hypot(x - 366.2, z + 83.7) < 6.0
        grouped.setdefault((form, "folded" if close else "silhouette"), []).append(r)
        grouped.setdefault((form, "guardian"), []).append(r)
    draws = []
    far = []
    for (form, name), rows in sorted(grouped.items()):
        p, n, col, ii = shape(
            form, "guardian" if name == "guardian" else name == "folded"
        )
        normal = lambda r: np.array([-float(r["dx"]), 1.0, -float(r["dz"])])
        trans = []
        quats = []
        scales = []
        for r in rows:
            nn = normal(r)
            nn /= np.linalg.norm(nn)
            ax = np.cross([0, 1, 0], nn)
            ang = math.acos(nn[1])
            turn = Rotation.from_rotvec(
                ax * ang / max(np.linalg.norm(ax), 1e-8)
            ) * Rotation.from_rotvec([0, float(r["wind_yaw"]), 0])
            trans.append([float(r["x"]), float(r["native_y"]), float(r["z"])])
            quats.append(turn.as_quat())
            scales.append([float(r["size"])] * 3)
        attrs = dict(
            TRANSLATION=emit(np.array(trans, "<f4"), 5126, "VEC3", bounds=True),
            ROTATION=emit(np.array(quats, "<f4"), 5126, "VEC4"),
            SCALE=emit(np.array(scales, "<f4"), 5126, "VEC3"),
        )
        (far if name == "guardian" else draws).append(
            node(
                "Stitchgrass " + str(form + 1) + " / " + name,
                [
                    prim(
                        ii.ravel(),
                        stamp,
                        dict(
                            POSITION=emit(p, 5126, "VEC3", bounds=True),
                            NORMAL=emit(n, 5126, "VEC3"),
                            COLOR_0=emit(col, 5126, "VEC3"),
                        ),
                    )
                ],
                attrs,
            )
        )
        stats[name]["roots"] += len(rows)
        stats[name]["triangles"] += len(ii) * len(rows)
    from litter import make as seat_litter

    old_woody = []
    for oi in d["scenes"][1]["nodes"]:
        source_node = d["nodes"][oi]
        if "windwillow" not in source_node["name"]:
            continue
        ix = source_node["extensions"]["EXT_mesh_gpu_instancing"]["attributes"][
            "TRANSLATION"
        ]
        old_woody.extend([dict(x=x, z=z) for x, y, z in values(d, raw, ix)])
    lp, ln, lc, li, litter_counts = seat_litter(points, tri, records, old_woody)
    litter_mat = len(d["materials"])
    d["materials"].append(
        dict(
            name="Spent hulls, fallen blades and weathered grit",
            doubleSided=True,
            pbrMetallicRoughness=dict(metallicFactor=0.0, roughnessFactor=0.91),
        )
    )
    litter_node = node(
        "True-soil litter, six-metre meadow material",
        [
            prim(
                li.ravel(),
                litter_mat,
                dict(
                    POSITION=emit(lp, 5126, "VEC3", bounds=True),
                    NORMAL=emit(ln, 5126, "VEC3"),
                    COLOR_0=emit(lc, 5126, "VEC3"),
                ),
            )
        ],
    )
    d["scenes"].append(
        dict(
            name="Rooted lee meadow / new surface and folded grass",
            nodes=[new_ground, *d["scenes"][1]["nodes"][1:], *draws, litter_node],
        )
    )
    d["scene"] = 2
    d["scenes"].append(
        dict(
            name="Rooted lee meadow / guardian silhouettes",
            nodes=[new_ground, *d["scenes"][1]["nodes"][1:], *far],
        )
    )
    # Matched second pose from the exact same eye, now looking down-slope-
    # left at the first genuine registered willow and its nearest grass shoot.
    eye = next(q for q in d["nodes"] if q.get("camera") == 0)["translation"]
    aim = [367.55, 13.30, -84.65]
    forward = np.array(aim) - eye
    forward /= np.linalg.norm(forward)
    right = np.cross(forward, [0, 1, 0])
    right /= np.linalg.norm(right)
    up = np.cross(right, forward)
    view = copy.deepcopy(d["cameras"][0])
    view["name"] = "Same 1.04 m eye, glancing at the stitchgrass"
    view["perspective"]["yfov"] = 2 * math.atan((36 / 34) * (500 / 840) / 2)
    ci = len(d["cameras"])
    d["cameras"].append(view)
    ni = len(d["nodes"])
    d["nodes"].append(
        dict(
            name=view["name"],
            camera=ci,
            translation=eye,
            rotation=Rotation.from_matrix(np.column_stack((right, up, -forward)))
            .as_quat()
            .tolist(),
        )
    )
    for state in d["scenes"]:
        state["nodes"].append(ni)
    note = dict(
        new_shoots=len(records),
        added_kit_meshes=len(draws),
        placed_triangles=stats,
        ground_litter_pieces=litter_counts,
        near_only_litter_triangles=len(li),
        source_primitive_triangles=len(source_tri),
        actual_V1_slope_triangles=len(tri),
        excluded_vertical_streamer_skirt_faces=int((~surface).sum()),
        real_ground_triangles_inside=int(inside.sum()),
        true_wet_triangles=len(sf),
        close_chart_m=[b - a, e - c],
        close_chart_px=size,
        ground_vertices_moved=0,
        matched_second_eye=dict(eye=eye, aim=aim, lens=34),
    )
    d["extras"]["stitchgrass"] = note
    d["buffers"][0]["byteLength"] = len(binary)
    save(d, binary, stage / "Twin-Needle-Stitchgrass.glb")
    from plant_only import extract

    for label, rows in [("eye", [*draws, litter_node]), ("guardian", far)]:
        extract(d, binary, rows, stage / ("stitchgrass-" + label + ".glb"))
    (stage / "kit.json").write_text(json.dumps(note, indent=2) + "\n")
    return note


if __name__ == "__main__":
    print(add(*(Path(p) for p in sys.argv[1:])))
