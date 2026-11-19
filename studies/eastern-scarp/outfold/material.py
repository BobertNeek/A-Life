"""Four independent pigment/roughness exposures and a shallow, non-displacing joint normal."""

import json, bpy


def mix_socket(nt, socket, replacement, factor, name, blend="MIX"):
    f = nt.nodes.new("ShaderNodeMixRGB")
    f.label = name
    f.blend_type = blend
    if socket.is_linked:
        nt.links.new(socket.links[0].from_socket, f.inputs[1])
    else:
        x = socket.default_value
        if isinstance(x, float):
            f.inputs[1].default_value = (x, x, x, 1)
        else:
            f.inputs[1].default_value = x[:]
    nt.links.new(factor, f.inputs[0])
    nt.links.new(replacement, f.inputs[2])
    nt.links.new(f.outputs[0], socket)


def bind(ground, surf, base, assets, panel=None):
    panel = panel or dict(
        meta="outfold.json",
        colour="exposure-colour.png",
        roughness="face-roughness.png",
        height="joint-height.png",
        depth=0.074,
        name="Four disconnected windows",
    )
    metadata = json.loads((assets / panel["meta"]).read_text())
    xmin, xmax, zmin, zmax = metadata["plan"]
    u0, u1, v0, v1 = metadata["side"]
    for m in {
        s.material
        for g in ground
        for s in g.material_slots
        if s.material and s.material.use_nodes
    }:
        nt = m.node_tree
        bs = surf.bsdf(m)
        plan = surf.coords(
            nt,
            (0, 1 / (xmax - xmin), -xmin / (xmax - xmin)),
            (1, 1 / (zmax - zmin), zmax / (zmax - zmin)),
        )
        face = surf.coords(
            nt,
            (1, -1 / (u1 - u0), -u0 / (u1 - u0)),
            (2, 1 / (v1 - v0), -v0 / (v1 - v0)),
        )
        tex = nt.nodes.new("ShaderNodeTexImage")
        tex.label = panel["name"] + " · real elevation"
        tex.image = bpy.data.images.load(
            str((assets / panel["colour"]).resolve()), check_existing=True
        )
        tex.image.alpha_mode = "CHANNEL_PACKED"
        tex.extension = "CLIP"
        nt.links.new(face, tex.inputs["Vector"])
        terrain = surf.image(nt, base / "scarp-mask.png", plan, True)
        a = nt.nodes.new("ShaderNodeMath")
        a.operation = "MULTIPLY"
        nt.links.new(terrain, a.inputs[0])
        nt.links.new(tex.outputs["Alpha"], a.inputs[1])
        mix_socket(
            nt,
            bs.inputs["Base Color"],
            tex.outputs["Color"],
            a.outputs[0],
            "Broken exposed plate, not a full cliff coat",
            panel.get("mix", "MIX"),
        )
        mix_socket(
            nt,
            bs.inputs["Roughness"],
            surf.image(nt, assets / panel["roughness"], face, True),
            a.outputs[0],
            "Exposed rock / local optical roughness",
        )
        bump = nt.nodes.new("ShaderNodeBump")
        bump.label = "Chipped joints · shading only"
        nt.links.new(a.outputs[0], bump.inputs["Strength"])
        bump.inputs["Distance"].default_value = panel["depth"]
        nt.links.new(
            surf.image(nt, assets / panel["height"], face, True),
            bump.inputs["Height"],
        )
        n = bs.inputs["Normal"]
        if n.is_linked:
            nt.links.new(n.links[0].from_socket, bump.inputs["Normal"])
        nt.links.new(bump.outputs["Normal"], n)
