"""Assemble the two matched final photographs without retouching or colour grading."""

from pathlib import Path
import json, sys, hashlib
from PIL import Image, ImageFont, ImageDraw


def make(held_eye, held_g, final):
    W = 1746
    w = 840
    m = 26
    g = 14
    sheet = Image.new("RGB", (W, 1362), "#f2efe3")
    d = ImageDraw.Draw(sheet)

    def put(x, y, s, size, ink="#293d37", b=False):
        f = (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if b else "")
            + ".ttf"
        )
        d.text((x, y), s, font=ImageFont.truetype(f, size), fill=ink, anchor="lt")

    d.rectangle((0, 0, W, 114), fill="#263b34")
    put(m, 16, "QUALIA    /    EASTERN HIGHLANDS", 16, "#d6d3b7", 1)
    put(m, 49, "Beneath the spruce", 44, "#fff6e4", 1)
    put(
        662,
        64,
        "Broken bedding, sheltered underbrush and the real middle-distance slope.",
        21,
        "#e4e1ca",
    )
    put(m, 139, "HELD SLOPE", 18, "#526961", 1)
    put(m + w + g, 139, "FEATHERED BEDDING AND WINDSHAPED ROOTS", 18, "#8e6046", 1)
    hashes = {}
    for i, (n, name, note, control) in enumerate(
        (
            (
                "spruce-lee",
                "The exposed face",
                "58 m guardian view: no broad polygon laid over the hill.",
                held_g,
            ),
            (
                "stone-eye",
                "Along the planted bank",
                "1.04 m creature height: the trees, windcups, coast and camera stay fixed.",
                held_eye,
            ),
        )
    ):
        y = 178 + i * 565
        put(m, y, "0" + str(i + 1) + "   /   " + name, 24, b=True)
        put(640, y + 4, note, 18, "#51675c")
        for j, p in enumerate([control, final / "after" / (n + ".png")]):
            im = Image.open(p).convert("RGB")
            sheet.paste(im, (m + j * (w + g), y + 39))
            hashes[("before" if j == 0 else "after") + "/" + n + ".png"] = (
                hashlib.sha256(p.read_bytes()).hexdigest()
            )
        d.line((m, y + 548, W - m, y + 548), fill="#c7cbb8", width=2)
    put(
        m,
        1315,
        "Original V1 elevations and physical geometry. Blender art study, not an in-game capture.",
        17,
        "#53655d",
    )
    sheet.save(
        final / "Beneath-the-Spruce.jpg", quality=93, subsampling=0, optimize=True
    )
    (final / "frame-hashes.json").write_text(json.dumps(hashes, indent=2) + "\n")


if __name__ == "__main__":
    make(*(Path(s) for s in sys.argv[1:4]))
