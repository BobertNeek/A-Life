"""Apply the one reviewed placement correction without rebuilding the island.

Run with the source's Blender 5.2.1 LTS, then register_approved_art.py. The base
terrain recipe imports the same correction. No prop mesh or height sample moves.
"""
import array
import hashlib
import json
import struct
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_placement_corrections import (
    CORRECTED_Y, KIND, ORIGINAL_Y, PLACEMENT_INDEX, PROXY_INDEX, XZ, corrected_height,
)

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/landscape/island'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def fields(collection, attribute, count, code='f'):
    values = array.array(code, [0]) * (len(collection) * count)
    collection.foreach_get(attribute, values)
    return digest(values.tobytes())


def scene_snapshot():
    """Audit all object transforms, mesh topology/paint/UV and packed images."""
    objects = {}
    for obj in bpy.data.objects:
        objects[obj.name] = {
            'location': list(obj.location), 'rotation': list(obj.rotation_euler),
            'scale': list(obj.scale), 'parent': obj.parent.name if obj.parent else None,
            'data': obj.data.name if obj.data else None, 'type': obj.type,
            'collections': sorted(c.name for c in obj.users_collection),
            'hide_render': obj.hide_render,
            'materials': [s.material.name if s.material else None for s in obj.material_slots],
        }
    meshes = {}
    for mesh in bpy.data.meshes:
        meshes[mesh.name] = {
            'vertices': fields(mesh.vertices, 'co', 3),
            'edges': fields(mesh.edges, 'vertices', 2, 'i'),
            'loops': fields(mesh.loops, 'vertex_index', 1, 'i'),
            'polygon_starts': fields(mesh.polygons, 'loop_start', 1, 'i'),
            'polygon_sizes': fields(mesh.polygons, 'loop_total', 1, 'i'),
            'material_indices': fields(mesh.polygons, 'material_index', 1, 'i'),
            'colors': {a.name: fields(a.data, 'color', 4) for a in mesh.color_attributes},
            'uvs': {a.name: fields(a.data, 'uv', 2) for a in mesh.uv_layers},
        }
    images = {i.name: [digest(p.packed_file.data) for p in i.packed_files] for i in bpy.data.images}
    return {'objects': objects, 'meshes': meshes, 'packed_images': images,
            'materials': sorted(m.name for m in bpy.data.materials),
            'collections': sorted(c.name for c in bpy.data.collections)}


def main():
    assert bpy.app.version == (5, 2, 1), 'Use the source-matched Blender 5.2.1 LTS'
    native = HERE / 'island-terrain.blend'
    props_path = HERE / 'props.json'
    world_path = ROOT / 'crates/alife_world/assets/island-v1.bin'
    original_props = props_path.read_bytes()
    props = json.loads(original_props)
    assert (json.dumps(props, separators=(',', ':')) + '\n').encode() == original_props
    p = props[PLACEMENT_INDEX]
    x, old_y, z = p['position']
    new_y = corrected_height(PLACEMENT_INDEX, p['kind'], x, z, old_y)
    original_world = world_path.read_bytes()
    w, d, _, _, _, count = struct.unpack_from('<IIfffI', original_world, 8)
    assert count == 1716 and original_world[:8] == b'HLAND001'
    offset = 32 + w * d * 4 + PROXY_INDEX * 24
    box = list(struct.unpack_from('<6f', original_world, offset))
    assert abs(box[4] - old_y) < .0001, 'Stale source proxy'
    bpy.ops.wm.open_mainfile(filepath=str(native))
    before = scene_snapshot()
    obj = bpy.data.objects[f'Island_{KIND}_{PLACEMENT_INDEX}']
    assert abs(obj.location.x - XZ[0]) < .00001 and abs(-obj.location.y - XZ[1]) < .00001
    assert abs(obj.location.z - old_y) < .0001, 'Native and runtime placements differ'
    obj.location.z = new_y
    expected = before
    expected['objects'][obj.name]['location'][2] = obj.location.z
    bpy.context.preferences.filepaths.save_version = 0
    # A corrected source is a no-op on repeat, avoiding needless reserialization.
    if old_y != new_y:
        bpy.ops.wm.save_as_mainfile(filepath=str(native), compress=True, check_existing=False)
    bpy.ops.wm.open_mainfile(filepath=str(native))
    after = scene_snapshot()
    assert after == expected, 'Native save changed unrelated transforms, geometry, paint or packed images'
    p['position'][1] = new_y
    updated_props = (json.dumps(props, separators=(',', ':')) + '\n').encode()
    # Preserve all terrain bytes and every other proxy; move only vertical bounds.
    updated_world = bytearray(original_world)
    box[4], box[5] = new_y, box[5] + (new_y - old_y)
    struct.pack_into('<2f', updated_world, offset + 16, *box[4:])
    assert updated_world[:offset + 16] == original_world[:offset + 16]
    assert updated_world[offset + 24:] == original_world[offset + 24:]
    props_path.write_bytes(updated_props)
    world_path.write_bytes(updated_world)
    receipt = {'result': 'PASS', 'blender': bpy.app.version_string,
               'placement_index': PLACEMENT_INDEX, 'proxy_index': PROXY_INDEX,
               'old_y_m': old_y, 'new_y_m': new_y,
               'native_object_count': len(after['objects']),
               'native_mesh_count': len(after['meshes']),
               'native_unrelated_snapshot_retained': True,
               'snapshot_sha256': digest(json.dumps(after, sort_keys=True).encode()),
               'terrain_samples_and_other_proxies_byte_identical': True}
    output = ROOT / 'target/artifacts/island-boulder-placement/source-reload.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt, indent=2), flush=True)


if __name__ == '__main__':
    main()
