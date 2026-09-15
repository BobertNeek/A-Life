"""Check shipped render vertices against the authoritative collision bake."""
import json,struct,math
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'crates/alife_game_app/assets/landscape/highlands'
baked=(ROOT/'crates/alife_world/assets/highlands-v1.bin').read_bytes()
w,d,ox,oz,spacing,count=struct.unpack_from('<IIfffI',baked,8)
heights=struct.unpack_from('<'+'f'*(w*d),baked,32)
raw=(HERE/'terrain-chunks.glb').read_bytes()
length=struct.unpack_from('<I',raw,12)[0];doc=json.loads(raw[20:20+length]);binary=28+length
def accessor(index):
    a=doc['accessors'][index];v=doc['bufferViews'][a['bufferView']]
    fmt={5126:'f',5125:'I',5123:'H',5121:'B'}[a['componentType']]*{'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[a['type']]
    stride=v.get('byteStride',struct.calcsize('<'+fmt));offset=binary+v.get('byteOffset',0)+a.get('byteOffset',0)
    return [struct.unpack_from('<'+fmt,raw,offset+i*stride) for i in range(a['count'])]
assert len(doc['meshes'])==420
totals=[0,0,0];checked=0;max_error=0
for mesh in doc['meshes']:
    lod=int(mesh['name'][-1]);p=mesh['primitives'][0]
    assert 'COLOR_0' in p['attributes'] and 'TEXCOORD_0' in p['attributes']
    positions=accessor(p['attributes']['POSITION'])
    for x,y,z in positions:
        i=round((x-ox)/spacing);j=round((z-oz)/spacing)
        assert 0<=i<w and 0<=j<d
        expected=heights[j*w+i]
        error=min(abs(y-expected),abs(y-(expected-20)))
        max_error=max(max_error,error);assert error<0.0001,(mesh['name'],x,y,z,expected)
        checked+=1
    indices=[v[0] for v in accessor(p['indices'])]
    totals[lod]+=len(indices)//3
    a,b,c=[positions[i] for i in indices[:3]]
    # Positive Y normal on the upper surface, never an inverted mountain.
    assert (b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2])>0
assert totals[2]<totals[1]<totals[0]
placements=json.loads((HERE/'props.json').read_text());assert len(placements)==3441 and count==1288
for p in placements:
    asset=HERE.parent/(p['kind']+'.glb');assert asset.exists()
    r=asset.read_bytes();n=struct.unpack_from('<I',r,12)[0];g=json.loads(r[20:20+n])
    assert all(not any(k in node for k in ['matrix','translation','rotation','scale']) for node in g['nodes']),asset
digest=0xcbf29ce484222325
for byte in baked:digest=((digest^byte)*0x100000001b3)&0xffffffffffffffff
print(json.dumps({'result':'PASS','checked_render_vertices':checked,'maximum_height_error':max_error,
    'all_chunks_triangles_by_lod':totals,'collision_proxies':count,'shared_prop_placements':len(placements),
    'height_binding_digest':f'{digest:016x}'},indent=2))
