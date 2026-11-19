"""Display both locked, full photographs and a real across-slope section."""

import json, csv, sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont


def main(photo, art, dest):
    site = json.loads((art / "site.json").read_text())
    m, g, w, h = 26, 14, 840, 500
    W = 2 * m + 2 * w + g
    im = Image.new("RGB", (W, 1513), "#f3f0e4")
    d = ImageDraw.Draw(im)
    ink = "#253937"
    sub = "#586a61"
    warm = "#906142"
    rule = "#bfcbba"

    def put(x, y, s, n, c=ink, b=False):
        f = (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if b else "")
            + ".ttf"
        )
        d.text((x, y), s, fill=c, font=ImageFont.truetype(f, n), anchor="lt")

    d.rectangle((0, 0, W, 118), fill=ink)
    put(m, 15, "QUALIA  /  EASTERN HIGH GROUND", 17, "#d4cfad", True)
    put(m, 49, "Lanternback Rim", 46, "#f7f3e4", True)
    put(
        690,
        62,
        "From the wind-scoured shoulder down to Sunspindle Hollow.",
        23,
        "#e0e1cf",
    )
    put(m, 139, "FROZEN ISLAND", 19, sub, True)
    put(m + w + g, 139, "FRACTURED STONE AND SHELTERED HEATH", 19, warm, True)
    for index, (tag, label, desc) in enumerate(
        [
            (
                "upper-rim",
                "01  /  Above the eastern slope",
                "143 m span. Real alders, erratics and terrain on both sides.",
            ),
            (
                "stone-eye",
                "02  /  Along the rock lip",
                "Eye 1.04 m above V1. Same lens and daylight in both photographs.",
            ),
        ]
    ):
        y = 182 + index * 586
        put(m, y, label, 25, ink, True)
        put(570, y + 4, desc, 19, sub)
        for j, side in enumerate(("before", "after")):
            a = Image.open(photo / side / (tag + ".png")).convert("RGB")
            if a.size != (w, h):
                raise ValueError("unexpected image size")
            im.paste(a, (m + j * (w + g), y + 43))
        d.line((m, y + 560, W - m, y + 560), fill=rule, width=2)
    y = 1370
    put(m, y, "A slope already in the island", 24, ink, True)
    put(
        m,
        y + 39,
        f"Native section: {site['section_head_m']:.1f} m at the upper shoulder to {site['section_foot_m']:.1f} m above the water plane.",
        19,
        sub,
    )
    put(
        m,
        y + 76,
        "The visible stone follows the existing triangles. Neither the skyline nor the collision surface was sculpted.",
        18,
        sub,
    )
    put(W - 295, y + 10, "BLENDER STUDY", 18, warm, True)
    put(W - 295, y + 47, "Not gameplay.", 18, sub)
    im.save(dest, quality=94, subsampling=0, optimize=True)


if __name__ == "__main__":
    main(*(Path(a) for a in sys.argv[1:4]))
