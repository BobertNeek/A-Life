"""Registered optical wind-duff over the same real Hollow and inland shore terrain."""

import json, bpy


def paint(land, surf, data):
    s = json.loads((data / "wind-duff.json").read_text())
    a, b, c, d = s["plan"]
    for mat in {
        sl.material
        for ob in land
        for sl in ob.material_slots
        if sl.material and sl.material.use_nodes
    }:
        nt = mat.node_tree
        bs = surf.bsdf(mat)
        uv = surf.coords(
            nt, (0, 1 / (b - a), -a / (b - a)), (1, 1 / (d - c), d / (d - c))
        )
        t = nt.nodes.new("ShaderNodeTexImage")
        t.label = "Registered exposed and sheltered turf"
        t.image = bpy.data.images.load(
            str((data / "wind-duff.png").resolve()), check_existing=True
        )
        t.image.alpha_mode = "CHANNEL_PACKED"
        t.extension = "CLIP"
        nt.links.new(uv, t.inputs["Vector"])
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.label = "Leave unbroken sward between wind shadows"
        col = bs.inputs["Base Color"]
        nt.links.new(col.links[0].from_socket, mix.inputs[1])
        nt.links.new(t.outputs["Color"], mix.inputs[2])
        nt.links.new(t.outputs["Alpha"], mix.inputs[0])
        nt.links.new(mix.outputs[0], col)
        bump = nt.nodes.new("ShaderNodeBump")
        bump.inputs["Distance"].default_value = 0.014
        nt.links.new(t.outputs["Alpha"], bump.inputs["Strength"])
        nt.links.new(
            surf.image(nt, data / "wind-duff-height.png", uv, True),
            bump.inputs["Height"],
        )
        n = bs.inputs["Normal"]
        if n.is_linked:
            nt.links.new(n.links[0].from_socket, bump.inputs["Normal"])
        nt.links.new(bump.outputs["Normal"], n)
