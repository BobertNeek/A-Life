"""Bind the engine's exported base-colour and normal bytes to the review plane.
Only Blender's lighting/BRDF is substituted. No invented colour noise or foam.
"""

import json
from pathlib import Path

import bpy


def bind(plane, folder):
    folder = Path(folder)
    p = json.loads((folder / "water.json").read_text())
    assert p["colour_space"] == "sRGB" and p["normal_space"] == "linear"
    mat = plane.data.materials[0]
    graph = mat.node_tree
    shader = graph.nodes.get("Principled BSDF")
    shader.inputs["Roughness"].default_value = p["roughness"]
    # Filament's reflectance=0.5 and Blender's IOR=1.5 both mean F0=4%.
    f0 = 0.16 * p["reflectance"] ** 2
    shader.inputs["IOR"].default_value = (1 + f0**0.5) / (1 - f0**0.5)
    span = p["span_xz"]
    ox, oz = p["origin_xz"]
    metre = p["ripple_metres"]
    for name in ("shore", "ripple"):
        uv = plane.data.uv_layers.new(name=name)
        for loop in plane.data.loops:
            pos = plane.matrix_world @ plane.data.vertices[loop.vertex_index].co
            x, z = pos.x, -pos.y
            # Bevy's source starts at the top-left, Blender UVs at bottom-left.
            uv.data[loop.index].uv = (
                ((x - ox) / span[0], 1 - (z - oz) / span[1])
                if name == "shore"
                else (x / metre, 1 - z / metre)
            )
        coords = graph.nodes.new("ShaderNodeUVMap")
        coords.uv_map = uv.name
        node = graph.nodes.new("ShaderNodeTexImage")
        node.image = bpy.data.images.load(str((folder / f"{name}.png").resolve()))
        graph.links.new(coords.outputs["UV"], node.inputs["Vector"])
        if name == "shore":
            node.extension = "EXTEND"
            graph.links.new(node.outputs["Color"], shader.inputs["Base Color"])
        else:
            node.extension = "REPEAT"
            node.image.colorspace_settings.name = "Non-Color"
            normal = graph.nodes.new("ShaderNodeNormalMap")
            normal.uv_map = uv.name
            graph.links.new(node.outputs["Color"], normal.inputs["Color"])
            graph.links.new(normal.outputs["Normal"], shader.inputs["Normal"])
    return p
