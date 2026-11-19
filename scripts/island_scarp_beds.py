"""Expose angled bedrock on the *existing* hillside of three planted V1 scarps.

Artistic material only: never a raised platform, deep backing wall or skirt.
The study divides selected real existing triangle faces to carry pigment and a
subtle mineral partial normal; every position stays on its original triangle.
"""

import copy, json
from pathlib import Path
import numpy as np
from finished_island_glb import parse, save, values
from island_scarp_seating import Foot
from island_inlet_ground import ease


def unit(v):
    return v / np.maximum(1e-8, np.linalg.norm(v, axis=-1, keepdims=True))


class Exposure:
    def __init__(self, terrain, prop, rock, index):
        foot = Foot(terrain, prop, rock)
        # These are *actually* supporting parts of the old uphill footprint.
        # The lower empty part below the old box supplies no deposit seeds.
        xz = (foot.touch * foot.scale) @ foot.rot + foot.center[[0, 2]]
        fy, dx, dz = foot.terrain.floor(xz[:, 0], xz[:, 1])
        yxz = np.stack((xz[:, 0], fy, xz[:, 1]), -1)
        self.p = np.median(yxz, axis=0)
        grad = np.array((np.median(dx), np.median(dz)))
        steep = np.linalg.norm(grad)
        horizontal = grad / max(steep, 0.01)
        self.strike = np.array((-horizontal[1], 0, horizontal[0]))
        self.dip = unit(np.array((-horizontal[0], -steep, -horizontal[1])))
        self.normal = unit(np.cross(self.dip, self.strike))
        self.phase = (index % 31) * 0.21
        self.size = prop["scale"]
        self.roughness = steep

    def ink(self, p, original, normals):
        rel = p - self.p
        # Use metres ALONG the 3D slope, not an x/z footprint which could
        # otherwise smear material forty vertical metres across the notch.
        a = rel @ self.strike
        v = rel @ self.dip
        off = rel @ self.normal
        phase = self.phase
        # Two unequal eroded beds continue into the hillside beside and above
        # the rock. Interrupted, meandering seams, not a halo under the prop.
        along = a + v * 0.37
        bend = 0.56 * np.sin(a * 0.38 + phase) + 0.30 * np.sin(a * 0.94 - v * 0.18)
        layers = v - 0.19 * a - bend
        cross = np.exp(-((off / (2.2 + 0.03 * np.abs(a))) ** 2))
        fan = np.exp(-(((along - 1.8) / 8.9) ** 2)) * np.exp(
            -(((layers + 1.3) / 3.2) ** 2)
        )
        broken = np.exp(-(((along + 4.0) / 5.3) ** 2)) * np.exp(
            -(((layers - 2.4) / 1.5) ** 2)
        )
        scab = np.clip(
            0.71
            + 0.20 * np.sin(0.62 * a + 0.47 * v + phase)
            + 0.15 * np.cos(1.08 * a - 0.24 * v),
            0.21,
            1,
        )
        weight = np.clip((fan * 0.95 + broken * 0.75) * cross * scab, 0, 0.90)
        window = (1 - ease(11, 15, np.abs(a))) * (1 - ease(7.2, 11, np.abs(v)))
        weight *= window
        # Existing native stone vertex pigments and its mineral texture both
        # center near 220/255 sRGB in the latter atlas, as does the terrain
        # substrate. Preserve grass and hillside texture outside the exposure.
        split = np.exp(-(((layers + 0.9 + 0.31 * np.sin(a * 0.5)) / 0.30) ** 2)) * (
            0.35 + 0.65 * ease(-6, 0, a)
        )
        chip = np.exp(-(((layers - 1.35 + 0.23 * np.cos(a * 0.68)) / 0.24) ** 2)) * (
            1 - ease(0, 5, a)
        )
        rusty = ease(
            -0.1,
            0.84,
            np.sin(a * 0.58 + v * 0.25 + phase) * 0.58
            + np.cos(0.66 * v - 0.32 * a) * 0.39,
        )
        target = np.stack(
            (0.164 + 0.072 * rusty, 0.180 + 0.032 * rusty, 0.188 + 0.008 * rusty), -1
        )
        target *= 1 - 0.27 * split[:, None] - 0.15 * chip[:, None]
        # Narrow, pale mineral rims have an interrupted adjacent break rather
        # than a shadow painted permanently on one compass-facing side.
        rim = np.exp(-(((layers + 0.56 + 0.31 * np.sin(a * 0.5)) / 0.31) ** 2)) * ease(
            -5, 1, a
        )
        target += rim[:, None] * np.array((0.024, 0.018, 0.013))
        rgb = original[:, :3] * (1 - weight[:, None]) + target * weight[:, None]
        # Small, neutral material relief atop the existing physical normal.
        # A diffuse adjustment on its own would read as a smudge; use the
        # signed derivative of local weathered bedding on the real face.
        slope = (
            split * (layers + 0.9 + 0.31 * np.sin(a * 0.5)) * 0.46
            + chip * (layers - 1.35 + 0.23 * np.cos(a * 0.68)) * 0.35
        ) * weight
        norm = unit(normals + slope[:, None] * self.dip)
        return np.column_stack((rgb, original[:, 3])), norm


def expose(habitat, terrain, source_rock, props_path, destination, indices):
    d, buf = parse(Path(habitat))
    m, b = parse(Path(source_rock))
    pr = m["meshes"][0]["primitives"][0]
    rock = np.asarray(values(m, b, pr["attributes"]["POSITION"]))
    props = json.loads(Path(props_path).read_text())
    beds = {i: Exposure(terrain, props[i], rock, i) for i in indices}
    doc = copy.deepcopy(d)
    raw = bytearray()
    receipt = {}

    def add(a, component, kind, target=34962, norm=False, bound=False):
        a = np.ascontiguousarray(a)
        raw.extend(b"\0" * (-len(raw) % 4))
        offset = len(raw)
        raw.extend(a.tobytes())
        k = len(doc["bufferViews"])
        doc["bufferViews"].append(
            dict(buffer=0, byteOffset=offset, byteLength=a.nbytes, target=target)
        )
        acc = dict(bufferView=k, componentType=component, type=kind, count=len(a))
        if norm:
            acc["normalized"] = True
        if bound:
            acc.update(min=a.min(0).tolist(), max=a.max(0).tolist())
        j = len(doc["accessors"])
        doc["accessors"].append(acc)
        return j

    # Keep the original shared texture and all unmodified chunk bufferViews,
    # and replace streams only for genuinely changed habitat chunks.
    raw.extend(buf)
    for mesh in doc["meshes"]:
        prim = mesh["primitives"][0]
        att = prim["attributes"]
        meta = d["accessors"][att["POSITION"]]
        lo = np.array(meta["min"])
        hi = np.array(meta["max"])
        here = [
            (i, e)
            for i, e in beds.items()
            if np.all(e.p + 17 > lo) and np.all(e.p - 17 < hi)
        ]
        if not here:
            continue
        p, n, u, c = [
            np.asarray(values(d, buf, att[k]))
            for k in ("POSITION", "NORMAL", "TEXCOORD_0", "COLOR_0")
        ]
        faces = np.asarray(values(d, buf, prim["indices"])).reshape(-1, 3)
        fin = []
        P = [p]
        N = [n]
        U = [u]
        C = [c]
        count = len(p)

        def pigment(points, cols, normals):
            for _, e in here:
                cols, normals = e.ink(points, cols, normals)
            return cols, normals

        cent = p[faces].mean(1)
        selected = np.zeros(len(faces), dtype=bool)
        for _, e in here:
            selected |= np.linalg.norm(cent - e.p, axis=1) < 17.3
        # A vertical mesh closing triangle is not a traversable native face.
        xy = p[faces][:, :, [0, 2]]
        v = xy[:, 1] - xy[:, 0]
        w = xy[:, 2] - xy[:, 0]
        selected &= np.abs(v[:, 0] * w[:, 1] - v[:, 1] * w[:, 0]) > 0.01
        if not selected.any():
            continue
        fin.extend(faces[~selected].tolist())
        resolution = 8
        weights = []
        slices = []
        idmap = {}
        for j in range(resolution + 1):
            for i in range(resolution + 1 - j):
                idmap[i, j] = len(weights)
                weights.append(
                    (1 - (j + i) / resolution, i / resolution, j / resolution)
                )
        for j in range(resolution):
            for i in range(resolution - j):
                slices.append([idmap[i, j], idmap[i + 1, j], idmap[i, j + 1]])
                if j + i + 2 <= resolution:
                    slices.append(
                        [idmap[i + 1, j], idmap[i + 1, j + 1], idmap[i, j + 1]]
                    )
        weights, slices = np.array(weights), np.array(slices)
        for ids in faces[selected]:
            pos = weights @ p[ids]
            nor = unit(weights @ n[ids])
            col_new, nor = pigment(pos, weights @ c[ids], nor)
            P.append(pos)
            N.append(nor)
            U.append(weights @ u[ids])
            C.append(col_new)
            fin.extend((count + slices).tolist())
            count += len(pos)
        P, N, U, C = map(np.concatenate, (P, N, U, C))
        att.update(
            POSITION=add(P.astype("<f4"), 5126, "VEC3", bound=True),
            NORMAL=add(N.astype("<f4"), 5126, "VEC3"),
            TEXCOORD_0=add(U.astype("<f4"), 5126, "VEC2"),
            COLOR_0=add(
                np.uint8(np.rint(np.clip(C, 0, 1) * 255)), 5121, "VEC4", norm=True
            ),
        )
        faces_out = np.asarray(fin, dtype="<u2" if count < 65536 else "<u4")
        prim["indices"] = add(
            faces_out.ravel(), 5123 if count < 65536 else 5125, "SCALAR", 34963
        )
        receipt[mesh["name"]] = {
            "actual_original_faces_repainted": int(selected.sum()),
            "subfaces_on_identical_planes": int(selected.sum() * resolution**2),
        }
    doc["buffers"][0]["byteLength"] = len(raw)
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    save(doc, raw, destination)
    return {
        "scope": "Actual near terrain triangles and original UV/material; localized mineral pigment and shallow shading relief only, no moved terrain or new foundations.",
        "changed_chunks": receipt,
    }
