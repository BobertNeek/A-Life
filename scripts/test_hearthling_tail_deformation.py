"""Re-import the shipping GLB and sample the repaired tail across its clips.

Run with Blender 5.2.1: blender -b --python-exit-code 1 --python
scripts/test_hearthling_tail_deformation.py. This is a discrete geometry check,
not a continuous-time, contact-physics or native gameplay guarantee.
"""
import sys
from pathlib import Path

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts'))
from repair_hearthling_tail import ANCHORS, ANCHOR_TOLERANCE, CAPS

bpy.ops.import_scene.gltf(filepath=str(
    ROOT / 'crates/alife_game_app/assets/creatures/hearthling/hearthling.glb'))
rig = bpy.data.objects['Hearthling_Rig']
mesh_object = bpy.data.objects['Hearthling_Runtime']
mesh = mesh_object.data
mesh.calc_loop_triangles()
for track in rig.animation_data.nla_tracks:
    track.mute = True

anchors = [Vector(point) for point in ANCHORS]
labels = {}
for vertex in mesh.vertices:
    hits = [i for i, point in enumerate(anchors)
            if (vertex.co - point).length <= ANCHOR_TOLERANCE]
    assert len(hits) <= 1, ('Ambiguous cap vertex', vertex.index)
    if hits:
        labels[vertex.index] = hits[0]
expected = {tuple(sorted(triangle)) for triangle in CAPS}
cap_faces = []
tail_faces = []
groups = {group.index: group.name for group in mesh_object.vertex_groups}
for triangle in mesh.loop_triangles:
    vertices = tuple(triangle.vertices)
    face_labels = tuple(sorted(labels.get(index, -1) for index in vertices))
    if face_labels in expected:
        cap_faces.append(vertices)
    elif any(any(groups[group.group].startswith('tail.') and group.weight > .05
                 for group in mesh.vertices[index].groups) for index in vertices):
        tail_faces.append(vertices)
assert len(cap_faces) == len(expected) == 11 and tail_faces

# Export splits some adjacent vertices at UV seams. Exclude faces that share a
# rest-space position with a cap triangle; all other reported overlaps fail.
def position_key(index):
    return tuple(round(value, 5) for value in mesh.vertices[index].co)

cap_keys = [{position_key(index) for index in face} for face in cap_faces]
tail_keys = [{position_key(index) for index in face} for face in tail_faces]
scene = bpy.context.scene
depsgraph = bpy.context.evaluated_depsgraph_get()
minimum_area = float('inf')
for clip in ('Hearthling_CuriousIdle', 'Hearthling_Walk', 'Hearthling_Sleep'):
    rig.animation_data.action = next(action for action in bpy.data.actions
                                     if action.name.startswith(clip))
    for inherited_scale in (.90, 1.0, 1.10):
        for sample in range(289):
            frame = 1 + sample / 4
            scene.frame_set(int(frame), subframe=frame % 1)
            rig.pose.bones['tail.00'].scale *= Vector((inherited_scale,) * 3)
            bpy.context.view_layer.update()
            evaluated = mesh_object.evaluated_get(depsgraph)
            points = [evaluated.matrix_world @ vertex.co for vertex in evaluated.data.vertices]
            for face in cap_faces:
                a, b, c = (points[index] for index in face)
                minimum_area = min(minimum_area, (b-a).cross(c-a).length / 2)
            cap_tree = BVHTree.FromPolygons(points, cap_faces, all_triangles=True)
            tail_tree = BVHTree.FromPolygons(points, tail_faces, all_triangles=True)
            overlaps = [(i, j) for i, j in cap_tree.overlap(tail_tree)
                        if not cap_keys[i] & tail_keys[j]]
            cap_overlaps = [(i, j) for i, j in cap_tree.overlap(cap_tree)
                            if i < j and not cap_keys[i] & cap_keys[j]]
            assert not overlaps and not cap_overlaps, (clip, inherited_scale, frame,
                                                       overlaps, cap_overlaps)
        print(clip, inherited_scale, '289 sampled poses PASS', flush=True)
assert minimum_area > 0, minimum_area
print('TAIL_DEFORMATION_PASS', 'minimum sampled triangle area', minimum_area, flush=True)
