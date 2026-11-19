"""Small patches of weathered shell grit around the registered coastal casts."""

import csv, sys, math, json
from pathlib import Path
from PIL import Image, ImageDraw
import numpy as np
from scipy.ndimage import gaussian_filter

BOUNDS = (-164.0, -69.0, 0.02, 2.34)


def paint(out):
    u0, u1, v0, v1 = BOUNDS
    w, h = 2850, 140
    rows = list(csv.DictReader((out / "shell-seat.csv").open()))
    density = np.zeros((h, w), "f")
    for row in rows:
        yz = float(row["native_y"])
        z = float(row["z"])
        r = float(row["radius_m"])
        cx, cy = (z - u0) * w / (u1 - u0), (v1 - yz) * h / (v1 - v0)
        if not (0 < cx < w and 0 < cy < h):
            continue
        x0, x1 = max(0, int(cx - 32)), min(w, int(cx + 33))
        y0, y1 = max(0, int(cy - 39)), min(h, int(cy + 40))
        y, x = np.mgrid[y0:y1, x0:x1]
        dx = (x - cx) * (u1 - u0) / w
        dy = (y - cy) * (v1 - v0) / h
        # Patches are enlarged toward a retained tide pocket, not dark disks
        # stamped beneath every visible shell. Leave bare scoured rock between.
        val = np.exp(
            -((dx / (r * 5.1 + 0.05)) ** 2)
            - ((dy + 0.025 + 0.2 * dx) / (r * 2.5 + 0.055)) ** 2
        )
        density[y0:y1, x0:x1] += val
    rng = np.random.default_rng(71421)
    q = rng.normal(size=(h, w)).astype("f")
    a = gaussian_filter(q, (3.8, 2.3))
    a /= a.std()
    f = gaussian_filter(q, 0.68)
    f /= f.std()
    level = np.minimum(1.0, density * 0.65)
    edge = np.clip(
        (0.66 * level + 0.12 * a + 0.055 * f - 0.11) * 1.7, 0, 0.78
    ) * np.minimum(1.0, density * 1.8)
    # Faded cream matrix broken by damp olive rock and tiny oxide freckles.
    mat = (
        np.array([142, 149, 125])[None, None, :]
        + (7.5 * a + 10 * f)[..., None]
        + np.array([14, 2, -6])[None, None, :] * np.maximum(0, a[..., None]) * 0.59
    )
    Image.fromarray(np.uint8(np.clip(np.dstack((mat, edge * 195)), 0, 255))).save(
        out / "tide-patina.png", optimize=True
    )
    (out / "patina.json").write_text(
        json.dumps(
            dict(
                side_bounds=BOUNDS, x_support=[357.0, 432.0], no_new_face_or_water=True
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    paint(Path(sys.argv[1]))
