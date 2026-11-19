"""Mount the shelf's optical geology over its original native substrate in Blender."""

import json, bpy


def coat(grounds, surf, folder):
    shape = json.loads((folder / "mountain-charts.json").read_text())
    a, b, c, d = shape["native_plan_xz"]

    def load(file):
        img = bpy.data.images.load(str((folder / file).resolve()), check_existing=True)
        img.colorspace_settings.name = "Non-Color"
        return img

    gain, resp = load("mountain-gain.png"), load("mountain-response.png")
    for mat in {
        s.material
        for ob in grounds
        for s in ob.material_slots
        if s.material and s.material.use_nodes
    }:
        nt = mat.node_tree
        bs = surf.bsdf(mat)
        uv = surf.coords(
            nt, (0, 1 / (b - a), -a / (b - a)), (1, 1 / (d - c), d / (d - c))
        )

        def tex(img, label):
            o = nt.nodes.new("ShaderNodeTexImage")
            o.image = img
            o.extension = "EXTEND"
            o.label = label
            nt.links.new(uv, o.inputs["Vector"])
            return o.outputs["Color"]

        fac = nt.nodes.new("ShaderNodeVectorMath")
        fac.operation = "SCALE"
        nt.links.new(tex(gain, "Three registered cloudy contacts"), fac.inputs[0])
        fac.inputs["Scale"].default_value = 255 / 128
        base = bs.inputs["Base Color"]
        m = nt.nodes.new("ShaderNodeVectorMath")
        m.operation = "MULTIPLY"
        if base.is_linked:
            nt.links.new(base.links[0].from_socket, m.inputs[0])
        else:
            m.inputs[0].default_value = base.default_value[:3]
        nt.links.new(fac.outputs[0], m.inputs[1])
        nt.links.new(m.outputs[0], base)
        sep = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(
            tex(resp, "Original-slope normal and lithic mantle"), sep.inputs[0]
        )
        bump = nt.nodes.new("ShaderNodeBump")
        bump.inputs["Distance"].default_value = 0.028
        nt.links.new(sep.outputs[2], bump.inputs["Strength"])
        nt.links.new(sep.outputs[1], bump.inputs["Height"])
        n = bs.inputs["Normal"]
        if n.is_linked:
            nt.links.new(n.links[0].from_socket, bump.inputs["Normal"])
        nt.links.new(bump.outputs["Normal"], n)
        rough = bs.inputs["Roughness"]
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.blend_type = "MIX"
        w = nt.nodes.new("ShaderNodeMath")
        w.operation = "MULTIPLY"
        w.inputs[1].default_value = 0.48
        nt.links.new(sep.outputs[2], w.inputs[0])
        nt.links.new(w.outputs[0], mix.inputs[0])
        if rough.is_linked:
            nt.links.new(rough.links[0].from_socket, mix.inputs[1])
        else:
            mix.inputs[1].default_value = (*([rough.default_value] * 3), 1)
        nt.links.new(sep.outputs[0], mix.inputs[2])
        nt.links.new(mix.outputs[0], rough)
