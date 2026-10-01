"""Shared Blender CLI helpers for the approved island/hand authoring files."""
import math
from pathlib import Path
import bpy
from mathutils import Vector, noise
from compact_glb_colors import compact_colors

ROOT = Path(__file__).resolve().parents[1]
REVIEW = ROOT / 'target/artifacts/island-hand-implementation'


def material(name, color, vertex=False, roughness=.82):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1)
    mat.use_nodes = True
    bsdf = next(n for n in mat.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    bsdf.inputs['Base Color'].default_value = (*color, 1)
    bsdf.inputs['Roughness'].default_value = roughness
    if vertex:
        node = mat.node_tree.nodes.new('ShaderNodeVertexColor')
        node.layer_name = 'ArtColor'
        mat.node_tree.links.new(node.outputs['Color'], bsdf.inputs['Base Color'])
    return mat


def mesh(name, verts, faces, mat, colors=None, smooth=False, collection=None):
    data = bpy.data.meshes.new(name)
    data.from_pydata(verts, [], faces)
    data.update()
    ob = bpy.data.objects.new(name, data)
    (collection or bpy.context.scene.collection).objects.link(ob)
    data.materials.append(mat)
    for p in data.polygons:
        p.use_smooth = smooth
    if colors:
        attr = data.color_attributes.new(name='ArtColor', type='BYTE_COLOR', domain='POINT')
        for dst, color in zip(attr.data, colors):
            dst.color = (*color[:3], 1)
    return ob


def focus(ob, target):
    ob.rotation_euler = (Vector(target) - ob.location).to_track_quat('-Z', 'Y').to_euler()


def setup_scene():
    noise.seed_set(93026)
    scene = bpy.context.scene
    scene.unit_settings.system = 'METRIC'
    scene.unit_settings.scale_length = 1
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 16
    scene.cycles.use_denoising = True
    scene.render.resolution_x = 1100
    scene.render.resolution_y = 800
    scene.render.resolution_percentage = 100
    scene.world.use_nodes = True
    bg = next(n for n in scene.world.node_tree.nodes if n.type == 'BACKGROUND')
    bg.inputs['Color'].default_value = (.24, .34, .49, 1)
    bg.inputs['Strength'].default_value = .65
    scene.view_settings.view_transform = 'AgX'
    light = bpy.data.lights.new('Review daylight', 'SUN')
    light.energy = 3.0
    light.angle = .10
    sun = bpy.data.objects.new('Review daylight', light)
    scene.collection.objects.link(sun)
    sun.rotation_euler = (.48, -.6, -.6)
    return scene


def camera(name, eye, target, ortho=None, lens=42):
    data = bpy.data.cameras.new(name)
    data.lens = lens
    data.clip_end = 10000
    if ortho:
        data.type = 'ORTHO'
        data.ortho_scale = ortho
    ob = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(ob)
    ob.location = eye
    focus(ob, target)
    return ob


def render(cam, path, resolution=None):
    scene = bpy.context.scene
    scene.camera = cam
    if resolution:
        scene.render.resolution_x, scene.render.resolution_y = resolution
    path.parent.mkdir(parents=True, exist_ok=True)
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


def export(objects, path, **extra):
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objects:
        ob.select_set(True)
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=str(path), export_format='GLB',
        use_selection=True, export_vertex_color='ACTIVE', **extra)
    compact_colors(path)


def reference(path, name, collection):
    ob = bpy.data.objects.new(name, None)
    ob.empty_display_type = 'IMAGE'
    ob.data = bpy.data.images.load(str(path), check_existing=True)
    ob.data.pack()
    ob.empty_display_size = 10
    ob.hide_render = True
    ob.hide_viewport = True
    collection.objects.link(ob)
    return ob


def fit_dimensions(ob, dimensions):
    bounds = [v.co for v in ob.data.vertices]
    lo = Vector(tuple(min(v[k] for v in bounds) for k in range(3)))
    hi = Vector(tuple(max(v[k] for v in bounds) for k in range(3)))
    for v in ob.data.vertices:
        v.co = Vector(((v.co.x-(lo.x+hi.x)*.5)*dimensions[0]/(hi.x-lo.x),
                       (v.co.y-(lo.y+hi.y)*.5)*dimensions[1]/(hi.y-lo.y),
                       (v.co.z-lo.z)*dimensions[2]/(hi.z-lo.z)))
    ob.data.update()
    ob['target_width_depth_height_m'] = dimensions


def blueprint(objects, name, directory, views):
    """Render actual edges; preserve source geometry, hide only for this render."""
    scene = bpy.context.scene
    originals = {o: o.hide_render for o in scene.objects}
    for o in scene.objects:
        if o.type == 'MESH':
            o.hide_render = o not in objects
    wires = []
    cyan = material('Blueprint cyan', (.05, .72, 1.0))
    shader = next(n for n in cyan.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    shader.inputs['Emission Color'].default_value = (.03, .4, .8, 1)
    shader.inputs['Emission Strength'].default_value = .4
    max_height = max(max(v.co.z for v in o.data.vertices) for o in objects)
    for ob in objects:
        clone = ob.copy()
        clone.data = ob.data.copy()
        clone.name = name + '_actual_edges_' + ob.name
        scene.collection.objects.link(clone)
        clone.data.materials.clear()
        clone.data.materials.append(cyan)
        mod = clone.modifiers.new('Actual polygon edges', 'WIREFRAME')
        mod.thickness = max_height * .00065
        mod.use_replace = True
        wires.append(clone)
        ob.hide_render = True
    for label, cam in views:
        render(cam, directory / (name + '-' + label + '.png'), (1200, 900))
    for ob in wires:
        bpy.data.objects.remove(ob, do_unlink=True)
    for ob, hidden in originals.items():
        ob.hide_render = hidden
