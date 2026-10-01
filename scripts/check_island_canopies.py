"""CPU silhouette coverage gate for the approved tree overview LODs."""
import argparse
import json
import math
import struct
import zlib
from pathlib import Path
from check_island_art import Glb

SIZE = 96
KINDS = ('Broadleaf', 'Conifer', 'Sapling')


def primitive(path, kind, level):
    glb = Glb(path / (kind + '.glb'))
    mesh = next(m for m in glb.doc['meshes'] if m['name'] == kind + '_L' + str(level))
    return glb, mesh['primitives'][0]


def points(glb, primitive, yaw):
    c, s = math.cos(yaw), math.sin(yaw)
    # Orthographic camera, 25 degrees above the horizon, same frame for all LODs.
    tilt = math.radians(25)
    return [(x*c-z*s, y*math.cos(tilt)-(x*s+z*c)*math.sin(tilt))
            for x, y, z in glb.accessor(primitive['attributes']['POSITION'])]


def silhouette(glb, primitive, projected, bounds):
    x0, y0, x1, y1 = bounds
    projected = [((x-x0)/(x1-x0)*(SIZE-4)+2, (y1-y)/(y1-y0)*(SIZE-4)+2)
                 for x, y in projected]
    mask = bytearray(SIZE*SIZE)
    indices = [i[0] for i in glb.accessor(primitive['indices'])]
    def edge(a, b, x, y):return (x-a[0])*(b[1]-a[1])-(y-a[1])*(b[0]-a[0])
    for i in range(0, len(indices), 3):
        a, b, c = [projected[j] for j in indices[i:i+3]]
        sign = 1 if edge(a, b, *c) >= 0 else -1
        for y in range(max(0, math.floor(min(a[1], b[1], c[1]))), min(SIZE, math.ceil(max(a[1], b[1], c[1])))):
            for x in range(max(0, math.floor(min(a[0], b[0], c[0]))), min(SIZE, math.ceil(max(a[0], b[0], c[0])))):
                if all(sign*edge(p, q, x+.5, y+.5) >= 0 for p, q in ((a,b), (b,c), (c,a))):
                    mask[y*SIZE+x] = 1
    return mask


def write_masks(path, masks):
    # Standalone geometric QA figure; no screenshot processing or external upload.
    width, height = SIZE*len(masks), SIZE
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for mask in masks:
            for x in range(SIZE):raw.extend((225, 233, 214) if mask[y*SIZE+x] else (24, 33, 38))
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind+data)&0xffffffff)
    path.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
                     +chunk(b'IDAT', zlib.compress(raw))+chunk(b'IEND', b''))


def check(baseline, candidate, output):
    output.mkdir(parents=True, exist_ok=True)
    rows = []
    for kind in KINDS:
        before, bp = primitive(baseline, kind, 2)
        after, ap = primitive(candidate, kind, 2)
        near, np = primitive(baseline, kind, 0)
        # Authoritative nearby art and its middle LOD remain byte-for-byte geometry.
        for level in (0, 1):
            old, op = primitive(baseline, kind, level)
            new, cp = primitive(candidate, kind, level)
            assert old.accessor(op['indices']) == new.accessor(cp['indices'])
            assert op['attributes'].keys() == cp['attributes'].keys()
            for name in op['attributes']:
                assert old.accessor(op['attributes'][name]) == new.accessor(cp['attributes'][name]), (kind, level, name)
        old_triangles = len(before.accessor(bp['indices']))//3
        new_triangles = len(after.accessor(ap['indices']))//3
        assert new_triangles <= old_triangles, (kind, new_triangles, old_triangles)
        directions = []
        for angle in (0, 45, 90, 135):
            yaw = math.radians(angle)
            pts = points(near, np, yaw)
            bounds = (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))
            masks = [silhouette(g, p, points(g,p,yaw), bounds) for g,p in ((near,np),(before,bp),(after,ap))]
            area = sum(masks[0])
            old_coverage = sum(a and b for a,b in zip(masks[0],masks[1]))/area
            new_coverage = sum(a and b for a,b in zip(masks[0],masks[2]))/area
            assert new_coverage >= .70, (kind, angle, new_coverage)
            assert new_coverage > old_coverage, (kind, angle, old_coverage, new_coverage)
            write_masks(output/(kind+'-'+str(angle)+'.png'), masks)
            directions.append({'yaw_degrees':angle,'before_near_silhouette_coverage':old_coverage,
                               'after_near_silhouette_coverage':new_coverage})
        rows.append({'kind':kind,'before_triangles':old_triangles,'after_triangles':new_triangles,'views':directions})
    receipt = {'result':'PASS','method':'96px orthographic silhouette; near / previous L2 / repaired L2',
               'unchanged_near_and_middle_geometry':True,'trees':rows}
    (output/'canopy-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps(receipt,indent=2))


if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--baseline',type=Path,required=True)
    parser.add_argument('--candidate',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    check(args.baseline,args.candidate,args.output)
