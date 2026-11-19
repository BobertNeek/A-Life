"""Photograph the detail in its unmodified archived parent habitat, not a tabletop."""

from pathlib import Path
import json, sys, runpy, struct, math
import bpy

here = Path(__file__).resolve().parent
native, scene_dir, art, hollow, micro, out = map(
    Path, sys.argv[sys.argv.index("--") + 1 :][:6]
)
mode = sys.argv[-1]
stage_mode = "setup-desk" if mode == "desk" else "setup"
args = sys.argv
sys.argv = [
    "blender",
    "--",
    *[str(f) for f in (native, scene_dir, art, hollow, out)],
    stage_mode,
]
try:
    parent = runpy.run_path(
        str(here.parent / "photograph.py"), run_name="lanternback_photographic_stage"
    )
finally:
    sys.argv = args
sc = parent["sc"]
cameras = dict(parent["cameras"])
# A third stationary eye sees the ground between actual rooted plants. No
# moving or reframing objects; both before and after include identical parents.
raw = (native / "island-v1.bin").read_bytes()
w, d, ox, oz, cs, _ = struct.unpack_from("<IIfffI", raw, 8)


def floor(x, z):
    u, v = (x - ox) / cs, (z - oz) / cs
    i, j = math.floor(u), math.floor(v)
    u -= i
    v -= j

    def at(i, j):
        return struct.unpack_from("<f", raw, 32 + 4 * (j * w + i))[0]

    a, b, c, e = at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)
    return a + (b - a) * u + (c - b) * v if u >= v else a + (c - e) * u + (e - a) * v


x, z = 246.02, -141.34
ax, az = 245.11, -143.45
crop = [
    "pennon-eye",
    [x, floor(x, z) + 1.04, z],
    [ax, floor(ax, az) + 0.050, az],
    0,
    30,
    0,
]
cameras[crop[0]] = parent["cam"](crop[0], crop)


def shot(cam, side):
    (out / side).mkdir(exist_ok=True, parents=True)
    sc.camera = cameras[cam]
    sc.render.filepath = str(out / side / (cam + ".png"))
    bpy.ops.render.render(write_still=True)


if mode in ("all", "draft", "before"):
    # For the two unchanged upper-rim/stone-eye stations use their prior
    # lossless right photographs directly, never a new approx. control.
    shot("pennon-eye", "before")


def bind():
    reader = parent["load_at"](
        here.parents[1] / "leeward-passage/material.py", "worlds"
    )
    ground = parent["ground"]
    reader._bind_layer(
        ground, micro, (), "contact.json", "contact-soil.png", 48.0, 0.0016, 0.24
    )
    for m in parent["materials"](ground):
        nt = m.node_tree
        chart = next(
            (
                n
                for n in nt.nodes
                if n.type == "TEX_IMAGE"
                and n.image
                and n.image.name.startswith("contact-soil")
            ),
            None,
        )
        if not chart:
            continue
        bs = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
        t = nt.nodes.new("ShaderNodeTexImage")
        t.image = bpy.data.images.load(
            str(micro / "soil-tooth.png"), check_existing=True
        )
        t.image.colorspace_settings.name = "Non-Color"
        t.extension = "CLIP"
        nt.links.new(chart.inputs[0].links[0].from_socket, t.inputs[0])
        b = nt.nodes.new("ShaderNodeBump")
        b.inputs["Distance"].default_value = 0.006
        gain = nt.nodes.new("ShaderNodeMath")
        gain.operation = "MULTIPLY"
        gain.inputs[1].default_value = 0.68
        nt.links.new(chart.outputs["Alpha"], gain.inputs[0])
        nt.links.new(gain.outputs[0], b.inputs["Strength"])
        old = bs.inputs["Normal"]
        if old.is_linked:
            nt.links.new(old.links[0].from_socket, b.inputs["Normal"])
        nt.links.new(t.outputs["Color"], b.inputs["Height"])
        nt.links.new(b.outputs[0], old)
    return parent["grab"](micro / "windfall-and-shale.glb") + parent["grab"](
        micro / "windfall-middle.glb"
    )


if mode != "before":
    things = bind()
    cells = json.loads((micro / "cells.json").read_text())
    visible = {}

    def choose(camera):
        # Distance to full cell support (not just its diagonal midpoint).
        o = cameras[camera]
        eye = [o.location.x, o.location.z, -o.location.y]
        ntri = 0
        ngroups = 0
        for item in things:
            cell = cells[item.name]
            # Select against the full-detail bounds for both alternatives,
            # so a few centimetres of middle-mesh reduction cannot double-draw.
            bounds = cells[item.name.replace("Clod_Mid", "Clod_Near")]
            delta = [
                max(0, lo - v, v - hi)
                for lo, v, hi in zip(bounds["min"], eye, bounds["max"])
            ]
            d = sum(c * c for c in delta) ** 0.5
            lod = "Near" if d <= 6.75 else "Mid" if d <= 16 else None
            draw = (
                lod is not None
                and item.name.startswith("Clod_" + lod + "_")
                and not cameras[camera].data.type == "ORTHO"
            )
            item.hide_render = not draw
            if draw:
                ntri += cell["triangles"]
                ngroups += 1
        visible[camera] = {"mesh_groups": ngroups, "triangles": ntri}

    if mode in ("all", "after", "draft"):
        if mode != "draft":
            for name in ("upper-rim", "stone-eye"):
                choose(name)
                shot(name, "after")
        choose("pennon-eye")
        shot("pennon-eye", "after")
    elif mode == "desk":
        sc.camera = cameras["pennon-eye"]
        choose("pennon-eye")
        for item in things:
            item.hide_set(item.hide_render)
        txt = bpy.data.texts.new("Pennon Drift | field notes")
        txt.write(
            (micro / "contact.json").read_text()
            + "\n"
            + json.dumps(crop, indent=2)
            + "\nDistance-selected five-metre cells retain individually modeled slit pennon valves, split sheath edges and shale grains. All rest on their actual V1 triangle. Photography retains full native distant valley; editable includes local true heights, all parent rim art and editable material maps."
        )
        for s in bpy.data.screens:
            for a in s.areas:
                if a.type == "VIEW_3D":
                    v = a.spaces.active
                    v.region_3d.view_rotation = sc.camera.rotation_euler.to_quaternion()
                    v.region_3d.view_location = parent["cv"]([ax, floor(ax, az), az])
                    v.region_3d.view_distance = 3.2
                    v.region_3d.view_perspective = "CAMERA"
                    v.overlay.show_overlays = False
                    v.shading.type = "MATERIAL"
        out.mkdir(exist_ok=True, parents=True)
        bpy.ops.file.pack_all()
        bpy.ops.wm.save_as_mainfile(
            filepath=str(out / "Pennon-Drift.blend"), compress=True
        )
if mode in ("all", "after"):
    (out / "visible-draws.json").write_text(json.dumps(visible, indent=2) + "\n")
(out / "station.json").write_text(
    json.dumps(
        dict(
            ground_camera=crop,
            resolution=[840, 500],
            samples=sc.cycles.samples,
            held="3de1d24c3e3d6f9b848b1ca61cda0da012f5374b",
            method="Cycles offline artist comparison, no game",
        ),
        indent=2,
    )
    + "\n"
)
