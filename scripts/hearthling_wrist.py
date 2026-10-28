"""Cut only the distal hand component, not a spatially masked surface patch.

The component is identified on the positive side of the wrist plane before
bisection. Its complete incident-face neighborhood is the only bisect geometry;
all selection-boundary edges are proximal and therefore never intersect the
plane. This avoids both collateral torso cuts and open mask-boundary chains.
"""
import bmesh
from mathutils import Vector


def cut_distal_wrist(bm, bone, hand_group_index, cut_y=-.055, epsilon=1e-6):
    matrix = bone.matrix_local.copy()
    inverse = matrix.inverted()
    local = {v:inverse @ v.co for v in bm.verts}
    positive = {v for v,p in local.items() if p.y > cut_y + epsilon}
    deform = bm.verts.layers.deform.active
    assert deform is not None, 'Expected existing skin weights'
    target = Vector((0,.040,-.035))
    seeds = [v for v in positive if v[deform].get(hand_group_index,0) > .25]
    assert seeds, ('No distal hand seed',bone.name)
    seed = min(seeds,key=lambda v:(local[v]-target).length_squared)
    component = {seed}
    todo = [seed]
    while todo:
        v = todo.pop()
        for e in v.link_edges:
            other = e.other_vert(v)
            if other in positive and other not in component:
                component.add(other)
                todo.append(other)
    # Safety checks only; do not use a geometric/weight mask to select faces.
    assert len(component) < len(local)*.25, ('Hand is not an isolated branch',bone.name,len(component))
    assert all(abs(local[v].x)<.30 and abs(local[v].z)<.25 and local[v].y<.40 for v in component), ('Unexpected distal component extent',bone.name)
    incident = {f for v in component for f in v.link_faces}
    edges = {e for f in incident for e in f.edges}
    vertices = {v for f in incident for v in f.verts}
    original_boundary = {e for e in bm.edges if e.is_boundary}
    retained = {v:tuple(v.co) for v in bm.verts if v not in component}
    untouched_faces = {f for f in bm.faces if f not in incident}
    face_vertices = {f:tuple(f.verts) for f in untouched_faces}
    bmesh.ops.bisect_plane(bm, geom=list(vertices)+list(edges)+list(incident),
                          plane_co=matrix @ Vector((0,cut_y,0)),
                          plane_no=(bone.tail_local-bone.head_local).normalized(),
                          dist=epsilon, use_snap_center=False,
                          clear_inner=False, clear_outer=False)
    bmesh.ops.delete(bm,geom=list(component),context='VERTS')
    assert all(v.is_valid and tuple(v.co)==p for v,p in retained.items()), 'Moved/deleted a non-distal vertex'
    assert all(f.is_valid and tuple(f.verts)==vs for f,vs in face_vertices.items()), 'Changed a non-hand face'
    assert all(e.is_valid and e.is_boundary for e in original_boundary), 'Changed an existing boundary'
    boundary = [e for e in bm.edges if e.is_boundary and e not in original_boundary]
    assert boundary and all(abs((inverse@v.co).y-cut_y)<1e-5 for e in boundary for v in e.verts), ('Off-plane wrist boundary',bone.name)
    ring = {v for e in boundary for v in e.verts}
    assert all(sum(e in boundary for e in v.link_edges)==2 for v in ring), ('Open wrist chain',bone.name)
    reached = set()
    todo = [next(iter(ring))]
    while todo:
        v = todo.pop()
        if v in reached: continue
        reached.add(v)
        todo.extend(e.other_vert(v) for e in v.link_edges if e in boundary)
    assert reached==ring, ('More than one wrist ring',bone.name)
    return {'wrist_edges':boundary, 'wrist_vertices':list(ring),
            'wrist_points':[inverse@v.co for v in ring],
            'affected_original_faces':incident,
            'report':{'side':bone.name,'removed_vertices':len(component),
                      'incident_faces':len(incident),'wrist_edges':len(boundary),
                      'wrist_vertices':len(ring),'unchanged_faces':len(untouched_faces),
                      'local_bounds':[[min((inverse@v.co)[k] for v in ring),
                                       max((inverse@v.co)[k] for v in ring)] for k in range(3)]}}
