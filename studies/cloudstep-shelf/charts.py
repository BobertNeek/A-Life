"""Portable meter-registered optical fields over true native mountain triangles.
Multiplicative RGB has exact 128-neutral limits; response stores roughness,
normal-grain height and cover. No position channel, geometry or displacement.
"""

from pathlib import Path
import json, sys
import numpy as np
from PIL import Image
from weathering import BOX, surface, Inlet


def make(native, out):
    out.mkdir(parents=True, exist_ok=True)
    a, b, c, d = BOX
    W, H = 576, 690
    island = Inlet(native / "island-v1.bin")
    colour = np.empty((H, W, 3), "B")
    response = np.empty_like(colour)
    for i in range(0, H, 16):
        stop = min(H, i + 16)
        xx, zz = np.meshgrid(
            a + (np.arange(W) + 0.5) * (b - a) / W,
            c + (np.arange(i, stop) + 0.5) * (d - c) / H,
        )
        pigment, rough, height, cover, *_ = surface(island, xx, zz)
        colour[i:stop] = np.uint8(np.clip(np.rint(pigment * 128), 0, 255))
        response[i:stop] = np.uint8(
            np.clip(np.rint(255 * np.stack([rough, height, cover], -1)), 0, 255)
        )
    Image.fromarray(colour).save(out / "mountain-gain.png", optimize=True)
    Image.fromarray(response).save(out / "mountain-response.png", optimize=True)
    note = dict(
        native_plan_xz=BOX,
        metres_per_texel=[(b - a) / W, (d - c) / H],
        rgb_gain_linear_neutral=128,
        workbench_channels=[
            "native roughness target",
            "optical shallow surface response",
            "native exposure weight",
        ],
        max_optical_bump_distance_m=0.028,
        geometry_displacement_m=0,
        description="Three dissimilar broken geological contacts, not repeated elevation bands.",
    )
    (out / "mountain-charts.json").write_text(json.dumps(note, indent=2) + "\n")
    return note


if __name__ == "__main__":
    print(make(*(Path(p) for p in sys.argv[1:])))
