"""Same approved water, mapped through the artist's alternate channel bend."""

import json
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.ndimage import map_coordinates
from form import BOX, Reach


def water(surface, original, dest):
    base = Path(original)
    out = Path(dest)
    out.mkdir(exist_ok=True, parents=True)
    model = Reach(surface)
    opt = json.loads((base / "water.json").read_text())
    rgba = np.array(Image.open(base / "shore.png"))
    h, w = rgba.shape[:2]
    ox, oz = opt["origin_xz"]
    sx, sz = opt["span_xz"]
    pad = 24.0
    x0, x1 = [int(np.floor((x - ox) * w / sx)) for x in (BOX[0] - pad, BOX[1] + pad)]
    z0, z1 = [int(np.floor((z - oz) * h / sz)) for z in (BOX[2] - pad, BOX[3] + pad)]
    j, i = np.mgrid[z0 : z1 + 1, x0 : x1 + 1]
    x = ox + (i + 0.5) * sx / w
    z = oz + (j + 0.5) * sz / h
    source = model.unwarp_x(x, z)
    u = (source - ox) * w / sx - 0.5
    for c in range(rgba.shape[2]):
        mapped = map_coordinates(
            rgba[..., c].astype(float), [j, u], order=1, mode="nearest"
        )
        rgba[z0 : z1 + 1, x0 : x1 + 1, c] = np.uint8(np.rint(mapped))
    Image.fromarray(rgba).save(out / "shore.png", optimize=True)
    (out / "ripple.png").write_bytes((base / "ripple.png").read_bytes())
    opt.pop("terrain_binding", None)
    opt["art_inputs"] = (
        "Editable Blender concept only, no game terrain binding. Reprojects the selected source water through the local bend."
    )
    (out / "water.json").write_text(json.dumps(opt, indent=2) + "\n")
