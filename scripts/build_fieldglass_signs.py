"""Paint Hand fieldglass states and their temporary optical marks."""

from pathlib import Path

from PIL import Image, ImageDraw

out = Path(__file__).resolve().parents[1] / "assets/ui/fieldglass"
out.mkdir(parents=True, exist_ok=True)
ivory, mint, gold = "#faecd1", "#98dcc7", "#f7a65c"
for state in ("food", "toy", "carried", "consumed"):
    im = Image.new("RGBA", (192, 192))
    d = ImageDraw.Draw(im)
    d.line([(113, 117), (151, 157)], fill="#92745d", width=24)
    d.line([(118, 123), (149, 154)], fill=gold, width=12)
    d.ellipse((20, 20, 139, 139), fill="#254943", outline=ivory, width=8)
    d.arc((31, 31, 128, 128), 201, 303, fill=mint, width=5)
    if state == "food":
        d.ellipse((54, 66, 101, 108), fill=gold)
        d.polygon([(55, 80), (72, 51), (85, 51), (99, 83)], fill=gold)
        d.ellipse((78, 44, 99, 55), fill=mint)
    elif state == "toy":
        d.ellipse((51, 52, 107, 108), fill=gold)
        d.arc((39, 41, 102, 101), 5, 103, fill=ivory, width=8)
        d.arc((63, 65, 120, 123), 179, 285, fill=mint, width=6)
    elif state == "carried":
        d.ellipse((75, 46, 103, 74), fill=gold)
        d.line([(47, 98), (61, 87), (78, 91), (101, 77)], fill=mint, width=12)
        d.line([(50, 98), (77, 108), (105, 85)], fill=ivory, width=7)
    else:
        d.arc((55, 63, 104, 105), 1, 177, fill=ivory, width=8)
        d.line([(78, 86), (88, 60)], fill=mint, width=6)
        d.ellipse((84, 51, 103, 63), fill=mint)
    im.resize((64, 64), Image.Resampling.LANCZOS).save(out / f"{state}.png")

# In-world optics, not an item or a reward. The split arcs surround a live
# target; the little empty cup is the same husk pictured in the consumed lens.
for state in ("locator", "afterimage"):
    im = Image.new("RGBA", (256, 192))
    d = ImageDraw.Draw(im)
    for start, end in ((22, 76), (105, 159), (201, 248), (290, 338)):
        for w, c in ((10, "#285049cc"), (4, "#d6f6e2fa")):
            d.arc((24, 91, 232, 166), start, end, fill=c, width=w)
    if state == "afterimage":
        for w, c in ((13, "#285049ee"), (6, ivory)):
            d.arc((98, 48, 157, 98), 2, 178, fill=c, width=w)
            d.line([(127, 76), (138, 46)], fill=c, width=w)
        d.ellipse((134, 32, 159, 47), fill=mint, outline="#285049", width=3)
    im.resize((128, 96), Image.Resampling.LANCZOS).save(out / f"{state}.png")
