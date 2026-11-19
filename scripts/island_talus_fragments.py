"""Produce the 22 native scree fragments as fracture planes, not little drums.

Retain each constituent's own centre and bounds. Each clast starts as an
intersection of an oblique bedding plane, foot and uneven conjugate breaks.
Same authored fragments, a restrained weather lip on the near rendition,
and no stone bridges, new placements, boxes or terrain edits.
"""

import math
import numpy as np
from scipy.spatial import ConvexHull, HalfspaceIntersection
from island_erratic_blocks import unit


def cleave(body, detail, seed, *, preserve_peak=False):
    r = np.random.default_rng(9163 + seed * 5923)
    lo, hi = body.min(0), body.max(0)
    ext = hi - lo
    mid = (lo + hi) * 0.5
    theta = r.uniform(0, math.tau)
    along = np.array([np.cos(theta), 0.0, np.sin(theta)])
    side = np.array([-along[2], 0.0, along[0]])
    up = np.array([0.0, 1.0, 0.0])
    # Two boulder-break families. The stronger bed is deliberately asymmetric;
    # cut angles vary between original clasts, not all in parallel round a ring.
    ns = []
    off = []

    def plane(n, dist):
        n = unit(n)
        ns.append(n)
        off.append(-dist)

    # Negative outer directions don't mirror their positive faces exactly.
    for vec in [along, -along, side, -side]:
        plane(
            vec
            + up * r.uniform(-0.39, 0.45)
            + np.cross(vec, up) * r.uniform(-0.23, 0.25),
            r.uniform(0.73, 0.98),
        )
    bed = up + along * r.uniform(0.21, 0.80) + side * r.uniform(-0.31, 0.38)
    plane(bed, r.uniform(0.56, 0.87))
    plane(-up, 0.80)
    plane(
        -up * 0.73 - along * 0.67 + side * r.uniform(-0.36, 0.26), r.uniform(0.72, 0.92)
    )
    plane(
        -up * 0.60 + side * 0.59 + along * r.uniform(-0.73, 0.21), r.uniform(0.77, 0.94)
    )
    plane(
        along * r.choice((-1, 1)) * 0.81 + side * 0.70 + up * r.uniform(0.24, 0.74),
        r.uniform(0.64, 1.0),
    )
    plane(
        side * -0.76 + along * r.uniform(-0.69, 0.38) + up * 0.62, r.uniform(0.61, 0.94)
    )
    plane(
        up * 0.75 - side * 0.36 + along * r.uniform(-0.55, 0.38),
        r.uniform(0.69, 0.95),
    )
    plane(side * 0.65 + along * 0.61 - up * 0.38, r.uniform(0.78, 1.0))
    e = np.column_stack((ns, off))
    orig = HalfspaceIntersection(e, np.zeros(3)).intersections
    # Cleave off a vulnerable tip on a shallow bedding plane rather than
    # drawing a cone, shark fin or a repeated horizontal plateau. Most
    # members of one fall share their bedding strike, with a few tumbled out.
    heading = 0.57 + r.normal(0, 0.38) + (1.40 if seed % 5 == 3 else 0.0)
    cap = unit(
        up + np.array([math.cos(heading), 0, math.sin(heading)]) * r.uniform(0.15, 0.41)
    )
    projection = orig @ cap
    ceiling = projection.max() - r.uniform(0.13, 0.22) * np.ptp(projection)
    e = np.r_[e, [[*cap, -ceiling]]]
    orig = HalfspaceIntersection(e, np.zeros(3)).intersections
    # These are tiny edge breaks, not whole rings of smooth geometry. Preserve
    # each true broad face through its own bevelled vertex fan.
    if detail:
        candidates = []
        setop = ConvexHull(orig)
        for vi in setop.vertices:
            pt = orig[vi]
            incident = np.abs(e[:, :3] @ pt + e[:, 3]) < 1e-7
            inward = unit(np.sum(e[incident, :3], axis=0))
            # Stop one or two new fresh tips short of being rounded off.
            if r.random() < 0.27:
                continue
            cut = r.uniform(0.021, 0.078)
            candidates.append([*inward, -float(inward @ pt) + cut])
        if candidates:
            e = np.r_[e, candidates]
    p = HalfspaceIntersection(e, np.zeros(3)).intersections
    p = np.unique(p.round(7), axis=0)
    p = lo + (p - p.min(0)) * ext / np.maximum(1e-8, np.ptp(p, axis=0))
    if not preserve_peak:
        # Only the assembled old volume is a native obstacle. Within it,
        # broad thin splinters and a few thicker split blocks are physically
        # plausible; conserve original x/z centres and all actual soles.
        # Neither the mid-distance coarser bevel mesh nor a future change to
        # the number of tiny chamfers should change the body height.
        bulk = np.random.default_rng(8891 + seed * 999)
        ratio = bulk.uniform(0.80, 0.96) if seed % 5 == 1 else bulk.uniform(0.47, 0.77)
        p[:, 1] = lo[1] + (p[:, 1] - lo[1]) * ratio
    mid = (p.min(0) + p.max(0)) * 0.5
    h = ConvexHull(p)
    t = h.simplices.copy()
    fn = np.cross(p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]])
    bad = np.einsum("ij,ij->i", fn, p[t].mean(1) - mid) < 0
    t[bad] = t[bad][:, [0, 2, 1]]
    return p, t
