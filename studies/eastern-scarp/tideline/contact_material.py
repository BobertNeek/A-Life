"""Clipped side projection on original triangles, shared by the big photo and the editable desk."""

from pathlib import Path
import json, bpy


def bind(ground, surf, cups):
    data = json.loads((cups / "patina.json").read_text())
    u0, u1, v0, v1 = data["side_bounds"]
    for mat in {
        q.material
        for g in ground
        for q in g.material_slots
        if q.material and q.material.use_nodes
    }:
        nt = mat.node_tree
        bs = surf.bsdf(mat)
        uv = surf.coords(
            nt,
            (1, -1 / (u1 - u0), -u0 / (u1 - u0)),
            (2, 1 / (v1 - v0), -v0 / (v1 - v0)),
        )
        tx = nt.nodes.new("ShaderNodeTexImage")
        tx.image = bpy.data.images.load(
            str(cups / "tide-patina.png"), check_existing=True
        )
        tx.image.alpha_mode = "CHANNEL_PACKED"
        tx.extension = "CLIP"
        nt.links.new(uv, tx.inputs["Vector"])
        p = nt.nodes.new("ShaderNodeNewGeometry")
        sep = nt.nodes.new("ShaderNodeSeparateXYZ")
        nt.links.new(p.outputs["Position"], sep.inputs[0])
        lo, hi = data["x_support"]
        ms = []
        for op, lim in [("GREATER_THAN", lo), ("LESS_THAN", hi)]:
            m = nt.nodes.new("ShaderNodeMath")
            m.operation = op
            m.inputs[1].default_value = lim
            nt.links.new(sep.outputs[0], m.inputs[0])
            ms.append(m.outputs[0])
        fac = tx.outputs["Alpha"]
        for gate in ms:
            mul = nt.nodes.new("ShaderNodeMath")
            mul.operation = "MULTIPLY"
            nt.links.new(fac, mul.inputs[0])
            nt.links.new(gate, mul.inputs[1])
            fac = mul.outputs[0]
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.label = "Crumbled tidal shell / local sunken contacts"
        col = bs.inputs["Base Color"]
        if col.is_linked:
            nt.links.new(col.links[0].from_socket, mix.inputs[1])
        else:
            mix.inputs[1].default_value = col.default_value[:]
        nt.links.new(tx.outputs["Color"], mix.inputs[2])
        nt.links.new(fac, mix.inputs[0])
        nt.links.new(mix.outputs[0], col)


def close_camera(sc, cv, eye, target):
    cam = bpy.data.cameras.new("Foothold")
    cam.lens = 33
    cam.clip_end = 3200
    o = bpy.data.objects.new("Foothold", cam)
    sc.collection.objects.link(o)
    o.location = cv(eye)
    o.rotation_euler = (cv(target) - o.location).to_track_quat("-Z", "Y").to_euler()
    return o
