"""An exposed rock heel inside an unchanged *solid* original BoulderA box.

Unlike loose shingle, never drape or lower the continuum below the saved
opaque obstacle. Find where a genuine rising triangle crosses each original
body's projected footprint. Only that supported part can be a wider socket;
the downhill free side must bevel back into it or remain an honest overhang.
Outputs one keyed artist mesh, not a new generic BoulderA.
"""

from pathlib import Path
from copy import deepcopy
import math, struct
import numpy as np
from scipy.spatial import ConvexHull, cKDTree
from finished_island_glb import parse, save, values
from island_erratic_blocks import unit
from island_talus_contact import members
from island_inlet_ground import Inlet, ease
from island_erratic_glb import streams
from island_erratic_rind import project


def binding(surface, prop, xyz):
    x, y, z = prop["position"]
    sc = prop["scale"]
    a = prop["yaw"]
    rot = np.array([[math.cos(a), -math.sin(a)], [math.sin(a), math.cos(a)]])
    lo, hi = xyz.min(0), xyz.max(0)
    corners = np.array(
        [[xx, zz] for xx in [lo[0], hi[0]] for zz in [lo[2], hi[2]]]
    ) @ rot * sc + [x, z]
    target = np.r_[corners.min(0), corners.max(0), y + lo[1] * sc, y + hi[1] * sc]
    raw = Path(surface).read_bytes()
    w, d, _, _, _, num = struct.unpack_from("<IIfffI", raw, 8)
    boxes = np.frombuffer(raw, "<f4", num * 6, 32 + w * d * 4).reshape(-1, 6)
    delta = np.max(np.abs(boxes - target), axis=1)
    ix = int(delta.argmin())
    if delta[ix] > 0.012:
        raise ValueError(
            "Native obstacle does not match the source stone; this is not loose shingle"
        )
    old = Inlet(surface)

    def ground(xz):
        q = np.atleast_2d(xz) @ rot * sc + [x, z]
        return (old.floor(q[:, 0].copy(), q[:, 1].copy())[0] - y) / sc

    return ground, boxes[ix], ix


def fit(points, norms, colors, ground, level, key):
    q, ids = np.unique(np.round(points, 6), axis=0, return_index=True)
    old_n, old_c = norms[ids], colors[ids]
    xz = q[:, [0, 2]]
    rim = ConvexHull(xz)
    lo, hi = q.min(0), q.max(0)
    height = hi[1] - lo[1]
    # Candidate socket sites lie INSIDE the original projected body, not a
    # displaced toe, halo or paint darkening outside the physical volume.
    num = max(19, int(47 * np.max(np.ptp(xz, axis=0))))
    xx, zz = np.meshgrid(np.linspace(lo[0], hi[0], num), np.linspace(lo[2], hi[2], num))
    grid = np.c_[xx.ravel(), zz.ravel()]
    inp = grid[(grid @ rim.equations[:, :2].T + rim.equations[:, 2] < -0.006).all(1)]
    substrate = ground(inp)
    contact = inp[substrate >= lo[1] + 0.024]
    if len(contact) < 8:
        raise ValueError(
            "Projected stone has no substantial grounded socket; leave it unmounted"
        )
    keyfoot = ConvexHull(contact)
    if keyfoot.volume / rim.volume < 0.085:
        raise ValueError("Insufficient supported area for a solid rock heel")
    # Use long natural external facets, not the steps of our sampling grid.
    hub = contact[keyfoot.vertices].mean(0)
    outline = contact[keyfoot.vertices]
    dirs = np.column_stack(
        [
            np.cos(np.linspace(0.12, math.tau + 0.12, 16, endpoint=False)),
            np.sin(np.linspace(0.12, math.tau + 0.12, 16, endpoint=False)),
        ]
    )
    foot = outline[np.unique(np.argmax((outline - hub) @ dirs.T, axis=0))]
    hf = ground(foot)
    base = np.maximum(lo[1], np.minimum(hf - 0.044, lo[1] + height * 0.12))
    foot3 = np.c_[foot[:, 0], base, foot[:, 1]]
    # Keep the old contour from middle shoulders to peak. The actual downhill
    # root has a much steeper, lower fractured face, not the rounded old belly.
    boundary = lo[1] + height * (0.46 if level == 0 else 0.43)
    retained = q[q[:, 1] >= boundary]
    old_sole = q[q[:, 1] <= lo[1] + height * 0.02 + 0.0001]
    keep_sole = old_sole[ground(old_sole[:, [0, 2]]) >= old_sole[:, 1] + 0.012]
    # Compose a mid-height broken waterline, where real rock continues into
    # the bank. Its X/Z still lies inside the convex projection and all
    # extrema stay in the native saved box. No stone is extruded into air below it.
    ext = foot - hub
    seam = (
        hub
        + ext
        * (0.97 + 0.015 * np.sin(foot[:, 0] * 6.0 + foot[:, 1] * 4.0 + key))[:, None]
    )
    seam_y = np.minimum(
        boundary - 0.005, np.maximum(ground(seam) + height * 0.13, base + height * 0.18)
    )
    shelf = np.c_[seam[:, 0], seam_y, seam[:, 1]]
    new = np.unique(np.round(np.vstack([retained, foot3, keep_sole, shelf]), 6), axis=0)
    h = ConvexHull(new)
    v = new[h.vertices]
    index = np.full(len(new), -1)
    index[h.vertices] = np.arange(len(v))
    t = index[h.simplices]
    fn = np.cross(v[t[:, 1]] - v[t[:, 0]], v[t[:, 2]] - v[t[:, 0]])
    rev = np.einsum("ij,ij->i", fn, v[t].mean(1) - v.mean(0)) < 0
    t[rev] = t[rev][:, [0, 2, 1]]
    fn = unit(np.cross(v[t[:, 1]] - v[t[:, 0]], v[t[:, 2]] - v[t[:, 0]]))
    normal = np.zeros_like(v)
    for j in range(3):
        np.add.at(normal, t[:, j], fn)
    normal = unit(normal)
    near = cKDTree(q)
    distance, m = near.query(v)
    blend = (1 - ease(0.001, 0.075, distance)) * ease(
        boundary * 0.8, boundary + height * 0.18, v[:, 1]
    )
    normal = unit((1 - blend[:, None]) * normal + blend[:, None] * old_n[m])
    # Actual V1 A/B/C each has 3 separate bodies in integrated glacial chart.
    uv = project(v, t, 0.91 * (ord("A") - 64) + 2.14 * key)
    return (
        v,
        t,
        normal,
        old_c[m],
        uv,
        dict(
            projected_body_area=float(rim.volume),
            available_socket_area=float(keyfoot.volume),
            true_bed_minmax=[float(substrate.min()), float(substrate.max())],
            visible_upper_held_vertices=len(retained),
            base_min=float(v[:, 1].min()),
        ),
    )


def mount(source, surface, prop, index, dest):
    if prop["kind"] != "BoulderA" or Path(source).resolve() == Path(dest).resolve():
        raise ValueError("Only the specifically authored real A study is supported")
    d, b = parse(source)
    out = deepcopy(d)
    out.update(meshes=[], bufferViews=[], accessors=[])
    buf = bytearray()
    put = streams(out, buf)
    report = {}
    pr = d["meshes"][0]["primitives"][0]
    ground, box, ob = binding(
        surface, prop, np.array(values(d, b, pr["attributes"]["POSITION"]))
    )
    for lod, m in enumerate(d["meshes"]):
        pr = m["primitives"][0]
        src, n, c = (
            np.array(values(d, b, pr["attributes"][k]))
            for k in ("POSITION", "NORMAL", "COLOR_0")
        )
        f = np.array(values(d, b, pr["indices"])).reshape(-1, 3)
        if lod == 2:
            p, n, u, c = (
                src,
                n,
                np.array(values(d, b, pr["attributes"]["TEXCOORD_0"])),
                c,
            )
            ii = f.ravel()
            notes = []
        else:
            ps = []
            ns = []
            us = []
            cs = []
            notes = []
            bodies = sorted(
                members(src, f), key=lambda ids: -np.prod(np.ptp(src[ids], axis=0))
            )
            if len(bodies) != 3:
                raise ValueError("Expected three original stone bodies.")
            for k, ids in enumerate(bodies):
                pp, t, nn, cc, uu, note = fit(src[ids], n[ids], c[ids], ground, lod, k)
                ps.extend(pp[t].reshape(-1, 3))
                ns.extend(nn[t].reshape(-1, 3))
                us.extend(uu)
                cs.extend(cc[t].reshape(-1, 4))
                notes.append(note)
            rows = np.column_stack((ps, ns, us, cs)).astype("<f4")
            rows, ii = np.unique(rows, axis=0, return_inverse=True)
            p, n, u, c = np.split(rows, [3, 6, 8], axis=1)
        name = f"BoulderASeat{index}_L{lod}"
        at = dict(
            POSITION=put(np.asarray(p, "<f4"), 5126, "VEC3", bounds=True),
            NORMAL=put(np.asarray(n, "<f4"), 5126, "VEC3"),
            TEXCOORD_0=put(np.asarray(u, "<f4"), 5126, "VEC2"),
            COLOR_0=put(
                np.uint8(np.round(np.clip(c, 0, 1) * 255)),
                5121,
                "VEC4",
                normalized=True,
            ),
        )
        out["meshes"].append(
            dict(
                name=name,
                primitives=[
                    dict(
                        attributes=at,
                        indices=put(
                            np.array(ii, dtype="<u2"), 5123, "SCALAR", indices=True
                        ),
                        material=pr["material"],
                    )
                ],
            )
        )
        for node in out["nodes"]:
            if node.get("mesh") == lod:
                node["name"] = name
        report[f"L{lod}"] = dict(triangles=len(ii) // 3, components=notes)
    out["buffers"] = [dict(byteLength=len(buf))]
    Path(dest).parent.mkdir(parents=True, exist_ok=True)
    save(out, buf, dest)
    return dict(
        anchor=index,
        physical_original_box=box.tolist(),
        physical_box_index=ob,
        prop=prop,
        levels=report,
        policy="Ordinary V1 authoring only. Saved obstacle, original parent transform, other rocks and physical terrain are unchanged. Every new heel is inside its original solid body projection and above the old floor, and intersects positive true ground.",
    )
