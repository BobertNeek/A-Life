"""Remove the authoring generator's pinched middle rings, not the collision hull.

Each weathered block remains inside its own original 3D hull. All support
vertices and the whole foot stay *exactly* where they were. This keeps the
authoritative rocks' rotated AABBs and anchors, not just a convenient local box.
"""

import numpy as np
from scipy.spatial import ConvexHull


def unit(v):
    return v / np.maximum(1e-12, np.linalg.norm(v, axis=-1, keepdims=True))


def weather(p, normal, tri, labels, kind):
    q = p.astype("f8", copy=True)
    q0 = q.copy()
    groups = []
    for lab in np.unique(labels):
        select = np.flatnonzero(labels == lab)
        shell = q0[select]
        hull = ConvexHull(shell)
        planes = hull.equations
        normal3, offset = planes[:, :3], planes[:, 3]
        distances = shell @ normal3.T + offset
        low = shell[:, 1].min()
        span = np.ptp(shell[:, 1])
        locked = (distances.max(1) > -2e-5) | (
            shell[:, 1] < low + max(0.065, 0.09 * span)
        )
        # Vertical support is untouched; unfold the alternating 10% waist only
        # as far as the next actual supporting plane. No puffed round ball.
        center = (shell.min(0) + shell.max(0)) / 2
        outward = shell - center
        outward[:, 1] = 0
        outward = unit(outward)
        d = outward @ normal3.T
        dist = np.min(
            np.where(d > 1e-5, -distances / np.maximum(d, 1e-5), np.inf), axis=1
        )
        # Loose scree already had a separate finished sculpt. Keep it.
        distance = np.where(
            locked | ~np.isfinite(dist) | (kind == "Scree"),
            0,
            np.clip(dist, 0, 0.36 if kind == "Cliff" else 0.14) * 0.88,
        )
        q[select] += outward * distance[:, None]
        assert np.array_equal(q[select][locked], shell[locked])
        assert (q[select] @ normal3.T + offset).max() < 3e-6
        assert np.array_equal(q[select][hull.vertices], shell[hull.vertices])
        groups.append(
            dict(
                changed=int(np.count_nonzero(distance > 1e-5)),
                max_m=float(distance.max()),
                min_y=float(low),
                hull_support_vertices=len(hull.vertices),
            )
        )
    if kind == "Scree":
        return p, normal, groups
    # Smooth the height-ring joins on each angular face but never shade
    # across a true, hard top or the inner edge of a completely different block.
    unique, vi = np.unique(np.round(p, 5), axis=0, return_inverse=True)
    tris = q[tri]
    cross = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    fn = unit(cross)
    old_tri = p[tri]
    oldfn = unit(np.cross(old_tri[:, 1] - old_tri[:, 0], old_tri[:, 2] - old_tri[:, 0]))
    incident = [set() for _ in unique]
    for j, vertices in enumerate(vi[tri]):
        for v in vertices:
            incident[v].add(j)
    norms = normal.astype("f8", copy=True)
    for k in range(len(q)):
        ids = list(incident[vi[k]])
        if not ids:
            continue
        ids = np.asarray(ids)
        weight = np.clip((oldfn[ids] @ normal[k] - 0.45) / 0.55, 0, 1)
        if not weight.any():
            continue
        # Area weighting alone merges long triangles preferentially. Per-corner
        # angular weights are independent of the artist's diagonal triangulation.
        point = q[k]
        row = tri[ids]
        pivot = np.argmin(np.linalg.norm(q[row] - point, axis=-1), axis=1)
        rr = np.arange(len(ids))
        a = unit(q[row[rr, (pivot + 1) % 3]] - point)
        b = unit(q[row[rr, (pivot + 2) % 3]] - point)
        angle = np.arccos(np.clip(np.sum(a * b, axis=1), -1, 1))
        avg = (fn[ids] * (weight * angle)[:, None]).sum(0)
        if np.linalg.norm(avg) > 1e-6:
            norms[k] = unit(normal[k] * 0.27 + unit(avg) * 0.73)
    return q.astype("<f4"), norms.astype("<f4"), groups
