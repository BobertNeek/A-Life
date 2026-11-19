"""Matched art windows, exact source pixels retained without sharpening/resampling."""
from pathlib import Path
import sys
from PIL import Image,ImageDraw,ImageFont
root,out=map(Path,sys.argv[1:])
w,h=560,280
plate=Image.new('RGB',(2*w+24,2*h+170),(24,37,38))
d=ImageDraw.Draw(plate)
def font(n,bold=False):
    try:return ImageFont.truetype('DejaVuSans'+('-Bold' if bold else '')+'.ttf',n)
    except OSError:return ImageFont.load_default()
d.text((10,9),'Qualia: fern and moss at creature height',font=font(24,True),fill='#f1f1df')
d.text((12,43),'Previous',font=font(16),fill='#d0dccd')
d.text((w+18,43),'Revised',font=font(16),fill='#d0dccd')
for row,(name,box,label) in enumerate([
    ('copse-eye',(440,209,1000,489),'Existing fern on the slope'),
    ('sunlit-margin',(0,320,560,600),'Moss and litter at the wooded margin')]):
    y=70+row*(h+33)
    for col,side in enumerate(('before','after')):
        im=Image.open(root/side/(name+'.png')).convert('RGB')
        plate.paste(im.crop(box),(8+col*(w+8),y))
    d.text((10,y+h+3),label,font=font(15),fill='#bacdc1')
d.text((10,plate.height-21),'Matched 1.04 m Blender artwork. Same V1 terrain, lens and daylight. Not an in-game capture.',font=font(14),fill='#e1e4d6')
plate.save(out,quality=95,optimize=True)
