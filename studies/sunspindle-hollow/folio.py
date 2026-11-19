"""Two full, matched photographs, plus one honest section across the dry basin."""

import csv, json, sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

FONT = Path("/usr/share/fonts/truetype/dejavu")


def face(n, bold=False):
    return ImageFont.truetype(
        str(FONT / ("DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf")), n
    )


def compose(base, out, control=None):
    control = control or base
    site = json.loads((base / "hollow/site.json").read_text())
    W, H = 900, 540
    m, g = 30, 16
    CW = 2 * W + 2 * m + g
    paper = "#f4f1e6"
    ink = "#253937"
    sub = "#54685e"
    ochre = "#996e43"
    rule = "#c9cbbb"
    image = Image.new("RGB", (CW, 1580), paper)
    d = ImageDraw.Draw(image)

    def text(x, y, t, s, col=ink, b=False):
        d.text((x, y), t, font=face(s, b), fill=col, anchor="lt")

    d.rectangle((0, 0, CW, 125), fill=ink)
    text(m, 19, "QUALIA   /   EAST ISLAND", 18, "#ddcfac", True)
    text(m, 52, "Sunspindle Hollow", 51, "#fbf7e9", True)
    text(718, 67, "A dry fold between the foredune and the woodland.", 28, "#d7e1d1")
    text(m, 147, "EXISTING GROUND", 18, sub, True)
    text(m + W + g, 147, "REGISTERED FIELD ART", 18, ochre, True)
    for i, (tag, title, explain) in enumerate(
        [
            (
                "slack-eye",
                "01   At creature height",
                "1.04 m above the original soil. Same lens, light and background.",
            ),
            (
                "slack-guardian",
                "02   Above the hollow",
                "Same 132 m orthographic span. No dune or shoreline was moved.",
            ),
        ]
    ):
        y = 189 + i * 614
        text(m, y, title, 25, ink, True)
        text(495, y + 3, explain, 20, sub)
        for j, side in enumerate(["before", "after"]):
            raw = Image.open(
                (control if side == "before" else base) / side / (tag + ".png")
            ).convert("RGB")
            if raw.size != (1000, 600):
                raise ValueError("Unmatched studio gate size")
            image.paste(
                raw.resize((W, H), Image.Resampling.LANCZOS), (m + j * (W + g), y + 42)
            )
        d.line((m, y + 598, CW - m, y + 598), fill=rule, width=2)
    y = 1435
    rs = list(csv.DictReader((base / "hollow/sections.csv").open()))
    r = min(rs, key=lambda a: abs(float(a["z"]) + 54))
    text(m, y, "Why this grows here", 25, ink, True)
    text(
        m,
        y + 41,
        f"At one section: west shoulder {float(r['west_18m_y']):.1f} m, hollow floor {float(r['lowest_y']):.1f} m, dune side {float(r['east_18m_y']):.1f} m.",
        21,
        sub,
    )
    text(
        m,
        y + 83,
        "Four plant forms, each root registered to the native triangles. The sheltered basin remains dry.",
        20,
        sub,
    )
    text(1465, y + 16, "BLENDER ART", 20, ochre, True)
    text(1465, y + 52, "Not an in-game capture.", 18, sub)
    image.save(out, quality=93, subsampling=0, optimize=True)


if __name__ == "__main__":
    compose(
        Path(sys.argv[1]),
        Path(sys.argv[2]),
        Path(sys.argv[3]) if len(sys.argv) > 3 else None,
    )
