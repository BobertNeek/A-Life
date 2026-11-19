"""Apply the slope-dispersed seedfall coat only where measured beneath real crowns."""

import bpy, json


def coat(ground, surf, folder):
    return tint(
        ground,
        surf,
        folder / "forest-shed-gain.png",
        json.loads((folder / "held-copse.json").read_text())["box_xz"],
        "Actual pine bough deposition",
    )


def tint(ground, surf, path, box, title):
    a, b, c, d = box
    img = bpy.data.images.load(str(path.resolve()), check_existing=True)
    img.colorspace_settings.name = "Non-Color"
    for mat in {
        s.material
        for ob in ground
        for s in ob.material_slots
        if s.material and s.material.use_nodes
    }:
        nt = mat.node_tree
        uv = surf.coords(
            nt, (0, 1 / (b - a), -a / (b - a)), (1, 1 / (d - c), d / (d - c))
        )
        t = nt.nodes.new("ShaderNodeTexImage")
        t.label = title
        t.extension = "EXTEND"
        t.image = img
        nt.links.new(uv, t.inputs["Vector"])
        value = nt.nodes.new("ShaderNodeVectorMath")
        value.operation = "SCALE"
        value.inputs["Scale"].default_value = 255 / 128
        nt.links.new(t.outputs["Color"], value.inputs[0])
        old = surf.bsdf(mat).inputs["Base Color"]
        blend = nt.nodes.new("ShaderNodeVectorMath")
        blend.operation = "MULTIPLY"
        if old.is_linked:
            nt.links.new(old.links[0].from_socket, blend.inputs[0])
        else:
            blend.inputs[0].default_value = old.default_value[:3]
        nt.links.new(value.outputs[0], blend.inputs[1])
        nt.links.new(blend.outputs[0], old)
