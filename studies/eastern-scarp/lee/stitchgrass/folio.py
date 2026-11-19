"""Contact pair from completed source-camera photographs; no synthetic landscape."""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import sys


def pair(folder):
    w, h = 720, 428
    panel = Image.new("RGB", (w * 2 + 42, h * 2 + 132), "#e9e6d9")
    pen = ImageDraw.Draw(panel)

    def face(sz, bold=False):
        name = "DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf"
        try:
            return ImageFont.truetype(name, sz)
        except OSError:
            return ImageFont.load_default()

    pen.text((15, 10), "TWIN NEEDLE RISE", font=face(24, True), fill="#263831")
    pen.text(
        (257, 14),
        "A low wind meadow on the original island slope",
        font=face(19),
        fill="#43594e",
    )
    for col, title in enumerate(
        ["Held wooded rise", "New folded grass, seed heads and ground litter"]
    ):
        pen.text((16 + col * (w + 10), 49), title, font=face(16, True), fill="#33453c")
    labels = [
        ("eye", "Same 1.04 m station, looking toward the two original spruce"),
        ("glance", "Same station, looking down toward the nearest planted willow"),
    ]
    for row, (stem, title) in enumerate(labels):
        y = 73 + row * (h + 27)
        for col, state in enumerate([1, 2]):
            path = folder / (stem + "-" + str(state) + ".png")
            pic = Image.open(path).convert("RGB")
            if pic.size != (w, h):
                raise ValueError("Do not silently crop unlike cameras")
            panel.paste(pic, (16 + col * (w + 10), y))
        pen.text((17, y + h + 3), title, font=face(14), fill="#43594e")
    pen.text(
        (16, panel.height - 19),
        "Offline GLB art study on the real V1 surface; local source-mesh light, no native game capture.",
        font=face(13),
        fill="#52685b",
    )
    panel.save(folder / "Twin-Needle-Meadow.jpg", quality=89, optimize=True)


if __name__ == "__main__":
    pair(Path(sys.argv[1]))


def one_glance(folder):
    """When only the exact amended close pair exists, never fill rows from obsolete studies."""
    names = [
        ("glance-1.png", "Held rise and original windwillow"),
        ("glance-2.png", "Rooted meadow, seed heads and leaf litter"),
    ]
    paper = Image.new("RGB", (1460, 511), "#e8e5d9")
    ink = ImageDraw.Draw(paper)

    def font(sz, bold=False):
        try:
            return ImageFont.truetype(
                "DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf", sz
            )
        except OSError:
            return ImageFont.load_default()

    ink.text((11, 8), "TWIN NEEDLE RISE", font=font(20, True), fill="#293c32")
    ink.text(
        (228, 10),
        "The same 1.04 m eye, looking down into the meadow",
        font=font(17),
        fill="#445c4e",
    )
    for i, (src, label) in enumerate(names):
        pic = Image.open(folder / src).convert("RGB")
        if pic.size != (720, 428):
            raise ValueError("Mismatched art desk size")
        x = 7 + i * 727
        ink.text((x + 3, 38), label, font=font(15), fill="#334837")
        paper.paste(pic, (x, 60))
    ink.text(
        (10, 493),
        "CPU illustration of the editable GLB at original island coordinates. Source-mesh sunlight; not Blender or an in-game capture.",
        font=font(12),
        fill="#455d4f",
    )
    paper.save(folder / "Twin-Needle-Ground.jpg", quality=89, optimize=True)
