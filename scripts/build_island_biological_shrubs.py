"""Export the two editable island prop overrides; never rebuild terrain or actors.

Default: load island-biological-shrubs.blend and export its current edits.
--author-from: initialize that source from the exact 9a4e3b3d runtime baselines.
The explicit bootstrap guard prevents applying the sculpt/color pass twice.
"""
import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from compact_glb_colors import compact_colors

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/alife_game_app/assets/landscape/island'
KINDS = ('BerryBush', 'CoastalShrub')
BASE_HASHES = {
    'BerryBush': 'fb06dd77414de407d372026dfb5310bb7c7262bb714c64a194c54ee2bed52a61',
    'CoastalShrub': '2bf616a3cabb24b230cba57aa5e82298fb20e6dc04aaa84716969e26249fbbd1',
}


def bounds(mesh):
    return [min(v.co[k] for v in mesh.vertices) for k in range(3)], [
        max(v.co[k] for v in mesh.vertices) for k in range(3)]


def point_colors(mesh):
    attr = mesh.color_attributes.active_color
    sums = [Vector((0, 0, 0)) for _ in mesh.vertices]
    counts = [0 for _ in mesh.vertices]
    for loop, color in zip(mesh.loops, attr.data):
        sums[loop.vertex_index] += Vector(color.color[:3])
        counts[loop.vertex_index] += 1
    return [s / n for s, n in zip(sums, counts)]


def components(mesh, colors):
    graph = [set() for _ in mesh.vertices]
    for edge in mesh.edges:
        a, b = edge.vertices
        graph[a].add(b)
        graph[b].add(a)
    remaining = set(range(len(mesh.vertices)))
    result = []
    while remaining:
        first = min(remaining)
        remaining.remove(first)
        stack, group = [first], []
        while stack:
            index = stack.pop()
            group.append(index)
            for neighbor in graph[index]:
                if neighbor in remaining:
                    remaining.remove(neighbor)
                    stack.append(neighbor)
        color = sum((colors[i] for i in group), Vector()) / len(group)
        kind = ('fruit' if color.x > color.y * 1.3 and color.z > color.x * .55
                else 'wood' if color.x > color.y * 1.2 else 'leaf')
        center = sum((mesh.vertices[i].co for i in group), Vector()) / len(group)
        result.append((group, kind, center))
    return result


def clamp(value):
    return max(0., min(1., value))


def mix(a, b, weight):
    return tuple(x * (1 - weight) + y * weight for x, y in zip(a, b))


def reshape_and_paint(ob, kind):
    mesh = ob.data
    lo, hi = bounds(mesh)
    width, depth, height = (hi[k] - lo[k] for k in range(3))
    old_colors = point_colors(mesh)
    groups = components(mesh, old_colors)
    semantics = {}
    last_leaf = None
    coarse_fruit = []
    for indices, category, center in groups:
        if category == 'leaf':
            last_leaf = center.copy()
        for i in indices:
            semantics[i] = category
        if kind == 'BerryBush' and category == 'fruit':
            assert last_leaf is not None
            displacement = (center - last_leaf) * .62 + Vector((0, 0, -.025 * height))
            if ob.name.endswith('_L2'):
                coarse_fruit.append((indices, center + displacement))
                continue
            xy_scale, z_scale = (1.55, 1.8) if ob.name.endswith('_L1') else (2.15, 2.6)
            for i in indices:
                delta = mesh.vertices[i].co - center
                mesh.vertices[i].co = center + displacement + Vector(
                    (delta.x * xy_scale, delta.y * xy_scale, delta.z * z_scale))
        elif kind == 'CoastalShrub' and category == 'leaf':
            for i in indices:
                delta = mesh.vertices[i].co - center
                mesh.vertices[i].co = center + Vector(
                    (delta.x * 1.25, delta.y * .82, delta.z * .35 + delta.x * .16))
    # Three pre-existing one-face berries become one three-sided overview pod.
    # The open attachment side sits inside the crown; no triangles are added.
    for start in range(0, len(coarse_fruit), 3):
        cluster = coarse_fruit[start:start + 3]
        assert len(cluster) == 3
        center = sum((c for _, c in cluster), Vector()) / 3
        radius = width * .063
        rim = [center + Vector((radius * math.cos(j * math.tau / 3),
                              radius * math.sin(j * math.tau / 3), height * .025))
               for j in range(3)]
        tip = center + Vector((0, 0, -height * .065))
        for side, (indices, _) in enumerate(cluster):
            faces = [p for p in mesh.polygons if set(p.vertices).issubset(indices)]
            assert len(faces) == 1 and len(faces[0].vertices) == 3
            for index, position in zip(faces[0].vertices, (rim[side], tip, rim[(side + 1) % 3])):
                mesh.vertices[index].co = position
    if kind == 'CoastalShrub':
        for vertex in mesh.vertices:
            z = (vertex.co.z - lo[2]) / height
            vertex.co.x += .24 * width * z * z
    if kind in ('BerryBush', 'CoastalShrub'):
        new_lo, new_hi = bounds(mesh)
        for vertex in mesh.vertices:
            for k in range(3):
                vertex.co[k] = lo[k] + (vertex.co[k] - new_lo[k]) * (
                    (hi[k] - lo[k]) / (new_hi[k] - new_lo[k]))
        bpy.context.view_layer.objects.active = ob
        ob.select_set(True)
        bpy.ops.mesh.customdata_custom_splitnormals_clear()
        ob.select_set(False)
        mesh.update()
    attr = mesh.color_attributes.active_color
    attr.name = 'ArtColor'
    for loop, entry in zip(mesh.loops, attr.data):
        vertex = mesh.vertices[loop.vertex_index]
        x, y, z = vertex.co
        t = (z - lo[2]) / height
        category = semantics[loop.vertex_index]
        original = old_colors[loop.vertex_index]
        if kind == 'BerryBush':
            if category == 'fruit':
                color = mix((.39, .065, .21), (.62, .19, .39), clamp(t * .65))
            elif category == 'wood':
                color = (.12, .085, .046)
            else:
                color = mix((.11, .235, .058), (.245, .36, .10), clamp((t - .55) * 1.7))
                shade = clamp(original.y / .30) * .17 + .83
                color = tuple(c * shade for c in color)
        elif kind == 'CoastalShrub':
            if category == 'wood':
                color = (.145, .115, .075)
            else:
                tip = clamp((t - .65) * 2.85) * clamp(.58 + x / width)
                color = mix((.075, .23, .145), (.24, .44, .34), tip)
                shade = clamp(original.y / .30) * .16 + .84
                color = tuple(c * shade for c in color)
        entry.color = (*color, 1)
    ob['art_owner'] = 'island-biological-shrubs.blend'
    ob['art_pass'] = '2026-10-09 biological silhouettes and vertex-color signatures'
    ob['origin_contract'] = 'ground-centred; metres; Z-up source / Y-up runtime'
    ob['baseline_bounds_xyz'] = [*lo, *hi]


def author(directory, source):
    for kind, expected in BASE_HASHES.items():
        path = directory / (kind + '.glb')
        assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, (
            'Bootstrap needs the original 9a4e3b3d GLB, not an already refreshed export', kind)
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    for kind in KINDS:
        bpy.ops.import_scene.gltf(filepath=str(directory / (kind + '.glb')))
        for level in range(3):
            reshape_and_paint(bpy.data.objects[kind + '_L' + str(level)], kind)
    shared = bpy.data.materials['Island shared painted surface']
    shared.use_nodes = True
    shader = next(n for n in shared.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    shader.inputs['Roughness'].default_value = .82
    color = shared.node_tree.nodes.new('ShaderNodeVertexColor')
    color.layer_name = 'ArtColor'
    shared.node_tree.links.new(color.outputs['Color'], shader.inputs['Base Color'])
    for ob in bpy.data.objects:
        if ob.type == 'MESH':
            ob.data.materials.clear()
            ob.data.materials.append(shared)
    for material in list(bpy.data.materials):
        if material is not shared and material.users == 0:
            bpy.data.materials.remove(material)
    bpy.context.scene.unit_settings.system = 'METRIC'
    bpy.context.scene.unit_settings.scale_length = 1
    bpy.context.preferences.filepaths.save_version = 0
    for index, kind in enumerate(KINDS):
        for level in range(3):
            ob = bpy.data.objects[kind + '_L' + str(level)]
            ob.location = ((index - 2) * 3, 0, 0)
            ob.hide_render = level != 0
            ob.hide_set(level != 0)
    source.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(source))


def export(output):
    receipt = {}
    for kind in KINDS:
        objects = [bpy.data.objects[kind + '_L' + str(i)] for i in range(3)]
        originals = [(o, o.location.copy(), o.hide_get(), o.hide_render) for o in objects]
        bpy.ops.object.select_all(action='DESELECT')
        for ob in objects:
            ob.location = (0, 0, 0)
            ob.hide_set(False)
            ob.hide_render = False
            ob.select_set(True)
            assert tuple(ob.scale) == (1., 1., 1.)
            assert all(abs(x) < 1e-6 for x in ob.rotation_euler)
        path = output / (kind + '.glb')
        output.mkdir(parents=True, exist_ok=True)
        bpy.ops.export_scene.gltf(filepath=str(path), export_format='GLB',
            use_selection=True, export_animations=False, export_vertex_color='ACTIVE')
        compact_colors(path)
        receipt[kind] = {'triangles': [sum(len(p.vertices) - 2 for p in o.data.polygons)
                                    for o in objects], 'runtime_bytes': path.stat().st_size}
        for ob, location, hidden, render_hidden in originals:
            ob.location = location
            ob.hide_set(hidden)
            ob.hide_render = render_hidden
    print('BIOLOGICAL_PROP_EXPORT', json.dumps(receipt), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ASSETS / 'island-biological-shrubs.blend')
    parser.add_argument('--output-dir', type=Path, default=ASSETS)
    parser.add_argument('--author-from', type=Path)
    arguments = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
    args = parser.parse_args(arguments)
    if args.author_from:
        author(args.author_from, args.source)
    else:
        bpy.ops.wm.open_mainfile(filepath=str(args.source))
    export(args.output_dir)


if __name__ == '__main__':
    main()
