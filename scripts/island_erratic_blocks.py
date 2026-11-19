"""Weather existing glacial boulders without combining their separate stones.

The old stone family uses almost cylindrical height rings. A silhouette at a
real rock exposes them even after albedo dressing. Keep each disconnected
block as its own bounded body; never span the empty bit between neighbours.
"""

import numpy as np, math
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from scipy.spatial import ConvexHull, cKDTree


def unit(a):
    return a / np.maximum(1e-9, np.linalg.norm(a, axis=-1, keepdims=True))


def fade(a, b, x):
    y = np.clip((x - a) / (b - a), 0, 1)
    return y * y * (3 - 2 * y)


def clusters(position, triangles):
    vertices, inv = np.unique(np.round(position, 5), axis=0, return_inverse=True)
    t = inv[triangles]
    a = t.ravel()
    b = t[:, [1, 2, 0]].ravel()
    count, which = connected_components(
        coo_matrix((np.ones(len(a)), (a, b)), (len(vertices), len(vertices))),
        directed=False,
    )
    result = []
    for i in range(count):
        body = vertices[which == i]
        if len(body) >= 4:
            result.append(body)
    return sorted(result, key=lambda b: -np.prod(np.maximum(0.001, np.ptp(b, axis=0))))


def block(body, detail, seed):
    hull = ConvexHull(body)
    support = body[hull.vertices]
    lo, hi = body.min(0), body.max(0)
    half = (hi - lo) * 0.5
    center = (hi + lo) * 0.5
    half = np.maximum(0.012, half)
    base = body[body[:, 1] < lo[1] + 0.015 * half[1]]
    foot = base[ConvexHull(base[:, [0, 2]]).vertices] if len(base) >= 4 else base
    beyond = body[ConvexHull(body[:, [0, 2]]).vertices]
    fixed = np.unique(np.r_[foot, beyond, body[[np.argmax(body[:, 1])]]], axis=0)
    if detail < 42:
        support = fixed
    original = unit((support - center) / half)
    j = np.arange(detail) + 0.5
    y = 1 - 2 * j / detail
    h = np.sqrt(1 - y * y)
    phi = math.pi * (3 - math.sqrt(5))
    yaw = seed * 0.55
    dirs = np.r_[
        original,
        np.stack((np.cos(j * phi + yaw) * h, y, np.sin(j * phi + yaw) * h), -1),
    ]
    tri = ConvexHull(dirs).simplices.copy()
    normals = hull.equations[:, :3]
    offset = hull.equations[:, 3]
    delta = dirs * half
    den = delta @ normals.T
    radius = np.min(
        np.where(
            den > 1e-8, -(normals @ center + offset) / np.maximum(den, 1e-10), np.inf
        ),
        axis=1,
    )
    p = center + delta * radius[:, None]
    p[: len(support)] = support
    q = (p - center) / half
    x, y, z = q.T
    # A broad tilted facet, a single cleaved shoulder and a narrow trailing
    # fracture. None forms a closed horizontal belt round the stone.
    sx, sz = math.cos(yaw), math.sin(yaw)
    u = x * sx + z * sz
    v = -x * sz + z * sx
    cut = fade(0.22, 1.06, 0.79 * u + 0.56 * y - 0.21 * v)
    hair = (
        np.exp(-(((v + 0.36 * u - 0.31 * y - 0.24) / 0.125) ** 2))
        * fade(-0.44, 0.72, y)
        * fade(-0.65, 0.13, u)
    )
    # Start rounding at the old convex edges and preserve the flatter rub
    # planes between them. Keep the uncompressed real sides, cap and toe.
    distance = cKDTree((fixed - center) / half).query(q)[0]
    polish = fade(0.07, 0.77, distance)
    powered = np.sum(np.abs(q) ** 2.7, axis=1) ** (1 / 2.7)
    erosion = (
        0.018 + 0.10 * cut + 0.065 * hair + 0.24 * np.clip(powered - 0.88, 0, 0.63)
    ) * polish
    p = center + (p - center) * (1 - erosion[:, None])
    pin = cKDTree(fixed).query(support)[0] < 1e-6
    p[: len(support)][pin] = support[pin]
    n = np.cross(p[tri[:, 1]] - p[tri[:, 0]], p[tri[:, 2]] - p[tri[:, 0]])
    bad = np.einsum("ij,ij->i", n, p[tri].mean(1) - center) < 0
    tri[bad] = tri[bad][:, [0, 2, 1]]
    return p, tri
