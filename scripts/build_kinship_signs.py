"""Rasterize the small, font-free enamel signs used in the field album."""

from pathlib import Path

from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parents[1] / "assets/ui/kinship"
S = 4
INK = (250, 240, 211, 255)
JADE = (155, 222, 191, 255)
AMBER = (249, 190, 103, 255)


def make(name, draw):
    img = Image.new("RGBA", (48 * S, 48 * S))
    ctx = ImageDraw.Draw(img)

    def line(points, width=3, fill=INK):
        ctx.line(
            [(x * S, y * S) for x, y in points],
            fill=fill,
            width=width * S,
            joint="curve",
        )

    def oval(box, fill=INK, outline=None, width=2):
        ctx.ellipse(
            tuple(v * S for v in box), fill=fill, outline=outline, width=width * S
        )

    draw(line, oval)
    img.resize((48, 48), Image.Resampling.LANCZOS).save(OUT / f"{name}.png")


def family(line, oval):
    line([(10, 30), (10, 22), (24, 22), (24, 12)], 3, JADE)
    line([(38, 30), (38, 22), (24, 22)], 3, JADE)
    for x, y in [(24, 10), (10, 35), (38, 35)]:
        oval((x - 5, y - 5, x + 5, y + 5))


def seed(line, oval):
    oval((10, 23, 27, 40), AMBER)
    line([(21, 29), (24, 21), (30, 15)], 3, JADE)
    for box in [(21, 8, 31, 23), (28, 11, 41, 19)]:
        oval(box, JADE)


def veil(line, oval):
    oval((10, 10, 38, 38), None, (162, 182, 185, 255), 3)
    for x in (16, 24, 32):
        oval((x - 1, 23, x + 1, 25))


def memorial(line, oval):
    line([(8, 34), (40, 34)], 3, JADE)
    line([(24, 33), (24, 13)], 3)
    oval((13, 15, 24, 23), JADE)
    oval((24, 9, 36, 18), INK)


def handpath(line, oval):
    for x, y in [(6, 29), (16, 24), (26, 21)]:
        oval((x - 2, y - 2, x + 2, y + 2), AMBER)
    line([(30, 15), (39, 20), (32, 26)], 3, INK)


def previous(line, oval):
    line([(29, 10), (15, 24), (29, 38)], 4)


def next_page(line, oval):
    line([(19, 10), (33, 24), (19, 38)], 4)


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for sign in (family, seed, veil, memorial, handpath, previous, next_page):
        make("next" if sign is next_page else sign.__name__, sign)
