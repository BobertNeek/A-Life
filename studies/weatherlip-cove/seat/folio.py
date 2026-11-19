"""Identical image windows from three old and new locked native-coordinate photographs."""
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

FONT=Path('/usr/share/fonts/truetype/dejavu')
def f(n,b=False):return ImageFont.truetype(str(FONT/('DejaVuSans-Bold.ttf' if b else 'DejaVuSans.ttf')),n)

def compose(before,after,report,out):
    import json
    info=json.loads(report.read_text())
    v,w,g=22,800,15
    bg='#f5f1e7';dark='#243b38'; muted='#576860';copper='#99633f';line='#c4c8b7'
    W=v*2+w*2+g
    paper=Image.new('RGB',(W,2300),bg);d=ImageDraw.Draw(paper)
    def cap(x,y,text,size,fill=dark,bold=False):
        d.text((x,y),text,font=f(size,bold),fill=fill,anchor='lt')
    d.rectangle((0,0,W,130),fill=dark)
    cap(v,19,'QUALIA   /   WEST COAST • ROCK WORKSHOP',20,'#d2ddc4',True)
    cap(v,60,'Weatherlip Cove: where the stones meet the ground',42,'#fbf7ec',True)
    cap(v,153,'PREVIOUS COVE',23,copper,True);cap(v+w+g,153,'FITTED HEEL AND COASTAL SURFACE',23,copper,True)
    cap(v,188,'Two matched 1.04 m crops and one full guardian frame. Original rock positions, solid footprints, slope and sun on both sides.',21,muted)
    y=235
    for key,tag,cut,title in [(1619,'bight-beach',(308,125,708,425),'01  /  Sheltered bench: warm erratic'),
                              (1785,'bight-tooth',(570,10,970,310),'02  /  Northern toe: dark erratic')]:
        first,second=[p/(tag+'.png') for p in (before,after)]
        cap(v,y,title,27,dark,True)
        r=info[str(key)]['levels']['L0']['bodies'][0]
        cap(690,y+4,f"Main body: bed-supported samples {100*r['old_contact_fraction']:.0f}% → {100*r['new_contact_fraction']:.0f}%*",20,muted)
        for col,p in enumerate([first,second]):
            with Image.open(p) as im:
                if im.size!=(1000,600):raise ValueError('This is not the locked photograph.')
                q=im.convert('RGB').crop(cut).resize((w,600),Image.Resampling.LANCZOS)
                paper.paste(q,(v+(w+g)*col,y+45))
        d.line((v,y+658,W-v,y+658),fill=line,width=2)
        y+=684
    cap(v,y,'03  /  The complete bight',27,dark,True)
    cap(600,y+4,'Full guardian frame. The 337 m shore is not repainted or raised.',20,muted)
    for col,p in enumerate([before,after]):
        with Image.open(p/'bight-high.png') as im:
            q=im.convert('RGB').resize((w,480),Image.Resampling.LANCZOS)
            paper.paste(q,(v+(w+g)*col,y+45))
    y+=553
    cap(v,y,'*Within 1.5 cm of, or intersecting, original ground; sampled across the same plan of the largest stone in each group.',18,muted)
    cap(v,y+35,'Actual underside clearance remains where the saved solid ends above the downhill bed. No shadow decal or raised soil.',18,muted)
    cap(v,y+72,'Matched offline Blender artwork • original V1 • no runtime or collision integration',18,copper,True)
    paper.crop((0,0,W,y+115)).save(out,quality=92,subsampling=0,optimize=True)
if __name__=='__main__':compose(*map(Path,sys.argv[1:]))
