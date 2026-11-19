"""A small root adhesion glaze in true world-space, not sprites in the sky."""

import bpy, json


def bind(ground, surf, data):
    cfg = json.loads((data / "lace-budget.json").read_text())
    u0, u1, v0, v1 = cfg["side"]
    for mat in {
        slot.material
        for ob in ground
        for slot in ob.material_slots
        if slot.material and slot.material.use_nodes
    }:
        nt = mat.node_tree
        bs = surf.bsdf(mat)
        co = surf.coords(
            nt,
            (1, -1 / (u1 - u0), -u0 / (u1 - u0)),
            (2, 1 / (v1 - v0), -v0 / (v1 - v0)),
        )
        t = nt.nodes.new("ShaderNodeTexImage")
        t.image = bpy.data.images.load(
            str((data / "lace-hold.png").resolve()), check_existing=True
        )
        t.image.alpha_mode = "CHANNEL_PACKED"
        t.extension = "CLIP"
        nt.links.new(co, t.inputs[0])
        sep = nt.nodes.new("ShaderNodeSeparateXYZ")
        geo = nt.nodes.new("ShaderNodeNewGeometry")
        nt.links.new(geo.outputs["Position"], sep.inputs[0])
        strength = t.outputs["Alpha"]
        for comp, lim in [("GREATER_THAN", 357), ("LESS_THAN", 432)]:
            op = nt.nodes.new("ShaderNodeMath")
            op.operation = comp
            op.inputs[1].default_value = lim
            nt.links.new(sep.outputs[0], op.inputs[0])
            a = nt.nodes.new("ShaderNodeMath")
            a.operation = "MULTIPLY"
            nt.links.new(strength, a.inputs[0])
            nt.links.new(op.outputs[0], a.inputs[1])
            strength = a.outputs[0]
        col = bs.inputs["Base Color"]
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.label = "Bladderlace holdfast / no distant frond geometry"
        nt.links.new(strength, mix.inputs[0])
        nt.links.new(col.links[0].from_socket, mix.inputs[1])
        nt.links.new(t.outputs["Color"], mix.inputs[2])
        nt.links.new(mix.outputs[0], col)
