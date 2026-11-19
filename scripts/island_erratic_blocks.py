"""Weather existing glacial boulders without combining their separate stones.

The old stone family uses almost cylindrical height rings. A silhouette at a
real rock exposes them even after albedo dressing. Keep each disconnected
block as its own bounded body; never span the empty bit between neighbours.
"""

import numpy as np, math
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from scipy.spatial import ConvexHull


def unit(a):
    return a / np.maximum(1e-9, np.linalg.norm(a, axis=-1, keepdims=True))


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


def abraded(body, detail, seed):
    """A lightly water-worn stony block, not a many-faceted ellipsoid.

    Erode the original convex stone by a small ball; round its *edges* by
    adding that ball back. Areas between the rounded edges remain geological
    planes. The shading direction on each bevel is available analytically.
    """
    from scipy.spatial import HalfspaceIntersection

    hull = ConvexHull(body)
    planes = np.unique(hull.equations.round(7), axis=0)
    interior = body[hull.vertices].mean(0)
    lo, hi = body.min(0), body.max(0)
    extent = hi - lo
    pins = body[np.unique(np.r_[np.argmin(body, axis=0), np.argmax(body, axis=0)])]
    lev = lo[1] + 0.012 * extent[1]
    sole = body[body[:, 1] <= lev]
    foot = sole[ConvexHull(sole[:, [0, 2]]).vertices] if len(sole) > 3 else sole
    pins = np.unique(np.r_[pins, foot], axis=0)
    # Clip a generous upper *shoulder*, not the summit or the original toe.
    # Prefer directions with actual available material clear of the supports.
    menu = []
    for a in np.linspace(0, 2 * math.pi, 21, endpoint=False) + 0.37 * seed:
        for y in (0.32, 0.71):
            n = unit(np.array([math.cos(a), y, math.sin(a)]))
            top = float((body @ n).max())
            slack = top - float((pins @ n).max())
            menu.append((slack, top, n))
    slack, top, side = max(menu, key=lambda a: a[0])
    if slack > 0.035 * extent.min():
        planes = np.r_[
            planes, [[*side, -top + min(0.62 * slack, 0.097 * extent.min())]]
        ]
    normals = planes[:, :3]
    starts = planes[:, 3]
    r = min(0.081 * extent.min(), 0.28 * float((-(normals @ interior + starts)).min()))
    inner = HalfspaceIntersection(
        np.column_stack((normals, starts + r)), interior
    ).intersections
    inner = np.unique(inner.round(7), axis=0)
    # Sample actual support points on the rounded convex solid rather than
    # materializing the whole Minkowski-sum mesh. At each true broad plane
    # retain three separated on-plane points carrying its exact normal.
    j = np.arange(detail) + 0.5
    y = 1 - 2 * j / detail
    t = math.pi * (3 - math.sqrt(5)) * j + seed
    h = np.sqrt(1 - y * y)
    globe = np.stack((np.cos(t) * h, y, np.sin(t) * h), -1)
    dirs = np.r_[globe, np.eye(3), -np.eye(3)]
    cloud = list(inner[(dirs @ inner.T).argmax(1)] + r * dirs)
    sn = list(dirs)
    flat = []
    for normal, offset in zip(normals, starts):
        verts = inner[np.abs(inner @ normal + offset + r) < 1e-5]
        if len(verts) < 3:
            continue
        axis = np.eye(3)[np.argmin(abs(normal))]
        tangent = unit(np.cross(axis, normal))
        side = np.cross(normal, tangent)
        chart = np.stack([verts @ tangent, verts @ side], axis=1)
        ring = ConvexHull(chart)
        flat.append((ring.volume, normal, verts, chart))
    for _, n, verts, chart in sorted(flat, key=lambda f: -f[0])[: max(3, detail // 6)]:
        # Choose wide triangles with an authentic underlying planar normal;
        # this is what prevents a small smooth model turning into an egg.
        i, j = np.unravel_index(
            np.square(chart[:, None] - chart[None, :]).sum(2).argmax(),
            (len(verts), len(verts)),
        )
        d = chart[j] - chart[i]
        k = np.argmax(
            abs((chart[:, 0] - chart[i, 0]) * d[1] - (chart[:, 1] - chart[i, 1]) * d[0])
        )
        picked = np.asarray([i, j, k])
        cloud.extend(verts[picked] + r * n)
        sn.extend([n] * len(picked))
    cloud = np.array(cloud)
    sn = np.array(sn)
    # Seat the original floor as an actual contiguous foot, not isolated
    # mid-height support vertices. Isolated old pins were producing dark
    # triangular stress marks in the artist clay. The actual native collider
    # is the transformed local AABB; fix its six dimensions after beveling.
    mn, mx = cloud.min(0), cloud.max(0)
    cloud = lo + (cloud - mn) * (hi - lo) / (mx - mn)
    lateral = foot[:, [0, 2]] - foot[:, [0, 2]].mean(0)
    normal_pins = unit(
        np.stack((lateral[:, 0], np.full(len(foot), -0.017), lateral[:, 1]), -1)
    )
    pp = np.r_[cloud, foot]
    skin = np.r_[sn, normal_pins]
    result = ConvexHull(pp)
    ids = result.vertices
    lookup = np.full(len(pp), -1)
    lookup[ids] = np.arange(len(ids))
    p = pp[ids]
    tri = lookup[result.simplices]
    shade = skin[ids]
    fn = np.cross(p[tri[:, 1]] - p[tri[:, 0]], p[tri[:, 2]] - p[tri[:, 0]])
    wrong = np.einsum("ij,ij->i", fn, p[tri].mean(1) - interior) < 0
    tri[wrong] = tri[wrong][:, [0, 2, 1]]
    return p, tri, shade
