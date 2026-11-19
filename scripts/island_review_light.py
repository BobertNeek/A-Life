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
    scene.use_nodes = True
    graph = scene.node_tree
    graph.nodes.clear()
    result = graph.nodes.new("CompositorNodeRLayers")
    haze = graph.nodes.new("CompositorNodeMath")
    haze.operation = "MULTIPLY"
    haze.inputs[1].default_value = p["air_strength"]
    graph.links.new(result.outputs["Mist"], haze.inputs[0])
    combine = graph.nodes.new("CompositorNodeMixRGB")
    combine.blend_type = "MIX"
    graph.links.new(haze.outputs[0], combine.inputs[0])
    graph.links.new(result.outputs["Image"], combine.inputs[1])
    combine.inputs[2].default_value = (*linear(p["air_srgb"]), 1)
    output = graph.nodes.new("CompositorNodeComposite")
    graph.links.new(combine.outputs[0], output.inputs["Image"])
    return p
