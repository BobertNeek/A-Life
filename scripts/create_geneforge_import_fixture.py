#!/usr/bin/env python3
"""Create disposable GeneForge-like Blender sources for importer contract tests."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


WORKSPACE = Path(__file__).resolve().parents[1]
DEFAULT_BLENDER = Path(r"C:\Program Files\Blender Foundation\Blender 5.1\blender.exe")
PRODUCTION_RECIPE = (
    WORKSPACE
    / "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json"
)
VARIANTS = (
    "valid",
    "alternate-texture",
    "binary-microdetail",
    "broken-texture",
    "duplicate-marker",
    "invalid-marker",
    "nonempty-marker",
    "selector-mismatch",
    "zero-marker",
    "evaluated-empty",
)


def canonical_digest(recipe: dict) -> str:
    canonical = dict(recipe)
    canonical["recipe_sha256"] = "0" * 64
    payload = json.dumps(
        canonical, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(payload).hexdigest()


def write_variant_recipes(output: Path) -> None:
    template = json.loads(PRODUCTION_RECIPE.read_text(encoding="utf-8"))
    for variant in VARIANTS:
        root = output / variant
        recipe = json.loads(json.dumps(template))
        for source in recipe["sources"]:
            blend = root / Path(source["blend_file"])
            source["sha256"] = hashlib.sha256(blend.read_bytes()).hexdigest().upper()
        recipe["recipe_sha256"] = canonical_digest(recipe)
        (root / "fixture_recipes.json").write_text(
            json.dumps(recipe, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
        )


def launch_blender(output: Path, blender: Path) -> None:
    command = [
        str(blender),
        "--background",
        "--factory-startup",
        "--python-exit-code",
        "23",
        "--python",
        str(Path(__file__).resolve()),
        "--",
        "--worker",
        "--output",
        str(output),
    ]
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    if completed.returncode:
        sys.stderr.write(completed.stdout)
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)


def outer_main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--blender-exe",
        type=Path,
        default=Path(os.environ.get("BLENDER_EXE", DEFAULT_BLENDER)),
    )
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    launch_blender(output, args.blender_exe)
    write_variant_recipes(output)
    print(f"fixture_root={output}")


def blender_arguments() -> argparse.Namespace:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true", required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def cube_mesh(bpy, name: str, offset: float):
    vertices = [
        (-0.12 + offset, -0.12, -0.12),
        (0.12 + offset, -0.12, -0.12),
        (0.12 + offset, 0.12, -0.12),
        (-0.12 + offset, 0.12, -0.12),
        (-0.12 + offset, -0.12, 0.12),
        (0.12 + offset, -0.12, 0.12),
        (0.12 + offset, 0.12, 0.12),
        (-0.12 + offset, 0.12, 0.12),
        (0.0 + offset, 0.0, 0.0),
        (0.1 + offset, 0.0, 0.0),
        (0.2 + offset, 0.0, 0.0),
    ]
    faces = [
        (0, 1, 2, 3),
        (4, 7, 6, 5),
        (0, 4, 5, 1),
        (1, 5, 6, 2),
        (2, 6, 7, 3),
        (4, 0, 3, 7),
        (8, 9, 10),
    ]
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    return mesh


def add_geometry_nodes_mirror(bpy, obj, *, empty: bool = False) -> None:
    group = bpy.data.node_groups.new(obj.name + "MirrorGeometry", "GeometryNodeTree")
    group.interface.new_socket(name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry")
    group.interface.new_socket(name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry")
    nodes = group.nodes
    links = group.links
    group_in = nodes.new("NodeGroupInput")
    group_out = nodes.new("NodeGroupOutput")
    if not empty:
        transform = nodes.new("GeometryNodeTransform")
        transform.inputs["Scale"].default_value = (-1.0, 1.0, 1.0)
        join = nodes.new("GeometryNodeJoinGeometry")
        links.new(group_in.outputs["Geometry"], join.inputs["Geometry"])
        links.new(group_in.outputs["Geometry"], transform.inputs["Geometry"])
        links.new(transform.outputs["Geometry"], join.inputs["Geometry"])
        links.new(join.outputs["Geometry"], group_out.inputs["Geometry"])
    modifier = obj.modifiers.new("Fixture Mirror Geometry Nodes", "NODES")
    modifier.node_group = group


def add_armature(bpy, obj) -> None:
    armature = bpy.data.armatures.new("FixtureArmatureData")
    rig = bpy.data.objects.new("FixtureArmature", armature)
    bpy.context.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bone = armature.edit_bones.new("FixtureBone")
    bone.head = (0.0, -0.2, 0.0)
    bone.tail = (0.0, 0.2, 0.0)
    bpy.ops.object.mode_set(mode="OBJECT")
    rig.select_set(False)
    group = obj.vertex_groups.new(name="FixtureBone")
    group.add(range(len(obj.data.vertices)), 1.0, "REPLACE")
    modifier = obj.modifiers.new("Fixture Armature", "ARMATURE")
    modifier.object = rig
    rig.pose.bones["FixtureBone"].location = (0.0, 0.0, 0.18)


def inject_topology_hazard(bmesh, obj, repairs: list[str]) -> None:
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.verts.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    if "repair-declared-non-manifold-edges" in repairs:
        first, second = bm.verts[0], bm.verts[1]
        extra = bm.verts.new((0.0, -0.3, 0.0))
        bm.faces.new((first, second, extra))
    if "repair-declared-boundary-edges" in repairs:
        bmesh.ops.delete(bm, geom=[bm.faces[0]], context="FACES_ONLY")
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()


def make_source(bpy, recipe: dict, root: Path, donor: str, variant: str) -> None:
    import bmesh

    bpy.ops.wm.read_factory_settings(use_empty=True)
    source = next(item for item in recipe["sources"] if item["donor"] == donor)
    texture_root = root / Path(source["texture_root"])
    microdetail_root = root / Path(source["microdetail_root"])
    texture_root.mkdir(parents=True, exist_ok=True)
    microdetail_root.mkdir(parents=True, exist_ok=True)
    texture_path = texture_root / "fixture_fur.png"
    microdetail_path = microdetail_root / "fixture_fur_detail.png"
    generated = bpy.data.images.new("fixture_fur-generated", width=4, height=4)
    donor_bias = {"norn": 0.12, "ettin": 0.36, "grendel": 0.60}[donor]
    texture_pixels = []
    microdetail_pixels = []
    for y in range(4):
        for x in range(4):
            detail = (donor_bias + x * 0.11 + y * 0.07) % 1.0
            if variant == "alternate-texture":
                texture_detail = 1.0 - detail
            else:
                texture_detail = detail
            texture_pixels.extend(
                (
                    texture_detail,
                    texture_detail * 0.7,
                    1.0 - texture_detail * 0.5,
                    1.0,
                )
            )
            microdetail = float((x + y) % 2) if variant == "binary-microdetail" else detail
            microdetail_pixels.extend((microdetail, microdetail, microdetail, 1.0))
    generated.pixels = texture_pixels
    generated.filepath_raw = str(texture_path)
    generated.file_format = "PNG"
    generated.save()
    generated.pixels = microdetail_pixels
    generated.filepath_raw = str(microdetail_path)
    generated.save()
    bpy.data.images.remove(generated)
    image = bpy.data.images.load(str(texture_path), check_existing=False)
    image.filepath = r"Z:\stale\fixture_fur.png"
    if variant == "broken-texture":
        texture_path.unlink()

    donor_assets = [asset for asset in recipe["part_assets"] if asset["donor"] == donor]
    object_names = []
    for asset in donor_assets:
        object_names.extend(asset["selector"]["include_objects"])
    object_names = list(dict.fromkeys(object_names))
    created = {}
    for index, name in enumerate(object_names):
        obj = bpy.data.objects.new(name, cube_mesh(bpy, name, index * 0.28))
        bpy.context.collection.objects.link(obj)
        obj["kc3dsbpy_visscript"] = next(
            (
                asset["selector"].get("object_visscripts", {}).get(name, "part=Fixture")
                for asset in donor_assets
                if name in asset["selector"]["include_objects"]
            ),
            "part=Fixture",
        )
        if variant == "selector-mismatch" and index == 0:
            obj["kc3dsbpy_visscript"] = "part=Unexpected&review=must-fail"
        uv_name = next(
            asset["selector"]["uv_map"]
            for asset in donor_assets
            if name in asset["selector"]["include_objects"]
        )
        has_uv_fallback = any(
            name in asset["selector"].get("uv_fallbacks", {})
            for asset in donor_assets
            if name in asset["selector"]["include_objects"]
        )
        if not has_uv_fallback:
            uv_layer = obj.data.uv_layers.new(name=uv_name)
            face_uvs = ((0.08, 0.08), (0.92, 0.08), (0.92, 0.92), (0.08, 0.92))
            for polygon in obj.data.polygons:
                for corner_index, loop_index in enumerate(polygon.loop_indices):
                    uv_layer.data[loop_index].uv = face_uvs[corner_index % len(face_uvs)]
        material = bpy.data.materials.new(name + "Material")
        material.use_nodes = True
        node = material.node_tree.nodes.new("ShaderNodeTexImage")
        node.image = image
        obj.data.materials.append(material)
        created[name] = obj

    for name, value in source["audited_non_marker_properties"].items():
        non_marker = bpy.data.objects.new(
            name,
            cube_mesh(bpy, name, len(object_names) * 0.28),
        )
        non_marker["kc3dsbpy_part_marker"] = value
        bpy.context.collection.objects.link(non_marker)

    marker_names = {int(key): value for key, value in recipe["marker_map"].items()}
    donor_marker_ids = range(1, 13) if donor == "ettin" else range(1, 15)
    for marker_id in donor_marker_ids:
        semantic = marker_names[marker_id]
        marker = bpy.data.objects.new(f"marker-{marker_id:02}-{semantic}", None)
        marker.empty_display_type = "PLAIN_AXES"
        marker.location = (
            ((marker_id % 3) - 1) * 0.1,
            marker_id * 0.01,
            marker_id * 0.005,
        )
        if variant == "invalid-marker" and marker_id == 14:
            marker["kc3dsbpy_part_marker"] = 99
        elif variant == "zero-marker" and donor == "norn" and marker_id == 14:
            marker["kc3dsbpy_part_marker"] = 0
        else:
            marker["kc3dsbpy_part_marker"] = marker_id
        bpy.context.collection.objects.link(marker)
    if variant == "duplicate-marker" and donor == "norn":
        duplicate = bpy.data.objects.new("duplicate-marker-01-head", None)
        duplicate["kc3dsbpy_part_marker"] = 1
        duplicate.location = (0.75, 0.0, 0.0)
        bpy.context.collection.objects.link(duplicate)

    if variant == "nonempty-marker" and donor == "norn":
        created[object_names[-1]]["kc3dsbpy_part_marker"] = 1

    first = created[object_names[0]]
    constraint = first.constraints.new("COPY_LOCATION")
    constraint.name = "Fixture Copy Location"
    constraint.target = bpy.data.objects["marker-01-head"]
    constraint.influence = 1.0
    add_armature(bpy, first)
    mirror_target = created[object_names[1]]
    add_geometry_nodes_mirror(
        bpy,
        mirror_target,
        empty=variant == "evaluated-empty" and donor == "ettin",
    )

    declared_repairs = {
        name: repairs
        for asset in donor_assets
        for name, repairs in asset["selector"].get("topology_repairs", {}).items()
        if name in created
    }
    for repair_target, repairs in declared_repairs.items():
        inject_topology_hazard(bmesh, created[repair_target], repairs)
        created[repair_target]["alife_declared_topology_repair"] = ",".join(repairs)

    loose_target = created[next(iter(declared_repairs), object_names[-1])]
    mesh = loose_target.data
    mesh.vertices.add(2)
    mesh.vertices[-2].co = (2.0, 2.0, 2.0)
    mesh.vertices[-1].co = (2.2, 2.0, 2.0)
    mesh.edges.add(1)
    mesh.edges[-1].vertices = (len(mesh.vertices) - 2, len(mesh.vertices) - 1)

    blend_path = root / Path(source["blend_file"])
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path), check_existing=False)


def worker_main() -> None:
    import bpy

    args = blender_arguments()
    output = args.output.resolve()
    recipe = json.loads(PRODUCTION_RECIPE.read_text(encoding="utf-8"))
    for variant in VARIANTS:
        root = output / variant
        for donor in ("norn", "ettin", "grendel"):
            make_source(bpy, recipe, root, donor, variant)


if __name__ == "__main__":
    if "--worker" in sys.argv:
        worker_main()
    else:
        outer_main()
