"""Opt-in authoring study. Generate versioned data, GLB and placements in OUT.
No production asset, default new game, save binding or training fixture is modified.
"""

import argparse
import hashlib
import json
from pathlib import Path
import struct

import numpy as np
from scipy.ndimage import gaussian_filter

from build_island_geology import Island, SURFACE, TERRAIN, build
from island_geology_paint import ramp
from island_placement_corrections import CORRECTIONS


def tri(h, x, z, src):
    i, j = (x - src.x) / src.spacing, (z - src.z) / src.spacing
    u, v = i % 1, j % 1
    i, j = int(i), int(j)
    a, b, c, d = h[j, i], h[j, i + 1], h[j + 1, i + 1], h[j + 1, i]
    return a + (b - a) * u + (c - b) * v if u >= v else a + (c - d) * u + (d - a) * v


def versioned_surface(src, props):
    h = src.h
    j, i = np.indices(h.shape)
    x, z = src.x + i * src.spacing, src.z + j * src.spacing
    # A coherent talus-and-shoulder envelope, rather than laying a second
    # family of ridges onto the known cartographic sine. Preserve the river cut.
    envelope = (
        0.38 * gaussian_filter(src.appearance, 0.95)
        + 0.39 * gaussian_filter(src.appearance, 2.1)
        + 0.23 * gaussian_filter(src.appearance, 4.8)
    )
    # Separate fracture systems: a broad west-east spur ending at a sheared
    # west summit, and an oblique, shorter buttress below the taller east crest.
    # Finite envelopes, uneven lengths and different orientations: no repeating carrier.
    for cx, cz, angle, long, wide, lift in [
        (-99, -205, -0.43, 77, 24, 19),
        (37, -260, 0.74, 64, 21, 15),
        (106, -170, -0.20, 48, 23, 11),
    ]:
        dx, dz = x - cx, z - cz
        u = dx * np.cos(angle) + dz * np.sin(angle)
        v = -dx * np.sin(angle) + dz * np.cos(angle)
        v += 3.2 * np.sin(u / 24 + 0.9)
        ledge = (
            np.exp(-(((u + 7) / long) ** 4))
            * np.exp(-(((v + 5) / wide) ** 2))
            * ramp(-12, 0, v)
        )
        lip = np.exp(-(((u + 0.2 * long) / (0.79 * long)) ** 4)) * np.exp(
            -(((v + 12) / 10) ** 2)
        )
        envelope += lift * (ledge - 0.34 * lip)
    # Break the western cone into a sloping, eroded table. The high eastern
    # peak keeps its pointed crest, so the pair has two distinct silhouettes.
    dx, dz = x + 101, z + 204
    cap = 198 + 0.37 * dx - 0.16 * dz + 2.2 * np.sin((dx - 0.4 * dz) / 19)
    west = 1 - ramp(35, 75, np.hypot(dx, dz))
    envelope -= west * np.maximum(0, envelope - cap)
    river_x = 22 + 33 * np.sin(-z / 110) + 0.13 * z
    bypass = 1 - ramp(-234, -188, z) * (1 - ramp(12, 31, abs(x - river_x)))
    wt = ramp(105, 146, h) * ramp(5, 62, -z) * bypass
    new = h + wt * (envelope - h)
    k = np.unravel_index(new.argmax(), new.shape)
    # Keep the original relief of the massif with a broad cap, not one pin vertex.
    new += (
        (240 - new[k]) * np.exp(-(((x - x[k]) / 48) ** 2 + ((z - z[k]) / 55) ** 2)) * wt
    )
    delta = new - h
    # A changed shoulder must not undo the five individually reviewed rock contacts.
    for idx in CORRECTIONS:
        px, _, pz = props[idx]["position"]
        delta *= ramp(11, 23, np.hypot(x - px, z - pz))
    return np.asarray(h + delta, dtype="<f4")


def generate(out):
    out = out.resolve()
    if out.is_relative_to(TERRAIN.parents[4]) or out == SURFACE.parent:
        raise ValueError("Write this study to a separate output directory.")
    out.mkdir(parents=True, exist_ok=True)
    src = Island(SURFACE)
    raw = bytearray(SURFACE.read_bytes())
    nh = src.h.size
    box = np.frombuffer(raw, "<f4", offset=32 + 4 * nh).reshape(-1, 6)
    props = json.loads((TERRAIN.parent / "props.json").read_text())
    h = versioned_surface(src, props)
    box_idx = 0
    moved = 0
    for prop in props:
        x, y, z = prop["position"]
        kind = prop["kind"]
        dy = float(tri(h, x, z, src) - tri(src.h, x, z, src))
        if dy:
            prop["position"][1] = y + dy
            moved += 1
        if kind in ("Conifer", "Broadleaf", "Sapling", "Cliff") or kind.startswith(
            "Boulder"
        ):
            b = box[box_idx]
            box_idx += 1
            assert abs(float(b[4]) - y) < 1e-3, (
                "proxy/placement mismatch",
                box_idx,
                prop,
            )
            b[4:] += dy
    assert box_idx == len(box)
    raw[32 : 32 + 4 * nh] = h.tobytes()
    field = out / "island-ridge-v2.bin"
    field.write_bytes(raw)
    (out / "props-ridge-v2.json").write_text(
        json.dumps(props, separators=(",", ":")) + "\n"
    )
    # The original carrier still exists on the protected foothills. Its
    # measured local amplitude drops away on the physically refashioned crest.
    surface = Island(field)
    triangles = build(
        TERRAIN, out / "terrain-chunks.glb", terrain=surface, original_heights=src.h
    )
    print(
        "source",
        hashlib.sha256(SURFACE.read_bytes()).hexdigest(),
        "versioned",
        hashlib.sha256(raw).hexdigest(),
        "triangles",
        triangles,
        "prop anchors moved",
        moved,
        "peak",
        h.max(),
        flush=True,
    )


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("out", type=Path)
    generate(ap.parse_args().out)
