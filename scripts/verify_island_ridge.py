"""Measure the opt-in map without consulting its painter or authored weights."""

import argparse, hashlib, json, struct
from pathlib import Path
import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from build_island_geology import Glb, SURFACE, TERRAIN
from island_placement_corrections import CORRECTIONS


def read(path):
    raw = path.read_bytes()
    nx, nz, ox, oz, spacing, count = struct.unpack_from("<IIfffI", raw, 8)
    assert nx == 321 and nz == 385 and spacing == 3.125
    h = np.frombuffer(raw, "<f4", nx * nz, 32).reshape(nz, nx)
    b = np.frombuffer(raw, "<f4", count * 6, 32 + 4 * nx * nz).reshape(count, 6)
    return h, b, ox, oz, spacing


def floor(h, px, pz, ox, oz, s):
    u, v = (px - ox) / s, (pz - oz) / s
    i, j = np.floor(u).astype(int), np.floor(v).astype(int)
    u, v = u - i, v - j
    a, b, c, d = h[j, i], h[j, i + 1], h[j + 1, i + 1], h[j + 1, i]
    return np.where(
        u >= v, a + (b - a) * u + (c - b) * v, a + (c - d) * u + (d - a) * v
    )


def route(h, b, ox, oz, s):
    """A conservative graph: only wholly dry triangles; discard an entire cell
    if any expanded solid body footprint intersects it. Passing connections
    cross a complete shared edge rather than clipping a vertex or rock."""
    a, m, n, d = h[:-1, :-1], h[:-1, 1:], h[1:, 1:], h[1:, :-1]
    grade = np.stack((np.hypot(m - a, n - m), np.hypot(n - d, d - a)), axis=-1) / s
    dry = (
        np.stack(
            (np.minimum(np.minimum(a, n), m), np.minimum(np.minimum(a, n), d)), axis=-1
        )
        >= -0.6
    )
    grade_ok = (grade <= 0.85) & dry
    keep = grade_ok.copy()
    for x, z, xx, zz, *_ in b:
        i, ii = np.floor((np.array((x - 0.2, xx + 0.2)) - ox) / s).astype(int)
        j, jj = np.floor((np.array((z - 0.2, zz + 0.2)) - oz) / s).astype(int)
        keep[max(0, j) : jj + 1, max(0, i) : ii + 1, :] = False
    flat = keep.ravel()
    ids = np.arange(flat.size, dtype=np.int32).reshape(keep.shape)
    l = np.concatenate(
        (ids[:, :, 0].ravel(), ids[:, :-1, 0].ravel(), ids[1:, :, 0].ravel())
    )
    r = np.concatenate(
        (ids[:, :, 1].ravel(), ids[:, 1:, 1].ravel(), ids[:-1, :, 1].ravel())
    )
    passable = flat[l] & flat[r]
    graph = coo_matrix(
        (np.ones(np.count_nonzero(passable), dtype=bool), (l[passable], r[passable])),
        shape=(len(flat), len(flat)),
    ).tocsr()
    _, labels = connected_components(graph, directed=False)

    def face(x, z):
        u, v = (x - ox) / s, (z - oz) / s
        return ids[int(v), int(u), int(v % 1 >= u % 1)]

    nursery = face(80, 250)
    assert flat[nursery]
    return (
        grade_ok,
        (labels == labels[nursery]).reshape(keep.shape),
        [
            bool(labels[face(x, z)] == labels[nursery])
            for x, z in [
                (80, 250),
                (99.9, 256.9),
                (98.3, 253.1),
                (100, 270),
                (85, 280),
                (60, 235),
                (120, 230),
            ]
        ],
    )


def verify(out):
    old, bo, ox, oz, s = read(SURFACE)
    new, bn, *_ = read(out / "island-ridge-v2.bin")
    assert (
        hashlib.sha256(SURFACE.read_bytes()).hexdigest()
        == "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
    )
    assert new.max() == 240.0 and np.all((old > 105) | (old == new))
    go, reach_o, walk_o = route(old, bo, ox, oz, s)
    gn, reach_n, walk_n = route(new, bn, ox, oz, s)
    np.testing.assert_array_equal(reach_o, reach_n)
    assert walk_o == walk_n and walk_n[0] and walk_n[1]
    mo = json.loads((TERRAIN.parent / "props.json").read_text())
    mn = json.loads((out / "props-ridge-v2.json").read_text())
    p = np.array([a["position"] for a in mo])
    q = np.array([a["position"] for a in mn])
    for a, b in zip(mo, mn, strict=True):
        assert (
            a["kind"] == b["kind"] and a["yaw"] == b["yaw"] and a["scale"] == b["scale"]
        )
    np.testing.assert_array_equal(p[:, [0, 2]], q[:, [0, 2]])
    y0 = floor(old, p[:, 0], p[:, 2], ox, oz, s)
    y1 = floor(new, q[:, 0], q[:, 2], ox, oz, s)
    np.testing.assert_allclose(q[:, 1] - y1, p[:, 1] - y0, atol=3e-5)
    np.testing.assert_array_equal(bo[:, :4], bn[:, :4])
    np.testing.assert_allclose(bo[:, 5] - bo[:, 4], bn[:, 5] - bn[:, 4], atol=4e-5)
    for idx in CORRECTIONS:
        np.testing.assert_array_equal(q[idx], p[idx])
    g = Glb(out / "terrain-chunks.glb")
    tc = [0, 0, 0]
    for mesh in g.doc["meshes"]:
        lod = int(mesh["name"][-1])
        prim = mesh["primitives"][0]
        pos = g.attr(prim["attributes"]["POSITION"])
        gx = np.rint((pos[:, 0] - ox) / s).astype(int)
        gz = np.rint((pos[:, 2] - oz) / s).astype(int)
        dh = new[gz, gx] - pos[:, 1]
        assert np.all((dh == 0) | (abs(dh - 20) < 2e-5))
        tc[lod] += g.attr(prim["indices"]).size // 3
    print(
        json.dumps(
            dict(
                grid_nodes=int(old.size),
                changed_nodes=int((old != new).sum()),
                max_height_change_m=float(abs(new - old).max()),
                walkable_grade_faces_before=int(go.sum()),
                walkable_grade_faces_after=int(gn.sum()),
                grade_pass_to_fail=int((go & ~gn).sum()),
                grade_fail_to_pass=int((~go & gn).sum()),
                nursery_component_before=int(reach_o.sum()),
                nursery_component_after=int(reach_n.sum()),
                nursery_faces_lost=int((reach_o & ~reach_n).sum()),
                nursery_faces_gained=int((~reach_o & reach_n).sum()),
                old_and_new_probe_reachability=walk_o,
                prop_placements=len(p),
                solid_bodies=len(bo),
                authored_rock_corrections=len(CORRECTIONS),
                triangles=tc,
            ),
            indent=2,
        )
    )


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("out", type=Path)
    verify(ap.parse_args().out)
