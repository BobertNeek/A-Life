"""Communicate the held sea and two fixed stations without altering their rendered pixels."""

from pathlib import Path
import hashlib, json, sys
from PIL import Image, ImageFont, ImageDraw


def make(p, before):
    W = 1746
    m = 26
    w = 840
    g = 14
    im = Image.new("RGB", (W, 1360), "#efede2")
    d = ImageDraw.Draw(im)

    def put(x, y, s, n, col="#253a35", bold=False):
        ft = (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if bold else "")
            + ".ttf"
        )
        d.text((x, y), s, font=ImageFont.truetype(ft, n), fill=col, anchor="lt")

    d.rectangle((0, 0, W, 115), fill="#223b3b")
    put(m, 16, "QUALIA     /     GULLGLASS HEAD", 16, "#d0d5bf", 1)
    put(m, 47, "Stone at the sea", 45, "#f8f2df", 1)
    put(
        660,
        61,
        "A real tide shelf, from the crest to the rock underfoot.",
        22,
        "#dce1d3",
    )
    put(m, 139, "BEFORE   /   GULLGLASS FOUNDATION", 18, "#5e6b60", 1)
    put(m + w + g, 139, "AFTER   /   THE LIVED-IN SHORE", 18, "#95694a", 1)
    receipt = {}
    for i, (tag, name, note) in enumerate(
        (
            (
                "outside",
                "The whole headland",
                "119 m guardian frame. The original heights and thirteen shoreline roots are held.",
            ),
            (
                "wave-eye",
                "The real standing eye",
                "1.04 m above the genuine shore, looking over the joints and tide casts.",
            ),
        )
    ):
        y = 179 + i * 563
        put(m, y, "0" + str(i + 1) + "  /  " + name, 24, bold=True)
        put(654, y + 3, note, 17, "#586b63")
        for j, side in enumerate(("before", "after")):
            path = (before if side == "before" else p / "after") / (tag + ".png")
            a = Image.open(path).convert("RGB")
            im.paste(a, (m + j * (w + g), y + 40))
            receipt[side + "/" + tag + ".png"] = hashlib.sha256(
                path.read_bytes()
            ).hexdigest()
        d.line((m, y + 551, W - m, y + 551), fill="#bec9bd", width=2)
    put(
        m,
        1313,
        "Blender art, not gameplay. Original ground and waterline; right adds local stone, casts and a subtle optical wash.",
        17,
        "#52665d",
    )
    im.save(p / "Gullglass-Reefside.jpg", quality=93, subsampling=0, optimize=True)
    (p / "frame-hashes.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    make(Path(sys.argv[1]), Path(sys.argv[2]))
