"""Pair actual locked artwork; neither shadows nor a model are inferred here."""
from pathlib import Path
import argparse
import json
from PIL import Image, ImageDraw, ImageFont

INK = "#1a2b29"
PAPER = "#f6f2e8"
MUTED = "#52655d"
ACCENT = "#8d5b37"
FONT = Path("/usr/share/fonts/truetype/dejavu")

def face(size, bold=False):
    return ImageFont.truetype(str(FONT / ("DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf")), size)

def create(broad, close, selected, palette, dest):
    w, h, gap, margin = 1000, 600, 18, 24
    stations = [
        ("reach-guardian", "01   Over the planted reach", "Guardian / orthographic", broad),
        ("reach-meadow", "02   Into the wind garden", "Meadow / eye 1.04 m", broad),
        ("reach-alder", "03   Under the bluff alder", "Turn back inland / eye 1.04 m", close),
        ("reach-strand", "04   The last dry turn", "Out to the strand / eye 1.04 m", close),
    ]
    data = json.loads(palette.read_text())
    canvas = Image.new("RGB", (2*w+2*margin+gap, 183+4*(h+83)+79), PAPER)
    d = ImageDraw.Draw(canvas)
    def label(x,y,content,sz,ink=INK,bold=False):
        d.text((x,y),content,font=face(sz,bold),fill=ink,anchor="lt")
    d.rectangle([0,0,canvas.width,116],fill=INK)
    label(margin,21,"QUALIA   /   ISLAND ART WORKSHOP",20,"#c5cfb0",True)
    label(margin,52,"Shellback Reach   /   A lighter planted coast",46,"#faf8f0",True)
    label(margin,136,"FULL ARTIST STUDY",22,ACCENT,True)
    label(margin+w+gap,136,"PLACED PALETTE   /   12 SPECIMENS, THREE LEVELS",22,ACCENT,True)
    for i,(key,name,kind,src) in enumerate(stations):
        y=174+i*(h+83)
        s=data["stations"][key]
        old=s["old_visible_artist_triangles"]
        new=s["selected_total_triangles"]
        label(margin,y,name,26,INK,True)
        label(margin+480,y+5,kind,20,MUTED)
        label(margin+w+gap,y+4,f"{old:,} to {new:,} artist triangles*     /     {100*(1-new/old):.0f}% fewer",21,MUTED)
        y+=43
        for j,folder in enumerate([src,selected]):
            with Image.open(folder/(key+".png")) as im:
                if im.size != (w,h):
                    raise ValueError(f"{key}: source dimensions changed; do not crop to conceal it")
                canvas.paste(im.convert("RGB"),(margin+j*(w+gap),y))
        d.line((margin,y+h+14,canvas.width-margin,y+h+14),fill="#ced1be",width=2)
    y=canvas.height-75
    label(margin,y,"*Triangles selected for the added artwork, before viewport or occlusion culling; not a measured game submission.",19,MUTED)
    label(margin,y+28,"Matched offline Blender photographs on unchanged V1. Same original site, lens, sun and soil. No gameplay or frame-rate claim.",19,MUTED)
    canvas.save(dest,quality=91,subsampling=0,optimize=True)

if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__)
    for n in ("broad","close","selected","palette","dest"):
        p.add_argument(n,type=Path)
    a=p.parse_args()
    create(a.broad,a.close,a.selected,a.palette,a.dest)
