"""CPU Blender material equivalent of a proposed optional near terrain layer.

Use exact 3D-world coordinates and actual original biome labels; preserve
all old albedo/maps/normals and the whole far view. This graph is artist
source, NOT a native renderer/pixel-equivalence or performance claim.
"""

from pathlib import Path
import json
import bpy


def bind(meshes, folder):
    folder = Path(folder)
    spec = json.loads((folder / "microfield.json").read_text())
    materials = {
        slot.material for obj in meshes for slot in obj.material_slots if slot.material
    }

    def read(name):
        image = bpy.data.images.load(
            str((folder / name).resolve()), check_existing=True
        )
        image.colorspace_settings.name = "Non-Color"
        image.alpha_mode = "CHANNEL_PACKED"
        return image

    micro = read("ground-microheight.png")
    field = read("ground-habitat-weights.png")
    for mat in materials:
        if not mat.use_nodes:
            continue
        tree = mat.node_tree
        new = tree.nodes.new
        link = tree.links.new
        p = next(n for n in tree.nodes if n.type == "BSDF_PRINCIPLED")
        previous = (
            p.inputs["Normal"].links[0].from_socket
            if p.inputs["Normal"].is_linked
            else None
        )
        source = new("ShaderNodeNewGeometry").outputs["Position"]

        def affine(inp, gain, bias):
            add = new("ShaderNodeVectorMath")
            add.operation = "ADD"
            link(inp, add.inputs[0])
            add.inputs[1].default_value = bias
            scale = new("ShaderNodeVectorMath")
            scale.operation = "MULTIPLY"
            link(add.outputs["Vector"], scale.inputs[0])
            scale.inputs[1].default_value = gain
            return scale.outputs["Vector"]

        def ink(img, coord, extend):
            n = new("ShaderNodeTexImage")
            n.image = img
            n.extension = extend
            n.interpolation = "Linear"
            link(coord, n.inputs["Vector"])
            return n

        x, z = spec["mask_origin_xz"]
        spacing = spec["mask_spacing"]
        mw, mh = spec["mask_width"], spec["mask_height"]
        # Blender world coordinates are (native x, -native z, native y).
        # Source mask image row zero is native z0, image V=1 at top.
        uv = affine(
            source,
            (1 / (mw * spacing), 1 / (mh * spacing), 0),
            (-x + spacing * 0.5, z + mh * spacing - spacing * 0.5, 0),
        )
        lookup = ink(field, uv, "EXTEND")
        length = spec["tile_metres"]
        muv = affine(source, (1 / length, 1 / length, 0), (0.137, 0.271, 0))
        layer = ink(micro, muv, "REPEAT")
        rgb = new("ShaderNodeVectorMath")
        rgb.operation = "DOT_PRODUCT"
        link(lookup.outputs["Color"], rgb.inputs[0])
        link(layer.outputs["Color"], rgb.inputs[1])
        alpha = new("ShaderNodeMath")
        alpha.operation = "MULTIPLY"
        link(lookup.outputs["Alpha"], alpha.inputs[0])
        link(layer.outputs["Alpha"], alpha.inputs[1])
        total = new("ShaderNodeMath")
        total.operation = "ADD"
        link(rgb.outputs["Value"], total.inputs[0])
        link(alpha.outputs[0], total.inputs[1])
        fade = new("ShaderNodeMapRange")
        fade.clamp = True
        fade.interpolation_type = "SMOOTHERSTEP"
        near, far = spec["near_fade_metres"]
        fade.inputs["From Min"].default_value = near
        fade.inputs["From Max"].default_value = far
        fade.inputs["To Min"].default_value = 0.90
        fade.inputs["To Max"].default_value = 0
        camera = new("ShaderNodeCameraData")
        link(camera.outputs["View Distance"], fade.inputs["Value"])
        bump = new("ShaderNodeBump")
        bump.name = "Close substrate; 4-channel data, real metres"
        bump.inputs["Distance"].default_value = (
            spec["millimetres_per_normalized_unit"] * 0.001
        )
        link(fade.outputs["Result"], bump.inputs["Strength"])
        link(total.outputs[0], bump.inputs["Height"])
        if previous:
            link(previous, bump.inputs["Normal"])
        link(bump.outputs["Normal"], p.inputs["Normal"])
