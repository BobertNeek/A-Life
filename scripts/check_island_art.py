"""Focused shipping check: mesh/physics agreement, LOD/pose/size contracts."""
import json
import math
import struct
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/island'


class Glb:
    def __init__(self,path):
        self.raw=path.read_bytes();n=struct.unpack_from('<I',self.raw,12)[0]
        self.doc=json.loads(self.raw[20:20+n]);self.binary=28+n
    def accessor(self,index):
        a=self.doc['accessors'][index];v=self.doc['bufferViews'][a['bufferView']]
        fmt={5126:'f',5125:'I',5123:'H',5121:'B'}[a['componentType']]*{'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}[a['type']]
        stride=v.get('byteStride',struct.calcsize('<'+fmt));offset=self.binary+v.get('byteOffset',0)+a.get('byteOffset',0)
        return [struct.unpack_from('<'+fmt,self.raw,offset+i*stride) for i in range(a['count'])]


def check():
    raw=(ROOT/'crates/alife_world/assets/island-v1.bin').read_bytes()
    assert raw[:8]==b'HLAND001'
    w,d,ox,oz,spacing,obstacles=struct.unpack_from('<IIfffI',raw,8)
    heights=struct.unpack_from('<'+'f'*(w*d),raw,32)
    spec=json.loads((HERE/'terrain-specification.json').read_text())
    glb=Glb(HERE/'terrain-chunks.glb');totals=[0,0,0];checked=0;max_error=0
    assert len(glb.doc['meshes'])==math.ceil((w-1)/32)*math.ceil((d-1)/32)*3
    for m in glb.doc['meshes']:
        level=int(m['name'][-1]);p=m['primitives'][0]
        assert 'COLOR_0' in p['attributes'] and 'TEXCOORD_0' in p['attributes']
        positions=glb.accessor(p['attributes']['POSITION'])
        for x,y,z in positions:
            i=round((x-ox)/spacing);j=round((z-oz)/spacing)
            assert 0<=i<w and 0<=j<d
            expected=heights[j*w+i]
            error=min(abs(y-expected),abs(y-(expected-spec['skirt_depth_m'])))
            max_error=max(max_error,error);assert error<.0001
            checked+=1
        indices=[row[0] for row in glb.accessor(p['indices'])];totals[level]+=len(indices)//3
        a,b,c=[positions[i] for i in indices[:3]]
        assert (b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2])>0
    assert totals==spec['triangles_by_lod'] and totals[2]<totals[1]<totals[0]
    assets=json.loads((HERE/'asset-specification.json').read_text())['assets']
    for kind,target in assets.items():
        g=Glb(HERE/(kind+'.glb'));meshes={m['name']:m for m in g.doc['meshes']}
        assert all(kind+'_L'+str(i) in meshes for i in range(3))
        p=meshes[kind+'_L0']['primitives'][0]
        vertices=g.accessor(p['attributes']['POSITION'])
        actual=[max(v[k] for v in vertices)-min(v[k] for v in vertices) for k in (0,2,1)]
        for a,b in zip(actual,target['width_depth_height_m']):assert abs(a-b)<.002,(kind,actual,target)
        assert all(not any(k in n for k in ['matrix','translation','rotation','scale']) for n in g.doc['nodes'])
    props=json.loads((HERE/'props.json').read_text());assert len(props)==spec['props'] and obstacles==spec['collision_proxies']
    assert all(p['kind'] in assets for p in props)
    hand=Glb(ROOT/'crates/alife_game_app/assets/hand/wizard-god-hand.glb')
    assert set(a['name'] for a in hand.doc['animations'])=={'Hover','Point','Grab','Carry','Release'}
    assert len(hand.doc['skins'][0]['joints'])==16
    for m in hand.doc['meshes']:
        for p in m['primitives']:
            assert 'JOINTS_0' in p['attributes'] and 'WEIGHTS_0' in p['attributes']
            assert all(abs(sum(row)-1)<.001 for row in hand.accessor(p['attributes']['WEIGHTS_0']))
    manifest=json.loads((ROOT/'crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json').read_text())
    runtime=[e for e in manifest['entries'] if not e['local_path'].endswith('Mountain_Valley.glb')]
    assert max(e['size_bytes'] for e in runtime)<=12*1024*1024
    assert sum(e['size_bytes'] for e in runtime)<=32*1024*1024
    print(json.dumps({'result':'PASS','checked_terrain_vertices':checked,'maximum_height_error_m':max_error,
        'triangles_by_lod':totals,'collision_proxies':obstacles,'placements':len(props),'hand_poses':5,
        'total_runtime_pack_bytes':sum(e['size_bytes'] for e in runtime)},indent=2))


if __name__=='__main__':check()
