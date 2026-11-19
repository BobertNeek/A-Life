"""Do not rebake the cove. Replace two held artist stones and nothing else."""
from pathlib import Path
import json
from heel import TARGETS, write
from rind import paint

def photograph(root,d,surface,board,out,blender):
    base=root/'studies/weatherlip-cove/gallery'
    stone=out/'seats';stone.mkdir(exist_ok=True)
    props=json.loads((d/'props.json').read_text());report={}
    for index,(kind,_) in TARGETS.items():
        report[str(index)]=write(d/(kind+'.glb'),surface,props[index],index,stone/(f'{kind}Seat{index}.glb'))
    paint(stone/'coastal-erratic',base)
    (stone/'contact.json').write_text(json.dumps(report,indent=2)+'\n')
    # The generic stock library continues to supply *all* other stones,
    # including the neighbouring 1472. Only the two named keys exist here.
    water=out/'shelf';water.mkdir(exist_ok=True)
    for name in ('ripple.png','shore.png'):(water/name).symlink_to((d/'cove-water'/name).resolve())
    (water/'water.json').write_bytes((root/'studies/island-art-week/water-selected.json').read_bytes())
    for tag in ('bight-high','bight-tooth','bight-beach'):
        blender('render_island_geology.py',d/'terrain-chunks.glb',out/'after',
                '--guardian-review' if tag=='bight-high' else '--copse-eye',
                '--weatherlip-view',tag,'--weatherlip-field',base,'--cove-seats',stone,
                '--surface',surface,'--placements',d/'props.json','--prop-library',d,
                '--coastal-light',d/'coastal-daylight.json','--marine-sky',d/'marine-day.glb',
                '--grove-forms',d/'grove-crowns.glb','--water-inputs',water,'--habitats',d/'habitat-ground.glb',
                '--wild-sward',d/'meadow-sward.glb','--meadow-library',d,'--meadow-habits',
                '--strandline',d/'strandline.glb','--copse-floor',d/'copse-floor.glb','--studio-samples',48)
    blender('studio_weatherlip_seats.py',base/'Weatherlip-Cove.blend',d,stone,out/'Weatherlip-Stone-Seats.blend')
    (out/'studio-note.json').write_text(json.dumps(board,indent=2)+'\n')
