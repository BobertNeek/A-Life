"""Place the unaltered locked photographs on a calm contact sheet."""

from pathlib import Path
import sys, json, zipfile, hashlib
from PIL import Image, ImageDraw, ImageFont


def make(held, out, detail):
    pages = (
        (
            "pennon-eye",
            "A fallen crown",
            "1.04 m eye. Looking between the original seven pennons.",
        ),
        (
            "stone-eye",
            "Across the windward bank",
            "1.04 m eye. The earlier coastal horizon and lens are held.",
        ),
        (
            "upper-rim",
            "The same patch from above",
            "143 m frame. No centimetre-scale loose fragments drawn.",
        ),
    )
    w = 840
    m = 26
    gap = 14
    WW = w * 2 + gap + 2 * m
    sheet = Image.new("RGB", (WW, 1900), "#f0eee3")
    p = ImageDraw.Draw(sheet)
    inks = "#253732"
    sub = "#5c6d63"
    warm = "#955f46"

    def text(x, y, s, sz, col=inks, b=False):
        n = (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if b else "")
            + ".ttf"
        )
        p.text((x, y), s, font=ImageFont.truetype(n, sz), fill=col, anchor="lt")

    p.rectangle((0, 0, WW, 120), fill=inks)
    text(m, 19, "QUALIA     /     LANTERNBACK RIM", 17, "#d3d3b4", True)
    text(m, 50, "Pennon Drift", 43, "#fbf7e9", True)
    text(
        620,
        61,
        "Shale, copper chaff and worn sheaths below a real cluster of crowns.",
        22,
        "#dde2d4",
    )
    text(m, 140, "HELD LANTERNBACK", 18, sub, True)
    text(m + w + gap, 140, "GROUND CONTACT AND WINDFALL", 18, warm, True)
    receipt = dict(
        method="Blender 4.3.2 Cycles, locked held island light. Native game not launched.",
        held="3de1d24c3e3d6f9b848b1ca61cda0da012f5374b",
        image_sha256={},
    )
    full = out / "originals.zip"
    with zipfile.ZipFile(full, "w", compression=zipfile.ZIP_STORED) as z:
        for i, (name, title, note) in enumerate(pages):
            top = 184 + i * 563
            text(m, top, f"0{i+1}   /   {title}", 23, inks, True)
            text(565, top + 2, note, 18, sub)
            for j, side in enumerate(("before", "after")):
                path = (held if i and j == 0 else out / side) / (name + ".png")
                data = path.read_bytes()
                dest = side + "/" + name + ".png"
                z.writestr(dest, data)
                receipt["image_sha256"][dest] = hashlib.sha256(data).hexdigest()
                im = Image.open(path).convert("RGB")
                sheet.paste(im, (m + j * (w + gap), top + 38))
            p.line((m, top + 548, WW - m, top + 548), fill="#c4cbbd", width=2)
    text(
        m,
        1874,
        "Offline Blender study on unchanged V1 ground. Not an in-game screenshot.",
        16,
        sub,
    )
    text(
        1070,
        1874,
        "Draw distance is an art decision here, not a performance result.",
        16,
        sub,
    )
    sheet.save(out / "Pennon-Drift.jpg", quality=94, subsampling=0, optimize=True)
    receipt["detail"] = json.loads((detail / "contact.json").read_text())
    receipt["close"] = json.loads((out / "station.json").read_text())
    (out / "capture.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    make(*(Path(a) for a in sys.argv[1:4]))
