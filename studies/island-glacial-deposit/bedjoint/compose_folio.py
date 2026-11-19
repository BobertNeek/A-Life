"""Mount the three untouched 1000x600 CPU-Blender comparisons on one art folio."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import sys
control,final_greens,dest=map(Path,sys.argv[1:])
rows=(
 ('talus-eye','01  THE GLACIAL BOULDER','Same 22 seated fragments. Only the large stone changes: rounded sole to a heel keyed into its actual uphill bed.'),
 ('woodland-gap','02  THROUGH THE WOODLAND GAP','Same fitted stone and forest floor. Shaded fronds and three-part leaves give way to open-grass tufts and lower silver rosettes.'),
 ('woodland-edge','03  TWO SPRUCE TO THE OPEN SHINGLE','Same 24.5 m original woodland, from the guardian. Compare planting rhythm and open ground, not a change of hillside or light.'),
)
w,h=Image.open(control/'before'/'talus-eye.png').size;gap=14;pad=20;head=118;band=75
folio=Image.new('RGB',(2*w+2*pad+gap,head+(h+band+pad)*3+16),(24,37,37));d=ImageDraw.Draw(folio)
def font(size,bold=False):return ImageFont.truetype(('DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf'),size)
d.text((pad,13),'The glacial boulder and its woodland edge',font=font(33,True),fill='#eff1de')
d.text((pad,57),'Matched CPU Blender artist captures on ordinary V1 terrain. Not a gameplay capture or a native installation.',font=font(21),fill='#d3dfca')
d.text((pad,92),'PREVIOUS',font=font(17,True),fill='#dfe3dc');d.text((pad+w+gap,92),'REVISED',font=font(17,True),fill='#dfe3dc')
for k,(tag,title,caption) in enumerate(rows):
    y=head+k*(h+band+pad);d.text((pad,y+3),title,font=font(23,True),fill='#e5ebd6');d.text((pad,y+39),caption,font=font(19),fill='#ced8cc')
    for j,side in enumerate(('before','after')):folio.paste(Image.open((final_greens if tag!='talus-eye' and side=='after' else control)/side/(tag+'.png')).convert('RGB'),(pad+j*(w+gap),y+band))
folio.save(dest,quality=93,optimize=True,subsampling=0)
