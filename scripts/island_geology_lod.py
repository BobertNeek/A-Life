"""Simplify only the distant copies against the unchanged three-metre near mesh."""

import numpy as np


def leaves(terrain, step, shore_tolerance, inland_tolerance):
    """Balance by incorporating smaller neighbours' edge points, including across chunks."""
    h = terrain.h
    cells = {}

    def split(x, z, n):
        tile = h[z : z + n + 1, x : x + n + 1]
        h00, h10, h01, h11 = tile[0, 0], tile[0, -1], tile[-1, 0], tile[-1, -1]
        t = np.arange(n + 1) / n
        u, v = t[None, :], t[:, None]
        plane = np.where(
            v >= u,
            (1 - v) * h00 + (v - u) * h01 + u * h11,
            (1 - u) * h00 + (u - v) * h10 + v * h11,
        )
        coast = tile.min() < 1.2 and tile.max() > -1.2
        tolerance = shore_tolerance if coast else inland_tolerance
        if n > 1 and np.max(abs(tile - plane)) > tolerance:
            m = n // 2
            for dz in (0, m):
                for dx in (0, m):
                    split(x + dx, z + dz, m)
        else:
            cells[x, z] = n

    for z in range(0, h.shape[0] - 1, step):
        for x in range(0, h.shape[1] - 1, step):
            split(x, z, step)
    corners = {
        (x + dx, z + dz)
        for (x, z), n in cells.items()
        for dx, dz in ((0, 0), (n, 0), (n, n), (0, n))
    }
    return cells, corners


def chunk(x0, z0, geometry):
    cells, corners = geometry
    keys, indices = {}, []

    def vertex(key):
        if key not in keys:
            keys[key] = len(keys)
        return keys[key]

    for z in range(z0, z0 + 32):
        for x in range(x0, x0 + 32):
            n = cells.get((x, z))
            if n is None:
                continue
            ring = (
                [(x + t, z) for t in range(n)]
                + [(x + n, z + t) for t in range(n)]
                + [(x + n - t, z + n) for t in range(n)]
                + [(x, z + n - t) for t in range(n)]
            )
            ids = [vertex((i, j, False)) for i, j in ring if (i, j) in corners]
            if len(ids) == 4:
                a, b, c, d = ids
                indices.extend((a, c, b, a, d, c))
            else:
                mid = vertex((x + n // 2, z + n // 2, False))
                for i, a in enumerate(ids):
                    indices.extend((mid, ids[(i + 1) % len(ids)], a))
    perimeter = (
        [(x0 + t, z0) for t in range(32)]
        + [(x0 + 32, z0 + t) for t in range(32)]
        + [(x0 + 32 - t, z0 + 32) for t in range(32)]
        + [(x0, z0 + 32 - t) for t in range(32)]
    )
    edge = [(i, j) for i, j in perimeter if (i, j, False) in keys]
    for k, (i, j) in enumerate(edge):
        nx, nz = edge[(k + 1) % len(edge)]
        a, b = vertex((i, j, False)), vertex((nx, nz, False))
        c, d = vertex((nx, nz, True)), vertex((i, j, True))
        indices.extend((a, b, c, a, c, d))
    return list(keys), np.array(indices, dtype="<u2")
