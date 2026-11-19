"""Self-contained two-state GLB art desk on authentic original geometry. No game/Blender runtime."""

from pathlib import Path
import copy, sys, json, math, io
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H), str(H.parents[2] / "scripts")]
from finished_island_glb import parse, save, values, payload
from island_erratic_glb import streams
from portable_ground import bake, CUT, sample


def make(native, scene, hollow, shelf, field):
    arrays, images, meta = bake(scene, native, hollow, field)
    A, B, C, D = CUT
    doc = dict(
        asset=dict(
            version="2.0",
            generator="Twin Needle Rise / real original-ground portable artist field cut",
        ),
        scene=1,
        scenes=[],
        nodes=[],
        meshes=[],
        materials=[],
        accessors=[],
        bufferViews=[],
        images=[],
        textures=[],
        cameras=[],
        extensionsUsed=["EXT_mesh_gpu_instancing"],
        extensionsRequired=["EXT_mesh_gpu_instancing"],
    )
    buf = bytearray()
    emit = streams(doc, buf)

    def put_image(data, name):
        buf.extend(b"\0" * (-len(buf) % 4))
        vid = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(buffer=0, byteOffset=len(buf), byteLength=len(data))
        )
        buf.extend(data)
        imgid = len(doc["images"])
        doc["images"].append(dict(name=name, mimeType="image/png", bufferView=vid))
        idx = len(doc["textures"])
        doc["textures"].append(dict(source=imgid))
        return idx

    def matte(name, img, rough=0.8):
        i = len(doc["materials"])
        doc["materials"].append(
            dict(
                name=name,
                doubleSided=True,
                pbrMetallicRoughness=dict(
                    baseColorTexture=dict(index=img),
                    roughnessFactor=rough,
                    metallicFactor=0.0,
                ),
            )
        )
        return i

    def meshnode(name, prims, extra=None):
        j = len(doc["meshes"])
        doc["meshes"].append(dict(name=name, primitives=prims))
        k = len(doc["nodes"])
        doc["nodes"].append(dict(name=name, mesh=j, **(extra or {})))
        return k

    p, n, uv, tri = arrays
    a0 = emit(p, 5126, "VEC3", bounds=True)
    a1 = emit(n, 5126, "VEC3")
    a2 = emit(uv, 5126, "VEC2")
    ix = emit(tri, 5123, "SCALAR", indices=True)
    grounds = []
    for name, paint in zip(
        ("Original Hollow pigment", "Proposed lee/rise pigment"), images
    ):
        mat = matte(name, put_image(paint, name))
        grounds.append(
            meshnode(
                name,
                [
                    dict(
                        attributes=dict(POSITION=a0, NORMAL=a1, TEXCOORD_0=a2),
                        indices=ix,
                        material=mat,
                    )
                ],
            )
        )
    common = []
    extras = []
    # Restrict the ordinary iridescent V1 ocean diagram to real y=0, NOT a
    # coloured false shelf or hand-shaped shoreline. Original bed clips it.
    src = np.array(Image.open(shelf / "shore.png").convert("RGBA"))
    sea = json.loads((shelf / "water.json").read_text())
    X, Z = np.meshgrid(np.linspace(A, B, 153), np.linspace(C, D, 93))
    ox, oz = sea["origin_xz"]
    sx, sz = sea["span_xz"]
    rgba = sample(src, (X - ox) / sx, (Z - oz) / sz)
    wat = io.BytesIO()
    Image.fromarray(np.uint8(np.clip(rgba[:, :, :3] * 255, 0, 255))).save(
        wat, format="PNG"
    )
    mat = matte(
        "Original native waterline / stock colour",
        put_image(wat.getvalue(), "Original optical colour only"),
        0.36,
    )
    aa = emit(
        np.array([[A, 0, C], [B, 0, C], [B, 0, D], [A, 0, D]], "<f4"),
        5126,
        "VEC3",
        bounds=True,
    )
    ab = emit(np.array([[0, 0], [1, 0], [1, 1], [0, 1]], "<f4"), 5126, "VEC2")
    ac = emit(np.array([[0, 1, 0]] * 4, "<f4"), 5126, "VEC3")
    ii = emit(np.array([0, 2, 1, 0, 3, 2], "<u2"), 5123, "SCALAR", indices=True)
    common.append(
        meshnode(
            "True zero sea level",
            [
                dict(
                    attributes=dict(POSITION=aa, NORMAL=ac, TEXCOORD_0=ab),
                    indices=ii,
                    material=mat,
                )
            ],
        )
    )

    def importer(file):
        d, raw = parse(file)
        mats = []
        textures = []
        for t in d.get("textures", []):
            im = d["images"][t["source"]]
            if "uri" in im:
                local = file.parent / im["uri"]
                data = local.read_bytes()
            else:
                v = d["bufferViews"][im["bufferView"]]
                o = v.get("byteOffset", 0)
                data = raw[o : o + v["byteLength"]]
            textures.append(put_image(data, im.get("name", im.get("uri", "Image"))))
        for mat in d["materials"]:
            m = copy.deepcopy(mat)
            for dic in (m, m.get("pbrMetallicRoughness", {})):
                for k, v in dic.items():
                    if k.endswith("Texture") and isinstance(v, dict):
                        v["index"] = textures[v["index"]]
            mats.append(len(doc["materials"]))
            doc["materials"].append(m)
        cache = {}
        meshes = {}

        def ac(i):
            if i in cache:
                return cache[i]
            q = d["accessors"][i]
            typ = q["componentType"]
            dtype = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[typ]
            a = np.frombuffer(payload(d, raw, i), dtype=dtype).reshape(q["count"], -1)
            idx = emit(
                a,
                typ,
                q["type"],
                indices=q["type"] == "SCALAR",
                bounds="min" in q,
                normalized=q.get("normalized", False),
            )
            cache[i] = idx
            return idx

        def mesh(node):
            i = node["mesh"]
            if i not in meshes:
                m = d["meshes"][i]
                pr = []
                for p in m["primitives"]:
                    pr.append(
                        dict(
                            indices=ac(p["indices"]),
                            attributes={k: ac(a) for k, a in p["attributes"].items()},
                            material=mats[p["material"]],
                        )
                    )
                meshes[i] = len(doc["meshes"])
                doc["meshes"].append(
                    dict(name=m.get("name", node["name"]), primitives=pr)
                )
            return meshes[i]

        return d, raw, mesh

    def node(name, mesh, **extra):
        i = len(doc["nodes"])
        doc["nodes"].append(dict(name=name, mesh=mesh, **extra))
        return i

    props = json.loads((native / "props.json").read_text())
    crowns = json.loads((native / "grove-crowns.json").read_text())
    td, tr, tree = importer(native / "grove-crowns.glb")
    sources = {a["name"]: a for a in td["nodes"]}
    mature = []
    for i, r in enumerate(props):
        x, y, z = r["position"]
        if not (A <= x <= B and C <= z <= D):
            continue
        if r["kind"] != "Conifer" or str(i) not in crowns:
            raise ValueError(
                "New originally placed source kind in this cut; don't drop or substitute it"
            )
        ref = sources["PineForm" + str(crowns[str(i)]["form"]) + "_L1"]
        jj = node(
            "Original pine #" + str(i),
            tree(ref),
            translation=r["position"],
            rotation=[0, math.sin(r["yaw"] * 0.5), 0, math.cos(r["yaw"] * 0.5)],
            scale=[r["scale"]] * 3,
        )
        common.append(jj)
        mature.append(i)

    def plants(file, old):
        pd, pr, pmesh = importer(file)
        count = 0
        for n in pd["nodes"]:
            att = n["extensions"]["EXT_mesh_gpu_instancing"]["attributes"]
            t = np.array(values(pd, pr, att["TRANSLATION"]), "<f4")
            f = (t[:, 0] >= A) & (t[:, 0] <= B) & (t[:, 2] >= C) & (t[:, 2] <= D)
            if not f.any():
                continue
            trans = {}
            count += int(f.sum())
            for name, a in att.items():
                dt = np.array(values(pd, pr, a), "<f4")[f]
                trans[name] = emit(
                    dt,
                    5126,
                    "VEC4" if name == "ROTATION" else "VEC3",
                    bounds=name == "TRANSLATION",
                )
            j = node(
                ("Held " if old else "Proposed ") + n["name"],
                pmesh(n),
                extensions=dict(EXT_mesh_gpu_instancing=dict(attributes=trans)),
            )
            (common if old else extras).append(j)
        return count

    old = plants(hollow / "registered-guardian.glb", True)
    new = plants(field / "rise-eye.glb", False)

    def camera(name, eye, aim, lens, span):
        origin = np.array(eye)
        v = np.array(aim) - origin
        v /= np.linalg.norm(v)
        right = np.cross(v, [0, 1, 0])
        right /= np.linalg.norm(right)
        up = np.cross(right, v)
        from scipy.spatial.transform import Rotation

        rot = Rotation.from_matrix(np.column_stack([right, up, -v])).as_quat().tolist()
        i = len(doc["cameras"])
        c = dict(name=name, type="perspective" if not span else "orthographic")
        if span:
            c["orthographic"] = dict(
                xmag=span / 2, ymag=span / 2 * 500 / 840, znear=0.05, zfar=1300
            )
        else:
            c["perspective"] = dict(
                yfov=2 * math.atan((36 / lens) * (500 / 840) / 2),
                aspectRatio=840 / 500,
                znear=0.05,
                zfar=1300,
            )
        doc["cameras"].append(c)
        j = len(doc["nodes"])
        doc["nodes"].append(dict(name=name, camera=i, translation=eye, rotation=rot))
        common.append(j)

    # Use the same source-grid height and poses as the pending photo, not a
    # camera chosen to exaggerate the intervention after the fact.
    sys.path.insert(0, str(H.parents[2] / "scripts"))
    from island_inlet_ground import Inlet

    hh = float(Inlet(native / "island-v1.bin").floor(366.2, -83.7)[0])
    camera(
        "Inside the lee, 1.04 m",
        [366.2, hh + 1.04, -83.7],
        [385.36, 22.34, -82.1],
        28,
        0,
    )
    camera(
        "Regional section, 129 m", [351.0, 82.0, 11.0], [372.1, 13.2, -82.4], 40, 129.0
    )
    doc["scenes"] = [
        dict(name="Original held community", nodes=[grounds[0], *common]),
        dict(name="Proposed continuous rise", nodes=[grounds[1], *common, *extras]),
    ]
    doc["extras"] = dict(
        description="Editable native-topology V1 field cut, colour study only. No sky/backdrop; authored normal grain not baked, not native gameplay.",
        records=dict(
            original_conifers=mature, original_hollow_roots=old, new_roots=new
        ),
        **meta
    )
    doc["buffers"] = [dict(byteLength=len(buf))]
    save(doc, buf, field / "Twin-Needle-Rise.glb")
    (field / "portable.json").write_text(json.dumps(doc["extras"], indent=2) + "\n")
    return doc["extras"]


if __name__ == "__main__":
    print(make(*(Path(p) for p in sys.argv[1:6])))
