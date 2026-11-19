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
