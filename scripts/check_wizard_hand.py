"""Check exported right-hand poses, measured contacts, and the narrow manifest receipt."""
import json
import math
import re
from pathlib import Path
from check_island_art import Glb

ROOT=Path(__file__).resolve().parents[1]
POSES=('Hover','Point','Grab','Carry','Release')


def multiply(a,b):
    return [[sum(a[i][k]*b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def transform(value):
    x,y,z,w=value['rotation'];s=value['scale'];t=value['translation']
    rows=[[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],
          [2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],
          [2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]]
    return [[rows[i][j]*s[j] for j in range(3)]+[t[i]] for i in range(3)]+[[0,0,0,1]]


def point(matrix,p):
    return [sum(matrix[i][j]*p[j] for j in range(3))+matrix[i][3] for i in range(3)]


def check(root=ROOT,require_production=True):
    directory=root/'crates/alife_game_app/assets/hand'
    spec=json.loads((directory/'hand-specification.json').read_text())
    if require_production:assert spec['blender'].startswith('5.2.'), 'Export with compatible Blender 5.2'
    assert spec['handedness']=='right' and spec['bones']==16
    hand=Glb(directory/'wizard-god-hand.glb');doc=hand.doc
    assert all('matrix' not in node for node in doc['nodes']), 'Expected Blender TRS export'
    assert set(a['name'] for a in doc['animations'])==set(POSES)
    assert len(doc['skins'][0]['joints'])==16
    names={n['name']:i for i,n in enumerate(doc['nodes'])}
    parents={child:i for i,n in enumerate(doc['nodes']) for child in n.get('children',[])}
    base=[{'translation':n.get('translation',[0,0,0]),'rotation':n.get('rotation',[0,0,0,1]),
           'scale':n.get('scale',[1,1,1])} for n in doc['nodes']]
    assert all(all(s>0 for s in v['scale']) for v in base), 'Bake reflection into the source'
    rust=(root/'crates/alife_game_app/src/production_voxel_renderer/god_hand.rs').read_text()
    block=rust.split('const POSE_CONTACTS:')[1].split('];')[0]
    anchors=[[float(v.strip()) for v in values.split(',')] for values in re.findall(r'Vec3::new\(([^)]+)\)',block)]
    assert len(anchors)==5
    measured={}
    for animation in doc['animations']:
        values=[dict(v) for v in base]
        for channel in animation['channels']:
            sampler=animation['samplers'][channel['sampler']]
            assert sampler.get('interpolation','LINEAR')!='CUBICSPLINE'
            samples=hand.accessor(sampler['output'])
            assert all(math.isfinite(x) for row in samples for x in row)
            assert all(max(abs(a-b) for a,b in zip(row,samples[0]))<1e-5 for row in samples)
            values[channel['target']['node']][channel['target']['path']]=samples[0]
        assert all(all(s>0 for s in v['scale']) for v in values)
        matrices={}
        def global_matrix(i):
            if i not in matrices:
                local=transform(values[i])
                matrices[i]=multiply(global_matrix(parents[i]),local) if i in parents else local
            return matrices[i]
        index=point(global_matrix(names['Index_2']),[0,spec['fingers_m']['Index']*.24,0])
        thumb=point(global_matrix(names['Thumb_2']),[0,.38,0])
        pose=animation['name'];pinching=pose in ('Grab','Carry')
        contact=[(a+b)*.5 for a,b in zip(index,thumb)] if pinching else index
        expected=spec['pose_contacts'][pose]['glTF_model_contact_xyz']
        anchor=anchors[POSES.index(pose)]
        assert max(abs(a-b) for a,b in zip(contact,expected))<1e-4,(pose,contact,expected)
        assert max(abs(a-b) for a,b in zip(contact,anchor))<1e-4,(pose,contact,anchor)
        if pinching:
            gap=math.sqrt(sum((a-b)**2 for a,b in zip(index,thumb)))
            assert abs(gap-.12)<.001,(pose,gap)
        measured[pose]=contact
    if require_production:
        manifest=json.loads((root/'crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json').read_text())
        entries=[e for e in manifest['entries'] if e['asset_id']=='approved-wizard-god-hand']
        assert len(entries)==1
        payload=(directory/'wizard-god-hand.glb').read_bytes();digest=0xcbf29ce484222325
        for byte in payload:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
        assert entries[0]['digest']==f'fnv1a64:{digest:016x}' and entries[0]['size_bytes']==len(payload)
    print(json.dumps({'result':'PASS','exported_pose_contacts':measured},indent=2))


if __name__=='__main__':check()
