"""Import optional crown forms once; instances keep every original prop transform."""

import json
import bpy


def sources(path, models):
    if path is None:
        return {}
    previous = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path.resolve()))
    for ob in set(bpy.data.objects) - previous:
        ob.hide_render = True
        if ob.type == "MESH":
            key, level = ob.name.rsplit("_L", 1)
            models[key, int(level)] = ob
    return json.loads(path.with_suffix(".json").read_text())


def form(mapping, index, kind, x, z):
    item = mapping.get(str(index))
    if not item:
        return kind
    assert (
        kind == "Conifer"
        and abs(item["xz"][0] - x) < 0.002
        and abs(item["xz"][1] - z) < 0.002
    )
    return f"PineForm{item['form']}"
