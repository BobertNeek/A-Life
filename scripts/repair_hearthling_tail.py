"""Close the selected Hearthling's small tail tunnel, without changing its rig.

The anchors describe the accepted native checkpoint in object-space coordinates;
indices are deliberately not used for matching.  This is a one-shot pass.
For an isolated checkpoint run Blender with --python this_file -- --source PATH
--output PATH. The output must not exist and can never be the source.
"""
import argparse
import os
from pathlib import Path
import tempfile

import bpy
import bmesh
from mathutils import Vector

# Native 42a0c5185 anchors, in near-ring then far-ring order. The full-source
# rebuild differs slightly in position and index, hence the bounded lookup.
ANCHORS = (
    (0.64201819896698, 0.6224460601806641, 1.3517262935638428),
    (0.6368868350982666, 0.6366276144981384, 1.380472183227539),
    (0.6471258997917175, 0.6319938898086548, 1.3948676586151123),
    (0.6754579544067383, 0.6276238560676575, 1.3806697130203247),
    (0.6879924535751343, 0.6288488507270813, 1.3631401062011719),
    (0.6871989965438843, 0.6336596012115479, 1.3475419282913208),
    (0.6616846323013306, 0.6105519533157349, 1.3302242755889893),
    (0.6483539342880249, 0.6491537690162659, 1.3536725044250488),
    (0.6390630006790161, 0.6607992649078369, 1.385093331336975),
    (0.6492798328399658, 0.6548412442207336, 1.3951303958892822),
    (0.6736438870429993, 0.6594290137290955, 1.388288974761963),
    (0.6904646158218384, 0.6579171419143677, 1.3713793754577637),
    (0.7060118317604065, 0.6854614019393921, 1.3625179529190063),
    (0.677134096622467, 0.6813592910766602, 1.3370027542114258),
    (0.6661427021026611, 0.652298092842102, 1.3387755155563354),
)
ANCHOR_TOLERANCE = 0.0002
# Local anchor labels, face winding as in the accepted surface.
STRIP = (
    (7,0,6), (7,6,14), (11,3,10), (11,4,3), (8,9,1),
    (5,4,11), (13,5,12), (2,9,10,3), (14,6,5),
    (13,14,5), (7,8,1), (7,1,0), (9,2,1), (5,11,12),
)
NEAR = (0,1,2,3,4,5,6)
FAR = (7,8,9,10,11,12,13,14)
# Explicit winding and triangulation: the tempting beauty triangulation of
# the far ring intersects another tail triangle in the sleep animation.
CAPS = (
    (7,13,14), (7,12,13), (7,11,12), (7,10,11), (7,9,10), (7,8,9),
    (3,2,1), (4,3,0), (5,4,0), (6,5,0), (0,3,1),
)


def _require(condition, message):
    if not condition:
        raise ValueError('Hearthling tail repair: ' + message)


def _cycle(ids):
    """Canonicalize by rotation, preserving winding."""
    ids = tuple(ids)
    return min(ids[i:] + ids[:i] for i in range(len(ids)))


def _anchor_indices(mesh):
    found = []
    for anchor in ANCHORS:
        point = Vector(anchor)
        hits = [v.index for v in mesh.vertices if (v.co - point).length <= ANCHOR_TOLERANCE]
        _require(len(hits) == 1, 'anchor missing or ambiguous: ' + repr(anchor))
        found.append(hits[0])
    _require(len(set(found)) == len(found), 'anchors do not map one-to-one')
    return found


def repair_tail():
    """Apply once, transactionally to an isolated mesh copy; leave pose context alone."""
    obj = bpy.data.objects.get('Hearthling_Tail')
    _require(obj is not None and obj.type == 'MESH', 'missing tail mesh')
    original = obj.data
    _require(original.users == 1, 'tail mesh has other users')
    counts = (len(original.vertices), len(original.edges), len(original.polygons))
    _require(counts in ((1300, 3861, 2559), (1300, 3861, 2561)),
             'unexpected mesh topology or repair already applied')
    # The full generator holds two pairs of tiny, isolated, oppositely wound
    # duplicate triangles in memory. Blender drops one from each pair when the
    # file is saved/reopened. Keep all of them here; do not change unrelated
    # geometry just to normalize the live and checkpoint representations.
    from collections import defaultdict
    by_vertices = defaultdict(list)
    for polygon in original.polygons:
        by_vertices[tuple(sorted(polygon.vertices))].append(polygon)
    duplicates = [group for group in by_vertices.values() if len(group) > 1]
    if counts[2] == 2561:
        _require(len(duplicates) == 2 and all(len(group) == 2 for group in duplicates),
                 'unexpected live duplicate faces')
        tiny_centers = []
        for pair in duplicates:
            a, b = pair
            vertices = set(a.vertices)
            _require(len(vertices) == 3 and max(a.area, b.area) < 1e-10 and
                     _cycle(tuple(a.vertices)) == _cycle(tuple(reversed(tuple(b.vertices)))) and
                     sum(bool(vertices.intersection(p.vertices)) for p in original.polygons) == 2,
                     'unexpected live degenerate components')
            tiny_centers.append(sum((original.vertices[i].co for i in vertices), Vector()) / 3)
        expected_centers = (Vector((.94050998,.70622486,.64543458)),
                            Vector((.87363364,.67658480,.63682117)))
        _require(all(sum((center - expected).length <= ANCHOR_TOLERANCE
                         for center in tiny_centers) == 1 for expected in expected_centers),
                 'unexpected live degenerate locations')
    else:
        _require(not duplicates, 'unexpected checkpoint duplicate faces')
    _require(original.shape_keys is None, 'shape keys are not supported')
    _require(original.uv_layers.active is not None, 'missing UV layer')
    _require(len(original.materials) == 1, 'unexpected material slots')
    material = original.materials[0]
    _require(material is not None and material.name == 'Hearthling | vertex-colored coat'
             and material.use_nodes and material.node_tree is not None, 'unexpected tail material')
    mapped = _anchor_indices(original)
    labels = {vertex: label for label, vertex in enumerate(mapped)}
    expected = {_cycle(face) for face in STRIP}
    actual = {_cycle(tuple(labels[i] for i in p.vertices)) for p in original.polygons
              if all(i in labels for i in p.vertices)}
    _require(actual == expected and len(actual) == len(STRIP), 'strip faces or winding differ')

    # Check the flat corner of the existing packed fur-normal atlas before
    # assigning it to the new cap; it is the same patch used by extremities.
    image = bpy.data.images.get('Hearthling_FurNormal')
    _require(image is not None and tuple(image.size) == (2048,2048), 'unexpected normal atlas')
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    # The checked image must actually feed this material's normal input.
    _require(any(texture.type == 'TEX_IMAGE' and texture.image == image
                 and any(link.from_node == texture and link.from_socket.name == 'Color'
                         and link.to_node.type == 'NORMAL_MAP' and link.to_socket.name == 'Color'
                         and any(next_link.from_node == link.to_node
                                 and next_link.from_socket.name == 'Normal'
                                 and next_link.to_node.type == 'BSDF_PRINCIPLED'
                                 and next_link.to_socket.name == 'Normal'
                                 for next_link in links)
                         for link in links)
                 for texture in nodes), 'tail material does not use the normal atlas')
    from array import array
    pixels = array('f', [0]) * (2048 * 2048 * 4)
    image.pixels.foreach_get(pixels)
    _require(all(abs(pixels[4*(y*2048+x)+k]-value) < .01
                 for y in range(8,57) for x in range(8,57)
                 for k, value in enumerate((.5,.5,1))), 'normal atlas neutral patch occupied')

    # Everything up to the final pointer swap operates on a copy. If any check
    # fails, the original mesh and all frame/action/NLA state remain untouched.
    trial = original.copy()
    bm = bmesh.new()
    try:
        bm.from_mesh(trial)
        bm.verts.ensure_lookup_table(); bm.edges.ensure_lookup_table(); bm.faces.ensure_lookup_table()
        faces = [f for f in bm.faces if all(v.index in labels for v in f.verts)]
        _require(len(faces) == 14, 'unexpected strip size')
        for f in faces:
            for e in f.edges:
                _require(e.is_manifold and len(e.link_faces) == 2, 'strip is not manifold')
                a, b = e.link_faces
                # Adjacent faces must traverse the shared edge in opposite directions.
                def direction(face):
                    loop = next(l for l in face.loops if l.edge == e)
                    return (loop.vert, loop.link_loop_next.vert)
                _require(direction(a) == direction(b)[::-1], 'inconsistent edge winding')
        bmesh.ops.delete(bm, geom=faces, context='FACES_ONLY')
        wires = [e for e in bm.edges if not e.link_faces]
        _require(len(wires) == 14, 'unexpected strip interior')
        bmesh.ops.delete(bm, geom=wires, context='EDGES')
        for ring in (NEAR, FAR):
            for a,b in zip(ring, ring[1:]+ring[:1]):
                edge = bm.edges.get((bm.verts[mapped[a]], bm.verts[mapped[b]]))
                _require(edge is not None and edge.is_boundary, 'missing or non-boundary ring edge')
        uv = bm.loops.layers.uv.active
        _require(uv is not None, 'missing corner UV layer')
        for face in CAPS:
            f = bm.faces.new([bm.verts[mapped[i]] for i in face])
            f.material_index = 0
            f.smooth = True
            for loop in f.loops:
                loop[uv].uv = (.015 + (loop.vert.co.x-.66)*.03,
                               .015 + (loop.vert.co.z-1.37)*.03)
        expected_faces = counts[2] - 3
        expected_boundaries = 6 if counts[2] == 2559 else 0
        _require((len(bm.verts),len(bm.edges),len(bm.faces)) ==
                 (1300,3856,expected_faces), 'unexpected repaired counts')
        _require(sum(e.is_boundary for e in bm.edges) == expected_boundaries and
                 sum(not e.is_manifold and not e.is_boundary for e in bm.edges) == 0,
                 'unexpected repaired boundaries or nonmanifold edges')
        for f in bm.faces:
            if all(v.index in labels for v in f.verts):
                _require(f.calc_area() > 1e-12, 'degenerate cap triangle')
                a,b,c = [loop[uv].uv.copy() for loop in f.loops]
                _require(abs((b-a).cross(c-a)) > 2e-12, 'degenerate cap UV')
                for edge in f.edges:
                    _require(edge.is_manifold, 'cap edge is not manifold')
                    a, b = edge.link_faces
                    def direction(face):
                        loop = next(l for l in face.loops if l.edge == edge)
                        return (loop.vert, loop.link_loop_next.vert)
                    _require(direction(a) == direction(b)[::-1], 'cap winding mismatch')
        bm.normal_update()
        bm.to_mesh(trial)
        trial.update()
    except BaseException:
        bm.free()
        bpy.data.meshes.remove(trial)
        raise
    bm.free()
    # This is the only visible mutation; all validation has succeeded.
    name = original.name
    obj.data = trial
    bpy.data.meshes.remove(original)
    trial.name = name
    return tuple(mapped)


def main():
    import sys
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
    source = args.source.resolve(strict=True)
    output = args.output.resolve()
    _require(source != output, 'refusing source overwrite')
    _require(not output.exists(), 'output already exists')
    _require(output.suffix.lower() == '.blend', 'output must be a .blend file')
    _require(output.parent.is_dir(), 'output directory does not exist')
    bpy.ops.wm.open_mainfile(filepath=str(source))
    repair_tail()
    # Save in the same directory, then link into place without clobbering a
    # file created concurrently. No partly-written requested output is exposed.
    fd, tmp = tempfile.mkstemp(prefix='.hearthling-tail-', suffix='.blend', dir=output.parent)
    os.close(fd); os.unlink(tmp)
    try:
        result = bpy.ops.wm.save_as_mainfile(filepath=tmp)
        _require('FINISHED' in result, 'temporary save failed')
        os.link(tmp, output)
    finally:
        if os.path.exists(tmp): os.unlink(tmp)
    print('HEARTHLING_TAIL_CHECKPOINT', output, flush=True)


if __name__ == '__main__':
    main()
