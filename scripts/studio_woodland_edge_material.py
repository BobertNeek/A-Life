"""Place the artist's RGBA woodland wash on the *existing* true terrain.

DCC-only. Do not make a parallel ground surface or change terrain normals.
"""

import json
from pathlib import Path
import bpy


def bind(objects, folder):
    folder = Path(folder)
    spec = json.loads((folder / "woodland-source.json").read_text())
    x0, x1, z0, z1 = spec["bounds"]
    chart = bpy.data.images.load(
        str((folder / "woodland-bed.png").resolve()), check_existing=True
    )
    chart.alpha_mode = "CHANNEL_PACKED"
    materials = {
        slot.material for obj in objects for slot in obj.material_slots if slot.material
    }
    for material in materials:
        if not material.use_nodes:
            continue
        tree = material.node_tree
        n = tree.nodes.new
        link = tree.links.new
        p = next(node for node in tree.nodes if node.type == "BSDF_PRINCIPLED")
        main = p.inputs["Base Color"]
        world = n("ShaderNodeNewGeometry").outputs["Position"]
        move = n("ShaderNodeVectorMath")
        move.operation = "ADD"
        link(world, move.inputs[0])
        move.inputs[1].default_value = (-x0, z1, 0)
        scale = n("ShaderNodeVectorMath")
        scale.operation = "MULTIPLY"
        link(move.outputs[0], scale.inputs[0])
        scale.inputs[1].default_value = (1 / (x1 - x0), 1 / (z1 - z0), 0)
        texture = n("ShaderNodeTexImage")
        texture.name = "Needle-leeward earth and broken moss at the original two spruce"
        texture.image = chart
        texture.extension = "CLIP"
        link(scale.outputs[0], texture.inputs["Vector"])
        mix = n("ShaderNodeMixRGB")
        mix.blend_type = "MIX"
        link(texture.outputs["Alpha"], mix.inputs[0])
        if main.is_linked:
            link(main.links[0].from_socket, mix.inputs[1])
        else:
            mix.inputs[1].default_value = tuple(main.default_value)
        link(texture.outputs["Color"], mix.inputs[2])
        link(mix.outputs[0], main)
