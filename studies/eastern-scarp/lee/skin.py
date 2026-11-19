"""Paint the ground *response* of each actual community, not a trail connecting stations.

A regional colour/roughness chart warms exposed copper-sedge litter, deepens
native spruce duff and preserves open light swards between protected colonies.
Used over the old Hollow skin on unchanged native ground, never as displacement.
"""

from pathlib import Path
import sys, json, csv, math
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [
    str(H),
    str(H.parents[2] / "scripts"),
    str(H.parents[1] / "lanternback-rim"),
]
from community import BOX
from strata import nz
from island_inlet_ground import ease


def field(art):
    # The old broad cell colour must remain dominant in continuous unplanted
    # turf. Roots add a real contact pocket; changes do not follow the camera.
    atlas = np.asarray(Image.open(art / "ground-communities.png"), "f") / 255
    row = list(csv.DictReader((art / "lee-roots.csv").open()))
    a, b, c, d = BOX
    h, w = atlas.shape[:2]
    xx, zz = np.meshgrid(
        a + (np.arange(w) + 0.5) * (b - a) / w, c + (np.arange(h) + 0.5) * (d - c) / h
    )
    contact = np.zeros((h, w, 3), "f")
    for r in row:
        x, z = float(r["x"]), float(r["z"])
        can = float(r["canopy_m"])
        g = int(r["habitat"]) - 1
        rx, rz = (x - a) * w / (b - a), (z - c) * h / (d - c)
        rad = can * 2.8 + 0.22
        x0, x1 = max(0, math.floor(rx - rad * w / (b - a))), min(
            w, math.ceil(rx + rad * w / (b - a)) + 1
        )
        z0, z1 = max(0, math.floor(rz - rad * h / (d - c))), min(
            h, math.ceil(rz + rad * h / (d - c)) + 1
        )
        dx = xx[z0:z1, x0:x1] - x
        dz = zz[z0:z1, x0:x1] - z
        angle = float(r["yaw"])
        co, si = math.cos(angle), math.sin(angle)
        u = co * dx + si * dz
        v = -si * dx + co * dz
        q = np.exp(-((u / (can * 1.24 + 0.12)) ** 2) - ((v / (can * 0.70 + 0.11)) ** 2))
        contact[z0:z1, x0:x1, g] = np.maximum(contact[z0:z1, x0:x1, g], q)
    big = nz(xx, zz, 4.1, 480) - 0.5
    mid = nz(xx, zz, 1.05, 489) - 0.5
    close = nz(xx * 0.72, zz, 0.27, 497) - 0.5
    # A spent bent blade gives long fibres rather than coarse, repeated
    # hill-parallel stripes. No fibre crosses out of its existing plant field.
    fibre = nz(xx * 0.42, zz * 1.65, 0.17, 503) - 0.5

    def packed(v):
        return np.uint8(np.clip(v, 0, 255))

    L, S, D = np.moveaxis(atlas, 2, 0)
    l, s, k = np.moveaxis(contact, 2, 0)
    A = 0.49 * L + 0.21 * l
    B = 0.48 * S + 0.32 * s
    C = 0.61 * D + 0.30 * k
    weights = np.stack((A, B, C), -1)
    total = np.minimum(0.91, weights.sum(-1))
    shade = np.sum(weights * np.array([0.35, 0.25, 0.40]), axis=-1)
    tint = np.sum(
        weights[..., None]
        * np.array([[133.0, 146.0, 97.0], [172.0, 155.0, 107.0], [107.0, 103.0, 79.0]])[
            None, None, :, :
        ],
        axis=2,
    ) / np.maximum(0.001, weights.sum(-1, keepdims=True))
    tint += (18 * big + 14 * mid + 9 * close + 10 * fibre + 8 * shade)[..., None]
    amber = np.clip(0.88 * s + 0.32 * l, 0, 1) * ease(0.085, 0.36, close + 0.44 * mid)
    tint += amber[..., None] * np.array([15.0, 7.0, -3.0])
    tint -= np.clip(0.40 * l + 0.33 * s + 0.60 * k, 0, 1)[..., None] * np.array(
        [19.0, 18.0, 14.0]
    )
    # Inside the mapped previous Hollow, old texture is held in unplanted
    # gaps. Outside coverage neutral RGB values cannot become a tile border.
    opts = np.dstack([tint, 255 * total])
    Image.fromarray(packed(opts)).save(art / "wind-duff.png", optimize=True)
    depth = np.clip(
        0.50
        + 0.045 * mid
        + 0.06 * close
        + 0.11 * fibre
        - 0.20 * (l + s + k)
        + 0.08 * amber,
        0.10,
        0.88,
    )
    Image.fromarray(packed(255 * depth)).save(
        art / "wind-duff-height.png", optimize=True
    )
    summ = dict(
        plan=BOX,
        existing_roots_undisturbed=220,
        minimum_added_root_exclusion_metres=0.67,
        active_source_area_percent=round(100 * float(np.mean(total > 0.14)), 2),
        actual_chart_grain_metre=round((b - a) / w, 3),
        underfoot_geometry_displacement_m=0,
    )
    (art / "wind-duff.json").write_text(json.dumps(summ, indent=2) + "\n")
    return summ


if __name__ == "__main__":
    print(field(Path(sys.argv[1])))
