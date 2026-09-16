"""Render A-Life alpha art roles from simple Blender low-poly/toon sources.

Run through scripts/render_alpha_art_blender_sprites.ps1. The output directory
is under target/ by default and is intentionally not committed until a human
promotes selected renders into the versioned alpha art pack.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

try:
    import bpy
    from mathutils import Vector
except Exception as exc:  # pragma: no cover - only runs inside Blender.
    raise SystemExit(f"This script must run inside Blender Python: {exc}")


def parse_args() -> argparse.Namespace:
    if "--" in sys.argv:
        argv = sys.argv[sys.argv.index("--") + 1 :]
    else:
        argv = []
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--out-dir", required=True)
    return parser.parse_args(argv)


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    engine_items = bpy.context.scene.render.bl_rna.properties["engine"].enum_items
    available_engines = {item.identifier for item in engine_items}
    if "BLENDER_EEVEE_NEXT" in available_engines:
        bpy.context.scene.render.engine = "BLENDER_EEVEE_NEXT"
    elif "BLENDER_EEVEE" in available_engines:
        bpy.context.scene.render.engine = "BLENDER_EEVEE"
    else:
        bpy.context.scene.render.engine = "CYCLES"
    bpy.context.scene.render.resolution_x = 192
    bpy.context.scene.render.resolution_y = 192
    bpy.context.scene.render.film_transparent = True
    if hasattr(bpy.context.scene, "eevee"):
        bpy.context.scene.eevee.taa_render_samples = 32


def material(name: str, color: tuple[float, float, float, float]) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        bsdf.inputs["Base Color"].default_value = color
        bsdf.inputs["Roughness"].default_value = 0.88
    return mat


def add_camera() -> None:
    bpy.ops.object.light_add(type="AREA", location=(0, -4, 6))
    bpy.context.object.name = "softbox"
    bpy.context.object.data.energy = 420
    bpy.context.object.data.size = 5
    bpy.ops.object.camera_add(location=(0, -6, 5.2), rotation=(1.047, 0, 0))
    cam = bpy.context.object
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = 4.2
    bpy.context.scene.camera = cam


def cube(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat) -> None:
    bpy.ops.mesh.primitive_cube_add(size=1, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(mat)


def sphere(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat) -> None:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=12, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(mat)


def cone(name: str, loc: tuple[float, float, float], radius: float, depth: float, mat) -> None:
    bpy.ops.mesh.primitive_cone_add(vertices=5, radius1=radius, radius2=0.0, depth=depth, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(mat)


def build_role(role: str) -> None:
    blue = material("creature_blue", (0.20, 0.82, 0.95, 1.0))
    teal = material("teal_shadow", (0.05, 0.35, 0.40, 1.0))
    red = material("hazard_red", (1.0, 0.12, 0.10, 1.0))
    green = material("leaf_green", (0.20, 0.88, 0.28, 1.0))
    brown = material("earth_brown", (0.45, 0.27, 0.12, 1.0))
    gray = material("stone_gray", (0.48, 0.52, 0.50, 1.0))
    yellow = material("sand_gold", (0.87, 0.68, 0.28, 1.0))
    water = material("water_teal", (0.13, 0.58, 0.78, 0.86))
    white = material("eye_white", (0.96, 1.0, 0.96, 1.0))
    black = material("eye_black", (0.02, 0.05, 0.05, 1.0))

    if role.startswith("terrain-"):
        color = {
            "terrain-safe-grass": green,
            "terrain-soil-path": brown,
            "terrain-resource-grove": green,
            "terrain-hazard-pressure": red,
            "terrain-stone-rough": gray,
            "terrain-water": water,
            "terrain-sand": yellow,
        }[role]
        cube(role, (0, 0, 0), (1.7, 1.7, 0.04), color)
        for i in range(7):
            x = -1.1 + (i % 4) * 0.7
            y = -0.9 + (i // 4) * 0.9
            sphere(f"{role}_detail_{i}", (x, y, 0.12), (0.10, 0.10, 0.06), color)
        return

    if role in {"creature-idle", "creature-hurt"}:
        sphere("body", (0, 0, 0.55), (0.95, 0.72, 0.45), blue)
        sphere("head", (0.05, -0.48, 1.0), (0.72, 0.45, 0.36), blue)
        sphere("eye_l", (-0.28, -0.83, 1.1), (0.12, 0.08, 0.12), white)
        sphere("eye_r", (0.38, -0.83, 1.1), (0.12, 0.08, 0.12), white)
        sphere("pupil_l", (-0.28, -0.9, 1.1), (0.04, 0.03, 0.04), black)
        sphere("pupil_r", (0.38, -0.9, 1.1), (0.04, 0.03, 0.04), black)
        cone("antenna_l", (-0.42, -0.34, 1.45), 0.04, 0.7, teal)
        cone("antenna_r", (0.48, -0.34, 1.45), 0.04, 0.7, teal)
        if role == "creature-hurt":
            cone("pain_spike", (0.72, -0.2, 1.2), 0.14, 0.5, red)
        return

    if role == "selection-ring":
        for i in range(16):
            x = 1.25 * __import__("math").cos(i * 0.3927)
            y = 1.25 * __import__("math").sin(i * 0.3927)
            sphere(f"ring_{i}", (x, y, 0.08), (0.06, 0.06, 0.03), yellow)
        return

    if role == "food":
        cone("stem", (0, 0, 0.45), 0.10, 0.8, green)
        sphere("leaf_l", (-0.25, -0.08, 0.82), (0.25, 0.12, 0.08), green)
        sphere("leaf_r", (0.25, 0.08, 0.82), (0.25, 0.12, 0.08), green)
        sphere("berry", (0.04, -0.05, 1.08), (0.14, 0.14, 0.14), red)
        return

    if role == "hazard":
        for i, x in enumerate((-0.42, 0.0, 0.42)):
            cone(f"crystal_{i}", (x, 0, 0.65 + i * 0.1), 0.22, 1.2 + i * 0.2, red)
        return

    if role == "rock-obstacle":
        for i, x in enumerate((-0.42, 0.05, 0.48)):
            sphere(f"rock_{i}", (x, 0.05 * i, 0.35), (0.42, 0.35, 0.28), gray)
        return

    if role == "prop-dressing":
        cone("grass_a", (-0.2, 0, 0.35), 0.07, 0.7, green)
        cone("grass_b", (0.1, 0, 0.42), 0.07, 0.85, green)
        sphere("pebble", (0.45, 0.1, 0.12), (0.16, 0.12, 0.08), gray)
        return

    raise ValueError(f"unsupported role {role}")


def render_role(role: str, output: Path) -> None:
    reset_scene()
    add_camera()
    build_role(role)
    output.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.scene.render.filepath = str(output)
    bpy.ops.render.render(write_still=True)


def main() -> None:
    args = parse_args()
    manifest = json.loads(Path(args.manifest).read_text(encoding="utf-8"))
    out_dir = Path(args.out_dir)
    for entry in manifest["roles"]:
        role = entry["role"]
        target = out_dir / Path(entry["blender_target_png_path"]).name
        render_role(role, target)
    print(f"Rendered {len(manifest['roles'])} A-Life alpha art roles to {out_dir}")


if __name__ == "__main__":
    main()
