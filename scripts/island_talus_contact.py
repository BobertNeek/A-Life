"""Seat actual, disconnected scree on immutable V1 triangles for an art study.

Scree has no physical obstacle by itself. Keep original prop transform, count
and horizontal extent limits; move natural individual stones, not the ground.
All visible stone faces stay rigid under rotation/translation plus, only as
necessary, one UNIFORM scale to keep each within its source rectangle.
"""

from pathlib import Path
import math
import numpy as np
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from finished_island_glb import parse, save, set_values, values
from island_inlet_ground import Inlet


def members(p, faces):
    xyz, inv = np.unique(np.round(p, 5), axis=0, return_inverse=True)
    corners = inv[faces]
    g = coo_matrix(
        (np.ones(corners.size), (corners.ravel(), corners[:, [1, 2, 0]].ravel())),
        (len(xyz), len(xyz)),
    )
    count, component = connected_components(g, directed=False)
    return [np.where(component[inv] == n)[0] for n in range(count)]


def rotation_to_bed(derivative):
    # Shortest rotation from up to this area's support normal. Limited to 50
    # degrees to avoid turning an isolated toe into a perpendicular fin.
    grade = np.linalg.norm(derivative)
    if grade < 1e-7:
        return np.eye(3)
    turn = min(math.atan(grade) * 0.94, math.radians(50))
    dx, dz = derivative / grade
    cross = np.array([[0, -dx, 0], [dx, 0, dz], [0, -dz, 0]])
    return np.eye(3) + math.sin(turn) * cross + (1 - math.cos(turn)) * (cross @ cross)


def seated(source, dest, surface, prop, index):
    if prop["kind"] != "Scree" or Path(source).resolve() == Path(dest).resolve():
        raise ValueError(
            "Requires distinct sculpt output and the actual loose shingle."
        )
    doc, data = parse(source)
    land = Inlet(surface)
    x, y, z = prop["position"]
    sc = prop["scale"]
    cs, sn = np.cos(prop["yaw"]), np.sin(prop["yaw"])
    yaw = np.array([[cs, -sn], [sn, cs]])
    origin = np.array([x, z])
    art = {}
    for level, mesh in enumerate(doc["meshes"]):
        prim = mesh["primitives"][0]
        pos, norm = (prim["attributes"][v] for v in ("POSITION", "NORMAL"))
        p, normals = (np.array(values(doc, data, acc)) for acc in (pos, norm))
        tri = np.array(values(doc, data, prim["indices"])).reshape(-1, 3)
        stones = members(p, tri)
        if len(stones) != 22:
            raise ValueError("Expected 22 disjoint original cleaved stones")
        moves = []
        for ids in stones:
            old = p[ids]
            body = np.unique(old, axis=0)
            lo, hi = body.min(0), body.max(0)
            mid = (lo + hi) * 0.5
            near = body[body[:, 1] <= lo[1] + max(0.004, 0.13 * (hi[1] - lo[1]))]
            footprint = (near[:, [0, 2]] @ yaw) * sc + origin
            centre = mid[[0, 2]] @ yaw * sc + origin
            h, dx, dz = land.floor(footprint[:, 0].copy(), footprint[:, 1].copy())
            # Each sole may cross a true cell diagonal. Fit both the local
            # surface normals and the finite observed height differences.
            _, xg, zg = land.floor(np.array(centre[0]), np.array(centre[1]))
            derivative = 0.77 * np.array([np.mean(dx), np.mean(dz)]) + 0.23 * np.array(
                [xg, zg]
            )
            rot = rotation_to_bed(yaw @ derivative)
            q = (old - mid) @ rot.T
            q -= (q.min(0) + q.max(0)) * 0.5
            ext = np.ptp(q, axis=0)[[0, 2]]
            keep = min(1.0, float(np.min((hi - lo)[[0, 2]] / np.maximum(1e-8, ext))))
            q = mid + q * keep
            qp = q[:, [0, 2]] @ yaw * sc + origin
            floor = land.floor(qp[:, 0].copy(), qp[:, 1].copy())[0]
            gaps = (y + q[:, 1] * sc) - floor
            # Support the bottom 12% of the actual vertical mass and embed
            # a little more on slick slopes, without flattening individual
            # stones or building material under unsupported terrain.
            gap_span = np.ptp(gaps)
            depth = np.clip(
                0.014 + 0.065 * gap_span + 0.015 * np.linalg.norm(derivative),
                0.015,
                0.055,
            )
            lift = -gaps.min() - gap_span * 0.13 - depth
            q[:, 1] += lift / sc
            old_gaps = y + near[:, 1] * sc - h
            moves.append(
                dict(
                    height_shift_m=round(float(lift), 4),
                    uniform_fit=round(keep, 4),
                    original_sole_gap_m=round(float(old_gaps.min()), 4),
                    lowest_revised_clearance_m=round(float((gaps + lift).min()), 4),
                )
            )
            p[ids], normals[ids] = q, normals[ids] @ rot.T
        set_values(doc, data, pos, p)
        set_values(doc, data, norm, normals)
        pn = p.astype("<f4")
        doc["accessors"][pos].update(min=pn.min(0).tolist(), max=pn.max(0).tolist())
        mesh["name"] = f"ScreeSeat{index}_L{level}"
        for node in doc["nodes"]:
            if node.get("mesh") == level:
                node["name"] = mesh["name"]
        art[f"L{level}"] = moves
    dest = Path(dest)
    dest.parent.mkdir(exist_ok=True, parents=True)
    save(doc, data, dest)
    return dict(
        index=index,
        original_transform=prop,
        scope="Offline variant at original V1 anchor only; decorative Scree is not a physical obstacle",
        levels=art,
    )
