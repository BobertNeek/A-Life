"""A small true-coordinate field sheet to reopen the art, explicitly not a perspective picture."""

from pathlib import Path
import csv, sys, json, math
import numpy as np
from PIL import Image, ImageDraw, ImageFont

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[2] / "scripts"))
from island_inlet_ground import Inlet


def board(native, field):
    b = Inlet(native / "island-v1.bin")
    props = json.loads((native / "props.json").read_text())
    r = json.loads((field / "rise.json").read_text())
    table = list(csv.DictReader((field / "lee-roots.csv").open()))
    sects = list(csv.DictReader((field / "rise-section.csv").open()))
    W, HT = 1040, 750
    res = Image.new("RGB", (W, HT), "#eeeada")
    pen = ImageDraw.Draw(res)

    def font(n, bold=False):
        return ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans"
            + ("-Bold" if bold else "")
            + ".ttf",
            n,
        )

    def label(x, y, s, f=15, c="#354941", bold=False):
        pen.text((x, y), s, font=font(f, bold), anchor="lt", fill=c)

    pen.rectangle([0, 0, W, 85], fill="#273c38")
    label(22, 13, "QUALIA   /   ISLAND FIELD SURVEY", 14, "#d5d9bd", 1)
    label(22, 35, "Twin Needle Rise", 34, "#faf1d9", 1)
    label(
        578, 49, "Measured on held V1, not a render or gameplay route.", 15, "#cdd8c8"
    )
    margin = 24
    M, N = 660, 384
    x0, x1 = 332.0, 420.0
    z0, z1 = -109.0, -58.0
    xx, zz = np.meshgrid(
        x0 + (np.arange(M) + 0.5) * (x1 - x0) / M,
        z1 - (np.arange(N) + 0.5) * (z1 - z0) / N,
    )
    h, gx, gz = b.floor(xx, zz)
    light = (
        0.35 + 0.55 * (0.19 * gx - 0.31 * gz + 1) / np.sqrt(1 + gx * gx + gz * gz)
    ).clip(0.35, 0.96)
    lf = np.clip(h / 24, 0, 1)
    col = (1 - lf[..., None]) * np.array([147, 156, 113]) + lf[..., None] * np.array(
        [197, 177, 126]
    )
    col *= light[..., None]
    water = h < 0
    col[water] = np.array([92, 150, 155]) + np.minimum(1, -h[water] / 5)[
        :, None
    ] * np.array([-21, -18, -11])
    # Pixel-distance contours stay hairline on both flats and steep slopes.
    # A constant world-height threshold visibly floods flat elevations.
    grad = np.hypot(gx * (x1 - x0) / M, gz * (z1 - z0) / N)
    du = abs((h + 1.0) % 2.0 - 1.0)
    ink = np.clip(1.0 - du / np.maximum(0.002, 0.76 * grad), 0.0, 1.0) * ~water
    col *= 1.0 - 0.24 * ink[..., None]
    xat = margin
    yat = 112
    res.paste(Image.fromarray(col.clip(0, 255).astype("B")), (xat, yat))

    def spot(x, z):
        return (xat + (x - x0) * M / (x1 - x0), yat + (z1 - z) * N / (z1 - z0))

    palette = {1: "#6f7438", 2: "#ab7649", 3: "#c0b9a4"}
    for row in table:
        x, y = spot(float(row["x"]), float(row["z"]))
        volume = 2.1 if int(row["habitat"]) != 3 else 2.6
        pen.ellipse(
            [x - volume, y - volume, x + volume, y + volume],
            fill=palette[int(row["habitat"])],
        )
    course = [spot(float(a["x"]), float(a["z"])) for a in sects]
    # Dot-dash sightline, not a physical painted trail nor route permission.
    for i in range(0, len(course) - 2, 5):
        pen.line(course[i : i + 3], fill="#f3e6c1", width=2)
    for k in [113, 638]:
        x, z = props[k]["position"][0::2]
        vx, vy = spot(x, z)
        pen.polygon(
            [(vx - 4, vy + 3), (vx + 4, vy + 3), (vx, vy - 6)],
            fill="#243f35",
            outline="#dbe5c2",
        )
    ex, et = spot(366.2, -83.7)
    pen.ellipse([ex - 4, et - 4, ex + 4, et + 4], outline="#f7eed4", width=2)
    pen.rectangle([xat, yat, xat + M, yat + N], outline="#5d7167", width=2)
    label(713, 117, "What is really here", 23, bold=True)
    for i, (v, sub) in enumerate(
        [
            ("64.54 m", "original cross-section"),
            ("16.81 m", "sheltered floor to crest"),
            ("56.63 m", "first clear sea-point sightline"),
            ("220", "old Hollow roots in the overlap"),
        ]
    ):
        y = 166 + i * 65
        label(715, y, v, 23, "#715d43", True)
        label(715, y + 29, sub, 14)
    for j, (colour, title) in enumerate(
        [
            ("#6f7438", "Sheltered windwillow"),
            ("#ab7649", "Exposed copper rush"),
            ("#a39c83", "Silver mat in the spruce lee"),
        ]
    ):
        y = 425 + j * 17
        pen.ellipse([715, y + 2, 721, y + 8], fill=colour)
        label(729, y, title, 12, "#435c4c")
    label(713, 478, "▲   Two original native conifers", 12, "#3b584b")
    label(24, 524, "Actual ground profile", 21, bold=True)
    label(575, 531, "The white dashes locate the section, not a path.", 14)
    px, py = 28, 688
    pw, ph = 980, 119
    vals = [(float(a["along_m"]), float(a["native_y"])) for a in sects]
    pts = [(px + d / 64.538 * pw, py - (h + 1) / 24 * ph) for d, h in vals]
    pen.polygon([(px, py), *pts, (pts[-1][0], py)], fill="#a39d75")
    pen.line(pts, fill="#4b6556", width=3)
    twin = (np.array(props[113]["position"]) + np.array(props[638]["position"])) * 0.5
    tree_dist = float(
        min(
            sects,
            key=lambda a: (float(a["x"]) - twin[0]) ** 2
            + (float(a["z"]) - twin[2]) ** 2,
        )["along_m"]
    )
    for along, name in [
        (0, "lower hollow"),
        (tree_dist, "twin spruce"),
        (56.63, "sea point clears"),
    ]:
        xx = px + along / 64.538 * pw
        pen.line([(xx, py + 2), (xx, py + 8)], fill="#455e52")
        label(min(W - 150, xx), py + 13, name, 13)
    label(
        23,
        728,
        "Source field sheet. Fixed 1.04 m eye, V1 heights and two original prop records. No physics or terrain modification.",
        12,
        "#62766a",
    )
    res.save(field / "Twin-Needle-Rise-survey.jpg", quality=91)


if __name__ == "__main__":
    board(*(Path(s) for s in sys.argv[1:3]))
