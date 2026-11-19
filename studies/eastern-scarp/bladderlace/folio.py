"""Current shore and genuine standing-height view down at the casts; do not enlarge a crop."""

from pathlib import Path
import sys, json, hashlib
from PIL import Image, ImageFont, ImageDraw


def layout(prior, photo):
    # Native photograph pixels display 1:1 on the board, unsharpened and unretouched.
    g, m, w = 14, 26, 840
    board = Image.new("RGB", (1746, 1360), "#f1eee0")
    d = ImageDraw.Draw(board)

    def label(x, y, words, sz, colour="#354b46", bold=False):
        face = (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if bold else "")
            + ".ttf"
        )
        d.text(
            (x, y), words, font=ImageFont.truetype(face, sz), anchor="lt", fill=colour
        )

    d.rectangle((0, 0, 1746, 116), fill="#263f40")
    label(m, 17, "QUALIA     /     GULLGLASS LITTORAL", 17, "#d8daca", True)
    label(m, 46, "The bladderlace beds", 44, "#fcf4df", True)
    label(
        716,
        60,
        "Seven rooted groups of folded shore fronds, all on the real island.",
        22,
        "#dce2d8",
    )
    label(m, 138, "BEFORE   /   GULLGLASS STONE AND CASTS", 18, "#647367", True)
    label(
        m + w + g, 138, "AFTER   /   OLIVE FRONDS AND AMBER FLOATS", 18, "#986949", True
    )
    record = {}
    for idx, (tag, name, context, before) in enumerate(
        (
            (
                "wave-eye",
                "The original coastal eye",
                "The same 29 mm camera, original sea level and authentic coast.",
                prior / "wave-eye.png",
            ),
            (
                "shelf-garden",
                "Beside a rooted shelf garden",
                "Same 1.04 m eye, looking down toward the real wet foothold.",
                photo / "before/shelf-garden.png",
            ),
        )
    ):
        y = 179 + idx * 563
        label(m, y, f"0{idx+1}  /  " + name, 24, bold=True)
        label(700, y + 4, context, 17, "#576c62")
        for j, src in enumerate((before, photo / "after" / (tag + ".png"))):
            img = Image.open(src).convert("RGB")
            if img.size != (840, 500):
                raise ValueError("Do not silently resize a source camera")
            board.paste(img, (m + j * (w + g), y + 40))
            record[("before" if not j else "after") + "/" + tag + ".png"] = (
                hashlib.sha256(src.read_bytes()).hexdigest()
            )
        d.line((m, y + 550, 1746 - m, y + 550), fill="#bcc9b9", width=2)
    label(
        m,
        1312,
        "Offline Blender study. Both sides hold the same V1 ground, casts, lighting and sea optics. Not a gameplay capture.",
        17,
        "#546a60",
    )
    board.save(photo / "Gullglass-Bladderlace.jpg", quality=93, subsampling=0)
    (photo / "frame-hashes.json").write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    layout(*[Path(n) for n in sys.argv[1:3]])
