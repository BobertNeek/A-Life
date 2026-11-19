"""Two actual plates plus an untouched-pixel botanical crop, no image synthesis."""

from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont


def main(held, new, out):
    w, h = 840, 500
    m, g = 26, 14
    W = m * 2 + 2 * w + g
    surface = Image.new("RGB", (W, 1355), "#eeecdf")
    p = ImageDraw.Draw(surface)

    def text(x, y, s, n, fill, b=False):
        f = (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if b else "")
            + ".ttf"
        )
        p.text((x, y), s, font=ImageFont.truetype(f, n), anchor="lt", fill=fill)

    ink = "#283d35"
    cream = "#f6f3de"
    sub = "#566c61"
    p.rectangle((0, 0, W, 114), fill=ink)
    text(m, 16, "QUALIA    /    LANTERNBACK HERBARIUM", 16, "#ccd0b7", True)
    text(m, 46, "The folded windcup", 44, cream, True)
    text(
        670,
        64,
        "A wordless flowering landmark at the scale of a small animal.",
        21,
        "#dfdfc9",
    )
    for j, tag in enumerate(
        (
            "HELD ROOTS AND YOUNG TRIANGULAR PENNONS",
            "SAME ORIGINAL ROOTS, NEW CUP AND VALVE SCULPTS",
        )
    ):
        text(m + j * (w + g), 134, tag, 17, sub, True)
    for i, (n, title, description) in enumerate(
        (
            (
                "pennon-eye",
                "At the flowering clump",
                "The new forms occupy the same seven planted transforms.",
            ),
            (
                "stone-eye",
                "From the bank",
                "Only the near heads change. The light, gravel, sea and distant flora are held.",
            ),
        )
    ):
        y = 180 + i * 566
        text(m, y, title, 23, ink, True)
        text(647, y + 2, description, 17, sub)
        for j, root in enumerate((held, new)):
            im = Image.open(root / (n + ".png")).convert("RGB")
            surface.paste(im, (m + j * (w + g), y + 39))
        p.line((m, y + 546, W - m, y + 546), fill="#c7ccbf", width=2)
    text(
        m,
        1313,
        "Native-island assets composed in Blender. These are not gameplay captures.",
        18,
        sub,
    )
    surface.save(out, quality=94, subsampling=0, optimize=True)


if __name__ == "__main__":
    main(*(Path(a) for a in sys.argv[1:4]))
