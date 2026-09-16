#!/usr/bin/env python3
"""Blender-process half of the deterministic GeneForge creature-part importer."""

from __future__ import annotations

import argparse
from collections import deque
import hashlib
import json
import math
from pathlib import Path
import re
import sys

# The public controller injects its already-loaded, validated constants and
# filesystem helpers. This avoids loading a second controller module when
# Blender executes the entrypoint as `__main__`.
_CONTROLLER_NAMES = (
    "GROUP_COLORS",
    "GROUP_REGIONS",
    "ImportFailure",
    "LODS",
    "PREPARATION_GROUP_SOCKET",
    "PREPARATION_LOD_ORDER",
    "PREPARATION_SLOT_GROUPS",
    "PREPARATION_SOCKET_ORDER",
    "PREPARATION_TORSO_ASSETS",
    "_assert_tree_has_no_reparse_entries",
    "_canonical_staging_root",
    "_is_symlink_or_reparse",
    "_relative_staged_path",
    "anatomy_mask",
    "canonical_path_is_within",
    "confined_existing_staged_path",
    "png_bytes",
    "preparation_key",
    "preparation_sort_key",
    "validate_preparation_contract",
)


def configure_controller_namespace(namespace: dict) -> None:
    missing = [name for name in _CONTROLLER_NAMES if name not in namespace]
    if missing:
        raise RuntimeError(f"GeneForge worker controller contract missing: {missing}")
    globals().update({name: namespace[name] for name in _CONTROLLER_NAMES})


def worker_args() -> argparse.Namespace:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true", required=True)
    parser.add_argument("action", choices=("inspect", "build"))
    parser.add_argument("--donor", required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--texture-root", type=Path, required=True)
    parser.add_argument("--microdetail-root", type=Path, required=True)
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
        try:
            image.reload()
        except RuntimeError as error:
            raise ImportFailure(
                f"{donor} failed to reload texture basename {basename}: {error}"
            ) from error
    return relinked


def selected_assets(recipe: dict, donor: str) -> list[dict]:
    return [asset for asset in recipe["part_assets"] if asset["donor"] == donor]


def marker_positions(bpy, recipe: dict, donor: str) -> dict[int, tuple[float, float, float]]:
    candidates = {}
    source = next(item for item in recipe["sources"] if item["donor"] == donor)
    exceptions = source["audited_non_marker_properties"]
    observed_exceptions = set()
    for obj in sorted(bpy.data.objects, key=lambda item: item.name):
        if "kc3dsbpy_part_marker" not in obj:
            continue
        raw_marker_id = obj["kc3dsbpy_part_marker"]
        if obj.type != "EMPTY":
            if exceptions.get(obj.name) == raw_marker_id:
                observed_exceptions.add(obj.name)
                continue
            raise ImportFailure(
                f"{donor} marker property requires EMPTY object; found {obj.type} {obj.name}"
            )
        marker_id = int(raw_marker_id)
        if marker_id != raw_marker_id:
            raise ImportFailure(f"{donor} has non-integral marker ID {raw_marker_id}")
        if marker_id <= 0:
            raise ImportFailure(f"{donor} has invalid marker ID {marker_id}")
        if marker_id in candidates:
            raise ImportFailure(f"{donor} has duplicate marker ID {marker_id}")
        candidates[marker_id] = tuple(float(value) for value in obj.matrix_world.translation)
    if observed_exceptions != set(exceptions):
        raise ImportFailure(
            f"{donor} audited non-marker property set drifted: "
            f"expected {sorted(exceptions)}, found {sorted(observed_exceptions)}"
        )
    required = {
        marker_id
        for asset in selected_assets(recipe, donor)
        for marker_id in asset["selector"]["marker_ids"]
    }
    if set(candidates) != required:
        expected = "1..12" if donor == "ettin" else "1..14"
        raise ImportFailure(
            f"{donor} marker IDs must be exactly {expected}; found {sorted(candidates)}"
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
    evaluated_transform_objects = []
    evaluated_deformation_objects = []
    for asset in assets:
        selector = asset["selector"]
        for name in selector["include_objects"]:
            obj = bpy.data.objects.get(name)
            if obj is None or obj.type != "MESH":
                raise ImportFailure(f"{donor} selector {asset['id']} missing exact mesh {name}")
            expected_visscript = selector["object_visscripts"][name]
            actual_visscript = str(obj.get("kc3dsbpy_visscript", ""))
            if actual_visscript != expected_visscript:
                raise ImportFailure(
                    f"{donor} object {name} kc3dsbpy_visscript mismatch: "
                    f"expected {expected_visscript!r}, found {actual_visscript!r}"
                )
            owner, mesh, raw = evaluated_mesh(
                obj, depsgraph, selector.get("evaluated_empty_policy", {}).get(name), donor
            )
            if raw:
                used_raw.append(name)
            if owner is not None:
                raw_matrix = tuple(value for row in obj.matrix_basis for value in row)
                evaluated_matrix = tuple(value for row in owner.matrix_world for value in row)
                if any(abs(a - b) > 1.0e-7 for a, b in zip(raw_matrix, evaluated_matrix)):
                    evaluated_transform_objects.append(name)
                if len(mesh.vertices) != len(obj.data.vertices) or any(
                    (mesh.vertices[index].co - obj.data.vertices[index].co).length > 1.0e-7
                    for index in range(min(len(mesh.vertices), len(obj.data.vertices)))
                ):
                    evaluated_deformation_objects.append(name)
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
        "evaluated_transform_objects": sorted(set(evaluated_transform_objects)),
        "evaluated_deformation_objects": sorted(set(evaluated_deformation_objects)),
        "audited_non_marker_properties": sorted(
            next(
                source["audited_non_marker_properties"]
                for source in recipe["sources"]
                if source["donor"] == donor
            )
        ),
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
    if slot == "tail":
        return "tail-back"
    return slot


def source_uv_coordinate(value: float) -> float:
    value = float(value)
    wrapped = value - math.floor(value)
    if abs(wrapped) <= 1.0e-9 and value > 0.0:
        return 1.0
    return max(0.0, min(1.0, wrapped))


def semantic_atlas_uv(group: str, source_uv: tuple[float, float]) -> tuple[float, float]:
    if group not in GROUP_REGIONS:
        raise ImportFailure(f"semantic group has no atlas region: {group}")
    column, row = GROUP_REGIONS[group]
    inset = 0.04
    span = 1.0 - inset * 2.0
    return (
        (column + inset + span * source_uv[0]) / 4.0,
        (row + inset + span * source_uv[1]) / 3.0,
    )


def semantic_source_uv(group: str, atlas_uv: tuple[float, float]) -> tuple[float, float]:
    column, row = GROUP_REGIONS[group]
    inset = 0.04
    span = 1.0 - inset * 2.0
    return (
        max(0.0, min(1.0, (atlas_uv[0] * 4.0 - column - inset) / span)),
        max(0.0, min(1.0, (atlas_uv[1] * 3.0 - row - inset) / span)),
    )


def semantic_detail_uv(group: str, corner: int) -> tuple[float, float]:
    source_uvs = ((0.15, 0.15), (0.85, 0.15), (0.5, 0.85))
    return semantic_atlas_uv(group, source_uvs[corner])


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


def vector_length(vector) -> float:
    return math.sqrt(sum(value * value for value in vector))


def normalized_vector(vector) -> tuple[float, float, float]:
    length = vector_length(vector)
    if not math.isfinite(length) or length <= 1.0e-12:
        return (0.0, 1.0, 0.0)
    return tuple(float(value / length) for value in vector)


def face_signature(face) -> tuple:
    return tuple(
        sorted(
            tuple(round(float(value), 12) for value in vertex.co)
            for vertex in face.verts
        )
    )


def stable_object_component(bm, asset_id: str, object_name: str) -> dict[int, str]:
    bm.faces.ensure_lookup_table()
    identity = json.dumps(
        [asset_id, object_name],
        ensure_ascii=True,
        separators=(",", ":"),
    ).encode("ascii")
    component_id = f"object-{hashlib.sha256(identity).hexdigest()[:20]}"
    return {face.index: component_id for face in bm.faces}


def remove_degenerate_and_duplicate_faces(bmesh, bm) -> int:
    bm.faces.ensure_lookup_table()
    removed = [face for face in bm.faces if face.calc_area() <= 1.0e-12]
    signatures = set()
    for face in sorted(
        (face for face in bm.faces if face not in removed),
        key=face_signature,
    ):
        signature = face_signature(face)
        if signature in signatures:
            removed.append(face)
        else:
            signatures.add(signature)
    if removed:
        bmesh.ops.delete(bm, geom=list(dict.fromkeys(removed)), context="FACES_ONLY")
    return len(removed)


def remove_loose_geometry(bmesh, bm) -> int:
    loose = [vertex for vertex in bm.verts if not vertex.link_faces]
    if loose:
        bmesh.ops.delete(bm, geom=loose, context="VERTS")
    return len(loose)


def repair_mesh_topology(bmesh, bm, repairs: list[str]) -> dict:
    metrics = {
        "removed_degenerate_faces": 0,
        "removed_duplicate_vertices": 0,
        "removed_loose_vertices": 0,
        "repaired_non_manifold_edges": 0,
        "filled_boundary_edges": 0,
    }
    metrics["removed_degenerate_faces"] += remove_degenerate_and_duplicate_faces(
        bmesh, bm
    )
    before_vertices = len(bm.verts)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=1.0e-12)
    metrics["removed_duplicate_vertices"] += before_vertices - len(bm.verts)
    metrics["removed_degenerate_faces"] += remove_degenerate_and_duplicate_faces(
        bmesh, bm
    )

    bm.edges.ensure_lookup_table()
    non_manifold = sorted(
        (edge for edge in bm.edges if len(edge.link_faces) > 2),
        key=lambda edge: tuple(
            sorted(tuple(round(float(value), 12) for value in vertex.co) for vertex in edge.verts)
        ),
    )
    if "repair-declared-non-manifold-edges" in repairs:
        excess_faces = set()
        for edge in non_manifold:
            linked = sorted(edge.link_faces, key=face_signature)
            excess_faces.update(linked[2:])
        if excess_faces:
            bmesh.ops.delete(
                bm,
                geom=sorted(excess_faces, key=face_signature),
                context="FACES_ONLY",
            )
        metrics["repaired_non_manifold_edges"] += len(non_manifold)

    if "repair-declared-boundary-edges" in repairs:
        bm.edges.ensure_lookup_table()
        boundary = sorted(
            (edge for edge in bm.edges if len(edge.link_faces) == 1),
            key=lambda edge: tuple(
                sorted(
                    tuple(round(float(value), 12) for value in vertex.co)
                    for vertex in edge.verts
                )
            ),
        )
        if boundary:
            bmesh.ops.holes_fill(bm, edges=boundary, sides=0)
        metrics["filled_boundary_edges"] += len(boundary)

    metrics["removed_degenerate_faces"] += remove_degenerate_and_duplicate_faces(
        bmesh, bm
    )
    metrics["removed_loose_vertices"] += remove_loose_geometry(bmesh, bm)
    if bm.faces:
        bmesh.ops.triangulate(
            bm,
            faces=list(bm.faces),
            quad_method="BEAUTY",
            ngon_method="EAR_CLIP",
        )
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.normal_update()
    remaining_non_manifold = [edge for edge in bm.edges if len(edge.link_faces) > 2]
    if remaining_non_manifold:
        raise ImportFailure(
            f"topology repair left {len(remaining_non_manifold)} non-manifold edges"
        )
    if "repair-declared-boundary-edges" in repairs:
        remaining_boundary = [edge for edge in bm.edges if len(edge.link_faces) == 1]
        if remaining_boundary:
            raise ImportFailure(
                f"topology repair left {len(remaining_boundary)} declared boundary edges"
            )
    return metrics


def material_luminance(obj, material_index: int) -> float:
    if material_index >= len(obj.material_slots):
        return 0.5
    material = obj.material_slots[material_index].material
    if material is None:
        return 0.5
    color = material.diffuse_color
    return max(
        0.0,
        min(1.0, 0.2126 * float(color[0]) + 0.7152 * float(color[1]) + 0.0722 * float(color[2])),
    )


def image_node_base_color_distance(material, start_node) -> int | None:
    if material.node_tree is None:
        return None
    pending = [(start_node, 0)]
    visited = set()
    while pending:
        node, distance = pending.pop(0)
        pointer = node.as_pointer()
        if pointer in visited or distance > 8:
            continue
        visited.add(pointer)
        for link in material.node_tree.links:
            if link.from_node != node:
                continue
            if (
                link.to_node.type == "BSDF_PRINCIPLED"
                and link.to_socket.name == "Base Color"
            ):
                return distance + 1
            pending.append((link.to_node, distance + 1))
    return None


def material_source_image(material):
    if material is None or not material.use_nodes or material.node_tree is None:
        return None
    auxiliary_tokens = (
        "alpha",
        "blend",
        "darkness",
        "hard",
        "normal",
        "rough",
        "metal",
        "specular",
    )
    candidates = []
    for node in material.node_tree.nodes:
        image = getattr(node, "image", None)
        if node.type != "TEX_IMAGE" or image is None:
            continue
        basename = Path(image.filepath).name or image.name
        distance = image_node_base_color_distance(material, node)
        auxiliary = any(token in basename.casefold() for token in auxiliary_tokens)
        candidates.append(
            (
                distance is None,
                distance if distance is not None else 99,
                auxiliary,
                basename.casefold(),
                node.name.casefold(),
                image,
            )
        )
    return min(candidates, key=lambda item: item[:-1])[-1] if candidates else None


def image_luminance_grid(image, cache: dict) -> list[int]:
    key = image.as_pointer()
    if key in cache:
        return cache[key]
    width, height = int(image.size[0]), int(image.size[1])
    if width <= 0 or height <= 0:
        raise ImportFailure(f"linked source image is empty: {image.name}")
    if (width, height) != (64, 64):
        image.scale(64, 64)
        width, height = 64, 64
    pixels = image.pixels[:]
    samples = []
    for y in range(64):
        for x in range(64):
            offset = (y * width + x) * 4
            luminance = (
                0.2126 * float(pixels[offset])
                + 0.7152 * float(pixels[offset + 1])
                + 0.0722 * float(pixels[offset + 2])
            )
            samples.append(round(max(0.0, min(1.0, luminance)) * 255.0))
    cache[key] = samples
    return samples


def material_texture_luminance(
    obj,
    material_index: int,
    source_uv: tuple[float, float],
    material_cache: dict,
    image_cache: dict,
    used_texture_files: set[str],
) -> float | None:
    if material_index >= len(obj.material_slots):
        return None
    material = obj.material_slots[material_index].material
    if material is None:
        return None
    key = material.as_pointer()
    if key not in material_cache:
        material_cache[key] = material_source_image(material)
    image = material_cache[key]
    if image is None:
        return None
    used_texture_files.add(Path(image.filepath).name or image.name)
    samples = image_luminance_grid(image, image_cache)
    x = min(63, max(0, round(source_uv[0] * 63)))
    y = min(63, max(0, round(source_uv[1] * 63)))
    return samples[y * 64 + x] / 255.0


def evaluated_geometry_detail(matrix, vertex, face, material_value: float) -> float:
    normal_matrix = matrix.to_3x3()
    world_normal = normal_matrix @ vertex.normal
    converted = normalized_vector((world_normal.x, world_normal.z, -world_normal.y))
    neighboring = [linked.normal for linked in vertex.link_faces]
    curvature = 0.0
    if neighboring:
        curvature = sum(
            1.0 - abs(float(face.normal.dot(other))) for other in neighboring
        ) / len(neighboring)
    signal = (
        abs(converted[0]) * 0.23
        + abs(converted[1]) * 0.31
        + abs(converted[2]) * 0.17
        + max(0.0, min(1.0, curvature)) * 0.19
        + material_value * 0.10
    )
    return max(0.0, min(1.0, signal))


def extract_geometry(bpy, recipe: dict, donor: str) -> tuple[dict, dict, list[str]]:
    import bmesh

    depsgraph = bpy.context.evaluated_depsgraph_get()
    asset_geometry = {}
    topology = {
        "removed_degenerate_faces": 0,
        "removed_duplicate_vertices": 0,
        "removed_loose_vertices": 0,
        "repaired_non_manifold_edges": 0,
        "filled_boundary_edges": 0,
    }
    material_cache = {}
    image_cache = {}
    used_texture_files = set()
    for asset in selected_assets(recipe, donor):
        grouped = {}
        selector = asset["selector"]
        repairs = selector.get("topology_repairs", {})
        for name in selector["include_objects"]:
            obj = bpy.data.objects[name]
            owner, mesh, _ = evaluated_mesh(
                obj, depsgraph, selector.get("evaluated_empty_policy", {}).get(name), donor
            )
            bm = bmesh.new()
            try:
                bm.from_mesh(mesh)
                try:
                    object_topology = repair_mesh_topology(
                        bmesh, bm, repairs.get(name, [])
                    )
                except ImportFailure as error:
                    raise ImportFailure(f"{donor} object {name}: {error}") from error
                for key, value in object_topology.items():
                    topology[key] += value
                uv_layer = bm.loops.layers.uv.get(selector["uv_map"])
                if uv_layer is None and bm.loops.layers.uv.keys():
                    uv_layer = bm.loops.layers.uv.active
                uv_fallback = selector.get("uv_fallbacks", {}).get(name)
                if uv_layer is None and uv_fallback != "semantic-detail-region":
                    raise ImportFailure(
                        f"{donor} object {name} is missing UV map {selector['uv_map']}"
                    )
                group = semantic_group(asset, name)
                output = grouped.setdefault(group, [])
                matrix = owner.matrix_world if owner is not None else obj.matrix_world
                component_by_face = stable_object_component(bm, asset["id"], name)
                for face in sorted(bm.faces, key=face_signature):
                    if len(face.loops) != 3:
                        raise ImportFailure(
                            f"{donor} object {name} was not deterministically triangulated"
                        )
                    points = [transform_point(matrix, loop.vert.co) for loop in face.loops]
                    if triangle_normal(points) is None:
                        topology["removed_degenerate_faces"] += 1
                        continue
                    material_value = material_luminance(obj, face.material_index)
                    corners = []
                    for corner_index, (loop, point) in enumerate(zip(face.loops, points)):
                        if uv_layer is None:
                            source_uv = None
                            uv_value = semantic_detail_uv(group, corner_index)
                        else:
                            uv = loop[uv_layer].uv
                            source_uv = (
                                source_uv_coordinate(uv.x),
                                source_uv_coordinate(uv.y),
                            )
                            uv_value = semantic_atlas_uv(
                                group,
                                source_uv,
                            )
                        detail = evaluated_geometry_detail(
                            matrix, loop.vert, face, material_value
                        )
                        if source_uv is not None:
                            texture_detail = material_texture_luminance(
                                obj,
                                face.material_index,
                                source_uv,
                                material_cache,
                                image_cache,
                                used_texture_files,
                            )
                            if texture_detail is not None:
                                detail = texture_detail * 0.72 + detail * 0.28
                        corners.append(
                            (
                                point,
                                uv_value,
                                detail,
                                component_by_face[face.index],
                                uv_layer is None,
                            )
                        )
                    output.append(corners)
            finally:
                bm.free()
                if owner is not None:
                    owner.to_mesh_clear()
        if not grouped or not any(grouped.values()):
            raise ImportFailure(f"{donor} asset {asset['id']} selected geometry is empty")
        asset_geometry[asset["id"]] = grouped
    if not asset_geometry:
        raise ImportFailure(f"{donor} selected geometry is empty")
    return asset_geometry, topology, sorted(used_texture_files)


def normalization(asset_geometry: dict, markers: dict) -> tuple[dict, dict, dict]:
    points = [
        corner[0]
        for grouped in asset_geometry.values()
        for triangles in grouped.values()
        for triangle in triangles
        for corner in triangle
    ]
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
        asset_id: {
            group: [
                [
                    (normalized(position), uv, detail, component, uvless)
                    for position, uv, detail, component, uvless in triangle
                ]
                for triangle in triangles
            ]
            for group, triangles in grouped.items()
        }
        for asset_id, grouped in asset_geometry.items()
    }
    transformed_markers = {
        marker_id: normalized((point[0], point[2], -point[1]))
        for marker_id, point in markers.items()
    }
    bounds = {}
    for asset_id, grouped in transformed.items():
        normalized_points = [
            corner[0]
            for triangles in grouped.values()
            for triangle in triangles
            for corner in triangle
        ]
        bounds[asset_id] = [
            [min(point[axis] for point in normalized_points) for axis in range(3)],
            [max(point[axis] for point in normalized_points) for axis in range(3)],
        ]
    return transformed, transformed_markers, bounds


def topology_metrics(grouped: dict) -> dict:
    faces = []
    edge_faces = {}
    component_triangle_counts = {}
    for group in sorted(grouped):
        for triangle in grouped[group]:
            component = triangle[0][3]
            if any(corner[3] != component for corner in triangle):
                raise ImportFailure("triangle crosses stable semantic component identity")
            positions = [tuple(round(value, 9) for value in corner[0]) for corner in triangle]
            face_index = len(faces)
            faces.append((group, component, positions))
            component_triangle_counts[component] = (
                component_triangle_counts.get(component, 0) + 1
            )
            for first, second in ((0, 1), (1, 2), (2, 0)):
                edge = tuple(
                    sorted(
                        (
                            (group, component, positions[first]),
                            (group, component, positions[second]),
                        )
                    )
                )
                edge_faces.setdefault(edge, []).append(face_index)
    if not faces:
        raise ImportFailure("LOD geometry is empty")
    adjacency = [set() for _ in faces]
    for linked in edge_faces.values():
        for face in linked:
            adjacency[face].update(other for other in linked if other != face)
    unseen = set(range(len(faces)))
    components = 0
    component_connected_counts = {}
    while unseen:
        components += 1
        first = min(unseen)
        unseen.remove(first)
        pending = [first]
        connected_faces = []
        while pending:
            current = pending.pop()
            connected_faces.append(current)
            for neighbor in adjacency[current]:
                if neighbor in unseen:
                    unseen.remove(neighbor)
                    pending.append(neighbor)
        declared = {faces[index][1] for index in connected_faces}
        if len(declared) != 1:
            raise ImportFailure(
                "geometrically connected faces cross stable component identities"
            )
        component = next(iter(declared))
        component_connected_counts[component] = (
            component_connected_counts.get(component, 0) + 1
        )
    return {
        "triangle_count": len(faces),
        "connected_components": components,
        "boundary_edges": sum(len(linked) == 1 for linked in edge_faces.values()),
        "non_manifold_edges": sum(len(linked) > 2 for linked in edge_faces.values()),
        "component_ids": sorted(component_triangle_counts),
        "component_triangle_counts": {
            component: component_triangle_counts[component]
            for component in sorted(component_triangle_counts)
        },
        "component_connected_counts": {
            component: component_connected_counts.get(component, 0)
            for component in sorted(component_triangle_counts)
        },
    }


def component_uv_detail_grid(samples: list[tuple]) -> list[float]:
    sums = [0.0] * (64 * 64)
    counts = [0] * (64 * 64)
    for _, uv, detail, uvless in samples:
        if uvless:
            raise ImportFailure("UV detail grid received fallback-only geometry")
        x = min(63, max(0, round(uv[0] * 63)))
        y = min(63, max(0, round(uv[1] * 63)))
        index = y * 64 + x
        sums[index] += detail
        counts[index] += 1
    occupied = [index for index, count in enumerate(counts) if count]
    if not occupied:
        raise ImportFailure("UV detail grid has no source samples")
    values = [None] * (64 * 64)
    queue = deque()
    for index in occupied:
        values[index] = sums[index] / counts[index]
        queue.append(index)
    while queue:
        index = queue.popleft()
        x, y = index % 64, index // 64
        for next_x, next_y in ((x, y - 1), (x - 1, y), (x + 1, y), (x, y + 1)):
            if not (0 <= next_x < 64 and 0 <= next_y < 64):
                continue
            next_index = next_y * 64 + next_x
            if values[next_index] is None:
                values[next_index] = values[index]
                queue.append(next_index)
    return values


def sample_uv_detail_grid(grid: list[float], uv: tuple[float, float]) -> float:
    x = max(0.0, min(63.0, uv[0] * 63.0))
    y = max(0.0, min(63.0, uv[1] * 63.0))
    x0, y0 = math.floor(x), math.floor(y)
    x1, y1 = min(63, x0 + 1), min(63, y0 + 1)
    tx, ty = x - x0, y - y0
    top = grid[y0 * 64 + x0] * (1.0 - tx) + grid[y0 * 64 + x1] * tx
    bottom = grid[y1 * 64 + x0] * (1.0 - tx) + grid[y1 * 64 + x1] * tx
    return top * (1.0 - ty) + bottom * ty


def decimate_asset(bpy, grouped: dict, ratio: float, triangle_budget: int) -> tuple[dict, dict]:
    total = sum(len(triangles) for triangles in grouped.values())
    target = min(triangle_budget, max(4, round(total * ratio)))
    if target >= total:
        metrics = topology_metrics(grouped)
        return grouped, metrics

    groups = sorted(grouped)
    components = sorted(
        {
            (group, triangle[0][3])
            for group, triangles in grouped.items()
            for triangle in triangles
        }
    )
    component_index = {
        component: index for index, component in enumerate(components)
    }
    vertices = []
    vertex_indices = {}
    faces = []
    face_uvs = []
    face_groups = []
    component_samples = {}
    for group in groups:
        for triangle in grouped[group]:
            face = []
            uvs = []
            component = triangle[0][3]
            if any(corner[3] != component for corner in triangle):
                raise ImportFailure("LOD triangle crosses exact source object identity")
            for position, uv, detail, _, uvless in triangle:
                key = (group, component) + tuple(
                    round(value, 12) for value in position
                )
                if key not in vertex_indices:
                    vertex_indices[key] = len(vertices)
                    vertices.append(position)
                face.append(vertex_indices[key])
                uvs.append(uv)
                component_samples.setdefault((group, component), []).append(
                    (position, uv, detail, uvless)
                )
            if len(set(face)) == 3:
                faces.append(face)
                face_uvs.append(uvs)
                face_groups.append(component_index[(group, component)])

    component_detail_grids = {}
    for component, samples in component_samples.items():
        uvless = samples[0][3]
        if any(sample[3] != uvless for sample in samples):
            raise ImportFailure(
                "stable component mixes authored and fallback UV policies"
            )
        if not uvless:
            component_detail_grids[component] = component_uv_detail_grid(samples)

    mesh = bpy.data.meshes.new("__alife_geneforge_lod_mesh")
    obj = bpy.data.objects.new("__alife_geneforge_lod_object", mesh)
    materials = []
    evaluated = None
    try:
        mesh.from_pydata(vertices, [], faces)
        mesh.update()
        uv_layer = mesh.uv_layers.new(name="alife_semantic_uv")
        for polygon, uvs, material_index in zip(mesh.polygons, face_uvs, face_groups):
            polygon.material_index = material_index
            for loop_index, uv in zip(polygon.loop_indices, uvs):
                uv_layer.data[loop_index].uv = uv
        for index, _ in enumerate(components):
            material = bpy.data.materials.new(f"__alife_geneforge_group_{index}")
            materials.append(material)
            mesh.materials.append(material)
        bpy.context.collection.objects.link(obj)
        modifier = obj.modifiers.new("Deterministic topology-preserving LOD", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = max(0.01, min(1.0, target / total))
        modifier.use_collapse_triangulate = True
        modifier.use_symmetry = False
        depsgraph = bpy.context.evaluated_depsgraph_get()
        evaluated = obj.evaluated_get(depsgraph)
        output_mesh = evaluated.to_mesh(
            preserve_all_data_layers=True, depsgraph=depsgraph
        )
        output_mesh.calc_loop_triangles()
        output_uv = output_mesh.uv_layers.get("alife_semantic_uv")
        if output_uv is None:
            raise ImportFailure("LOD decimator discarded semantic UV sampling")
        output = {}
        for triangle in output_mesh.loop_triangles:
            polygon = output_mesh.polygons[triangle.polygon_index]
            if polygon.material_index >= len(components):
                raise ImportFailure("LOD decimator discarded semantic group identity")
            group, component = components[polygon.material_index]
            corners = []
            for loop_index in triangle.loops:
                loop = output_mesh.loops[loop_index]
                vertex = output_mesh.vertices[loop.vertex_index]
                point = tuple(float(value) for value in vertex.co)
                uv = output_uv.data[loop_index].uv
                atlas_uv = (
                    max(0.0, min(1.0, float(uv.x))),
                    max(0.0, min(1.0, float(uv.y))),
                )
                samples = component_samples[(group, component)]
                uvless = samples[0][3]
                if uvless:
                    nearest = min(
                        samples,
                        key=lambda sample: (
                            (sample[1][0] - atlas_uv[0]) ** 2
                            + (sample[1][1] - atlas_uv[1]) ** 2,
                            tuple(round(value, 12) for value in sample[0]),
                        ),
                    )
                    detail = nearest[2]
                else:
                    detail = sample_uv_detail_grid(
                        component_detail_grids[(group, component)], atlas_uv
                    )
                corners.append(
                    (
                        point,
                        atlas_uv,
                        detail,
                        component,
                        uvless,
                    )
                )
            if triangle_normal([corner[0] for corner in corners]) is not None:
                output.setdefault(group, []).append(corners)
        repaired_output = {}
        for group, component in components:
            source_triangles = [
                triangle
                for triangle in grouped[group]
                if triangle[0][3] == component
            ]
            candidate_triangles = [
                triangle
                for triangle in output.get(group, [])
                if triangle[0][3] == component
            ]
            use_source = not candidate_triangles
            if not use_source:
                source_metrics = topology_metrics({group: source_triangles})
                candidate_metrics = topology_metrics({group: candidate_triangles})
                source_islands = source_metrics["component_connected_counts"].get(
                    component, 0
                )
                candidate_islands = candidate_metrics[
                    "component_connected_counts"
                ].get(component, 0)
                use_source = (
                    candidate_islands < 1
                    or candidate_islands > source_islands
                    or candidate_metrics["non_manifold_edges"] != 0
                    or candidate_metrics["boundary_edges"]
                    > source_metrics["boundary_edges"]
                )
            repaired_output.setdefault(group, []).extend(
                source_triangles if use_source else candidate_triangles
            )
        output = repaired_output
        metrics = topology_metrics(output)
        source_metrics = topology_metrics(grouped)
        if metrics["triangle_count"] >= total:
            raise ImportFailure(
                f"LOD decimator did not reduce {total} triangles toward target {target}"
            )
        if metrics["non_manifold_edges"]:
            raise ImportFailure("LOD decimator introduced non-manifold geometry")
        if metrics["component_ids"] != source_metrics["component_ids"]:
            raise ImportFailure("LOD decimator discarded or split a semantic component")
        for component, count in metrics["component_connected_counts"].items():
            if not 1 <= count <= source_metrics["component_connected_counts"][component]:
                raise ImportFailure(
                    "LOD decimator multiplied islands within a source object"
                )
        if metrics["boundary_edges"] > source_metrics["boundary_edges"]:
            raise ImportFailure("LOD decimator introduced open component boundaries")
        return output, metrics
    finally:
        if evaluated is not None:
            evaluated.to_mesh_clear()
        bpy.data.objects.remove(obj, do_unlink=True)
        if mesh.name in bpy.data.meshes:
            bpy.data.meshes.remove(mesh)
        for material in materials:
            if material.name in bpy.data.materials:
                bpy.data.materials.remove(material)


def emit_obj(grouped: dict) -> bytes:
    lines = ["# alife deterministic GeneForge export v2"]
    position_indices = {}
    uv_indices = {}
    normal_accumulators = {}
    faces_by_component = {}
    for group in sorted(grouped):
        for triangle in grouped[group]:
            points = [corner[0] for corner in triangle]
            a = tuple(points[1][axis] - points[0][axis] for axis in range(3))
            b = tuple(points[2][axis] - points[0][axis] for axis in range(3))
            area_vector = (
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            )
            if vector_length(area_vector) <= 1.0e-12:
                continue
            face = []
            component = triangle[0][3]
            for position, uv, _, corner_component, _ in triangle:
                if corner_component != component:
                    raise ImportFailure("OBJ triangle crosses stable component identity")
                position_key = (group, component) + tuple(
                    round(value, 9) for value in position
                )
                uv_key = tuple(round(value, 9) for value in uv)
                if position_key not in position_indices:
                    position_indices[position_key] = len(position_indices) + 1
                if uv_key not in uv_indices:
                    uv_indices[uv_key] = len(uv_indices) + 1
                accumulator = normal_accumulators.setdefault(position_key, [0.0, 0.0, 0.0])
                for axis in range(3):
                    accumulator[axis] += area_vector[axis]
                face.append((position_key, uv_key))
            faces_by_component.setdefault((group, component), []).append(face)
    normal_indices = {
        key: index + 1 for index, key in enumerate(position_indices)
    }
    positions_by_index = sorted(position_indices, key=position_indices.get)
    for key in positions_by_index:
        lines.append("v " + " ".join(f"{value:.9f}" for value in key[2:]))
    uvs_by_index = sorted(uv_indices, key=uv_indices.get)
    for uv in uvs_by_index:
        lines.append("vt " + " ".join(f"{value:.9f}" for value in uv))
    for key in positions_by_index:
        normal = normalized_vector(normal_accumulators[key])
        if not all(math.isfinite(value) for value in normal):
            raise ImportFailure("generated smooth normal is non-finite")
        lines.append("vn " + " ".join(f"{value:.9f}" for value in normal))
    for group, component in sorted(faces_by_component):
        lines.append(f"g {group}")
        lines.append(f"o {component}")
        for face in faces_by_component[(group, component)]:
            references = [
                f"{position_indices[position]}/{uv_indices[uv]}/{normal_indices[position]}"
                for position, uv in face
            ]
            lines.append("f " + " ".join(references))
    return ("\n".join(lines) + "\n").encode("ascii")


def load_microdetail_samples(bpy, microdetail_root: Path, donor: str) -> tuple[list[int], list[str]]:
    extensions = {".bmp", ".exr", ".jpeg", ".jpg", ".png", ".tga", ".tif", ".tiff"}
    paths = [
        path
        for path in sorted(microdetail_root.rglob("*"))
        if path.is_file() and path.suffix.casefold() in extensions
    ]
    if not paths:
        raise ImportFailure(f"{donor} microdetail root contains no supported images")
    images = []
    try:
        for path in paths:
            image = bpy.data.images.load(str(path), check_existing=False)
            try:
                image.reload()
            except RuntimeError as error:
                raise ImportFailure(
                    f"{donor} failed to reload microdetail image {path.name}: {error}"
                ) from error
            if image.size[0] <= 0 or image.size[1] <= 0:
                raise ImportFailure(f"{donor} microdetail image is empty: {path.name}")
            images.append(image)
        samples = []
        for y in range(64):
            for x in range(64):
                image = images[min(len(images) - 1, x * len(images) // 64)]
                width, height = int(image.size[0]), int(image.size[1])
                source_x = min(width - 1, x * width // 64)
                source_y = min(height - 1, y * height // 64)
                offset = (source_y * width + source_x) * 4
                rgba = image.pixels[offset : offset + 4]
                luminance = (
                    0.2126 * float(rgba[0])
                    + 0.7152 * float(rgba[1])
                    + 0.0722 * float(rgba[2])
                )
                samples.append(round(max(0.0, min(1.0, luminance)) * 255))
        return samples, [path.name for path in paths]
    finally:
        for image in images:
            bpy.data.images.remove(image)


def barycentric_weights(point, triangle) -> tuple[float, float, float] | None:
    (px, py) = point
    (ax, ay), (bx, by), (cx, cy) = triangle
    denominator = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
    if abs(denominator) <= 1.0e-12:
        return None
    first = ((by - cy) * (px - cx) + (cx - bx) * (py - cy)) / denominator
    second = ((cy - ay) * (px - cx) + (ax - cx) * (py - cy)) / denominator
    third = 1.0 - first - second
    if min(first, second, third) < -1.0e-9:
        return None
    return first, second, third


def source_microdetail_at(source_samples: list[int], source_uv) -> int:
    x = min(63, max(0, round(source_uv[0] * 63)))
    y = min(63, max(0, round(source_uv[1] * 63)))
    return source_samples[y * 64 + x]


def semantic_mask(grouped: dict, source_samples: list[int]) -> bytes:
    width = height = 64
    pixels = bytearray(width * height * 4)
    for group in sorted(grouped):
        if group not in GROUP_COLORS:
            raise ImportFailure(f"semantic group has no mask color: {group}")
        color = GROUP_COLORS[group][:3]
        painted = set()
        triangles = sorted(
            grouped[group],
            key=lambda triangle: (
                triangle[0][3],
                tuple(tuple(round(value, 9) for value in corner[1]) for corner in triangle),
                tuple(tuple(round(value, 9) for value in corner[0]) for corner in triangle),
            ),
        )
        for triangle in triangles:
            atlas_triangle = [corner[1] for corner in triangle]
            uvless = triangle[0][4]
            if any(corner[4] != uvless for corner in triangle):
                raise ImportFailure("semantic triangle mixes UV fallback policies")
            minimum_x = max(0, math.floor(min(uv[0] for uv in atlas_triangle) * width))
            maximum_x = min(
                width - 1, math.floor(max(uv[0] for uv in atlas_triangle) * width)
            )
            minimum_y = max(0, math.floor(min(uv[1] for uv in atlas_triangle) * height))
            maximum_y = min(
                height - 1, math.floor(max(uv[1] for uv in atlas_triangle) * height)
            )
            triangle_pixels = []
            for y in range(minimum_y, maximum_y + 1):
                for x in range(minimum_x, maximum_x + 1):
                    atlas_uv = ((x + 0.5) / width, (y + 0.5) / height)
                    weights = barycentric_weights(atlas_uv, atlas_triangle)
                    if weights is not None:
                        triangle_pixels.append((x, y, atlas_uv, weights))
            if not triangle_pixels:
                atlas_uv = (
                    sum(uv[0] for uv in atlas_triangle) / 3.0,
                    sum(uv[1] for uv in atlas_triangle) / 3.0,
                )
                x = min(width - 1, max(0, math.floor(atlas_uv[0] * width)))
                y = min(height - 1, max(0, math.floor(atlas_uv[1] * height)))
                triangle_pixels.append((x, y, atlas_uv, (1.0 / 3.0,) * 3))
            for x, y, atlas_uv, weights in triangle_pixels:
                geometry = sum(
                    weights[index] * triangle[index][2] for index in range(3)
                )
                if uvless:
                    alpha = round(max(0.0, min(1.0, geometry)) * 255.0)
                else:
                    source_value = source_microdetail_at(
                        source_samples, semantic_source_uv(group, atlas_uv)
                    )
                    alpha = round(
                        source_value * 0.35
                        + max(0.0, min(1.0, geometry)) * 255.0 * 0.65
                    )
                offset = (y * width + x) * 4
                pixels[offset : offset + 4] = bytes(
                    (*color, max(1, min(255, alpha)))
                )
                painted.add((x, y))
        if not painted:
            raise ImportFailure(f"semantic group has no rasterized UV coverage: {group}")
    return png_bytes(width, height, bytes(pixels))


def prepare_bridge_overlap_geometry(
    grouped: dict,
    sockets: dict,
    socket_names: list[str],
    overlap_depth: float,
) -> tuple[dict, list[dict]]:
    if not math.isfinite(overlap_depth) or overlap_depth <= 0.0:
        raise ImportFailure("bridge overlap depth must be finite and positive")
    prepared = {
        group: [tuple(tuple(corner) for corner in triangle) for triangle in triangles]
        for group, triangles in grouped.items()
    }
    evidence = []
    for socket_name in socket_names:
        if socket_name not in sockets:
            raise ImportFailure(f"bridge preparation is missing socket {socket_name}")
        target = tuple(float(value) for value in sockets[socket_name]["translation"])
        unique_positions = sorted(
            {
                tuple(corner[0])
                for triangles in prepared.values()
                for triangle in triangles
                for corner in triangle
            },
            key=lambda point: (
                sum((point[axis] - target[axis]) ** 2 for axis in range(3)),
                tuple(round(value, 12) for value in point),
            ),
        )
        if not unique_positions:
            raise ImportFailure(f"bridge preparation {socket_name} has no source vertices")
        selected = unique_positions[: min(3, len(unique_positions))]
        centroid = tuple(
            sum(point[axis] for point in unique_positions) / len(unique_positions)
            for axis in range(3)
        )
        replacements = {}
        applied_depths = []
        for point in selected:
            direction = tuple(target[axis] - point[axis] for axis in range(3))
            distance = vector_length(direction)
            if distance <= 1.0e-12:
                direction = tuple(target[axis] - centroid[axis] for axis in range(3))
                distance = vector_length(direction)
            if distance <= 1.0e-12:
                direction = (0.0, 1.0, 0.0)
                distance = 1.0
            applied = min(overlap_depth, max(overlap_depth * 0.25, distance * 0.5))
            unit = tuple(value / distance for value in direction)
            moved = tuple(point[axis] + unit[axis] * applied for axis in range(3))
            replacements[point] = moved
            applied_depths.append(applied)
        prepared = {
            group: [
                tuple(
                    (replacements.get(tuple(corner[0]), tuple(corner[0])), *corner[1:])
                    for corner in triangle
                )
                for triangle in triangles
            ]
            for group, triangles in prepared.items()
        }
        for previous in evidence:
            previous_anchor = tuple(previous["prepared_anchor"])
            if previous_anchor in replacements:
                previous["prepared_anchor"] = list(replacements[previous_anchor])
        evidence.append(
            {
                "socket": socket_name,
                "prepared_vertex_count": len(replacements),
                "applied_overlap_depth": max(applied_depths),
                "original_anchor": list(selected[0]),
                "prepared_anchor": list(replacements[selected[0]]),
            }
        )
    return prepared, evidence


def prepared_matrix(fit: dict, translation: list[float]) -> list[float]:
    x, y, z, w = fit["rotation_xyzw"]
    sx, sy, sz = fit["scale"]
    rotation = (
        (1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)),
        (2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)),
        (2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)),
    )
    return [
        rotation[0][0] * sx,
        rotation[0][1] * sy,
        rotation[0][2] * sz,
        translation[0],
        rotation[1][0] * sx,
        rotation[1][1] * sy,
        rotation[1][2] * sz,
        translation[1],
        rotation[2][0] * sx,
        rotation[2][1] * sy,
        rotation[2][2] * sz,
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]


def transform_affine_point(matrix: list[float], point) -> list[float]:
    return [
        sum(matrix[row * 4 + axis] * point[axis] for axis in range(3))
        + matrix[row * 4 + 3]
        for row in range(3)
    ]


def group_transforms_for_preparation(
    recipe: dict, asset: dict, preparation: dict
) -> list[dict]:
    contract = recipe["assembly_preparation_contract"]
    slot = preparation["logical_slot"]
    transforms = []
    if slot == "torso":
        evidence = []
        for bridge in preparation["bridge_geometry"]:
            evidence.append(
                {
                    "socket": bridge["socket"],
                    "source_anchor": bridge["source_anchor"],
                    "target_anchor": bridge["target_anchor"],
                    "transformed_source_anchor": bridge["transformed_source_anchor"],
                    "residual": bridge["residual"],
                    "prepared_vertex_count": bridge["prepared_vertex_count"],
                    "applied_overlap_depth": bridge["applied_overlap_depth"],
                }
            )
        transforms.append(
            {
                "source_family_id": preparation["family_id"],
                "source_asset_id": asset["id"],
                "target_torso_asset_id": preparation["target_torso_asset_id"],
                "lod": preparation["lod"],
                "runtime_group": "torso",
                "socket": "torso-frame",
                "transform_space": contract["transform_space"],
                "schema_digest": contract["schema_digest"],
                "prepared_matrix": preparation["prepared_matrix"],
                "residual": preparation["predicted_attachment_error"],
                "fit": preparation["fit"],
                "seam_offset": preparation["seam_offset"],
                "overlap_depth": preparation["overlap_depth"],
                "attachment_error_bound": preparation["attachment_error_bound"],
                "bridge_kind": preparation["bridge_kind"],
                "join_cover_kind": preparation["join_cover_kind"],
                "socket_evidence": sorted(
                    evidence,
                    key=lambda item: PREPARATION_SOCKET_ORDER.index(item["socket"]),
                ),
            }
        )
    else:
        for bridge in preparation["bridge_geometry"]:
            transforms.append(
                {
                    "source_family_id": preparation["family_id"],
                    "source_asset_id": asset["id"],
                    "target_torso_asset_id": preparation["target_torso_asset_id"],
                    "lod": preparation["lod"],
                    "runtime_group": bridge["runtime_group"],
                    "socket": bridge["socket"],
                    "transform_space": contract["transform_space"],
                    "schema_digest": contract["schema_digest"],
                    "prepared_matrix": bridge["prepared_matrix"],
                    "residual": bridge["residual"],
                    "fit": preparation["fit"],
                    "seam_offset": preparation["seam_offset"],
                    "overlap_depth": preparation["overlap_depth"],
                    "attachment_error_bound": preparation["attachment_error_bound"],
                    "bridge_kind": preparation["bridge_kind"],
                    "join_cover_kind": preparation["join_cover_kind"],
                    "bridge_geometry": [bridge],
                    "socket_evidence": [],
                }
            )
    return sorted(transforms, key=preparation_sort_key)


def assembly_preparations(
    recipe: dict,
    asset: dict,
    manifest: dict,
    manifests: dict,
    *,
    family_filter: int | None = None,
    target_torso_asset_id: str | None = None,
) -> list[dict]:
    contract = recipe["assembly_contract"]
    preparations = []
    for family in recipe["families"]:
        if family_filter is not None and family["id"] != family_filter:
            continue
        for slot, part in family["parts"].items():
            if part["asset_id"] != asset["id"]:
                continue
            if target_torso_asset_id is not None and slot == "torso":
                continue
            sockets = contract["slot_sockets"][slot]
            source_anchors = [manifest["sockets"][name]["translation"] for name in sockets]
            torso_asset_id = target_torso_asset_id or family["parts"]["torso"]["asset_id"]
            torso_manifest = manifests[(torso_asset_id, manifest["lod"])]
            authored_offset = [
                part["fit"]["translation"][axis] + part["seam_offset"][axis]
                for axis in range(3)
            ]
            if slot == "torso":
                translation = authored_offset
            else:
                target_anchors = [
                    torso_manifest["sockets"][name]["translation"] for name in sockets
                ]
                source_centroid = [
                    sum(anchor[axis] for anchor in source_anchors) / len(source_anchors)
                    for axis in range(3)
                ]
                target_centroid = [
                    sum(anchor[axis] for anchor in target_anchors) / len(target_anchors)
                    for axis in range(3)
                ]
                linear_source = transform_affine_point(
                    prepared_matrix(part["fit"], [0.0, 0.0, 0.0]),
                    source_centroid,
                )
                translation = [
                    target_centroid[axis] + authored_offset[axis] - linear_source[axis]
                    for axis in range(3)
                ]
            matrix = prepared_matrix(part["fit"], translation)
            source_geometry = {
                entry["socket"]: entry for entry in manifest["bridge_geometry"]
            }
            bridges = []
            for socket_name, source_anchor in zip(sockets, source_anchors):
                if slot == "torso":
                    bridge_matrix = matrix
                    transformed = transform_affine_point(bridge_matrix, source_anchor)
                    target_anchor = list(transformed)
                else:
                    target_anchor = [
                        torso_manifest["sockets"][socket_name]["translation"][axis]
                        + authored_offset[axis]
                        for axis in range(3)
                    ]
                    linear_source = transform_affine_point(
                        prepared_matrix(part["fit"], [0.0, 0.0, 0.0]),
                        source_anchor,
                    )
                    bridge_translation = [
                        target_anchor[axis] - linear_source[axis] for axis in range(3)
                    ]
                    bridge_matrix = prepared_matrix(part["fit"], bridge_translation)
                    transformed = transform_affine_point(bridge_matrix, source_anchor)
                residual = vector_length(
                    tuple(transformed[axis] - target_anchor[axis] for axis in range(3))
                )
                geometry = source_geometry[socket_name]
                runtime_group = {
                    "neck": "head",
                    "left-shoulder": "left-arm",
                    "right-shoulder": "right-arm",
                    "left-hip": "left-leg",
                    "right-hip": "right-leg",
                    "tail-base": "tail-back",
                }[socket_name]
                if slot == "torso":
                    runtime_group = "torso"
                if runtime_group not in manifest["expected_groups"]:
                    raise ImportFailure(
                        f"family {family['id']} {slot} socket {socket_name} has no runtime OBJ group {runtime_group}"
                    )
                bridges.append(
                    {
                        **geometry,
                        "runtime_group": runtime_group,
                        "source_anchor": list(source_anchor),
                        "target_anchor": target_anchor,
                        "transformed_source_anchor": transformed,
                        "prepared_matrix": bridge_matrix,
                        "residual": residual,
                    }
                )
            predicted_error = max(bridge["residual"] for bridge in bridges)
            if predicted_error > contract["attachment_error_limit"] + 1.0e-9:
                raise ImportFailure(
                    f"family {family['id']} {slot} transformed sockets exceed attachment-error bound: {predicted_error:.9f}"
                )
            preparation = {
                    "family_id": family["id"],
                    "family_label": family["label"],
                    "logical_slot": slot,
                    "asset_id": asset["id"],
                    "source_asset_id": asset["id"],
                    "lod": manifest["lod"],
                    "fit": part["fit"],
                    "seam_offset": part["seam_offset"],
                    "prepared_translation": translation,
                    "prepared_matrix": matrix,
                    "bridge_sockets": sockets,
                    "bridge_kind": f"{slot}-join-cover",
                    "join_cover_kind": part["join_cover_kind"],
                    "transform_mode": (
                        "per-group-socket-transforms"
                        if slot in {"arms", "legs"}
                        else "slot-transform"
                    ),
                    "target_torso_asset_id": torso_asset_id,
                    "overlap_depth": contract["default_overlap_depth"],
                    "attachment_error_bound": contract["attachment_error_limit"],
                    "predicted_attachment_error": predicted_error,
                    "bridge_geometry": bridges,
                }
            preparation["preparation_kind"] = (
                "cross-torso" if target_torso_asset_id is not None else "canonical"
            )
            preparation["group_transforms"] = group_transforms_for_preparation(
                recipe, asset, preparation
            )
            preparations.append(preparation)
    return sorted(
        preparations,
        key=lambda item: (
            item["family_id"],
            item["source_asset_id"],
            PREPARATION_TORSO_ASSETS.index(item["target_torso_asset_id"]),
            PREPARATION_LOD_ORDER.index(item["lod"]),
        ),
    )


def postprocess_assembly_preparations(recipe: dict, staging: Path) -> None:
    assets = {asset["id"]: asset for asset in recipe["part_assets"]}
    contract = validate_preparation_contract(recipe)
    _assert_tree_has_no_reparse_entries(staging)
    paths = [
        confined_existing_staged_path(
            staging, path.relative_to(staging), "staged socket manifest"
        )
        for path in sorted(staging.rglob("*_sockets.json"))
    ]
    manifests = {}
    manifest_paths = {}
    for path in paths:
        manifest = json.loads(path.read_text(encoding="utf-8"))
        key = (manifest["asset_id"], manifest["lod"])
        if key in manifests:
            raise ImportFailure(f"duplicate staged socket manifest {key}")
        manifests[key] = manifest
        manifest_paths[key] = path
    expected = len(recipe["part_assets"]) * len(LODS)
    if len(manifests) != expected:
        raise ImportFailure(
            f"assembly preparation expected {expected} socket manifests; found {len(manifests)}"
        )
    for key in sorted(manifests):
        manifest = manifests[key]
        asset = assets[manifest["asset_id"]]
        manifest["assembly_preparation_schema"] = contract["schema"]
        manifest["assembly_preparation_schema_digest"] = contract["schema_digest"]
        manifest["assembly_preparations"] = assembly_preparations(
            recipe, asset, manifest, manifests
        )
        manifest["cross_torso_preparations"] = []
        manifest["assembly_preparation_population"] = "canonical-only"
        manifest_paths[key].write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    validate_preparation_metadata(recipe, manifests, require_cross_torso=False)


def _validate_prepared_matrix(matrix, label: str) -> None:
    if (
        not isinstance(matrix, list)
        or len(matrix) != 16
        or not all(isinstance(value, (int, float)) and math.isfinite(value) for value in matrix)
        or matrix[12:] != [0.0, 0.0, 0.0, 1.0]
    ):
        raise ImportFailure(f"{label} is not a finite row-major affine matrix")


def _preparation_oracle_records(recipe: dict, manifests: dict) -> dict[tuple, dict]:
    assets = {asset["id"]: asset for asset in recipe["part_assets"]}
    records = {}
    for manifest_key in sorted(manifests):
        manifest = manifests[manifest_key]
        asset = assets[manifest["asset_id"]]
        candidates = assembly_preparations(recipe, asset, manifest, manifests)
        if asset["logical_slot"] != "torso":
            for family in recipe["families"]:
                if family["parts"][asset["logical_slot"]]["asset_id"] != asset["id"]:
                    continue
                canonical_target = family["parts"]["torso"]["asset_id"]
                for target in PREPARATION_TORSO_ASSETS:
                    if target != canonical_target:
                        candidates.extend(
                            assembly_preparations(
                                recipe,
                                asset,
                                manifest,
                                manifests,
                                family_filter=family["id"],
                                target_torso_asset_id=target,
                            )
                        )
        for candidate in candidates:
            key = (
                candidate["preparation_kind"],
                candidate["family_id"],
                candidate["source_asset_id"],
                candidate["target_torso_asset_id"],
                candidate["lod"],
            )
            if key in records:
                raise ImportFailure("assembly preparation oracle contains duplicate records")
            records[key] = candidate
    return records


def _oracle_values_match(actual, expected, *, tolerance: float = 1.0e-9) -> bool:
    if isinstance(expected, (int, float)) and not isinstance(expected, bool):
        return (
            isinstance(actual, (int, float))
            and not isinstance(actual, bool)
            and math.isfinite(actual)
            and math.isclose(actual, expected, rel_tol=0.0, abs_tol=tolerance)
        )
    if isinstance(expected, list):
        return isinstance(actual, list) and len(actual) == len(expected) and all(
            _oracle_values_match(left, right, tolerance=tolerance)
            for left, right in zip(actual, expected)
        )
    return actual == expected


def _validate_preparation_oracle(actual: dict, expected: dict) -> None:
    for field in (
        "fit",
        "seam_offset",
        "prepared_translation",
        "prepared_matrix",
        "predicted_attachment_error",
    ):
        if not _oracle_values_match(actual.get(field), expected[field]):
            raise ImportFailure(f"assembly preparation {field} drift from socket oracle")

    actual_bridges = actual.get("bridge_geometry")
    expected_bridges = expected["bridge_geometry"]
    if not isinstance(actual_bridges, list) or len(actual_bridges) != len(expected_bridges):
        raise ImportFailure("assembly preparation bridge evidence count drift")
    for actual_bridge, expected_bridge in zip(actual_bridges, expected_bridges):
        for field in (
            "socket",
            "runtime_group",
            "source_anchor",
            "target_anchor",
            "transformed_source_anchor",
            "prepared_matrix",
            "residual",
            "prepared_vertex_count",
            "applied_overlap_depth",
            "original_anchor",
            "prepared_anchor",
        ):
            if not _oracle_values_match(actual_bridge.get(field), expected_bridge[field]):
                raise ImportFailure(
                    f"assembly preparation bridge {field} drift from socket oracle"
                )

    actual_groups = actual.get("group_transforms")
    expected_groups = expected["group_transforms"]
    if not isinstance(actual_groups, list) or len(actual_groups) != len(expected_groups):
        raise ImportFailure("assembly group transform oracle count drift")
    for actual_group, expected_group in zip(actual_groups, expected_groups):
        for field in (
            "prepared_matrix",
            "residual",
            "bridge_geometry",
            "socket_evidence",
        ):
            if not _oracle_values_match(actual_group.get(field), expected_group.get(field)):
                raise ImportFailure(
                    f"assembly group {field} drift from independent socket oracle"
                )


def _validate_preparation_group(
    recipe: dict,
    manifest: dict,
    asset: dict,
    family: dict,
    slot_record: dict,
    group: dict,
    expected_kind: str,
) -> str:
    contract = recipe["assembly_preparation_contract"]
    slot = asset["logical_slot"]
    if (
        group.get("source_family_id") != family["id"]
        or group.get("source_asset_id") != asset["id"]
        or group.get("target_torso_asset_id") != slot_record["target_torso_asset_id"]
        or group.get("lod") != manifest["lod"]
        or group.get("transform_space") != contract["transform_space"]
        or group.get("schema_digest") != contract["schema_digest"]
    ):
        raise ImportFailure("assembly group transform identity or contract drift")
    if group.get("runtime_group") not in PREPARATION_SLOT_GROUPS[slot]:
        raise ImportFailure("assembly group transform runtime group is incompatible with its slot")
    expected_socket = PREPARATION_GROUP_SOCKET[group["runtime_group"]]
    if group.get("socket") != expected_socket:
        raise ImportFailure("assembly group transform socket identity drift")
    if expected_kind == "cross-torso" and slot == "torso":
        raise ImportFailure("cross-torso preparation unexpectedly contains a torso slot")
    _validate_prepared_matrix(group.get("prepared_matrix"), "assembly group transform")
    residual = group.get("residual")
    if not isinstance(residual, (int, float)) or not math.isfinite(residual) or not (0.0 <= residual <= contract["residual_limit"] + 1.0e-12):
        raise ImportFailure("assembly group transform residual exceeds 0.025")
    if group["runtime_group"] != "torso" and group["runtime_group"] not in manifest["expected_groups"]:
        raise ImportFailure("assembly group transform is absent from the source OBJ groups")
    if group["runtime_group"] == "torso":
        evidence = group.get("socket_evidence")
        if not isinstance(evidence, list) or [item.get("socket") for item in evidence] != PREPARATION_SOCKET_ORDER[:6]:
            raise ImportFailure("torso group transform must deduplicate six socket evidences")
        for item in evidence:
            if (
                not isinstance(item, dict)
                or item.get("socket") not in PREPARATION_SOCKET_ORDER[:6]
                or not all(
                    isinstance(value, (int, float)) and math.isfinite(value)
                    for vector in (
                        item.get("source_anchor"),
                        item.get("target_anchor"),
                        item.get("transformed_source_anchor"),
                    )
                    if isinstance(vector, list) and len(vector) == 3
                    for value in vector
                )
                or not all(
                    isinstance(value, (int, float)) and math.isfinite(value)
                    for value in item.get("source_anchor", [])
                )
                or len(item.get("source_anchor", [])) != 3
                or len(item.get("target_anchor", [])) != 3
                or len(item.get("transformed_source_anchor", [])) != 3
                or not isinstance(item.get("prepared_vertex_count"), int)
                or item["prepared_vertex_count"] <= 0
                or not isinstance(item.get("residual"), (int, float))
                or not math.isfinite(item["residual"])
                or item["residual"] > contract["residual_limit"] + 1.0e-12
            ):
                raise ImportFailure("torso socket evidence is invalid")
    return preparation_key(group)


def validate_preparation_metadata(
    recipe: dict, manifests: dict, *, require_cross_torso: bool = True
) -> dict:
    contract = validate_preparation_contract(recipe)
    assets = {asset["id"]: asset for asset in recipe["part_assets"]}
    families = {family["id"]: family for family in recipe["families"]}
    canonical_slots = []
    cross_slots = []
    canonical_groups = []
    cross_groups = []
    oracle_records = _preparation_oracle_records(recipe, manifests)
    for key in sorted(manifests):
        manifest = manifests[key]
        asset = assets.get(manifest.get("asset_id"))
        if asset is None:
            raise ImportFailure("assembly preparation references an unknown asset")
        if (
            manifest.get("assembly_preparation_schema") != contract["schema"]
            or manifest.get("assembly_preparation_schema_digest") != contract["schema_digest"]
            or not isinstance(manifest.get("cross_torso_preparations"), list)
        ):
            raise ImportFailure("socket manifest lacks the v2 assembly preparation contract")
        for expected_kind, records, output in (
            ("canonical", manifest.get("assembly_preparations", []), canonical_slots),
            ("cross-torso", manifest["cross_torso_preparations"], cross_slots),
        ):
            for slot_record in records:
                family = families.get(slot_record.get("family_id"))
                if family is None:
                    raise ImportFailure("assembly preparation references an unknown family")
                slot = asset["logical_slot"]
                canonical_target = family["parts"]["torso"]["asset_id"]
                target = slot_record.get("target_torso_asset_id")
                expected_targets = (
                    {canonical_target}
                    if expected_kind == "canonical"
                    else set(PREPARATION_TORSO_ASSETS) - {canonical_target}
                )
                if (
                    slot_record.get("preparation_kind") != expected_kind
                    or slot_record.get("source_asset_id") != asset["id"]
                    or slot_record.get("asset_id") != asset["id"]
                    or slot_record.get("lod") != manifest["lod"]
                    or target not in expected_targets
                    or (expected_kind == "cross-torso" and slot == "torso")
                    or len(slot_record.get("group_transforms", []))
                    != len(PREPARATION_SLOT_GROUPS[slot])
                ):
                    raise ImportFailure("assembly preparation slot identity or group count drift")
                oracle_key = (
                    expected_kind,
                    family["id"],
                    asset["id"],
                    target,
                    manifest["lod"],
                )
                expected_record = oracle_records.get(oracle_key)
                if expected_record is None:
                    raise ImportFailure("assembly preparation has no independent socket oracle")
                _validate_preparation_oracle(slot_record, expected_record)
                output.append(slot_record)
                for group in slot_record["group_transforms"]:
                    key_value = _validate_preparation_group(
                        recipe, manifest, asset, family, slot_record, group, expected_kind
                    )
                    (cross_groups if expected_kind == "cross-torso" else canonical_groups).append(
                        key_value
                    )
                group_keys = [preparation_key(group) for group in slot_record["group_transforms"]]
                if group_keys != sorted(group_keys, key=lambda value: preparation_sort_key(next(
                    group for group in slot_record["group_transforms"] if preparation_key(group) == value
                ))):
                    raise ImportFailure("assembly group transforms are not deterministically sorted")
        if asset["logical_slot"] == "torso" and manifest["cross_torso_preparations"]:
            raise ImportFailure("torso asset manifest contains cross-torso preparations")
    expected_cross_slots = 288 if require_cross_torso else 0
    expected_cross_groups = 432 if require_cross_torso else 0
    if len(canonical_slots) != 180 or len(cross_slots) != expected_cross_slots:
        raise ImportFailure(
            f"assembly preparation slot counts must be 180/{expected_cross_slots}; found {len(canonical_slots)}/{len(cross_slots)}"
        )
    if len(canonical_groups) != 252 or len(cross_groups) != expected_cross_groups:
        raise ImportFailure(
            f"assembly preparation group counts must be 252/{expected_cross_groups}; found {len(canonical_groups)}/{len(cross_groups)}"
        )
    if len(set(canonical_groups)) != 252 or len(set(cross_groups)) != expected_cross_groups:
        raise ImportFailure("assembly preparation group keys contain duplicates")
    expected_total_groups = 684 if require_cross_torso else 252
    if len(set(canonical_groups + cross_groups)) != expected_total_groups:
        raise ImportFailure(
            f"assembly preparation group keys do not form {expected_total_groups} unique records"
        )
    by_source = {}
    matrices_by_source = {}
    for group in canonical_groups + cross_groups:
        fields = group.split("|")
        if fields[4] == "torso":
            continue
        base = "|".join(fields[:2] + fields[3:])
        by_source.setdefault(base, {})[fields[2]] = group
        matrices_by_source.setdefault(base, {})[fields[2]] = tuple(
            next(
                candidate["prepared_matrix"]
                for manifest in manifests.values()
                for records in (
                    manifest["assembly_preparations"],
                    manifest["cross_torso_preparations"],
                )
                for slot_record in records
                for candidate in slot_record["group_transforms"]
                if preparation_key(candidate) == group
            )
        )
    expected_target_count = 3 if require_cross_torso else 1
    expected_targets = set(PREPARATION_TORSO_ASSETS) if require_cross_torso else None
    for base, targets in by_source.items():
        if expected_targets is not None and set(targets) != expected_targets:
            raise ImportFailure("source group does not resolve against all three torso targets")
        if len(set(targets.values())) != expected_target_count:
            raise ImportFailure("target torso identity was collapsed into a shared group key")
        matrices = matrices_by_source[base]
        if require_cross_torso and len(set(matrices.values())) != len(PREPARATION_TORSO_ASSETS):
            raise ImportFailure("assembly preparation matrix aliases target torso IDs")
    return {
        "canonical_slot_records": len(canonical_slots),
        "cross_torso_slot_records": len(cross_slots),
        "canonical_group_keys": len(canonical_groups),
        "cross_torso_group_keys": len(cross_groups),
        "total_group_keys": len(set(canonical_groups + cross_groups)),
        "canonical_keys": sorted(canonical_groups),
        "cross_torso_keys": sorted(cross_groups),
    }


def generated_sockets(recipe: dict, asset: dict, markers: dict, bounds) -> dict:
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
    if "tail-base" not in sockets and asset["logical_slot"] == "torso":
        torso_marker = markers[semantic_to_id["torso"]]
        sockets["tail-base"] = {
            "translation": [torso_marker[0], torso_marker[1], bounds[1][2]],
            "rotation_xyzw": [0.0, 0.0, 0.0, 1.0],
            "scale": [1.0, 1.0, 1.0],
            "overlap_depth": recipe["assembly_contract"]["default_overlap_depth"],
            "allowable_scale_ratio": [0.88, 1.12],
            "pattern_phase_anchor": [0.0, 0.0],
            "derived_from_marker_ids": [2],
        }
    return sockets


def socket_manifest(
    recipe: dict,
    asset: dict,
    donor: str,
    lod: str,
    markers: dict,
    bounds,
    ground_contacts,
    mask_path: str,
    anatomy_path: str,
    topology: dict,
    microdetail_files: list[str],
    bridge_geometry: list[dict],
) -> dict:
    marker_map = {int(key): value for key, value in recipe["marker_map"].items()}
    sockets = generated_sockets(recipe, asset, markers, bounds)
    landmarks = {
        semantic: list(markers[marker_id])
        for marker_id, semantic in marker_map.items()
        if marker_id in markers
    }
    landmarks.update(
        {name: list(position) for name, position in asset["landmarks"].items()}
    )
    return {
        "schema": "alife.creature_part_sockets.v2",
        "asset_id": asset["id"],
        "logical_slot": asset["logical_slot"],
        "donor": donor,
        "lod": lod,
        "coordinate_frame": {"handedness": "right", "up": "+Y", "forward": "-Z"},
        "bounds": {"min": bounds[0], "max": bounds[1]},
        "sockets": sockets,
        "landmarks": landmarks,
        "ground_contacts": ground_contacts,
        "semantic_mask": mask_path,
        "anatomy_mask": anatomy_path,
        "lod_topology": topology,
        "expected_groups": sorted(
            {
                semantic_group(asset, name)
                for name in asset["selector"]["include_objects"]
            }
        ),
        "microdetail": {
            "source_files": microdetail_files,
            "uvless_fallback": "evaluated-normal-curvature-material-output",
        },
        "assembly_preparation_schema": recipe["assembly_preparation_contract"]["schema"],
        "assembly_preparation_schema_digest": recipe["assembly_preparation_contract"]["schema_digest"],
        "bridge_geometry": bridge_geometry,
        "assembly_preparations": [],
        "cross_torso_preparations": [],
    }


def ground_contacts(asset: dict, grouped: dict) -> list[list[float]]:
    if asset["logical_slot"] != "legs":
        return []
    contacts = []
    for group in ("left-leg", "right-leg"):
        points = [corner[0] for triangle in grouped[group] for corner in triangle]
        minimum_y = min(point[1] for point in points)
        planted = [point for point in points if point[1] <= minimum_y + 1.0e-6]
        contacts.append(
            [
                sum(point[0] for point in planted) / len(planted),
                minimum_y,
                sum(point[2] for point in planted) / len(planted),
            ]
        )
    return contacts


def geometry_bounds(grouped: dict) -> list[list[float]]:
    points = [
        corner[0]
        for triangles in grouped.values()
        for triangle in triangles
        for corner in triangle
    ]
    return [
        [min(point[axis] for point in points) for axis in range(3)],
        [max(point[axis] for point in points) for axis in range(3)],
    ]


def staged_output_path(staging: Path, relative: str) -> Path:
    raw = _relative_staged_path(relative)
    root = _canonical_staging_root(staging)
    parent = root
    for component in raw.parent.parts:
        parent = parent / component
        if _is_symlink_or_reparse(parent):
            raise ImportFailure(
                f"generated output parent contains a symlink or reparse point: {relative}"
            )
        try:
            parent.lstat()
        except FileNotFoundError:
            try:
                parent.mkdir(exist_ok=True)
            except OSError as exc:
                raise ImportFailure(
                    f"could not create generated output parent {relative}: {exc}"
                ) from exc
        if _is_symlink_or_reparse(parent):
            raise ImportFailure(
                f"generated output parent contains a symlink or reparse point: {relative}"
            )
        if not parent.is_dir():
            raise ImportFailure(
                f"generated output parent is not a directory: {relative}"
            )
        canonical_parent = parent.resolve(strict=True)
        if not canonical_path_is_within(root, canonical_parent):
            raise ImportFailure(f"generated output escapes canonical staging: {relative}")
    path = root / raw
    if _is_symlink_or_reparse(path):
        raise ImportFailure(
            f"generated output contains a symlink or reparse point: {relative}"
        )
    try:
        path.lstat()
    except FileNotFoundError:
        return path
    canonical = path.resolve(strict=True)
    if not canonical_path_is_within(root, canonical):
        raise ImportFailure(f"generated output escapes canonical staging: {relative}")
    return canonical


def build_scene_outputs(
    bpy,
    recipe: dict,
    donor: str,
    markers: dict,
    staging: Path,
    microdetail_root: Path,
) -> dict:
    asset_geometry, topology, texture_files = extract_geometry(bpy, recipe, donor)
    asset_geometry, markers, bounds = normalization(asset_geometry, markers)
    source_samples, microdetail_files = load_microdetail_samples(
        bpy, microdetail_root, donor
    )
    detail_source_files = sorted(set(texture_files) | set(microdetail_files))
    outputs = []
    assets = selected_assets(recipe, donor)
    for asset in assets:
        grouped = asset_geometry[asset["id"]]
        lod_contracts = {lod["lod"]: lod for lod in asset["lods"]}
        previous_topology = None
        for lod, ratio, triangle_budget in LODS:
            try:
                lod_grouped, lod_topology = decimate_asset(
                    bpy, grouped, ratio, triangle_budget
                )
            except ImportFailure as error:
                raise ImportFailure(
                    f"{donor} asset {asset['id']} LOD {lod}: {error}"
                ) from error
            if previous_topology is not None:
                if lod_topology["triangle_count"] >= previous_topology["triangle_count"]:
                    raise ImportFailure(
                        f"{donor} asset {asset['id']} LOD {lod}: triangle count did not decrease strictly"
                    )
                for component, count in lod_topology[
                    "component_connected_counts"
                ].items():
                    if count > previous_topology["component_connected_counts"][component]:
                        raise ImportFailure(
                            f"{donor} asset {asset['id']} LOD {lod}: source-object islands increased"
                        )
            previous_topology = lod_topology
            contract = lod_contracts[lod]
            initial_bounds = geometry_bounds(lod_grouped)
            sockets = generated_sockets(recipe, asset, markers, initial_bounds)
            lod_grouped, bridge_geometry = prepare_bridge_overlap_geometry(
                lod_grouped,
                sockets,
                recipe["assembly_contract"]["slot_sockets"][asset["logical_slot"]],
                recipe["assembly_contract"]["default_overlap_depth"],
            )
            lod_bounds = geometry_bounds(lod_grouped)
            contacts = ground_contacts(asset, lod_grouped)
            obj_path = staged_output_path(staging, contract["generated_obj"])
            socket_path = staged_output_path(staging, contract["socket_manifest"])
            mask_path = staged_output_path(staging, contract["semantic_mask"])
            anatomy_path = staged_output_path(staging, contract["anatomy_mask"])
            obj_bytes = emit_obj(lod_grouped)
            obj_path.write_bytes(obj_bytes)
            semantic_bytes = semantic_mask(lod_grouped, source_samples)
            mask_path.write_bytes(semantic_bytes)
            anatomy_path.write_bytes(
                anatomy_mask(
                    semantic_bytes,
                    obj_bytes,
                    asset["anatomy_authoring"],
                    asset["logical_slot"],
                )
            )
            socket_path.write_text(
                json.dumps(
                    socket_manifest(
                        recipe,
                        asset,
                        donor,
                        lod,
                        markers,
                        lod_bounds,
                        contacts,
                        contract["semantic_mask"],
                        contract["anatomy_mask"],
                        lod_topology,
                        detail_source_files,
                        bridge_geometry,
                    ),
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            outputs.extend((obj_path, socket_path, mask_path, anatomy_path))
    return {
        "donor": donor,
        "topology": topology,
        "asset_count": len(assets),
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
        payload = build_scene_outputs(
            bpy,
            recipe,
            args.donor,
            markers,
            args.staging,
            args.microdetail_root,
        )
        payload["relinked_images"] = inventory["relinked_images"]
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
