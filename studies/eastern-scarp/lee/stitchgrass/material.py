"""Use the camera-independent thatch transfer over the existing DCC habitat material."""

import bpy


def paint(ground, surf, folder):
    a, b, c, d = 362.5, 381.25, -90.625, -78.125
    img = bpy.data.images.load(
        str((folder / "thatch-transfer.png").resolve()), check_existing=True
    )
    img.colorspace_settings.name = "Non-Color"
    img.alpha_mode = "CHANNEL_PACKED"
    for mat in {
        s.material
        for o in ground
        for s in o.material_slots
        if s.material and s.material.use_nodes
    }:
        nt = mat.node_tree
        bs = surf.bsdf(mat)
        color = bs.inputs["Base Color"]
        old = color.links[0].from_socket if color.is_linked else None
        if not old:
            continue
        uv = surf.coords(
            nt, (0, 1 / (b - a), -a / (b - a)), (1, 1 / (d - c), d / (d - c))
        )
        image = nt.nodes.new("ShaderNodeTexImage")
        image.image = img
        image.extension = "EXTEND"
        image.label = "Folded grass and organic seams, 1.95cm"
        nt.links.new(uv, image.inputs["Vector"])

        # Notebook encoding: linear gain = alpha*(255/128);
        # additive channel = colour*(255/1600) - 128/1600.
        def scale(input, s):
            o = nt.nodes.new("ShaderNodeVectorMath")
            o.operation = "SCALE"
            nt.links.new(input, o.inputs[0])
            if isinstance(s, (float, int)):
                o.inputs["Scale"].default_value = s
            else:
                nt.links.new(s, o.inputs["Scale"])
            return o.outputs[0]

        gain = nt.nodes.new("ShaderNodeMath")
        gain.operation = "MULTIPLY"
        nt.links.new(image.outputs["Alpha"], gain.inputs[0])
        gain.inputs[1].default_value = 255 / 128
        under = scale(old, gain.outputs[0])
        off = scale(image.outputs["Color"], 255 / 1600)
        sub = nt.nodes.new("ShaderNodeVectorMath")
        sub.operation = "SUBTRACT"
        nt.links.new(off, sub.inputs[0])
        sub.inputs[1].default_value = (0.08, 0.08, 0.08)
        add = nt.nodes.new("ShaderNodeVectorMath")
        add.operation = "ADD"
        nt.links.new(under, add.inputs[0])
        nt.links.new(sub.outputs[0], add.inputs[1])
        nt.links.new(add.outputs[0], color)
