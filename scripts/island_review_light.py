"""Blender authoring equivalent of the island's actual sun, fill and focus-relative fog.
The render is a material/lighting study: it does not claim Bevy shading or performance.
"""

import json
from pathlib import Path
import bpy
from mathutils import Vector


def linear(rgb):
    return tuple(
        v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in rgb
    )


def apply(scene, sun, light, path):
    p = json.loads(Path(path).read_text())
    sun.color = linear(p["sun_srgb"])
    dx, dy, dz = p["sun_ray_direction"]
    light.rotation_euler = Vector((dx, -dz, dy)).to_track_quat("-Z", "Y").to_euler()
    scene.world.node_tree.nodes["Background"].inputs["Color"].default_value = (
        *linear(p["sky_fill_srgb"]),
        1,
    )
    scene.world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.78
    # Use the clear colour for camera rays without replacing the sky's ambient fill.
    tree = scene.world.node_tree
    bg = tree.nodes["Background"]
    out = tree.nodes["World Output"]
    mix = tree.nodes.new("ShaderNodeMixShader")
    ray = tree.nodes.new("ShaderNodeLightPath")
    clear = tree.nodes.new("ShaderNodeBackground")
    clear.inputs["Color"].default_value = (*linear(p["clear_srgb"]), 1)
    clear.inputs["Strength"].default_value = 1.0
    tree.links.new(ray.outputs["Is Camera Ray"], mix.inputs[0])
    tree.links.new(bg.outputs[0], mix.inputs[1])
    tree.links.new(clear.outputs[0], mix.inputs[2])
    tree.links.new(mix.outputs[0], out.inputs["Surface"])
    scene.view_layers[0].use_pass_mist = True
    scene.world.mist_settings.falloff = "LINEAR"
    if bpy.app.version >= (5, 0, 0):
        graph = bpy.data.node_groups.new("Coastal air", "CompositorNodeTree")
        scene.compositing_node_group = graph
        graph.interface.new_socket(
            name="Image", in_out="OUTPUT", socket_type="NodeSocketColor"
        )
        output = graph.nodes.new("NodeGroupOutput")
        math_id = "ShaderNodeMath"
        mix_id = "ShaderNodeMix"
    else:
        scene.use_nodes = True
        graph = scene.node_tree
        graph.nodes.clear()
        output = graph.nodes.new("CompositorNodeComposite")
        math_id = "CompositorNodeMath"
        mix_id = "CompositorNodeMixRGB"
    result = graph.nodes.new("CompositorNodeRLayers")
    haze = graph.nodes.new(math_id)
    haze.operation = "MULTIPLY"
    haze.inputs[1].default_value = p["air_strength"]
    graph.links.new(result.outputs["Mist"], haze.inputs[0])
    combine = graph.nodes.new(mix_id)
    combine.blend_type = "MIX"
    if mix_id == "ShaderNodeMix":
        combine.data_type = "RGBA"
        a, b = [s for s in combine.inputs if s.name in ("A", "B") and s.type == "RGBA"]
        out = next(s for s in combine.outputs if s.type == "RGBA")
    else:
        a, b = combine.inputs[1:3]
        out = combine.outputs[0]
    graph.links.new(haze.outputs[0], combine.inputs[0])
    graph.links.new(result.outputs["Image"], a)
    b.default_value = (*linear(p["air_srgb"]), 1)
    graph.links.new(out, output.inputs[0])
    return p
