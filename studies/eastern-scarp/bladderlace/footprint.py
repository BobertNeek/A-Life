"""Original per-site cast silhouettes and a modest visual-only placement budget.

At 119 metres of frame width the source herb is just 1-3 pixels. A small,
native-seat wet olive holdfast mask carries that reading without 35 detailed
modeled blades. The real 1.04m eye sees near and middle meshes with disjoint
root indices, not all resolutions simultaneously.
"""

from pathlib import Path
import csv, json, math, sys
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter

H = Path(__file__).resolve().parent
sys.path[:0] = [
    str(H),
    str(H.parents[1] / "lanternback-rim"),
    str(H.parents[2] / "scripts"),
]
from fronds import assemble
from island_inlet_ground import ease


def make(out):
    rows = list(csv.DictReader((out / "lace-roots.csv").open()))

    def dist(r):
        return math.sqrt(
            (float(r["x"]) - 409.6) ** 2
            + (float(r["native_y"]) - 1.24677354) ** 2
            + (float(r["z"]) + 97.0) ** 2
        )

    def pick1(r):
        return dist(r) < 2.65

    def pick2(r):
        return 2.65 <= dist(r) < 14.0

    q1 = assemble(out / "lace-forms-near.glb", out, False, pick1, "standing-detail.glb")
    q2 = assemble(
        out / "lace-forms-middle.glb", out, True, pick2, "standing-context.glb"
    )
    # Tiny stains projected onto the original low rock, below the exact grip.
    # This is a stone optical glaze, not a 3-D crown cutout at a false angle.
    bounds = (-155.0, -74.0, -0.55, 1.75)
    u0, u1, v0, v1 = bounds
    W, H = 1782, 112
    mask = np.zeros((H, W), "f")
    for r in rows:
        sx = float(r["z"])
        sy = float(r["native_y"])
        rad = float(r["span_m"])
        cx = (sx - u0) / (u1 - u0) * W
        cy = (v1 - sy) / (v1 - v0) * H
        x0, x1 = max(0, int(cx - 17)), min(W, int(cx + 18))
        y0, y1 = max(0, int(cy - 19)), min(H, int(cy + 22))
        y, x = np.mgrid[y0:y1, x0:x1]
        dx = (x - cx) * (u1 - u0) / W
        dy = (y - cy) * (v1 - v0) / H
        # Narrow root-wrapped deposit, off-centre downhill. Fan shape has
        # pronounced gaps rather than repeating solid olive cushions.
        s = np.exp(
            -(((dx + dy * 0.26) / (rad * 0.56)) ** 2)
            - ((dy - 0.18 * rad) / (rad * 0.37)) ** 2
        )
        mask[y0:y1, x0:x1] = np.maximum(mask[y0:y1, x0:x1], s)
    rng = np.random.default_rng(6115)
    texture = gaussian_filter(rng.normal(size=mask.shape), 0.61)
    texture /= np.std(texture)
    alpha = np.clip(mask * (0.55 + 0.17 * np.clip(texture, -1.7, 1.7)), 0, 0.72)
    color = np.array([97.0, 112.0, 82.0]) + np.clip(texture, -2, 2)[
        ..., None
    ] * np.array([10.0, 9.0, 5.0])
    Image.fromarray(np.uint8(np.clip(np.dstack([color, 255 * alpha]), 0, 255))).save(
        out / "lace-hold.png", optimize=True
    )

    def count(p):
        return sum(a["root_count"] * a["triangles"] for a in p.values())

    def roots(p):
        return sum(a["root_count"] for a in p.values())

    receipt = dict(
        side=bounds,
        standing_near=[roots(q1), count(q1)],
        standing_middle=[roots(q2), count(q2)],
        guardian_modeled_triangles=0,
        other_anchors_remain_in_full_edit=True,
        static_only=True,
        model_lod_switch_m=[2.65, 14.0],
    )
    (out / "lace-budget.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


if __name__ == "__main__":
    print(make(Path(sys.argv[1])))
