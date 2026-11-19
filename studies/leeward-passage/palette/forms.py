"""Three silhouette levels cut from actual placed high-quality plant specimens.

No flat alpha sheets or camera-facing geometry. Retain the spread, stem and
outer tip of each dominant spray as the fine petiole and blade folds recede.
"""

import numpy as np

WOODY = {"hazel", "myrtle", "salt"}


def component_parts(tri):
    groups = []
    lo = 0
    upper = -1
    for i, t in enumerate(tri):
        if i and int(t.min()) > upper:
            groups.append(tri[lo:i])
            lo = i
        upper = max(upper, int(t.max()))
    groups.append(tri[lo:])
    return groups


def mesh(old, level):
    # All inputs measured on the original stem. Source coordinates, original
    # pigments and exact botanical origin are retained; only the polygons vary.
    p, c, t = old.xyz.copy(), old.rgb.copy(), old.tri
    tris = []
    pins = []
    vout = []
    cout = []

    def patch(verts, col, faces, rootpins=()):
        n = len(vout)
        vout.extend(verts)
        cout.extend(col)
        pins.extend(n + i for i in rootpins)
        for aa, bb, cc in faces:
            if np.cross(verts[bb] - verts[aa], verts[cc] - verts[aa])[1] < 0:
                bb, cc = cc, bb
            tris.append((n + aa, n + bb, n + cc))

    if old.family in WOODY:
        # Original brush: eight vertices in its two connected cane segments,
        # then eleven seven-point folded leaves per bowed shoot.
        block = 85
        if len(p) % block:
            raise ValueError("This source is not the anchored eleven-leaf scrub.")
        shoots = list(range(len(p) // block))
        for j in shoots:
            b = j * block
            P = p[b : b + block]
            C = c[b : b + block]
            if level < 2:
                if level == 0:
                    patch(
                        P[:8],
                        C[:8],
                        [(0, 1, 2), (0, 2, 3), (4, 5, 6), (4, 6, 7)],
                        (0, 1),
                    )
                else:
                    patch(
                        P[[0, 1, 6, 7]], C[[0, 1, 6, 7]], [(0, 1, 2), (0, 2, 3)], (0, 1)
                    )
            if level == 0:
                for n in range(11):
                    a = 8 + n * 7
                    leaf = P[a : a + 7].copy()
                    tone = C[a : a + 7]
                    # Keep the ridge, the tipped end and a little of the wide
                    # upper shoulders. Six triangles become four, no alpha.
                    leaf[1] = leaf[1] * 0.79 + leaf[4] * 0.21
                    leaf[2] = leaf[2] * 0.79 + leaf[5] * 0.21
                    patch(
                        leaf[[0, 1, 2, 3, 6]],
                        tone[[0, 1, 2, 3, 6]],
                        [(0, 1, 3), (0, 3, 2), (1, 4, 3), (3, 4, 2)],
                    )
            elif level == 1:
                for side in (0, 1):
                    for row, stop in ((0, 1), (2, 3), (4, 4)):
                        a = 8 + 7 * (2 * row + side)
                        z = 8 + 7 * (2 * stop + side)
                        v = np.asarray([P[a], P[a + 6], P[z + 6], P[z + 2], P[z + 3]])
                        if row == stop:
                            patch(
                                P[a : a + 7][[0, 1, 2, 6]],
                                C[a : a + 7][[0, 1, 2, 6]],
                                [(0, 1, 3), (0, 3, 2)],
                            )
                        else:
                            patch(
                                v,
                                C[[a, a + 6, z + 6, z + 2, z + 3]],
                                [(0, 1, 4), (1, 2, 4), (2, 3, 4), (3, 0, 4)],
                            )
                a = 8 + 7 * 10
                patch(
                    P[a : a + 7][[0, 1, 2, 6]],
                    C[a : a + 7][[0, 1, 2, 6]],
                    [(0, 1, 3), (0, 3, 2)],
                )
            else:
                # A folded shoot, not a conical shrub. Actual geometry supplies
                # the two flanks and last tip. Keep holes between the shoots.
                bef = P[0:2].mean(0)
                lower = P[4:6].mean(0)
                a, z = 8 + 7 * 4 + 6, 8 + 7 * 5 + 6
                end = 8 + 7 * 10 + 6
                vv = np.array(
                    [
                        bef,
                        lower,
                        P[a],
                        P[z],
                        P[end],
                        (P[a] + P[z] + P[end]) / 3 + [0, 0.014, 0],
                    ]
                )
                patch(
                    vv,
                    C[[0, 4, a, z, end, end]],
                    [(0, 1, 2), (0, 3, 1), (1, 2, 5), (1, 5, 3), (2, 4, 5), (5, 4, 3)],
                    (0,),
                )
    else:
        parts = component_parts(t)
        blades = [i for i, side in enumerate(parts) if np.ptp(side) + 1 == 9]
        peaks = sorted(blades, key=lambda i: -max(p[np.unique(parts[i]), 1]))
        for k, g in enumerate(parts):
            base = int(g.min())
            n = int(g.max() - base + 1)
            P = p[base : base + n]
            C = c[base : base + n]
            if n == 9:
                if level == 0 and k in peaks[:2]:
                    patch(P, C, g - base, (0, 1))
                    continue
                if level == 2:
                    if k not in peaks[:4]:
                        continue
                    # Blade width from the real widest section, not a single
                    # hair at the root. A folded translucent plane is not used.
                    near = P[[0, 1]].mean(0)
                    patch(
                        np.array([near, P[2], P[3], P[8]]),
                        C[[0, 2, 3, 8]],
                        [(0, 1, 2), (1, 3, 2)],
                        (0,),
                    )
                elif level == 1:
                    vv = P[[0, 1, 2, 3, 8]].copy()
                    vv[2] = P[2] * 0.54 + P[5] * 0.46
                    vv[3] = P[3] * 0.54 + P[6] * 0.46
                    patch(
                        vv,
                        C[[0, 1, 2, 3, 8]],
                        [(0, 1, 2), (1, 3, 2), (2, 3, 4)],
                        (0, 1),
                    )
                else:
                    patch(
                        P[[0, 1, 2, 3, 5, 6, 8]],
                        C[[0, 1, 2, 3, 5, 6, 8]],
                        [(0, 1, 2), (1, 3, 2), (2, 3, 5), (2, 5, 4), (4, 5, 6)],
                        (0, 1),
                    )
            elif level == 0:
                patch(P, C, g - base)
            elif level == 1 and n == 5:
                if old.family == "samphire" and k % 3 == 0:
                    continue
                patch(P[[0, 1, 3, 4]], C[[0, 1, 3, 4]], [(0, 1, 3), (0, 3, 2)])
            elif level == 1 and n == 4:
                patch(P, C, g - base)
    if not tris:
        raise ValueError("Empty botanical form")
    return (
        np.array(vout),
        np.array(cout),
        np.array(tris, dtype="i4"),
        np.array(pins, dtype="i4"),
    )
