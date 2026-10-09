"""Verify that the shipped Fern candidate changes only normalized color bytes."""
import argparse
import hashlib
import json
import math
import struct
from pathlib import Path

from retain_fern_geometry import Glb

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/alife_game_app/assets/landscape/island'
BASE_SHA256 = '6dcccfdb1044e35d60cf0b14ebbe9de2ad220be590d11511d0b9e3d161cd62f3'


def check(baseline, candidate):
    old, new = Glb(baseline), Glb(candidate)
    assert hashlib.sha256(old.raw).hexdigest() == BASE_SHA256
    assert len(old.raw) == len(new.raw) == 97100
    assert old.doc == new.doc, 'GLB JSON contract changed'
    assert struct.unpack_from('<III', new.raw) == (0x46546c67, 2, len(new.raw))
    assert old.doc['nodes'] == [{'mesh': i, 'name': 'Fern_L' + str(i)} for i in range(3)]
    assert len(new.doc['materials']) == 1 and not new.doc.get('textures')
    assert not new.doc.get('skins') and not new.doc.get('animations')
    permitted, rows = set(), []
    for level, mesh in enumerate(new.doc['meshes']):
        assert mesh['name'] == 'Fern_L' + str(level)
        assert len(mesh['primitives']) == 1
        p = mesh['primitives'][0]
        assert p['material'] == 0
        assert set(p['attributes']) == {'POSITION', 'NORMAL', 'COLOR_0'}
        for semantic in ('POSITION', 'NORMAL'):
            assert old.rows(p['attributes'][semantic]) == new.rows(p['attributes'][semantic])
        assert old.rows(p['indices']) == new.rows(p['indices'])
        positions, normals = new.rows(p['attributes']['POSITION']), new.rows(p['attributes']['NORMAL'])
        assert all(math.isfinite(x) for v in positions + normals for x in v)
        vertices = len(positions)
        indices = [v[0] for v in new.rows(p['indices'])]
        assert all(0 <= i < vertices for i in indices)
        assert len(indices) // 3 == [784, 352, 156][level]
        assert vertices == [1568, 1056, 468][level]
        color_index = p['attributes']['COLOR_0']
        colors, previous = new.rows(color_index), old.rows(color_index)
        assert len(colors) == vertices
        assert all(c[3] == 255 for c in colors)
        for offset in new.byte_offsets(color_index):
            permitted.update(range(offset, offset + 3))
        rows.append(dict(lod=level, triangles=len(indices) // 3, vertices=vertices,
            primitives=1, bounds_y_up=[f(v[k] for v in positions) for f in (min, max) for k in range(3)],
            bounds_change_m=0, changed_vertex_colors=sum(a != b for a, b in zip(colors, previous)),
            unique_colors_before=len(set(previous)), unique_colors_after=len(set(colors))))
    differences = {i for i, (a, b) in enumerate(zip(old.raw, new.raw)) if a != b}
    assert differences and differences.issubset(permitted), 'Non-color byte changed'
    return dict(result='PASS', baseline_sha256=BASE_SHA256,
        candidate_sha256=hashlib.sha256(new.raw).hexdigest(),
        bytes_before=len(old.raw), bytes_after=len(new.raw), size_change_bytes=0,
        changed_bytes=len(differences), only_rgb_vertex_color_bytes_changed=True,
        glb_json_byte_retained=True, geometry_normals_topology_byte_retained=True,
        material_count=1, textures=0, identity_transforms=True,
        source_units='metres, Z-up', runtime_units='metres, Y-up',
        bounds_and_pivots_retained=True, lods=rows,
        validation='Focused binary/accessor check; Khronos validator recorded separately if run',
        cpu_render_review=False, game_rendering=False, physical_gpu=False,
        source_reconstructed_from_committed_runtime_glb=True,
        original_blender_5_2_source_modified=False,
        gameplay_changes=False, published=False)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline', type=Path, default=ASSETS / 'fern-readability/Fern.baseline.glb')
    p.add_argument('--candidate', type=Path, default=ASSETS / 'Fern.glb')
    p.add_argument('--output', type=Path, default=ROOT / 'target/artifacts/fern-readability/structural-check.json')
    a = p.parse_args()
    report = json.dumps(check(a.baseline, a.candidate), indent=2) + '\n'
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(report)
    print(report, end='')
