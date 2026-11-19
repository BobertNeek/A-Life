"""Registered pigment and local ground response on the existing terrain material."""

import bpy, json
from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec


def coat(meshes, field):
    src = Path(__file__).resolve().parents[1] / "leeward-passage" / "material.py"
    sp = spec_from_file_location("held_ground_chart", src)
    old = module_from_spec(sp)
    sp.loader.exec_module(old)
    old._bind_layer(
        meshes, field, (), "site.json", "hollow-ground.png", 26.0, 0.007, 0.27
    )
    for mat in {s.material for o in meshes for s in o.material_slots if s.material}:
        nt = mat.node_tree
        if not nt:
            continue
        tx = next(
            (
                n
                for n in nt.nodes
                if n.type == "TEX_IMAGE"
                and n.image
                and n.image.name.startswith("hollow-ground")
            ),
            None,
        )
        if not tx:
            continue
        bs = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
        rough = bs.inputs["Roughness"]
        mix = nt.nodes.new("ShaderNodeMixRGB")
        mix.label = "Dry mica and fibrous bed"
        if rough.is_linked:
            nt.links.new(rough.links[0].from_socket, mix.inputs[1])
        else:
            mix.inputs[1].default_value = (rough.default_value,) * 3 + (1,)
        # Coarse mineral and shoot litter cross-grain, without any displacement.
        pos = nt.nodes.new("ShaderNodeNewGeometry").outputs["Position"]
        stretch = nt.nodes.new("ShaderNodeVectorMath")
        stretch.operation = "MULTIPLY"
        stretch.inputs[1].default_value = (3.0, 0.83, 1.6)
        nt.links.new(pos, stretch.inputs[0])
        wave = nt.nodes.new("ShaderNodeTexNoise")
        wave.inputs["Scale"].default_value = 7.2
        wave.inputs["Detail"].default_value = 2.4
        wave.inputs["Roughness"].default_value = 0.53
        nt.links.new(stretch.outputs[0], wave.inputs["Vector"])
        ramp = nt.nodes.new("ShaderNodeMapRange")
        ramp.inputs["To Min"].default_value = 0.68
        ramp.inputs["To Max"].default_value = 0.94
        nt.links.new(wave.outputs["Fac"], ramp.inputs["Value"])
        nt.links.new(tx.outputs["Alpha"], mix.inputs[0])
        nt.links.new(ramp.outputs[0], mix.inputs[2])
        nt.links.new(mix.outputs[0], rough)
        # Fine debris has its own world scale, independent from the 82 m
        # macro chart. Use a restrained colour multiply and actual normal
        # relief; the existing mineral soil remains visible in open gaps.
        texcoord = nt.nodes.new("ShaderNodeVectorMath")
        texcoord.operation = "MULTIPLY"
        texcoord.inputs[1].default_value = (1 / 1.78, 1 / 1.78, 0.0)
        nt.links.new(pos, texcoord.inputs[0])
        bark = nt.nodes.new("ShaderNodeTexImage")
        bark.image = bpy.data.images.load(
            str((field / "sheath-loam.png").resolve()), check_existing=True
        )
        nt.links.new(texcoord.outputs[0], bark.inputs["Vector"])
        mapht = nt.nodes.new("ShaderNodeTexImage")
        mapht.image = bpy.data.images.load(
            str((field / "sheath-tooth.png").resolve()), check_existing=True
        )
        mapht.image.colorspace_settings.name = "Non-Color"
        nt.links.new(texcoord.outputs[0], mapht.inputs["Vector"])
        bc = bs.inputs["Base Color"]
        dust = nt.nodes.new("ShaderNodeMixRGB")
        dust.blend_type = "MULTIPLY"
        dust.label = "Broken awns, mica and seedbed"
        nt.links.new(bc.links[0].from_socket, dust.inputs[1])
        nt.links.new(bark.outputs["Color"], dust.inputs[2])
        control = nt.nodes.new("ShaderNodeMath")
        control.operation = "MULTIPLY"
        control.inputs[1].default_value = 0.49
        nt.links.new(tx.outputs["Alpha"], control.inputs[0])
        nt.links.new(control.outputs[0], dust.inputs[0])
        nt.links.new(dust.outputs[0], bc)
        ht = bs.inputs["Normal"]
        b = nt.nodes.new("ShaderNodeBump")
        b.inputs["Distance"].default_value = 0.004
        b.inputs["Strength"].default_value = 0.23
        nt.links.new(mapht.outputs["Color"], b.inputs["Height"])
        if ht.is_linked:
            nt.links.new(ht.links[0].from_socket, b.inputs["Normal"])
        # Critical: mask the bump as well as the pigment outside the art site.
        k = nt.nodes.new("ShaderNodeMath")
        k.operation = "MULTIPLY"
        k.inputs[1].default_value = 0.23
        nt.links.new(tx.outputs["Alpha"], k.inputs[0])
        nt.links.new(k.outputs[0], b.inputs["Strength"])
        nt.links.new(b.outputs[0], ht)


def plant(path):
    bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
