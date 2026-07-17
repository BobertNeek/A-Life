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
VARIANTS = ("valid", "broken-texture", "invalid-marker", "evaluated-empty")


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


def make_source(bpy, recipe: dict, root: Path, donor: str, variant: str) -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    source = next(item for item in recipe["sources"] if item["donor"] == donor)
    texture_root = root / Path(source["texture_root"])
    texture_root.mkdir(parents=True, exist_ok=True)
    texture_path = texture_root / "fixture_fur.png"
    generated = bpy.data.images.new("fixture_fur-generated", width=2, height=2)
    generated.generated_color = (0.3, 0.6, 0.9, 1.0)
    generated.filepath_raw = str(texture_path)
    generated.file_format = "PNG"
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
        obj = bpy.data.objects.new(name, cube_mesh(bpy, name, (index % 5) * 0.28))
        bpy.context.collection.objects.link(obj)
        obj["kc3dsbpy_visscript"] = next(
            (
                asset["selector"].get("object_visscripts", {}).get(name, "part=Fixture")
                for asset in donor_assets
                if name in asset["selector"]["include_objects"]
            ),
            "part=Fixture",
        )
        uv_name = next(
            asset["selector"]["uv_map"]
            for asset in donor_assets
            if name in asset["selector"]["include_objects"]
        )
        obj.data.uv_layers.new(name=uv_name)
        material = bpy.data.materials.new(name + "Material")
        material.use_nodes = True
        node = material.node_tree.nodes.new("ShaderNodeTexImage")
        node.image = image
        obj.data.materials.append(material)
        created[name] = obj

    marker_names = {int(key): value for key, value in recipe["marker_map"].items()}
    for marker_id, semantic in marker_names.items():
        marker = bpy.data.objects.new(f"marker-{marker_id:02}-{semantic}", None)
        marker.empty_display_type = "PLAIN_AXES"
        marker.location = (((marker_id % 3) - 1) * 0.1, 0.0, 0.0)
        marker["kc3dsbpy_part_marker"] = 99 if variant == "invalid-marker" and marker_id == 14 else marker_id
        bpy.context.collection.objects.link(marker)

    first = created[object_names[0]]
    constraint = first.constraints.new("COPY_LOCATION")
    constraint.name = "Fixture Copy Location"
    constraint.target = bpy.data.objects["marker-01-head"]
    constraint.influence = 0.0
    add_armature(bpy, first)
    mirror_target = created[object_names[1]]
    add_geometry_nodes_mirror(
        bpy,
        mirror_target,
        empty=variant == "evaluated-empty" and donor == "ettin",
    )

    repair_target = next(
        (
            name
            for asset in donor_assets
            for name in asset["selector"].get("topology_repairs", {})
            if name in created
        ),
        object_names[-1],
    )
    mesh = created[repair_target].data
    mesh.vertices.add(2)
    mesh.vertices[-2].co = (2.0, 2.0, 2.0)
    mesh.vertices[-1].co = (2.2, 2.0, 2.0)
    mesh.edges.add(1)
    mesh.edges[-1].vertices = (len(mesh.vertices) - 2, len(mesh.vertices) - 1)
    created[repair_target]["alife_declared_topology_repair"] = "repair-declared-non-manifold-edges"

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
