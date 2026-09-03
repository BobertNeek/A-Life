"""Normalize committed A-Life true 2.5D glTF assets through Blender.

This is an offline art-pipeline tool. It imports each glTF listed in the
true_25d manifest, bakes world transforms into mesh coordinates, anchors the
asset origin at base-center, scales the largest dimension to 1.0 world unit,
optionally applies decimation if an asset exceeds the triangle threshold, and
exports embedded glTF back to the destination.

The generated assets remain presentation-only. No runtime code, cognition
state, actions, weights, or portable IDs are produced by this script.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
import sys
from typing import Any

try:
    import bpy
    from mathutils import Vector
except Exception as exc:  # pragma: no cover - only runs inside Blender.
    raise SystemExit(f"This script must run inside Blender Python: {exc}")


TARGET_MAX_DIMENSION_UNITS = 1.0
DEFAULT_DECIMATION_THRESHOLD_TRIANGLES = 512
ORIGIN_ANCHOR = "base-center"


def parse_args() -> argparse.Namespace:
    if "--" in sys.argv:
        argv = sys.argv[sys.argv.index("--") + 1 :]
    else:
        argv = []
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--root", default=".")
    parser.add_argument("--out-dir", default="")
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--update-manifest", action="store_true")
    parser.add_argument(
        "--decimation-threshold-triangles",
        type=int,
        default=DEFAULT_DECIMATION_THRESHOLD_TRIANGLES,
    )
    parser.add_argument("--receipt", default="")
    return parser.parse_args(argv)


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    for datablocks in (
        bpy.data.meshes,
        bpy.data.materials,
        bpy.data.images,
        bpy.data.textures,
        bpy.data.curves,
        bpy.data.cameras,
        bpy.data.lights,
    ):
        for datablock in list(datablocks):
            if datablock.users == 0:
                datablocks.remove(datablock)


def mesh_objects() -> list[bpy.types.Object]:
    return [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]


def object_world_bbox_vertices(obj: bpy.types.Object) -> list[Vector]:
    return [obj.matrix_world @ Vector(corner) for corner in obj.bound_box]


def scene_bounds(objects: list[bpy.types.Object]) -> tuple[Vector, Vector]:
    points: list[Vector] = []
    for obj in objects:
        points.extend(object_world_bbox_vertices(obj))
    if not points:
        raise ValueError("imported asset has no mesh bounds")
    min_v = Vector((min(p.x for p in points), min(p.y for p in points), min(p.z for p in points)))
    max_v = Vector((max(p.x for p in points), max(p.y for p in points), max(p.z for p in points)))
    return min_v, max_v


def triangle_count(objects: list[bpy.types.Object]) -> int:
    count = 0
    for obj in objects:
        mesh = obj.data
        for poly in mesh.polygons:
            count += max(1, len(poly.vertices) - 2)
    return count


def vertex_count(objects: list[bpy.types.Object]) -> int:
    return sum(len(obj.data.vertices) for obj in objects)


def material_count() -> int:
    return len([mat for mat in bpy.data.materials if mat.users > 0])


def apply_decimation_if_needed(objects: list[bpy.types.Object], threshold: int) -> bool:
    before = triangle_count(objects)
    if before <= threshold:
        return False
    ratio = max(0.05, min(1.0, threshold / float(before)))
    for obj in objects:
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        modifier = obj.modifiers.new("alife_low_poly_decimate", "DECIMATE")
        modifier.ratio = ratio
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    return True


def bake_normalized_geometry(objects: list[bpy.types.Object]) -> dict[str, float]:
    before_min, before_max = scene_bounds(objects)
    before_size = before_max - before_min
    before_max_dimension = max(before_size.x, before_size.y, before_size.z)
    if before_max_dimension <= 0.0:
        raise ValueError("asset has zero-size bounds")

    scale = TARGET_MAX_DIMENSION_UNITS / before_max_dimension
    center_x = (before_min.x + before_max.x) * 0.5
    center_y = (before_min.y + before_max.y) * 0.5
    min_z = before_min.z

    for obj in objects:
        mesh = obj.data
        world_matrix = obj.matrix_world.copy()
        for vertex in mesh.vertices:
            world = world_matrix @ vertex.co
            vertex.co = Vector(
                (
                    (world.x - center_x) * scale,
                    (world.y - center_y) * scale,
                    (world.z - min_z) * scale,
                )
            )
        obj.location = (0.0, 0.0, 0.0)
        obj.rotation_euler = (0.0, 0.0, 0.0)
        obj.scale = (1.0, 1.0, 1.0)
        mesh.update()

    after_min, after_max = scene_bounds(objects)
    after_size = after_max - after_min
    return {
        "before_max_dimension_units": float(before_max_dimension),
        "after_max_dimension_units": float(max(after_size.x, after_size.y, after_size.z)),
        "after_min_z_units": float(after_min.z),
        "after_center_x_units": float((after_min.x + after_max.x) * 0.5),
        "after_center_y_units": float((after_min.y + after_max.y) * 0.5),
    }


def export_binary_glb(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        export_apply=True,
        export_yup=True,
        export_animations=False,
    )


def normalize_asset(
    root: Path,
    entry: dict[str, Any],
    destination: Path,
    decimation_threshold: int,
) -> dict[str, Any]:
    source = root / entry["relative_path"]
    reset_scene()
    bpy.ops.import_scene.gltf(filepath=str(source))
    objects = mesh_objects()
    if not objects:
        raise ValueError(f"{source} imported without mesh objects")

    decimated = apply_decimation_if_needed(objects, decimation_threshold)
    bounds = bake_normalized_geometry(objects)
    export_binary_glb(destination)

    # Re-import the exported file for metrics exactly as Bevy/manifest validation
    # will see them.
    reset_scene()
    bpy.ops.import_scene.gltf(filepath=str(destination))
    exported_objects = mesh_objects()
    exported_min, exported_max = scene_bounds(exported_objects)
    exported_size = exported_max - exported_min
    exported_max_dimension = max(exported_size.x, exported_size.y, exported_size.z)
    triangles = triangle_count(exported_objects)
    vertices = vertex_count(exported_objects)
    materials = material_count()

    entry.update(
        {
            "node_count": len(exported_objects),
            "mesh_count": len(exported_objects),
            "material_count": materials,
            "vertex_count": vertices,
            "index_count": triangles * 3,
            "file_size_bytes": destination.stat().st_size,
            "blender_normalized": True,
            "origin_anchor": ORIGIN_ANCHOR,
            "transform_applied": True,
            "max_dimension_units": round(float(exported_max_dimension), 6),
            "decimation_threshold_triangles": decimation_threshold,
            "triangle_count": triangles,
            "decimation_applied": decimated,
        }
    )
    return {
        "role": entry["role"],
        "path": entry["relative_path"],
        "destination": str(destination),
        "decimation_applied": decimated,
        "triangle_count": triangles,
        "vertex_count": vertices,
        "file_size_bytes": destination.stat().st_size,
        **bounds,
        "exported_max_dimension_units": float(exported_max_dimension),
        "exported_min_z_units": float(exported_min.z),
    }


def main() -> None:
    args = parse_args()
    root = Path(args.root).resolve()
    manifest_path = Path(args.manifest)
    if not manifest_path.is_absolute():
        manifest_path = root / manifest_path
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

    if args.in_place == bool(args.out_dir):
        raise SystemExit("Specify exactly one of --in-place or --out-dir")

    output_dir = Path(args.out_dir).resolve() if args.out_dir else None
    receipts: list[dict[str, Any]] = []
    for entry in manifest["entries"]:
        if args.in_place:
            destination = (root / entry["relative_path"]).with_suffix(".glb")
            backup = destination.with_suffix(destination.suffix + ".bak")
            if destination.exists():
                shutil.copy2(destination, backup)
            try:
                receipt = normalize_asset(root, entry, destination, args.decimation_threshold_triangles)
            finally:
                backup.unlink(missing_ok=True)
        else:
            assert output_dir is not None
            destination = output_dir / Path(entry["relative_path"]).with_suffix(".glb").name
            receipt = normalize_asset(root, entry, destination, args.decimation_threshold_triangles)
        entry["relative_path"] = str(Path(entry["relative_path"]).with_suffix(".glb")).replace("\\", "/")
        receipts.append(receipt)

    if args.update_manifest:
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    if args.receipt:
        receipt_path = Path(args.receipt)
        if not receipt_path.is_absolute():
            receipt_path = root / receipt_path
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        receipt = {
            "schema": "alife.true25d.blender_normalization_receipt.v1",
            "assets": receipts,
            "target_max_dimension_units": TARGET_MAX_DIMENSION_UNITS,
            "origin_anchor": ORIGIN_ANCHOR,
            "decimation_threshold_triangles": args.decimation_threshold_triangles,
            "runtime_dependency": False,
            "can_emit_actions": False,
            "can_rewrite_weights": False,
            "can_change_simulation_semantics": False,
        }
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

    print(
        "Normalized "
        f"{len(receipts)} true 2.5D assets; "
        f"max triangles={max(item['triangle_count'] for item in receipts)}; "
        f"max file bytes={max(item['file_size_bytes'] for item in receipts)}"
    )


if __name__ == "__main__":
    main()
