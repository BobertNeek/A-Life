"""Blender integration tests for the one-shot tail repair.

Run: blender -b --python-exit-code 1 --python scripts/test_hearthling_tail_repair.py --
     --source PATH [--source SECOND_PATH]
Both a native selected checkpoint and a full-source pre-pass checkpoint should
be provided for release verification. This test never writes the inputs.
"""
import argparse
import sys
from pathlib import Path

import bpy
import bmesh

sys.path.insert(0, str(Path(__file__).resolve().parent))
from repair_hearthling_tail import repair_tail, _anchor_indices


def snapshot():
    obj = bpy.data.objects['Hearthling_Tail']; mesh = obj.data
    coat = mesh.color_attributes['CoatColor']; uv = mesh.uv_layers.active
    vertices = [(tuple(v.co), tuple(coat.data[v.index].color),
                 tuple((g.group,g.weight) for g in v.groups)) for v in mesh.vertices]
    faces = {tuple(p.vertices): (p.material_index, p.use_smooth,
                                tuple(tuple(uv.data[i].uv) for i in p.loop_indices))
             for p in mesh.polygons}
    scene = bpy.context.scene; rig = bpy.data.objects['Hearthling_Rig']
    anim = rig.animation_data
    context = (scene.frame_current, scene.frame_subframe,
               anim.action.name if anim.action else None, anim.use_nla,
               tuple((t.name,t.mute,tuple((s.name, s.action.name if s.action else None,
                                           s.frame_start,s.frame_end) for s in t.strips))
                     for t in anim.nla_tracks))
    material = mesh.materials[0]
    nodes = material.node_tree.nodes
    material_state = (tuple(m.name if m else None for m in mesh.materials),
                      tuple((n.name, n.type, getattr(getattr(n, 'image', None), 'name', None)) for n in nodes),
                      tuple((link.from_node.name, link.from_socket.name, link.to_node.name, link.to_socket.name)
                            for link in material.node_tree.links))
    return vertices, faces, context, (len(mesh.vertices),len(mesh.edges),len(mesh.polygons)), material_state


def fresh(path):
    bpy.ops.wm.open_mainfile(filepath=str(path))
    scene = bpy.context.scene; rig = bpy.data.objects['Hearthling_Rig']
    rig.animation_data.action = bpy.data.actions['Hearthling_Sleep']
    for i, track in enumerate(rig.animation_data.nla_tracks): track.mute = bool(i % 2)
    scene.frame_set(37, subframe=.25)
    return bpy.data.objects['Hearthling_Tail']


def must_reject_unchanged(label, reason):
    before = snapshot()
    try: repair_tail()
    except ValueError as error:
        assert reason in str(error), (label, str(error))
    else: raise AssertionError(label + ' unexpectedly accepted')
    assert snapshot() == before, label + ' changed mesh or animation context on rejection'


def make_live_representation(path, variant='valid'):
    """Construct the generator's two live duplicate pairs without a file save.

    Saving this special in-memory representation would normalize away two faces,
    so reconstruct it from each checkpoint for direct positive/negative controls.
    """
    obj = fresh(path); mesh = obj.data
    vertices = [tuple(v.co) for v in mesh.vertices]
    edges = [tuple(e.vertices) for e in mesh.edges]
    faces = [tuple(p.vertices) for p in mesh.polygons]
    tiny = [p for p in mesh.polygons if p.area < 1e-10]
    assert len(tiny) == 2
    additions = [tuple(reversed(p.vertices)) for p in tiny]
    if variant == 'same winding': additions[0] = tuple(tiny[0].vertices)
    if variant == 'nonisolated':
        ordinary = next(p for p in mesh.polygons if p.area > 1e-6)
        additions[0] = tuple(reversed(ordinary.vertices))
    faces.extend(additions)
    colors = [tuple(d.color) for d in mesh.color_attributes['CoatColor'].data]
    uvs = [tuple(d.uv) for d in mesh.uv_layers.active.data]
    weights = [[(g.group, g.weight) for g in v.groups] for v in mesh.vertices]
    names = [group.name for group in obj.vertex_groups]
    materials = list(mesh.materials)
    smooth = [p.use_smooth for p in mesh.polygons]
    synthetic = bpy.data.meshes.new('Live tail test')
    synthetic.from_pydata(vertices, edges, faces)
    for material in materials: synthetic.materials.append(material)
    uv = synthetic.uv_layers.new(name=mesh.uv_layers.active.name)
    coat = synthetic.color_attributes.new(name='CoatColor', type='FLOAT_COLOR', domain='POINT')
    for i, value in enumerate(colors): coat.data[i].color = value
    for i, value in enumerate(uvs): uv.data[i].uv = value
    for i, value in enumerate(smooth): synthetic.polygons[i].use_smooth = value
    if variant == 'displaced tiny':
        for i in tiny[0].vertices: synthetic.vertices[i].co.x += .01
    synthetic.update(); obj.data = synthetic
    for name in names: obj.vertex_groups.new(name=name)
    for i, groups in enumerate(weights):
        for group, weight in groups: obj.vertex_groups[group].add([i], weight, 'REPLACE')
    assert (len(synthetic.vertices),len(synthetic.edges),len(synthetic.polygons)) == (1300,3861,2561)
    return obj


def run_live_controls(path):
    make_live_representation(path)
    before = snapshot(); repair_tail(); after = snapshot()
    assert before[0] == after[0] and before[2] == after[2] and before[4] == after[4]
    common = before[1].keys() & after[1].keys()
    assert len(common) == 2547 and all(before[1][k] == after[1][k] for k in common)
    assert after[3] == (1300,3856,2558)
    must_reject_unchanged('live repeat', 'unexpected mesh topology')
    for variant in ('same winding', 'nonisolated', 'displaced tiny'):
        make_live_representation(path, variant)
        reason = 'unexpected live degenerate locations' if variant == 'displaced tiny' else 'unexpected live degenerate components'
        must_reject_unchanged('live ' + variant, reason)
    print('TAIL_REPAIR_LIVE_CONTROLS_PASS', path, flush=True)


def run(path):
    obj = fresh(path); before = snapshot(); ids = _anchor_indices(obj.data)
    result = repair_tail(); after = snapshot()
    assert tuple(ids) == result
    assert after[0] == before[0], 'vertices, colors or weights changed'
    assert after[2] == before[2], 'frame, action or NLA changed'
    assert after[4] == before[4], 'material or atlas linkage changed'
    common = before[1].keys() & after[1].keys()
    assert len(common) == 2545 and all(before[1][key] == after[1][key] for key in common), 'retained faces or UVs changed'
    assert len(before[1].keys()-after[1].keys()) == 14
    assert len(after[1].keys()-before[1].keys()) == 11
    assert after[3] == (1300,3856,2556)
    must_reject_unchanged('repeat', 'unexpected mesh topology')

    obj = fresh(path); ids = _anchor_indices(obj.data)
    obj.data.vertices[ids[0]].co.x += .01
    must_reject_unchanged('missing anchor', 'anchor missing or ambiguous')

    obj = fresh(path); ids = _anchor_indices(obj.data)
    other = next(v for v in obj.data.vertices if v.index not in ids)
    other.co = obj.data.vertices[ids[0]].co.copy()
    must_reject_unchanged('ambiguous anchor', 'anchor missing or ambiguous')

    obj = fresh(path); ids = _anchor_indices(obj.data)
    bm = bmesh.new(); bm.from_mesh(obj.data); bm.faces.ensure_lookup_table()
    face = next(f for f in bm.faces if {v.index for v in f.verts} == {ids[7],ids[0],ids[6]})
    face.normal_flip(); bm.to_mesh(obj.data); bm.free(); obj.data.update()
    must_reject_unchanged('reversed strip winding', 'strip faces or winding differ')

    obj = fresh(path); obj.data.materials.append(obj.data.materials[0])
    must_reject_unchanged('unexpected material count', 'unexpected material slots')

    obj = fresh(path); obj.data.materials[0] = obj.data.materials[0].copy()
    obj.data.materials[0].name = 'Unexpected coat'
    must_reject_unchanged('unexpected material identity', 'unexpected tail material')

    obj = fresh(path)
    texture = next(n for n in obj.data.materials[0].node_tree.nodes if n.type == 'TEX_IMAGE')
    texture.image = None
    must_reject_unchanged('material missing atlas connection', 'does not use the normal atlas')
    print('TAIL_REPAIR_TEST_PASS', path, flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', action='append', type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
    for path in args.source:
        path = path.resolve(strict=True)
        run(path)
        run_live_controls(path)
