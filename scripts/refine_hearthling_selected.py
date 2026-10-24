"""Selected 09/16/19 polish, applied after the revision-9 face and before UV bake.

Round the existing paw cages and soften the crown without adding geometry,
changing the skeleton, or rewriting skin weights. The original source remains
the recoverable starting point for export_hearthling.py.
"""
import math

import bpy
from mathutils import Vector


def _smooth(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def _relax(ob, weights, factor, iterations):
    adjacent = [[] for _ in ob.data.vertices]
    for edge in ob.data.edges:
        a, b = edge.vertices
        adjacent[a].append(b)
        adjacent[b].append(a)
    for _ in range(iterations):
        before = [v.co.copy() for v in ob.data.vertices]
        for v, weight in zip(ob.data.vertices, weights):
            row = adjacent[v.index]
            if weight > 0 and row:
                average = sum((before[i] for i in row), Vector()) / len(row)
                v.co = before[v.index].lerp(average, factor * weight)
    ob.data.update()


def refine_selected_references():
    rig = bpy.data.objects['Hearthling_Rig']
    assert rig.get('closeup_reference_revision') == 9
    assert not rig.get('selected_reference_revision'), 'Open the retained source first'
    previous = rig.data.pose_position
    rig.data.pose_position = 'REST'
    bpy.context.view_layer.update()

    # The retained fingers/thumbs have ten vertices around each swept ring.
    # Their near-constant proximal radius reads as a flat-sided tube. Keep the
    # attachment fixed, shorten the sweep and give it a rounded paw-pad profile.
    digits = [ob for ob in bpy.context.scene.objects
              if ob.type == 'MESH' and
              ob.name.startswith(('Hearthling_Finger', 'Hearthling_Thumb'))]
    assert len(digits) == 8
    for ob in digits:
        vertices = list(ob.data.vertices)
        assert len(vertices) % 10 == 0
        assert tuple(ob.data.polygons[0].vertices) == (0, 1, 11, 10)
        rings = len(vertices) // 10
        centers = [sum((v.co for v in vertices[i * 10:(i + 1) * 10]), Vector()) / 10
                   for i in range(rings)]
        root_radius = sum((v.co - centers[0]).length for v in vertices[:10]) / 10
        a = centers[0]
        c = a + (centers[-1] - a) * 0.86
        axis = (centers[1] - a).normalized()
        b = a + axis * ((c - a).length * 0.5)
        across = (vertices[0].co - a).normalized()
        across = (across - axis * across.dot(axis)).normalized()
        around = axis.cross(across)
        angles = [math.atan2((v.co - a).dot(around), (v.co - a).dot(across))
                  for v in vertices[:10]]
        for i in range(rings):
            row = vertices[i * 10:(i + 1) * 10]
            t = i / (rings - 1)
            radius = root_radius * (0.85 + 0.15 * _smooth(t / 0.25))
            radius *= math.sqrt(max(0.008, 1.0 - ((t - 0.20) / 0.80) ** 2))
            target = (1 - t) ** 2 * a + 2 * (1 - t) * t * b + t * t * c
            axis = ((1 - t) * (b - a) + t * (c - b)).normalized()
            across = (across - axis * across.dot(axis)).normalized()
            around = axis.cross(across)
            for v, angle in zip(row, angles):
                v.co = target + radius * (across * math.cos(angle) + around * math.sin(angle))
        ob.data.update()

    body = bpy.data.objects['Hearthling_Body']
    hand_groups = {body.vertex_groups['hand.' + s].index: s for s in ['L', 'R']}
    hand_weights = [max((g.weight for g in v.groups if g.group in hand_groups),
                       default=0.0) for v in body.data.vertices]
    _relax(body, [_smooth((w - 0.2) / 0.8) for w in hand_weights], 0.12, 2)

    # Extend the existing brown digit colour into the palm, with a soft fur cuff
    # at the wrist. This is vertex paint in the same shared coat material.
    coat = body.data.color_attributes['CoatColor']
    brown = (0.115, 0.034, 0.014, 1.0)
    for v in body.data.vertices:
        for group in v.groups:
            side = hand_groups.get(group.group)
            if side is None:
                continue
            bone = rig.data.bones['hand.' + side]
            axis = (bone.tail_local - bone.head_local).normalized()
            along = (v.co - bone.head_local).dot(axis) / bone.length
            w = _smooth((group.weight - 0.15) / 0.70) * _smooth((along + 0.05) / 0.35)
            old = coat.data[v.index].color[:]
            coat.data[v.index].color = tuple(old[k] * (1 - w) + brown[k] * w
                                            for k in range(4))
    body.data.update()

    # Preserve the eye apertures, lids, brows and muzzle. Relax only the high
    # crown ridges, where the selected polished face has softer fur transitions.
    head = bpy.data.objects['Hearthling_Head']
    crown = [_smooth((v.co.z - 1.86) / 0.10) *
             _smooth((0.40 - abs(v.co.x)) / 0.10) for v in head.data.vertices]
    _relax(head, crown, 0.15, 2)

    rig['selected_reference_revision'] = 10
    rig['selected_reference_inputs'] = '09-rounded-hand-paws;16-closest-current-polish;19-soft-woodland-realism'
    rig.data.pose_position = previous
    bpy.context.view_layer.update()
    print({'selected_reference_revision': 10, 'rounded_digits': len(digits),
           'topology': 'unchanged', 'skeleton_and_weights': 'unchanged'}, flush=True)
