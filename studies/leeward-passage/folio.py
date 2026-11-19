"""Put the full-size, unretouched matched photographs and a real terrain profile on a folio."""

import argparse
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from reach import Reach

VIEWS = (
    (
        "reach-guardian",
        "01   NURSERY MEADOW TO OPEN SEA",
        "The whole 219 m crossing. The old island, every solid landmark, water and daylight are held still.",
    ),
    (
        "reach-meadow",
        "02   LOOKING BACK INTO THE MEADOW",
        "1.04 m eye, 51 m from the start. A quiet centre winds between rooted sweeps and the native wild grass.",
    ),
    (
        "reach-alder",
        "03   AT THE BLUFF ALDER",
        "1.04 m eye, looking back toward the original tree. This alder, its neighbours and their shadows stay fixed.",
    ),
    (
        "reach-strand",
        "04   OVER THE LAST SALT BLUFF",
        "1.04 m eye, looking seaward. Follow exposed alder roots and weathered siltstone over the last sandy bluff.",
    ),
)


def fonts(size, bold=False):
    return ImageFont.truetype("DejaVuSans" + ("-Bold" if bold else "") + ".ttf", size)


def compose(before, after, terrain, out, strand=None):
    reach = Reach(terrain)
    w, h = Image.open(before / "before" / "reach-guardian.png").size
    pad = 27
    gap = 13
    head = 275
    im = Image.new(
        "RGB", (2 * w + pad * 2 + gap, head + len(VIEWS) * (h + 95) + 13), (25, 37, 36)
    )
    pen = ImageDraw.Draw(im)

    def add(x, y, txt, sz, bd=False, c="#e6e8d6"):
        pen.text((x, y), txt, font=fonts(sz, bd), fill=c)

    add(pad, 17, "SHELLBACK REACH", 42, True)
    add(
        pad,
        78,
        "A dry walk from the nursery meadow, through low till and the alder bluff, to the open southern sea.",
        25,
    )
    add(
        pad,
        118,
        "Same-site CPU Blender artwork on ordinary V1. Original left; authored passage right. Not runtime footage.",
        20,
        False,
        "#b8cbc4",
    )
    # Horizontal distance in metres; vertical axis exaggerated and labeled
    # to make the shallow swale and last descent legible at a glance.
    x0, x1 = pad + 24, im.width - pad - 24
    y0 = 239
    lo, hi = 0, 12.0
    q = [
        (x0 + (x1 - x0) * m / reach.m[-1], y0 - (float(reach.floor(*p)[0]) - lo) * 5.2)
        for p, m in zip(reach.p, reach.m)
    ]
    pen.polygon([(x0, y0), *q, (x1, y0)], fill="#586346")
    pen.line(q, fill="#ccbc83", width=3)
    pen.line((x0, y0, x1, y0), fill="#759586", width=2)
    for s, label in (
        (1, "0 m · nursery margin"),
        (49, "wild meadow"),
        (104, "glacial swale"),
        (169, "alder bluff"),
        (218, "219 m · shore"),
    ):
        xx, zz = reach.station(s, 0)
        yy = y0 - float(reach.floor(xx, zz)[0]) * 5.2
        cx = x0 + (x1 - x0) * s / reach.m[-1]
        pen.ellipse((cx - 4, yy - 4, cx + 4, yy + 4), fill="#e2d5a7")
        size = fonts(17)
        tb = pen.textbbox((0, 0), label, font=size)
        pen.text(
            (min(x1 - tb[2], max(x0, cx - tb[2] * 0.5)), 246),
            label,
            font=size,
            fill="#d0d5bf",
        )
    add(
        im.width - pad - 348,
        151,
        "Original terrain profile · 0–12 m high",
        16,
        False,
        "#c8cfb8",
    )
    for j, (name, title, caption) in enumerate(VIEWS):
        yy = head + j * (h + 95)
        add(pad, yy + 5, title, 25, True)
        add(pad, yy + 44, caption, 18, False, "#cbd3c4")
        right = strand if name in ("reach-alder", "reach-strand") and strand else after
        left = strand if name == "reach-alder" and strand else before
        for k, folder in enumerate((left / "before", right / "after")):
            pic = Image.open(folder / (name + ".png")).convert("RGB")
            if pic.size != (w, h):
                raise ValueError("The actual camera outputs are not a matched pair")
            im.paste(pic, (pad + k * (w + gap), yy + 76))
    im.save(out, quality=94, optimize=True, subsampling=0)


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    for name in ("before", "after", "terrain", "out"):
        cli.add_argument(name, type=Path)
    cli.add_argument(
        "--strand",
        type=Path,
        help="Terminal close-only source folder; distant controls stay unchanged",
    )
    args = cli.parse_args()
    compose(args.before, args.after, args.terrain, args.out, args.strand)
