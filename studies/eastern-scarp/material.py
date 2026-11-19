"""Blender-only physical-coordinate materials; no replacement ground or fabricated sea level."""

from pathlib import Path
import bpy, json


def bsdf(mat):
    return next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")


def image(nt, p, vec, linear=False, repeat=False):
    t = nt.nodes.new("ShaderNodeTexImage")
    t.image = bpy.data.images.load(str(p.resolve()), check_existing=True)
    t.extension = "REPEAT" if repeat else "EXTEND"
    if linear:
        t.image.colorspace_settings.name = "Non-Color"
    nt.links.new(vec, t.inputs["Vector"])
    return t.outputs["Color"]


def coords(nt, ax1, ax2):
    geo = nt.nodes.new("ShaderNodeNewGeometry")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    nt.links.new(geo.outputs["Position"], sep.inputs[0])
    combine = nt.nodes.new("ShaderNodeCombineXYZ")
    for j, (axis, gain, offset) in enumerate((ax1, ax2)):
        op = nt.nodes.new("ShaderNodeMath")
        op.operation = "MULTIPLY_ADD"
        op.inputs[1].default_value = gain
        op.inputs[2].default_value = offset
        nt.links.new(sep.outputs[axis], op.inputs[0])
        nt.links.new(op.outputs[0], combine.inputs[j])
    return combine.outputs[0]


def sea(obj, path):
    cfg = json.loads((path / "water.json").read_text())
    ox, oz = cfg["origin_xz"]
    sx, sz = cfg["span_xz"]
    m = cfg["ripple_metres"]
    mat = bpy.data.materials.new("Held stock V1 optical shelf • 58 m ripple")
    mat.use_nodes = True
    obj.data.materials.append(mat)
    nt = mat.node_tree
    bs = bsdf(mat)
    color_uv = coords(nt, (0, 1 / sx, -ox / sx), (1, 1 / sz, 1 + oz / sz))
    nt.links.new(image(nt, path / "shore.png", color_uv), bs.inputs["Base Color"])
    bs.inputs["Roughness"].default_value = cfg["roughness"]
    # Blender and Bevy both use a dielectric BSDF, not necessarily an identical
    # BRDF/tone mapping. Use the native reflectance's default F0 convention.
    f0 = 0.16 * cfg["reflectance"] ** 2
    ior = (1 + f0**0.5) / (1 - f0**0.5)
    bs.inputs["IOR"].default_value = ior
    uv = obj.data.uv_layers.new(name="Native V1 metric ripple")
    for poly in obj.data.polygons:
        for loop in poly.loop_indices:
            xy = (
                obj.matrix_world
                @ obj.data.vertices[obj.data.loops[loop].vertex_index].co
            )
            uv.data[loop].uv = (xy.x / m, xy.y / m)
    tex = nt.nodes.new("ShaderNodeUVMap")
    tex.uv_map = uv.name
    nm = nt.nodes.new("ShaderNodeNormalMap")
    nm.uv_map = uv.name
    nm.inputs["Strength"].default_value = 1
    nt.links.new(
        image(nt, path / "ripple.png", tex.outputs["UV"], True, True),
        nm.inputs["Color"],
    )
    nt.links.new(nm.outputs["Normal"], bs.inputs["Normal"])


def stone(ground, field):
    spec = json.loads((field / "face.json").read_text())
    xmin, xmax, zmin, zmax = spec["plan"]
    u0, u1, v0, v1 = spec["side"]
    mats = {
        s.material
        for o in ground
        for s in o.material_slots
        if s.material and s.material.use_nodes
    }
    for mat in mats:
        nt = mat.node_tree
        bs = bsdf(mat)
        maskuv = coords(
            nt,
            (0, 1 / (xmax - xmin), -xmin / (xmax - xmin)),
            (1, 1 / (zmax - zmin), zmax / (zmax - zmin)),
        )
        sideuv = coords(
            nt,
            (1, -1 / (u1 - u0), -u0 / (u1 - u0)),
            (2, 1 / (v1 - v0), -v0 / (v1 - v0)),
        )
        mask = image(nt, field / "scarp-mask.png", maskuv, True)
        col = image(nt, field / "face-pigment.png", sideuv)
        height = image(nt, field / "face-relief.png", sideuv, True)
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.label = "East-facing native slope, vertical pigment"
        orig = bs.inputs["Base Color"]
        if orig.is_linked:
            nt.links.new(orig.links[0].from_socket, mix.inputs[1])
        else:
            mix.inputs[1].default_value = orig.default_value[:]
        mul = nt.nodes.new("ShaderNodeMath")
        mul.operation = "MULTIPLY"
        mul.inputs[1].default_value = 0.88
        nt.links.new(mask, mul.inputs[0])
        nt.links.new(mul.outputs[0], mix.inputs[0])
        smalluv = coords(nt, (1, -1 / 1.51, 0), (2, 1 / 1.51, 0))
        grain = image(nt, field / "mica-colour.png", smalluv, repeat=True)
        mod = nt.nodes.new("ShaderNodeMixRGB")
        mod.blend_type = "MULTIPLY"
        mod.inputs[0].default_value = 0.26
        nt.links.new(col, mod.inputs[1])
        nt.links.new(grain, mod.inputs[2])
        nt.links.new(mod.outputs[0], mix.inputs[2])
        nt.links.new(mix.outputs[0], orig)
        bump = nt.nodes.new("ShaderNodeBump")
        bump.inputs["Distance"].default_value = 0.034
        mul = nt.nodes.new("ShaderNodeMath")
        mul.operation = "MULTIPLY"
        mul.inputs[1].default_value = 0.71
        nt.links.new(mask, mul.inputs[0])
        nt.links.new(mul.outputs[0], bump.inputs["Strength"])
        nt.links.new(height, bump.inputs["Height"])
        n = bs.inputs["Normal"]
        if n.is_linked:
            nt.links.new(n.links[0].from_socket, bump.inputs["Normal"])
        micro = nt.nodes.new("ShaderNodeBump")
        micro.inputs["Distance"].default_value = 0.008
        amp = nt.nodes.new("ShaderNodeMath")
        amp.operation = "MULTIPLY"
        amp.inputs[1].default_value = 0.56
        nt.links.new(mask, amp.inputs[0])
        nt.links.new(amp.outputs[0], micro.inputs["Strength"])
        nt.links.new(
            image(nt, field / "mica-height.png", smalluv, True, True),
            micro.inputs["Height"],
        )
        nt.links.new(bump.outputs[0], micro.inputs["Normal"])
        nt.links.new(micro.outputs[0], n)
