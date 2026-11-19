"""Produce one coherent open-bight folio; never open a game or rewrite native data."""
from pathlib import Path
import json
import numpy as np
from PIL import Image
from shore import Bight,EXTENT,TAGS,colour
from forms import build

def assets(surface,props,out):
    out.mkdir(parents=True,exist_ok=True)
    b=Bight(surface); report=b.survey(props,out)
    x0,x1,z0,z1=EXTENT; w,h=787,2266
    chart=np.zeros((h,w,4),'B')
    xs=x0+(np.arange(w)+.5)*(x1-x0)/w
    for i in range(0,h,70):
        zz=z0+(np.arange(i,min(i+70,h))+.5)*(z1-z0)/h
        x,z=np.meshgrid(xs,zz)
        chart[i:i+len(zz)]=colour(b,x,z)
    Image.fromarray(chart).save(out/'weatherlip-ground.png',compress_level=7)
    report['chart_metres_per_texel']=[(x1-x0)/w,(z1-z0)/h]
    report.update(build(b,surface,out))
    (out/'site.json').write_text(json.dumps(report,indent=2)+'\n')

def photograph(root,d,surface,board,out,blender):
    field=out/'weatherlip'
    assets(surface,d/'props.json',field)
    water=out/'shelf';water.mkdir(exist_ok=True)
    for n in ('ripple.png','shore.png'):
        (water/n).symlink_to((d/'cove-water'/n).resolve())
    (water/'water.json').write_bytes((root/'studies/island-art-week/water-selected.json').read_bytes())
    for tag in TAGS:
        for side in ('before','after'):
            blender('render_island_geology.py',d/'terrain-chunks.glb',out/side,
                    '--guardian-review' if tag=='bight-high' else '--copse-eye',
                    '--weatherlip-view',tag,
                    *(['--weatherlip-field',field] if side=='after' else []),
                    '--surface',surface,'--placements',d/'props.json','--prop-library',d,
                    '--coastal-light',d/'coastal-daylight.json','--marine-sky',d/'marine-day.glb',
                    '--grove-forms',d/'grove-crowns.glb','--water-inputs',water,
                    '--habitats',d/'habitat-ground.glb','--wild-sward',d/'meadow-sward.glb',
                    '--meadow-library',d,'--meadow-habits','--strandline',d/'strandline.glb',
                    '--copse-floor',d/'copse-floor.glb','--studio-samples',board.get('samples',48))
    blender('studio_weatherlip_desk.py',surface,field,out/'Weatherlip-Cove.blend')
    (out/'studio-note.json').write_text(json.dumps(board,indent=2)+'\n')
