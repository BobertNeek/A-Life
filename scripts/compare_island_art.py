"""Leave the actual matched pixels alone: only add an external label and gutter."""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont


def make(before, after, title, out):
    a, b = (Image.open(p).convert("RGB") for p in (before, after))
    assert a.size == b.size
    w, h = a.size
    result = Image.new("RGB", (w * 2 + 26, h + 90), (25, 39, 40))
    draw = ImageDraw.Draw(result)
    def font(size, suffix=""):
        try:
            return ImageFont.truetype("DejaVuSans"+suffix+".ttf",size)
        except OSError:
            return ImageFont.load_default()
    draw.text((13,9),title,fill="#F0F2DF",font=font(23,"-Bold"))
    for i,(im,label) in enumerate(((a,"Previous"),(b,"Revised"))):
        x=9+i*(w+8)
        draw.text((x+4,44),label,fill="#C3D1C6",font=font(18))
        result.paste(im,(x,76))
    result.save(out,quality=94,optimize=True)


if __name__=="__main__":
    cli=argparse.ArgumentParser(description=__doc__)
    for key in ("before","after","out"):cli.add_argument(key,type=Path)
    cli.add_argument("--title",required=True)
    a=cli.parse_args()
    make(a.before,a.after,a.title,a.out)
