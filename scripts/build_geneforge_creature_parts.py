#!/usr/bin/env python3
"""Deterministic Blender 5.1 importer for staged GeneForge creature parts."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib


WORKSPACE = Path(__file__).resolve().parents[1]
ARTIFACT_ROOT = (WORKSPACE / "target/artifacts").resolve()
DEFAULT_BLENDER = Path(r"C:\Program Files\Blender Foundation\Blender 5.1\blender.exe")
REQUIRED_BLENDER_VERSION = "5.1.0"
LODS = (("full", 1, 1200), ("compact", 2, 800), ("impostor", 4, 400))
GROUP_COLORS = {
    "head": (230, 92, 88, 255),
    "torso": (64, 166, 184, 255),
    "left-arm": (244, 177, 76, 255),
    "right-arm": (244, 177, 76, 255),
    "left-leg": (95, 177, 104, 255),
    "right-leg": (95, 177, 104, 255),
    "tail": (154, 108, 180, 255),
}


class ImportFailure(RuntimeError):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_recipe_digest(recipe: dict) -> str:
    canonical = dict(recipe)
    canonical["recipe_sha256"] = "0" * 64
    payload = json.dumps(
        canonical, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(payload).hexdigest()


def load_recipe(path: Path) -> dict:
    recipe = json.loads(path.read_text(encoding="utf-8"))
    actual = canonical_recipe_digest(recipe)
    if recipe.get("recipe_sha256", "").lower() != actual:
        raise ImportFailure(
            f"recipe digest mismatch: expected {recipe.get('recipe_sha256')}, calculated {actual}"
        )
    if recipe.get("blender_version") != REQUIRED_BLENDER_VERSION:
        raise ImportFailure("recipe does not pin Blender 5.1.0")
    return recipe


def ensure_artifact_path(path: Path, label: str) -> Path:
    resolved = path.resolve()
    try:
        resolved.relative_to(ARTIFACT_ROOT)
    except ValueError as error:
        raise ImportFailure(f"{label} must stay under {ARTIFACT_ROOT}") from error
    return resolved


def discover_blender(explicit: Path | None) -> Path:
    candidates = []
    if explicit is not None:
        candidates.append(explicit)
    if os.environ.get("BLENDER_EXE"):
        candidates.append(Path(os.environ["BLENDER_EXE"]))
    on_path = shutil.which("blender")
    if on_path:
        candidates.append(Path(on_path))
    candidates.append(DEFAULT_BLENDER)
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    raise ImportFailure(
        "Blender 5.1.0 not found; set BLENDER_EXE or install " + str(DEFAULT_BLENDER)
    )


def probe_blender_version(blender: Path) -> None:
    if blender.suffix.lower() in {".cmd", ".bat"}:
        command = [os.environ.get("COMSPEC", "cmd.exe"), "/c", str(blender), "--version"]
    else:
        command = [str(blender), "--version"]
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    output = completed.stdout + "\n" + completed.stderr
    match = re.search(r"Blender\s+(\d+\.\d+\.\d+)", output)
    if not match:
        raise ImportFailure(f"could not read Blender version from {blender}")
    found = match.group(1)
    if found != REQUIRED_BLENDER_VERSION:
        raise ImportFailure(
            f"GeneForge importer requires Blender {REQUIRED_BLENDER_VERSION}; found {found}"
        )


def validate_source_files(source_root: Path, recipe: dict) -> None:
    for source in recipe["sources"]:
        path = source_root / Path(source["blend_file"])
        if not path.is_file():
            raise ImportFailure(f"missing {source['donor']} source file: {path}")
        actual = sha256_file(path).upper()
        if actual != source["sha256"].upper():
            raise ImportFailure(
                f"{source['donor']} source digest mismatch: expected {source['sha256']}, found {actual}"
            )
        texture_root = source_root / Path(source["texture_root"])
        if not texture_root.is_dir():
            raise ImportFailure(f"missing {source['donor']} texture root: {texture_root}")


def worker_command(
    blender: Path,
    action: str,
    source: dict,
    source_root: Path,
    recipe_path: Path,
    output_json: Path,
    staging: Path | None = None,
) -> list[str]:
    command = [
        str(blender),
        "--background",
        "--python-exit-code",
        "23",
        "--python",
        str(Path(__file__).resolve()),
        "--",
        "--worker",
        action,
        "--donor",
        source["donor"],
        "--source",
        str(source_root / Path(source["blend_file"])),
        "--texture-root",
        str(source_root / Path(source["texture_root"])),
        "--recipes",
        str(recipe_path),
        "--output-json",
        str(output_json),
    ]
    if staging is not None:
        command.extend(("--staging", str(staging)))
    return command


def run_worker(command: list[str]) -> None:
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    if completed.returncode:
        combined = completed.stdout + "\n" + completed.stderr
        marker = re.search(r"ALIFE_IMPORT_ERROR:([^\r\n]+)", combined)
        if marker:
            raise ImportFailure(marker.group(1).strip())
        raise ImportFailure(
            f"Blender worker failed with exit {completed.returncode}:\n{combined[-4000:]}"
        )


def run_inventory_or_validation(args, recipe: dict, blender: Path) -> list[dict]:
    results = []
    with tempfile.TemporaryDirectory(prefix="geneforge-inspect-", dir=ARTIFACT_ROOT) as temp:
        temp_root = Path(temp)
        for source in recipe["sources"]:
            output = temp_root / f"{source['donor']}.json"
            run_worker(
                worker_command(
                    blender,
                    "inspect",
                    source,
                    args.source_root,
                    args.recipes,
                    output,
                )
            )
            results.append(json.loads(output.read_text(encoding="utf-8")))
    return results


def command_inventory(args, recipe: dict, blender: Path) -> None:
    results = run_inventory_or_validation(args, recipe, blender)
    payload = {
        "schema": "alife.geneforge_source_inventory.v1",
        "blender_version": REQUIRED_BLENDER_VERSION,
        "sources": results,
    }
    output = ensure_artifact_path(args.output, "inventory output")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"inventory={output}")


def command_validate(args, recipe: dict, blender: Path) -> None:
    results = run_inventory_or_validation(args, recipe, blender)
    print(f"validated_sources={len(results)}")
    print(f"relinked_images={sum(item['relinked_images'] for item in results)}")
    print("marker_ids=1..14")


def command_build(args, recipe: dict, blender: Path) -> None:
    staging = ensure_artifact_path(args.staging, "staging output")
    staging.parent.mkdir(parents=True, exist_ok=True)
    temporary = staging.with_name(staging.name + f".tmp-{os.getpid()}")
    if temporary.exists():
        shutil.rmtree(temporary)
    temporary.mkdir(parents=True)
    receipts = []
    try:
        for source in recipe["sources"]:
            output = temporary / f".{source['donor']}-worker.json"
            run_worker(
                worker_command(
                    blender,
                    "build",
                    source,
                    args.source_root,
                    args.recipes,
                    output,
                    temporary,
                )
            )
            receipts.append(json.loads(output.read_text(encoding="utf-8")))
            output.unlink()
        outputs = {
            path.relative_to(temporary).as_posix(): sha256_file(path)
            for path in sorted(temporary.rglob("*"))
            if path.is_file()
        }
        receipt = {
            "schema": "alife.geneforge_build_receipt.v1",
            "blender_version": REQUIRED_BLENDER_VERSION,
            "recipe_sha256": recipe["recipe_sha256"],
            "donor_count": len(receipts),
            "lods": [name for name, _, _ in LODS],
            "sources": receipts,
            "topology": {
                key: sum(item["topology"][key] for item in receipts)
                for key in (
                    "removed_degenerate_faces",
                    "removed_duplicate_vertices",
                    "removed_loose_vertices",
                    "repaired_declared_non_manifold",
                )
            },
            "outputs": outputs,
        }
        (temporary / "build_receipt.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        if staging.exists():
            shutil.rmtree(staging)
        temporary.replace(staging)
    except BaseException:
        if temporary.exists():
            shutil.rmtree(temporary)
        raise
    print(f"staging={staging}")
    print(f"outputs={len(receipt['outputs'])}")


def png_bytes(width: int, height: int, pixels: bytes) -> bytes:
    signature = b"\x89PNG\r\n\x1a\n"

    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    rows = b"".join(b"\0" + pixels[y * width * 4 : (y + 1) * width * 4] for y in range(height))
    return signature + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def draw_line(pixels: bytearray, width: int, height: int, start, end, color) -> None:
    x0, y0 = start
    x1, y1 = end
    steps = max(abs(x1 - x0), abs(y1 - y0), 1)
    for step in range(steps + 1):
        x = round(x0 + (x1 - x0) * step / steps)
        y = round(y0 + (y1 - y0) * step / steps)
        if 0 <= x < width and 0 <= y < height:
            offset = (y * width + x) * 4
            pixels[offset : offset + 4] = bytes(color)


def render_obj_preview(obj_path: Path, output: Path) -> None:
    vertices = []
    triangles = []
    group = "torso"
    for line in obj_path.read_text(encoding="ascii").splitlines():
        if line.startswith("v "):
            vertices.append(tuple(float(value) for value in line.split()[1:4]))
        elif line.startswith("g "):
            group = line[2:].split(".", 1)[0]
        elif line.startswith("f "):
            refs = [int(field.split("/", 1)[0]) - 1 for field in line.split()[1:4]]
            triangles.append((group, refs))
    if not vertices or not triangles:
        raise ImportFailure(f"cannot preview empty OBJ {obj_path}")
    width = height = 256
    pixels = bytearray(bytes((38, 42, 46, 255)) * width * height)
    xs = [vertex[0] for vertex in vertices]
    ys = [vertex[1] for vertex in vertices]
    span = max(max(xs) - min(xs), max(ys) - min(ys), 1.0e-6)
    scale = 210.0 / span
    cx = (max(xs) + min(xs)) * 0.5
    cy = (max(ys) + min(ys)) * 0.5
    projected = [
        (round(128 + (x - cx) * scale), round(128 - (y - cy) * scale))
        for x, y, _ in vertices
    ]
    for semantic, indices in triangles:
        color = GROUP_COLORS.get(semantic, (210, 210, 215, 255))
        for first, second in ((0, 1), (1, 2), (2, 0)):
            draw_line(pixels, width, height, projected[indices[first]], projected[indices[second]], color)
    output.write_bytes(png_bytes(width, height, bytes(pixels)))


def command_preview(args, recipe: dict) -> None:
    staging = ensure_artifact_path(args.staging, "preview staging input")
    output = ensure_artifact_path(args.output, "preview output")
    output.mkdir(parents=True, exist_ok=True)
    count = 0
    for donor in (source["donor"] for source in recipe["sources"]):
        for lod, _, _ in LODS:
            obj = staging / f"production_voxel_v1/creature_parts/generated/geneforge/{donor}_{lod}_parts.obj"
            if not obj.is_file():
                raise ImportFailure(f"missing staged OBJ for preview: {obj}")
            render_obj_preview(obj, output / f"{donor}_{lod}.png")
            count += 1
    print(f"previews={count}")
    print(f"preview_root={output}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("inventory", "validate-sources", "build", "preview"):
        child = subparsers.add_parser(command)
        child.add_argument("--source-root", type=Path, required=True)
        child.add_argument("--recipes", type=Path, required=True)
        child.add_argument("--blender-exe", type=Path)
        if command == "inventory":
            child.add_argument("--output", type=Path, required=True)
        elif command == "build":
            child.add_argument("--staging", type=Path, required=True)
        elif command == "preview":
            child.add_argument("--staging", type=Path, required=True)
            child.add_argument("--output", type=Path, required=True)
    return parser


def outer_main() -> None:
    args = build_parser().parse_args()
    args.source_root = args.source_root.resolve()
    args.recipes = args.recipes.resolve()
    recipe = load_recipe(args.recipes)
    blender = discover_blender(args.blender_exe)
    probe_blender_version(blender)
    validate_source_files(args.source_root, recipe)
    if args.command == "inventory":
        command_inventory(args, recipe, blender)
    elif args.command == "validate-sources":
        command_validate(args, recipe, blender)
    elif args.command == "build":
        command_build(args, recipe, blender)
    else:
        command_preview(args, recipe)


def worker_args() -> argparse.Namespace:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true", required=True)
    parser.add_argument("action", choices=("inspect", "build"))
    parser.add_argument("--donor", required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--texture-root", type=Path, required=True)
    parser.add_argument("--recipes", type=Path, required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--staging", type=Path)
    return parser.parse_args(argv)


def relink_images(bpy, texture_root: Path, object_names: set[str], donor: str) -> int:
    by_basename = {}
    for path in sorted(texture_root.rglob("*")):
        if path.is_file():
            by_basename.setdefault(path.name.casefold(), path)
    required_images = {}
    for object_name in sorted(object_names):
        obj = bpy.data.objects.get(object_name)
        if obj is None:
            continue
        for material_slot in obj.material_slots:
            material = material_slot.material
            if material is None or not material.use_nodes or material.node_tree is None:
                continue
            for node in material.node_tree.nodes:
                image = getattr(node, "image", None)
                if image is not None:
                    required_images[image.name] = image
    relinked = 0
    for image in required_images.values():
        if image.source not in {"FILE", "SEQUENCE", "MOVIE"} or image.packed_file:
            continue
        basename = Path(image.filepath).name or image.name
        names = [basename, image.name]
        for name in tuple(names):
            stripped = re.sub(r"\.\d{3}$", "", name)
            names.append(stripped)
            if not Path(stripped).suffix:
                names.extend(stripped + extension for extension in (".png", ".jpg", ".jpeg"))
                underscored = stripped.replace(" ", "_")
                names.extend(
                    underscored + extension for extension in (".png", ".jpg", ".jpeg")
                )
        candidate = next(
            (by_basename[name.casefold()] for name in names if name.casefold() in by_basename),
            None,
        )
        if candidate is None:
            raise ImportFailure(f"{donor} missing texture basename {basename}")
        if Path(bpy.path.abspath(image.filepath)).resolve() != candidate.resolve():
            image.filepath = str(candidate)
            relinked += 1
    return relinked


def selected_assets(recipe: dict, donor: str) -> list[dict]:
    return [asset for asset in recipe["part_assets"] if asset["donor"] == donor]


def marker_positions(bpy, recipe: dict, donor: str) -> dict[int, tuple[float, float, float]]:
    candidates = {}
    for obj in sorted(bpy.data.objects, key=lambda item: item.name):
        if obj.type != "EMPTY" or "kc3dsbpy_part_marker" not in obj:
            continue
        marker_id = int(obj["kc3dsbpy_part_marker"])
        candidates.setdefault(marker_id, tuple(float(value) for value in obj.matrix_world.translation))
    required = {
        marker_id
        for asset in selected_assets(recipe, donor)
        for marker_id in asset["selector"]["marker_ids"]
    }
    invalid = set(candidates) - set(range(1, 15))
    missing = required - set(candidates)
    if invalid or missing:
        raise ImportFailure(
            f"marker IDs must be exactly 1..14 with all selected donor markers present; found {sorted(candidates)}"
        )
    return candidates


def evaluated_mesh(obj, depsgraph, fallback: str | None, donor: str):
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh(preserve_all_data_layers=True, depsgraph=depsgraph)
    if mesh and len(mesh.vertices) and len(mesh.polygons):
        return evaluated, mesh, False
    if mesh:
        evaluated.to_mesh_clear()
    if fallback == "validated-raw-mesh" and len(obj.data.vertices) and len(obj.data.polygons):
        return None, obj.data, True
    raise ImportFailure(f"{donor} evaluated geometry is empty for object {obj.name}")


def inspect_scene(bpy, recipe: dict, donor: str, texture_root: Path) -> tuple[dict, dict]:
    assets = selected_assets(recipe, donor)
    required_objects = {
        name for asset in assets for name in asset["selector"]["include_objects"]
    }
    relinked = relink_images(bpy, texture_root, required_objects, donor)
    markers = marker_positions(bpy, recipe, donor)
    depsgraph = bpy.context.evaluated_depsgraph_get()
    selected = []
    used_raw = []
    for asset in assets:
        selector = asset["selector"]
        for name in selector["include_objects"]:
            obj = bpy.data.objects.get(name)
            if obj is None or obj.type != "MESH":
                raise ImportFailure(f"{donor} selector {asset['id']} missing exact mesh {name}")
            owner, mesh, raw = evaluated_mesh(
                obj, depsgraph, selector.get("evaluated_empty_policy", {}).get(name), donor
            )
            if raw:
                used_raw.append(name)
            if owner is not None:
                owner.to_mesh_clear()
            selected.append(name)
    inventory = {
        "donor": donor,
        "mesh_objects": sum(1 for obj in bpy.data.objects if obj.type == "MESH"),
        "selected_objects": sorted(set(selected)),
        "marker_ids": sorted(markers),
        "has_constraint": any(obj.constraints for obj in bpy.data.objects),
        "has_geometry_nodes": any(
            modifier.type == "NODES" for obj in bpy.data.objects for modifier in obj.modifiers
        ),
        "has_armature": any(
            obj.type == "ARMATURE" or any(modifier.type == "ARMATURE" for modifier in obj.modifiers)
            for obj in bpy.data.objects
        ),
        "has_declared_non_manifold": any(
            "alife_declared_topology_repair" in obj for obj in bpy.data.objects
        ),
        "primary_uv": assets[0]["selector"]["uv_map"],
        "relinked_images": relinked,
        "validated_raw_fallbacks": sorted(used_raw),
    }
    return inventory, markers


def semantic_group(asset: dict, object_name: str) -> str:
    for role, names in asset.get("detail_groups", {}).items():
        if object_name in names:
            return f"head.{role}"
    slot = asset["logical_slot"]
    lower = object_name.casefold()
    if slot == "arms":
        return "left-arm" if " l" in lower or "_l" in lower else "right-arm"
    if slot == "legs":
        return "left-leg" if " l" in lower or "_l" in lower or "2l" in lower else "right-leg"
    return slot


def semantic_detail_uv(group: str, corner: int) -> tuple[float, float]:
    role = group.split(".", 1)[1]
    center = {
        "eyes": (0.1, 0.1),
        "lids": (0.3, 0.1),
        "hair": (0.5, 0.1),
        "teeth": (0.7, 0.1),
        "tongue": (0.9, 0.1),
    }[role]
    offsets = ((-0.025, -0.025), (0.025, -0.025), (0.0, 0.025))
    return (center[0] + offsets[corner][0], center[1] + offsets[corner][1])


def transform_point(matrix, coordinate) -> tuple[float, float, float]:
    world = matrix @ coordinate
    return (float(world.x), float(world.z), -float(world.y))


def triangle_normal(points) -> tuple[float, float, float] | None:
    a = tuple(points[1][axis] - points[0][axis] for axis in range(3))
    b = tuple(points[2][axis] - points[0][axis] for axis in range(3))
    cross = (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )
    length = math.sqrt(sum(value * value for value in cross))
    if not math.isfinite(length) or length <= 1.0e-12:
        return None
    return tuple(value / length for value in cross)


def extract_geometry(bpy, recipe: dict, donor: str) -> tuple[dict, dict]:
    depsgraph = bpy.context.evaluated_depsgraph_get()
    grouped = {}
    topology = {
        "removed_degenerate_faces": 0,
        "removed_duplicate_vertices": 0,
        "removed_loose_vertices": 0,
        "repaired_declared_non_manifold": 0,
    }
    seen_objects = set()
    for asset in selected_assets(recipe, donor):
        selector = asset["selector"]
        repairs = selector.get("topology_repairs", {})
        for name in selector["include_objects"]:
            if name in seen_objects:
                continue
            seen_objects.add(name)
            obj = bpy.data.objects[name]
            owner, mesh, _ = evaluated_mesh(
                obj, depsgraph, selector.get("evaluated_empty_policy", {}).get(name), donor
            )
            mesh.calc_loop_triangles()
            used_vertices = {loop.vertex_index for polygon in mesh.polygons for loop in mesh.loops[polygon.loop_start : polygon.loop_start + polygon.loop_total]}
            topology["removed_loose_vertices"] += len(mesh.vertices) - len(used_vertices)
            if name in repairs:
                topology["repaired_declared_non_manifold"] += 1
            uv_layer = mesh.uv_layers.get(selector["uv_map"])
            if uv_layer is None and mesh.uv_layers:
                uv_layer = mesh.uv_layers.active or mesh.uv_layers[0]
            uv_fallback = selector.get("uv_fallbacks", {}).get(name)
            if uv_layer is None and uv_fallback != "semantic-detail-region":
                if owner is not None:
                    owner.to_mesh_clear()
                raise ImportFailure(f"{donor} object {name} is missing UV map {selector['uv_map']}")
            group = semantic_group(asset, name)
            output = grouped.setdefault(group, [])
            for triangle in mesh.loop_triangles:
                loops = list(triangle.loops)
                points = [transform_point(obj.matrix_world, mesh.vertices[mesh.loops[index].vertex_index].co) for index in loops]
                normal = triangle_normal(points)
                if normal is None:
                    topology["removed_degenerate_faces"] += 1
                    continue
                corners = []
                for corner_index, (loop_index, point) in enumerate(zip(loops, points)):
                    if uv_layer is None:
                        uv_value = semantic_detail_uv(group, corner_index)
                    else:
                        uv = uv_layer.data[loop_index].uv
                        uv_value = (float(uv.x) % 1.0, float(uv.y) % 1.0)
                    corners.append((point, uv_value, normal))
                output.append(corners)
            if owner is not None:
                owner.to_mesh_clear()
    if not grouped or not any(grouped.values()):
        raise ImportFailure(f"{donor} selected geometry is empty")
    return grouped, topology


def normalization(grouped: dict, markers: dict) -> tuple[dict, dict, list[list[float]]]:
    points = [corner[0] for triangles in grouped.values() for triangle in triangles for corner in triangle]
    minimum = [min(point[axis] for point in points) for axis in range(3)]
    maximum = [max(point[axis] for point in points) for axis in range(3)]
    scale = 2.0 / max(maximum[axis] - minimum[axis] for axis in range(3))
    center_x = (minimum[0] + maximum[0]) * 0.5
    center_z = (minimum[2] + maximum[2]) * 0.5
    ground = minimum[1]

    def normalized(point):
        return (
            (point[0] - center_x) * scale,
            (point[1] - ground) * scale,
            (point[2] - center_z) * scale,
        )

    transformed = {
        group: [[(normalized(position), uv, normal) for position, uv, normal in triangle] for triangle in triangles]
        for group, triangles in grouped.items()
    }
    transformed_markers = {
        marker_id: normalized((point[0], point[2], -point[1]))
        for marker_id, point in markers.items()
    }
    normalized_points = [corner[0] for triangles in transformed.values() for triangle in triangles for corner in triangle]
    bounds = [
        [min(point[axis] for point in normalized_points) for axis in range(3)],
        [max(point[axis] for point in normalized_points) for axis in range(3)],
    ]
    return transformed, transformed_markers, bounds


def emit_obj(grouped: dict, lod_step: int, triangle_budget: int) -> tuple[bytes, int]:
    lines = ["# alife deterministic GeneForge export v1"]
    vertices = []
    indices = {}
    faces_by_group = {}
    duplicate_count = 0
    sampled = {
        group: (triangles[::lod_step] or triangles[:1])
        for group, triangles in grouped.items()
    }
    sampled_total = sum(len(triangles) for triangles in sampled.values())
    for group in sorted(sampled):
        source_triangles = sampled[group]
        group_budget = max(
            1,
            min(
                len(source_triangles),
                round(triangle_budget * len(source_triangles) / sampled_total),
            ),
        )
        triangles = [
            source_triangles[index * len(source_triangles) // group_budget]
            for index in range(group_budget)
        ]
        for triangle in triangles:
            face = []
            for position, uv, normal in triangle:
                key = tuple(round(value, 9) for value in (*position, *uv, *normal))
                if key in indices:
                    duplicate_count += 1
                else:
                    indices[key] = len(vertices) + 1
                    vertices.append((position, uv, normal))
                face.append(indices[key])
            if len(set(face)) == 3:
                faces_by_group.setdefault(group, []).append(face)
    for position, _, _ in vertices:
        lines.append("v " + " ".join(f"{value:.6f}" for value in position))
    for _, uv, _ in vertices:
        lines.append("vt " + " ".join(f"{value:.6f}" for value in uv))
    for _, _, normal in vertices:
        lines.append("vn " + " ".join(f"{value:.6f}" for value in normal))
    for group in sorted(faces_by_group):
        lines.append(f"g {group}")
        for face in faces_by_group[group]:
            lines.append("f " + " ".join(f"{index}/{index}/{index}" for index in face))
    return ("\n".join(lines) + "\n").encode("ascii"), duplicate_count


def socket_manifest(
    recipe: dict,
    donor: str,
    lod: str,
    markers: dict,
    bounds,
    ground_contacts,
    mask_path: str,
    topology: dict,
) -> dict:
    marker_map = {int(key): value for key, value in recipe["marker_map"].items()}
    semantic_to_id = {semantic: marker_id for marker_id, semantic in marker_map.items()}
    socket_semantics = {
        "neck": "head",
        "left-shoulder": "left-upper-arm",
        "right-shoulder": "right-upper-arm",
        "left-hip": "left-thigh",
        "right-hip": "right-thigh",
        "tail-base": "tail-root",
    }
    sockets = {
        name: {
            "translation": list(markers[semantic_to_id[semantic]]),
            "rotation_xyzw": [0.0, 0.0, 0.0, 1.0],
            "scale": [1.0, 1.0, 1.0],
            "overlap_depth": 0.02,
            "allowable_scale_ratio": [0.88, 1.12],
            "pattern_phase_anchor": [0.0, 0.0],
        }
        for name, semantic in socket_semantics.items()
        if semantic in semantic_to_id and semantic_to_id[semantic] in markers
    }
    landmarks = {
        semantic: list(markers[marker_id])
        for marker_id, semantic in marker_map.items()
        if marker_id in markers
    }
    return {
        "schema": "alife.creature_part_sockets.v2",
        "donor": donor,
        "lod": lod,
        "coordinate_frame": {"handedness": "right", "up": "+Y", "forward": "-Z"},
        "bounds": {"min": bounds[0], "max": bounds[1]},
        "sockets": sockets,
        "landmarks": landmarks,
        "ground_contacts": ground_contacts,
        "semantic_mask": mask_path,
        "topology": topology,
    }


def semantic_mask() -> bytes:
    width = height = 64
    colors = list(GROUP_COLORS.values())
    pixels = bytearray()
    for y in range(height):
        color = colors[min(len(colors) - 1, y * len(colors) // height)]
        pixels.extend(bytes(color) * width)
    return png_bytes(width, height, bytes(pixels))


def build_scene_outputs(bpy, recipe: dict, donor: str, markers: dict, staging: Path) -> dict:
    grouped, topology = extract_geometry(bpy, recipe, donor)
    grouped, markers, bounds = normalization(grouped, markers)
    ground_contacts = []
    for group in ("left-leg", "right-leg"):
        points = [
            corner[0]
            for triangle in grouped[group]
            for corner in triangle
        ]
        minimum_y = min(point[1] for point in points)
        planted = [point for point in points if point[1] <= minimum_y + 1.0e-6]
        ground_contacts.append(
            [
                sum(point[0] for point in planted) / len(planted),
                minimum_y,
                sum(point[2] for point in planted) / len(planted),
            ]
        )
    generated_root = staging / "production_voxel_v1/creature_parts/generated/geneforge"
    mask_root = staging / "production_voxel_v1/models/geneforge"
    generated_root.mkdir(parents=True, exist_ok=True)
    mask_root.mkdir(parents=True, exist_ok=True)
    outputs = []
    duplicate_total = 0
    for lod, step, triangle_budget in LODS:
        obj_path = generated_root / f"{donor}_{lod}_parts.obj"
        socket_path = generated_root / f"{donor}_{lod}_sockets.json"
        mask_path = mask_root / f"{donor}_{lod}_semantic.png"
        obj_bytes, duplicates = emit_obj(grouped, step, triangle_budget)
        duplicate_total += duplicates
        obj_path.write_bytes(obj_bytes)
        relative_mask = mask_path.relative_to(staging).as_posix()
        socket_path.write_text(
            json.dumps(
                socket_manifest(
                    recipe,
                    donor,
                    lod,
                    markers,
                    bounds,
                    ground_contacts,
                    relative_mask,
                    topology,
                ),
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        mask_path.write_bytes(semantic_mask())
        outputs.extend((obj_path, socket_path, mask_path))
    topology["removed_duplicate_vertices"] = duplicate_total
    return {
        "donor": donor,
        "topology": topology,
        "output_count": len(outputs),
        "outputs": [path.relative_to(staging).as_posix() for path in outputs],
    }


def blender_worker_main() -> None:
    import bpy

    args = worker_args()
    recipe = json.loads(args.recipes.read_text(encoding="utf-8"))
    bpy.ops.wm.open_mainfile(filepath=str(args.source), load_ui=False)
    inventory, markers = inspect_scene(bpy, recipe, args.donor, args.texture_root)
    if args.action == "inspect":
        payload = inventory
    else:
        if args.staging is None:
            raise ImportFailure("build worker requires staging")
        payload = build_scene_outputs(bpy, recipe, args.donor, markers, args.staging)
        payload["relinked_images"] = inventory["relinked_images"]
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    try:
        if "--worker" in sys.argv:
            blender_worker_main()
        else:
            outer_main()
    except (ImportFailure, OSError, ValueError, KeyError) as error:
        if "--worker" in sys.argv:
            print(f"ALIFE_IMPORT_ERROR:{error}", file=sys.stderr)
        else:
            print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
