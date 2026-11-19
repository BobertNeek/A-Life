"""Compose three full-frame matched art photographs and an actual coastal section."""
from pathlib import Path
import csv,json,sys
from PIL import Image,ImageDraw,ImageFont

FONT=Path('/usr/share/fonts/truetype/dejavu')
def face(n,bold=False):return ImageFont.truetype(str(FONT/('DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf')),n)

def compose(folder,dest,controls=None):
    controls=controls or folder
    paper='#f4f1e7';dark='#243a38';muted='#52655e';warm='#936344';rule='#c4c9ba'
    W,H=1000,600; m,g=24,18
    canvas=Image.new('RGB',(m*2+W*2+g,245+3*(H+91)+456+220),paper)
    draw=ImageDraw.Draw(canvas)
    def label(x,y,s,n,ink=dark,bold=False):draw.text((x,y),s,font=face(n,bold),fill=ink,anchor='lt')
    draw.rectangle((0,0,canvas.width,149),fill=dark)
    label(m,20,'QUALIA     /     WEST COAST FIELD STUDY',20,'#ccdbbf',True)
    label(m,55,'Weatherlip Cove',52,'#fbf7ec',True)
    label(m+525,79,'A single native shoreline, from exposed headland to sheltered strand.',27,'#dbe2d0')
    label(m,169,'EXISTING ISLAND',24,warm,True)
    label(m+W+g,169,'ROCK, SAND AND LIVING MARGINS',24,warm,True)
    label(m,207,'Same position, light, ocean and original planted landmarks on both sides. Full-frame Blender artwork, not gameplay.',21,muted)
    rows=[('bight-high','01   /   The open bight','From the water: a high northern shoulder, a wooded middle and a lower sandy end.'),
          ('bight-tooth','02   /   Northern rock toe','Eye 1.04 m above V1, looking past original boulder #1785 toward the head.'),
          ('bight-beach','03   /   Sheltered stone bench','Eye 1.04 m above V1, facing original erratics #1619 and #1472.')]
    for i,(key,header,line) in enumerate(rows):
        y=246+i*(H+91)
        label(m,y,header,28,dark,True);label(560,y+5,line,22,muted)
        for j,side in enumerate(('before','after')):
            with Image.open((controls if side=='before' else folder)/side/(key+'.png')) as photo:
                if photo.size!=(W,H):raise ValueError('The paired source dimension changed.')
                canvas.paste(photo.convert('RGB'),(m+j*(W+g),y+46))
        draw.line((m,y+H+66,canvas.width-m,y+H+66),fill=rule,width=2)
    y=245+3*(H+91)
    label(m,y+4,'Detail   /   At the toes of the northern head',26,dark,True)
    label(m+700,y+8,'Identical source pixels from the second row, shown at 2×.',21,muted)
    rect=(475,388,975,579)
    for j,side in enumerate(('before','after')):
        with Image.open((controls if side=='before' else folder)/side/'bight-tooth.png') as photo:
            crop=photo.crop(rect).convert('RGB').resize((1000,382),Image.Resampling.LANCZOS)
            canvas.paste(crop,(m+j*(W+g),y+48))
    draw.line((m,y+442,canvas.width-m,y+442),fill=rule,width=2)
    y+=456
    with (folder/'weatherlip'/'native-sections.csv').open() as f:sections=list(csv.DictReader(f))
    def section(z):
        r=min(sections,key=lambda a:abs(float(a['native_z'])-z))
        return [float(r['bed_y_minus_2m']),0.,float(r['bed_y_plus_3m']),float(r['bed_y_plus_15m']),float(r['bed_y_plus_38m'])]
    north,south=section(-91),section(211)
    label(m,y+8,'What the island already gives us',26,dark,True)
    label(m,y+56,f'15 m inland: {north[3]:.1f} m up on the headland, just {south[3]:.1f} m up at the sheltered end.',23,muted)
    label(m,y+98,'The new beds follow that difference. No shoreline, solid rock, tree position or collision was moved.',20,muted)
    label(m,y+160,'337 m of actual sea-level contour   •   Three locked cameras   •   Original V1',21,warm,True)
    xa,ya=1315,y+175
    for sh,cl,name in ((north,'#7c806a','Windward head'),(south,'#ad7c54','Sheltered sand')):
        coord=[(xa+(dist+2)*10.8,ya-h*5.0) for dist,h in zip((-2,0,3,15,38),sh)]
        draw.line(coord,fill=cl,width=5,joint='curve')
        label(xa+456,coord[-1][1]-4,name,18,cl,True)
    canvas.save(dest,quality=91,subsampling=0,optimize=True)
if __name__=='__main__':compose(Path(sys.argv[1]),Path(sys.argv[2]),Path(sys.argv[3]) if len(sys.argv)>3 else None)
