"""Draw the soil, growing, harvest, delivery and return signs for the Hand."""

from pathlib import Path

from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parents[1] / "assets/ui/orchard"
OUT.mkdir(parents=True, exist_ok=True)
INK = (250, 239, 210, 255)
MINT = (143, 223, 184, 255)
GOLD = (255, 171, 94, 255)
WATER = (123, 208, 224, 255)


def sign(name, paint):
    im = Image.new("RGBA", (192, 192))
    d = ImageDraw.Draw(im)

    def line(points, fill=INK, width=4):
        d.line(
            [(4 * x, 4 * y) for x, y in points],
            fill=fill,
            width=width * 4,
            joint="curve",
        )

    def oval(box, fill):
        d.ellipse(tuple(round(i * 4) for i in box), fill=fill)

    def poly(points, fill):
        d.polygon([(4 * x, 4 * y) for x, y in points], fill=fill)

    paint(line, oval, poly)
    im.resize((48, 48), Image.Resampling.LANCZOS).save(OUT / f"{name}.png")


def sow(line, oval, poly):
    line([(5, 35), (13, 39), (34, 39), (43, 35)], MINT)
    oval((17, 24, 27, 34), GOLD)
    line([(22, 23), (23, 18), (28, 14)], MINT, 3)
    oval((26, 8, 38, 16), MINT)
    oval((14, 9, 23, 19), MINT)


def water(line, oval, poly):
    poly([(8, 19), (21, 17), (27, 30), (14, 34)], INK)
    line([(10, 19), (7, 14), (5, 15), (7, 22)], INK, 2)
    line([(22, 22), (32, 18), (35, 20)], INK, 3)
    for x, y in [(36, 23), (39, 29), (33, 34)]:
        oval((x - 2, y - 1, x + 1, y + 3), WATER)
    line([(27, 41), (39, 39)], MINT, 2)


def watch(line, oval, poly):
    line(
        [
            (4, 24),
            (13, 16),
            (24, 13),
            (35, 16),
            (44, 24),
            (35, 32),
            (24, 35),
            (13, 32),
            (4, 24),
        ],
        INK,
        3,
    )
    oval((18, 18, 30, 30), MINT)
    oval((22, 21, 26, 26), (26, 60, 60, 255))


def pick(line, oval, poly):
    oval((9, 16, 29, 36), GOLD)
    line([(19, 17), (23, 10), (29, 9)], MINT, 3)
    oval((22, 7, 33, 12), MINT)
    line([(17, 41), (28, 39), (39, 28), (42, 29), (37, 39), (28, 43)], INK, 3)


def offer(line, oval, poly):
    oval((4, 8, 20, 24), GOLD)
    line([(12, 10), (16, 5), (21, 5)], MINT, 2)
    line([(18, 27), (29, 27)], INK, 2)
    line([(25, 23), (30, 27), (25, 31)], INK, 2)
    oval((31, 24, 44, 36), MINT)
    poly([(31, 27), (29, 19), (36, 24)], MINT)
    poly([(40, 25), (45, 20), (43, 29)], MINT)
    oval((34, 28, 36, 30), (30, 54, 54, 255))


def return_garden(line, oval, poly):
    line([(7, 17), (12, 10), (23, 7), (34, 11), (39, 20)], MINT, 3)
    line([(32, 17), (40, 22), (43, 14)], INK, 3)
    line([(10, 34), (16, 39), (32, 39), (37, 34)], INK, 3)
    line([(24, 34), (24, 23)], MINT, 3)
    oval((13, 18, 24, 27), MINT)
    oval((24, 16, 34, 24), GOLD)


sign("return", return_garden)

for art in (sow, water, watch, pick, offer):
    sign(art.__name__, art)
