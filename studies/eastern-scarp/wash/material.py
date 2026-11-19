"""Experimental small optical pockets on the original flat water, no mesh or ocean-wide rewrite."""

import bpy, json


def bind(obj, surf, field):
    cfg = json.loads((field / "wash.json").read_text())
    x0, x1, z0, z1 = cfg["plan"]
    mat = obj.data.materials[0]
    nt = mat.node_tree
    bs = surf.bsdf(mat)
    p = surf.coords(
        nt, (0, 1 / (x1 - x0), -x0 / (x1 - x0)), (1, 1 / (z1 - z0), z1 / (z1 - z0))
    )

    def image(name, raw=False):
        t = nt.nodes.new("ShaderNodeTexImage")
        t.image = bpy.data.images.load(
            str((field / name).resolve()), check_existing=True
        )
        t.extension = "CLIP"
        t.image.alpha_mode = "CHANNEL_PACKED"
        if raw:
            t.image.colorspace_settings.name = "Non-Color"
        nt.links.new(p, t.inputs["Vector"])
        return t

    wash = image("wash-colour.png")
    mix = nt.nodes.new("ShaderNodeMixRGB")
    mix.label = "Three submerged wash pockets • original shore"
    col = bs.inputs["Base Color"]
    nt.links.new(col.links[0].from_socket, mix.inputs[1])
    nt.links.new(wash.outputs["Color"], mix.inputs[2])
    nt.links.new(wash.outputs["Alpha"], mix.inputs[0])
    nt.links.new(mix.outputs[0], col)
    respons = image("wash-response.png", True)
    channels = nt.nodes.new("ShaderNodeSeparateColor")
    nt.links.new(respons.outputs["Color"], channels.inputs[0])
    nm = next(n for n in nt.nodes if n.type == "NORMAL_MAP")

    # Outside these referenced pockets both original quantities are exactly
    # preserved. Native optical colour and 58m normal images stay in place.
    def scaled(value, amount, origin):
        op = nt.nodes.new("ShaderNodeMath")
        op.operation = "MULTIPLY_ADD"
        op.inputs[1].default_value = amount
        op.inputs[2].default_value = origin
        nt.links.new(value, op.inputs[0])
        return op.outputs[0]

    old = nm.inputs["Strength"].default_value
    nt.links.new(
        scaled(channels.outputs["Red"], -0.32 * old, old), nm.inputs["Strength"]
    )
    rough = bs.inputs["Roughness"].default_value
    nt.links.new(scaled(channels.outputs["Green"], 0.31, rough), bs.inputs["Roughness"])
