#!/usr/bin/env python3
"""Contract tests for the deterministic GeneForge importer and recipe catalog."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path, PurePosixPath
import shutil
import struct
import subprocess
import sys
import unittest
from unittest import mock
import zlib


IMPORTER_MODULE_SPEC = importlib.util.spec_from_file_location(
    "build_geneforge_creature_parts", Path(__file__).with_name("build_geneforge_creature_parts.py")
)
assert IMPORTER_MODULE_SPEC is not None and IMPORTER_MODULE_SPEC.loader is not None
importer = importlib.util.module_from_spec(IMPORTER_MODULE_SPEC)
IMPORTER_MODULE_SPEC.loader.exec_module(importer)


WORKSPACE = Path(__file__).resolve().parents[1]
RECIPE_PATH = (
    WORKSPACE
    / "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json"
)
EXPECTED_SOURCE_HASHES = {
    "norn": "B6E5C1BC0E0EC69995748B211F45EFF787B9162DBC4856A1AB7F48F3E610FB4A",
    "ettin": "CC1D2AA1D310BCEA3D39FE495BF756A9B3650ECF0F3C9EEE8AC8488609202B0B",
    "grendel": "3289BBD6D7CAEDF7CCA44175E63B60B4140D26EE2E86CCAD7A89FA8724132E62",
}
EXPECTED_MICRODETAIL_ROOTS = {
    "norn": "Norn/Alpha Textures",
    "ettin": "Ettin/Alpha Textures",
    "grendel": "Grendel/Alpha Textures",
}
EXPECTED_MARKERS = {
    1: "head",
    2: "torso",
    3: "left-thigh",
    4: "left-shin",
    5: "left-foot",
    6: "right-thigh",
    7: "right-shin",
    8: "right-foot",
    9: "left-upper-arm",
    10: "left-lower-arm",
    11: "right-upper-arm",
    12: "right-lower-arm",
    13: "tail-root",
    14: "tail-tip",
}
EXPECTED_GROUP_COLORS = {
    "head": (230, 92, 88),
    "torso": (64, 166, 184),
    "left-arm": (244, 177, 76),
    "right-arm": (244, 177, 76),
    "left-leg": (95, 177, 104),
    "right-leg": (95, 177, 104),
    "tail-back": (154, 108, 180),
    "head.eyes": (238, 238, 224),
    "head.lids": (184, 80, 96),
    "head.hair": (114, 84, 145),
    "head.teeth": (235, 222, 188),
    "head.tongue": (213, 92, 126),
}
EXPECTED_ANATOMY_COLORS = {
    "primary": (248, 248, 248),
    "belly": (232, 176, 72),
    "muzzle": (226, 112, 128),
    "inner-ear": (238, 86, 154),
    "hands-feet": (72, 174, 218),
    "keratin-skin": (64, 52, 72),
    "secondary-marking": (84, 92, 214),
}
REQUIRED_ANATOMY_CHANNELS = {
    "head": {"primary", "muzzle", "inner-ear", "keratin-skin", "secondary-marking"},
    "torso": {"primary", "belly", "secondary-marking"},
    "arms": {"primary", "hands-feet", "secondary-marking"},
    "legs": {"primary", "hands-feet", "secondary-marking"},
    "tail": {"primary", "keratin-skin", "secondary-marking"},
}
REQUIRED_FEATURE_ANCHORS = {
    "head": {
        "left-ear": ("inner-ear", "head"),
        "right-ear": ("inner-ear", "head"),
        "muzzle": ("muzzle", "head"),
    },
    "torso": {"belly": ("belly", "torso")},
    "arms": {
        "left-hand": ("hands-feet", "left-arm"),
        "right-hand": ("hands-feet", "right-arm"),
    },
    "legs": {
        "left-foot": ("hands-feet", "left-leg"),
        "right-foot": ("hands-feet", "right-leg"),
    },
    "tail": {"tail-tip": ("keratin-skin", "tail-back")},
}
AUDITED_NEVER_OR_INVALID = {
    "ear_4L_chichi",
    "Ear_4L_civet",
    "tailtip_m_UNUSED",
    "extra_spike1",
    "extra_spike2",
    "RigLowerJaw",
}
IMPORTER = WORKSPACE / "scripts/build_geneforge_creature_parts.py"
FIXTURE_GENERATOR = WORKSPACE / "scripts/create_geneforge_import_fixture.py"
FIXTURE_ROOT = WORKSPACE / "target/artifacts/geneforge-import-fixture"
TEST_OUTPUT = WORKSPACE / "target/artifacts/geneforge-import-tests"


def load_recipe() -> dict:
    return json.loads(RECIPE_PATH.read_text(encoding="utf-8"))


def canonical_recipe_digest(recipe: dict) -> str:
    canonical = dict(recipe)
    canonical["recipe_sha256"] = "0" * 64
    payload = json.dumps(
        canonical, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(payload).hexdigest()


def png_with_chunks(chunks: list[tuple[bytes, bytes]]) -> bytes:
    output = bytearray(b"\x89PNG\r\n\x1a\n")
    for kind, payload in chunks:
        output.extend(struct.pack(">I", len(payload)))
        output.extend(kind)
        output.extend(payload)
        output.extend(struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF))
    return bytes(output)


def rgba8_ihdr(width: int = 64, height: int = 64) -> bytes:
    return struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)


def read_rgba_png(path: Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise AssertionError(f"{path} is not a PNG")
    offset = 8
    width = height = None
    compressed = bytearray()
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        offset += 12 + length
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", payload
            )
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise AssertionError(f"{path} is not an unfiltered RGBA8 PNG")
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
    if width is None or height is None:
        raise AssertionError(f"{path} is missing IHDR")
    raw = zlib.decompress(bytes(compressed))
    stride = width * 4
    pixels = bytearray()
    for row in range(height):
        start = row * (stride + 1)
        if raw[start] != 0:
            raise AssertionError(f"{path} uses unsupported PNG row filtering")
        pixels.extend(raw[start + 1 : start + 1 + stride])
    return width, height, bytes(pixels)


def obj_topology(path: Path) -> dict:
    positions = []
    normals = []
    faces = []
    group = None
    component = None
    for line in path.read_text(encoding="ascii").splitlines():
        if line.startswith("v "):
            positions.append(tuple(float(value) for value in line.split()[1:4]))
        elif line.startswith("vn "):
            normals.append(tuple(float(value) for value in line.split()[1:4]))
        elif line.startswith("g "):
            group = line[2:]
        elif line.startswith("o "):
            component = line[2:]
        elif line.startswith("f "):
            references = []
            for field in line.split()[1:]:
                position, _, normal = (int(value) - 1 for value in field.split("/"))
                references.append((position, normal))
            faces.append((group, component, references))
    position_keys = [tuple(round(value, 6) for value in position) for position in positions]
    edges = {}
    vertex_faces = {}
    face_normals = []
    component_triangle_counts = {}
    for face_index, (_, component, references) in enumerate(faces):
        if component is not None:
            component_triangle_counts[component] = (
                component_triangle_counts.get(component, 0) + 1
            )
        points = [positions[position] for position, _ in references]
        a = tuple(points[1][axis] - points[0][axis] for axis in range(3))
        b = tuple(points[2][axis] - points[0][axis] for axis in range(3))
        cross = (
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        )
        length = math.sqrt(sum(value * value for value in cross))
        face_normals.append(tuple(value / length for value in cross))
        for position, _ in references:
            vertex_faces.setdefault(position, []).append(face_index)
        for first, second in ((0, 1), (1, 2), (2, 0)):
            edge = tuple(sorted((position_keys[references[first][0]], position_keys[references[second][0]])))
            edges.setdefault(edge, []).append(face_index)
    adjacency = [set() for _ in faces]
    for linked in edges.values():
        for face in linked:
            adjacency[face].update(other for other in linked if other != face)
    components = 0
    component_connected_counts = {}
    unseen = set(range(len(faces)))
    while unseen:
        components += 1
        first = unseen.pop()
        pending = [first]
        connected_faces = [first]
        while pending:
            for neighbor in adjacency[pending.pop()]:
                if neighbor in unseen:
                    unseen.remove(neighbor)
                    pending.append(neighbor)
                    connected_faces.append(neighbor)
        declared = {faces[index][1] for index in connected_faces}
        if len(declared) != 1:
            raise AssertionError("connected OBJ island crosses declared component IDs")
        declared_component = next(iter(declared))
        component_connected_counts[declared_component] = (
            component_connected_counts.get(declared_component, 0) + 1
        )
    smooth_shared = 0
    for position, linked in vertex_faces.items():
        if len(linked) < 2:
            continue
        incident = [face_normals[index] for index in linked]
        if any(
            sum(a * b for a, b in zip(incident[0], other)) < 0.98
            for other in incident[1:]
        ):
            referenced_normals = {
                normal
                for face_index in linked
                for candidate, normal in faces[face_index][2]
                if candidate == position
            }
            if len(referenced_normals) == 1:
                smooth_shared += 1
    return {
        "faces": len(faces),
        "components": components,
        "boundary_edges": sum(len(linked) == 1 for linked in edges.values()),
        "non_manifold_edges": sum(len(linked) > 2 for linked in edges.values()),
        "smooth_shared_vertices": smooth_shared,
        "normal_count": len(normals),
        "component_ids": set(component_triangle_counts),
        "component_triangle_counts": component_triangle_counts,
        "component_connected_counts": component_connected_counts,
    }


def _independent_barycentric(point, triangle):
    (px, py) = point
    (ax, ay), (bx, by), (cx, cy) = triangle
    denominator = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
    if abs(denominator) <= 1.0e-12:
        return None
    first = ((by - cy) * (px - cx) + (cx - bx) * (py - cy)) / denominator
    second = ((cy - ay) * (px - cx) + (ax - cx) * (py - cy)) / denominator
    return (first, second, 1.0 - first - second)


def _independent_closest_uv_weights(point, triangle):
    candidates = []
    for first, second, opposite in ((0, 1, 2), (1, 2, 0), (2, 0, 1)):
        start = triangle[first]
        end = triangle[second]
        edge = (end[0] - start[0], end[1] - start[1])
        length_squared = edge[0] * edge[0] + edge[1] * edge[1]
        if length_squared <= 1.0e-18:
            amount = 0.0
        else:
            amount = max(
                0.0,
                min(
                    1.0,
                    ((point[0] - start[0]) * edge[0] + (point[1] - start[1]) * edge[1])
                    / length_squared,
                ),
            )
        projected = (start[0] + edge[0] * amount, start[1] + edge[1] * amount)
        weights = [0.0, 0.0, 0.0]
        weights[first] = 1.0 - amount
        weights[second] = amount
        weights[opposite] = 0.0
        distance_squared = sum((a - b) ** 2 for a, b in zip(point, projected))
        candidates.append((distance_squared, tuple(weights)))
    return min(candidates, key=lambda candidate: (candidate[0], candidate[1]))


def independent_obj_texel_projection(obj_path: Path, semantic_path: Path) -> list[dict]:
    positions = []
    uvs = []
    triangles = []
    group = None
    for line in obj_path.read_text(encoding="ascii").splitlines():
        fields = line.split()
        if not fields:
            continue
        if fields[0] == "v":
            positions.append(tuple(float(value) for value in fields[1:4]))
        elif fields[0] == "vt":
            uvs.append(tuple(float(value) for value in fields[1:3]))
        elif fields[0] == "g":
            group = fields[1]
        elif fields[0] == "f":
            references = [field.split("/") for field in fields[1:4]]
            triangles.append(
                {
                    "face": len(triangles),
                    "group": group,
                    "positions": tuple(positions[int(reference[0]) - 1] for reference in references),
                    "uvs": tuple(uvs[int(reference[1]) - 1] for reference in references),
                }
            )
    groups_by_color: dict[tuple[int, int, int], set[str]] = {}
    for runtime_group, color in EXPECTED_GROUP_COLORS.items():
        groups_by_color.setdefault(color, set()).add(runtime_group)
    exact_bins: dict[tuple[str, int, int], list[dict]] = {}
    for triangle in triangles:
        minimum_x = max(
            0,
            math.ceil(min(uv[0] for uv in triangle["uvs"]) * 64 - 0.5 - 1.0e-9),
        )
        maximum_x = min(
            63,
            math.floor(max(uv[0] for uv in triangle["uvs"]) * 64 - 0.5 + 1.0e-9),
        )
        minimum_y = max(
            0,
            math.ceil(min(uv[1] for uv in triangle["uvs"]) * 64 - 0.5 - 1.0e-9),
        )
        maximum_y = min(
            63,
            math.floor(max(uv[1] for uv in triangle["uvs"]) * 64 - 0.5 + 1.0e-9),
        )
        for bin_y in range(minimum_y, maximum_y + 1):
            for bin_x in range(minimum_x, maximum_x + 1):
                exact_bins.setdefault((triangle["group"], bin_x, bin_y), []).append(
                    triangle
                )
    _, _, semantic = read_rgba_png(semantic_path)
    projected = []
    for y in range(64):
        for x in range(64):
            offset = (y * 64 + x) * 4
            if semantic[offset + 3] == 0:
                continue
            color = tuple(semantic[offset : offset + 3])
            runtime_groups = groups_by_color[color]
            texel_uv = ((x + 0.5) / 64.0, (y + 0.5) / 64.0)
            candidates = [triangle for triangle in triangles if triangle["group"] in runtime_groups]
            exact = []
            exact_candidates = [
                triangle
                for runtime_group in sorted(runtime_groups)
                for triangle in exact_bins.get((runtime_group, x, y), ())
            ]
            for triangle in exact_candidates:
                weights = _independent_barycentric(texel_uv, triangle["uvs"])
                if weights is not None and min(weights) >= -1.0e-9:
                    exact.append((-min(weights), triangle["face"], triangle, weights))
            if exact:
                _, _, triangle, weights = min(exact)
                mode = "inside"
            else:
                nearest = []
                for triangle in candidates:
                    distance_squared, closest = _independent_closest_uv_weights(
                        texel_uv, triangle["uvs"]
                    )
                    nearest.append(
                        (distance_squared, triangle["face"], triangle, closest)
                    )
                _, _, triangle, weights = min(nearest)
                mode = "nearest"
            point = tuple(
                sum(weights[corner] * triangle["positions"][corner][axis] for corner in range(3))
                for axis in range(3)
            )
            projected.append(
                {
                    "x": x,
                    "y": y,
                    "group": triangle["group"],
                    "face": triangle["face"],
                    "weights": weights,
                    "point": point,
                    "mode": mode,
                    "overlap": len(exact),
                }
            )
    return projected


def normalized_axis(value: float, values: list[float]) -> float:
    lower, upper = min(values), max(values)
    return 0.5 if upper - lower <= 1.0e-12 else (value - lower) / (upper - lower)


class GeneForgeRecipeContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.recipe = load_recipe()
        cls.assets = {asset["id"]: asset for asset in cls.recipe["part_assets"]}

    def test_exact_source_hashes(self) -> None:
        self.assertEqual(
            {source["donor"]: source["sha256"] for source in self.recipe["sources"]},
            EXPECTED_SOURCE_HASHES,
        )

    def test_outer_v2_contracts_require_source_projected_anatomy_v2(self) -> None:
        self.assertEqual(self.recipe["schema"], "alife.geneforge_creature_part_catalog.v2")
        self.assertEqual(self.recipe["schema_version"], 2)
        self.assertEqual(self.recipe["importer_version"], "alife.geneforge_importer.v2")
        self.assertEqual(len(self.recipe["part_assets"]), 14)
        for asset in self.recipe["part_assets"]:
            authoring = asset["anatomy_authoring"]
            self.assertEqual(authoring["schema"], "alife.geneforge_anatomy_authoring.v2")
            self.assertEqual(authoring["coordinate_space"], "same-lod-staged-obj")
            self.assertEqual(authoring["default_channel"], "primary")
            self.assertEqual(
                authoring["projection"]["schema"],
                "alife.geneforge_anatomy_projection.v1",
            )
            self.assertEqual(
                authoring["projection"]["texel_sample"], "pixel-center"
            )
            self.assertIn("source_geometry", authoring["projection"])
            source_geometry = authoring["projection"]["source_geometry"]
            self.assertEqual(
                source_geometry["schema"],
                "alife.geneforge_source_geometry_classifier.v2",
            )
            self.assertTrue(source_geometry["groups"])
            expected_anchors = REQUIRED_FEATURE_ANCHORS[asset["logical_slot"]]
            self.assertEqual(set(source_geometry["feature_landmarks"]), set(expected_anchors))
            for name, (channel, runtime_group) in expected_anchors.items():
                anchor = source_geometry["feature_landmarks"][name]
                self.assertEqual(anchor["channel"], channel)
                self.assertEqual(anchor["runtime_group"], runtime_group)
                self.assertEqual(anchor["source_group"], runtime_group)
                self.assertEqual(anchor["method"], "source-geometry-anchor-v1")
            self.assertEqual(
                set(source_geometry["landmarks"]),
                set(asset["landmarks"]),
            )
            self.assertEqual(
                authoring["projection"]["triangle_tie_break"],
                "inside-max-min-barycentric-then-face-index;nearest-uv-then-face-index",
            )
            self.assertNotIn("zones", authoring)
            self.assertEqual(
                set(authoring["required_channels"]),
                REQUIRED_ANATOMY_CHANNELS[asset["logical_slot"]],
            )
            audit = authoring["source_projection_audit"]
            self.assertEqual(
                audit["schema"], "alife.geneforge_source_projection_audit.v1"
            )
            self.assertEqual(set(audit["lods"]), {"full", "compact", "impostor"})

    def test_all_lods_have_unique_confined_anatomy_outputs(self) -> None:
        paths = []
        for asset in self.recipe["part_assets"]:
            for lod in asset["lods"]:
                self.assertRegex(lod["anatomy_mask_sha256"], r"^[0-9a-f]{64}$")
                path = PurePosixPath(lod["anatomy_mask"])
                self.assertFalse(path.is_absolute())
                self.assertNotIn("..", path.parts)
                self.assertEqual(path.suffix, ".png")
                paths.append(path.as_posix())
        self.assertEqual(len(paths), 42)
        self.assertEqual(len(set(paths)), 42)

    def test_anatomy_projection_rejects_semantic_groups_absent_from_obj(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        asset = self.assets["norn-head"]
        lod = asset["lods"][0]
        _, _, pixels = importer.decode_rgba_png(
            (staging / lod["semantic_mask"]).read_bytes()
        )
        pixels = bytearray(pixels)
        occupied = next(index for index in range(0, len(pixels), 4) if pixels[index + 3])
        pixels[occupied : occupied + 3] = bytes(EXPECTED_GROUP_COLORS["torso"])
        semantic = importer.png_bytes(64, 64, bytes(pixels))
        with self.assertRaisesRegex(importer.ImportFailure, "same-group OBJ triangle"):
            importer.anatomy_mask(
                semantic,
                (staging / lod["generated_obj"]).read_bytes(),
                asset["anatomy_authoring"],
                "head",
            )

    def test_anatomy_rasterizer_is_deterministic_and_preserves_occupancy(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        asset = self.assets["norn-head"]
        lod = asset["lods"][0]
        semantic = (staging / lod["semantic_mask"]).read_bytes()
        obj = (staging / lod["generated_obj"]).read_bytes()
        profile = asset["anatomy_authoring"]
        first = importer.anatomy_mask(semantic, obj, profile, "head")
        second = importer.anatomy_mask(semantic, obj, profile, "head")
        self.assertEqual(first, second)
        _, _, semantic_pixels = importer.decode_rgba_png(semantic)
        _, _, anatomy_pixels = importer.decode_rgba_png(first)
        for semantic_alpha, anatomy_alpha in zip(semantic_pixels[3::4], anatomy_pixels[3::4]):
            self.assertEqual(semantic_alpha > 0, anatomy_alpha > 0)
        used = {tuple(anatomy_pixels[i : i + 3]) for i in range(0, len(anatomy_pixels), 4) if anatomy_pixels[i + 3]}
        self.assertTrue(set(EXPECTED_ANATOMY_COLORS.values()) >= used)

    def test_source_projected_anatomy_reprojects_to_independent_3d_predicates(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        color_to_channel = {value: key for key, value in EXPECTED_ANATOMY_COLORS.items()}
        for asset in self.recipe["part_assets"]:
            authoring = asset["anatomy_authoring"]
            self.assertEqual(authoring["schema"], "alife.geneforge_anatomy_authoring.v2")
            source_geometry = authoring["projection"]["source_geometry"]
            audit_lods = authoring["source_projection_audit"]["lods"]
            for lod in asset["lods"]:
                with self.subTest(asset=asset["id"], lod=lod["lod"]):
                    obj_path = staging / lod["generated_obj"]
                    semantic_path = staging / lod["semantic_mask"]
                    anatomy_path = staging / lod["anatomy_mask"]
                    projected = independent_obj_texel_projection(obj_path, semantic_path)
                    _, _, anatomy = read_rgba_png(anatomy_path)
                    records = {(record["x"], record["y"]): record for record in projected}
                    channels: dict[str, list[dict]] = {}
                    for y in range(64):
                        for x in range(64):
                            offset = (y * 64 + x) * 4
                            if anatomy[offset + 3] == 0:
                                self.assertNotIn((x, y), records)
                                continue
                            self.assertIn((x, y), records)
                            channel = color_to_channel[tuple(anatomy[offset : offset + 3])]
                            channels.setdefault(channel, []).append(records[(x, y)])

                    self.assertEqual(
                        set(channels), REQUIRED_ANATOMY_CHANNELS[asset["logical_slot"]]
                    )
                    evidence = audit_lods[lod["lod"]]
                    self.assertEqual(
                        evidence["obj_sha256"], hashlib.sha256(obj_path.read_bytes()).hexdigest()
                    )
                    self.assertEqual(
                        evidence["semantic_sha256"],
                        hashlib.sha256(semantic_path.read_bytes()).hexdigest(),
                    )
                    self.assertEqual(evidence["projected_texels"], len(projected))
                    self.assertEqual(
                        evidence["inside_texels"],
                        sum(record["mode"] == "inside" for record in projected),
                    )
                    self.assertEqual(
                        evidence["nearest_texels"],
                        sum(record["mode"] == "nearest" for record in projected),
                    )
                    self.assertEqual(
                        evidence["overlap_texels"],
                        sum(record["overlap"] > 1 for record in projected),
                    )
                    self.assertEqual(
                        set(evidence["source_landmark_projections"]),
                        set(source_geometry["landmarks"]),
                    )
                    self.assertEqual(
                        set(evidence["feature_anchor_ownership"]),
                        set(source_geometry["feature_landmarks"]),
                    )
                    self.assertTrue(evidence["geometry_classification"])
                    self.assertEqual(
                        evidence["channel_counts"],
                        {channel: len(points) for channel, points in sorted(channels.items())},
                    )

                    columns: dict[int, set[str]] = {}
                    for channel, channel_records in channels.items():
                        for record in channel_records:
                            columns.setdefault(record["x"], set()).add(channel)
                    self.assertTrue(
                        any(len(column_channels) > 1 for column_channels in columns.values()),
                        "anatomy classification collapsed to repeated vertical partitions",
                    )

                    all_x = [record["point"][0] for record in projected]
                    all_y = [record["point"][1] for record in projected]
                    all_z = [record["point"][2] for record in projected]
                    if asset["logical_slot"] == "head":
                        self.assertTrue(
                            all(record["group"] == "head.teeth" for record in channels["keratin-skin"])
                        )
                        self.assertTrue(
                            all(record["group"] == "head.hair" for record in channels["secondary-marking"])
                        )
                        for record in channels["inner-ear"]:
                            self.assertEqual(record["group"], "head")
                            lateral = abs(normalized_axis(record["point"][0], all_x) - 0.5)
                            self.assertGreaterEqual(lateral, 0.24)
                        for record in channels["muzzle"]:
                            self.assertEqual(record["group"], "head")
                            self.assertLessEqual(
                                abs(normalized_axis(record["point"][0], all_x) - 0.5), 0.34
                            )
                            self.assertLessEqual(normalized_axis(record["point"][1], all_y), 0.68)
                            self.assertLessEqual(normalized_axis(record["point"][2], all_z), 0.58)
                    elif asset["logical_slot"] == "torso":
                        for record in channels["belly"]:
                            self.assertLessEqual(
                                abs(normalized_axis(record["point"][0], all_x) - 0.5), 0.36
                            )
                            self.assertLessEqual(normalized_axis(record["point"][2], all_z), 0.58)
                    elif asset["logical_slot"] in {"arms", "legs"}:
                        for record in channels["hands-feet"]:
                            group_records = [
                                candidate
                                for candidate in projected
                                if candidate["group"] == record["group"]
                            ]
                            group_y = [candidate["point"][1] for candidate in group_records]
                            self.assertLessEqual(
                                normalized_axis(record["point"][1], group_y), 0.46
                            )
                    else:
                        root_z = max(all_z)
                        tip_z = min(all_z)
                        span = max(root_z - tip_z, 1.0e-12)
                        for record in channels["keratin-skin"]:
                            distance = (root_z - record["point"][2]) / span
                            self.assertGreaterEqual(distance, 0.72)
                        for record in channels["secondary-marking"]:
                            distance = (root_z - record["point"][2]) / span
                            self.assertGreaterEqual(distance, 0.28)
                            self.assertLessEqual(distance, 0.82)

    def test_source_feature_anchor_ownership_is_independent_of_production_classifier(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        color_to_channel = {value: key for key, value in EXPECTED_ANATOMY_COLORS.items()}
        for asset in self.recipe["part_assets"]:
            source_geometry = asset["anatomy_authoring"]["projection"]["source_geometry"]
            expected = REQUIRED_FEATURE_ANCHORS[asset["logical_slot"]]
            for lod in asset["lods"]:
                with self.subTest(asset=asset["id"], lod=lod["lod"]):
                    obj_path = staging / lod["generated_obj"]
                    semantic_path = staging / lod["semantic_mask"]
                    anatomy_path = staging / lod["anatomy_mask"]
                    projected = independent_obj_texel_projection(obj_path, semantic_path)
                    by_group = {}
                    for record in projected:
                        by_group.setdefault(record["group"], []).append(record)
                    evidence = asset["anatomy_authoring"]["source_projection_audit"]["lods"][lod["lod"]]
                    bounds = evidence["source_bounds"]
                    canonical = source_geometry["canonical_bounds"]
                    _, _, anatomy = read_rgba_png(anatomy_path)
                    for name, (channel, runtime_group) in expected.items():
                        anchor = source_geometry["feature_landmarks"][name]
                        target = tuple(
                            bounds["min"][axis]
                            + (anchor["point"][axis] - canonical["min"][axis])
                            / (canonical["max"][axis] - canonical["min"][axis])
                            * (bounds["max"][axis] - bounds["min"][axis])
                            for axis in range(3)
                        )
                        candidates = by_group[runtime_group]
                        nearest = min(
                            candidates,
                            key=lambda record: (
                                sum((record["point"][axis] - target[axis]) ** 2 for axis in range(3)),
                                record["face"],
                                record["y"],
                                record["x"],
                            ),
                        )
                        offset = (nearest["y"] * 64 + nearest["x"]) * 4
                        self.assertEqual(
                            tuple(anatomy[offset : offset + 3]),
                            EXPECTED_ANATOMY_COLORS[channel],
                        )
                        ownership = evidence["feature_anchor_ownership"][name]
                        self.assertEqual(ownership["channel"], channel)
                        self.assertEqual(ownership["runtime_group"], runtime_group)
                        self.assertEqual(ownership["group"], nearest["group"])
                        self.assertEqual((ownership["x"], ownership["y"]), (nearest["x"], nearest["y"]))

    def test_lexical_live_output_reparse_is_rejected_before_resolution(self) -> None:
        root = TEST_OUTPUT / "augment-live-output-reparse"
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        output = root / "live-recipes.json"
        output.write_text(RECIPE_PATH.read_text(encoding="utf-8"), encoding="utf-8")
        with mock.patch.object(
            importer,
            "_is_symlink_or_reparse",
            side_effect=lambda path: Path(path) == output,
        ):
            with self.assertRaisesRegex(importer.ImportFailure, "live prior output recipe"):
                importer._load_live_authority_recipe(output, output)

    def test_directory_durability_false_is_a_typed_promotion_failure(self) -> None:
        root = TEST_OUTPUT / "augment-durability-failure"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        (temporary / "build_receipt.json").write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with mock.patch.object(importer, "_flush_directory", return_value=False):
            with self.assertRaisesRegex(importer.ImportFailure, "directory durability"):
                importer._promote_augmented_pair(
                    staging, temporary, backup, recipe_temporary, output
                )

    def test_pair_recipe_digest_rejects_marker_selected_non_object_json(self) -> None:
        root = TEST_OUTPUT / "augment-recovery-non-object-generation"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        (temporary / "build_receipt.json").write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "prepared"
                else None,
            )
        marker = importer._augmentation_transaction_path(staging)
        transaction = json.loads(marker.read_text(encoding="utf-8"))
        non_object = b"[]"
        Path(transaction["recipe_backup"]).write_bytes(non_object)
        transaction["old_recipe_file_sha256"] = hashlib.sha256(non_object).hexdigest()
        marker.write_text(json.dumps(transaction), encoding="utf-8")
        with self.assertRaises(importer.ImportFailure):
            importer._recover_augmentation_transaction(staging, output)

    def test_decode_rgba_png_rejects_malformed_or_unbounded_inputs_as_import_failure(self) -> None:
        filtered_size = 64 * (1 + 64 * 4)
        valid_rows = b"".join(b"\0" + bytes(64 * 4) for _ in range(64))
        malformed = {
            "truncated-header": b"\x89PNG\r\n\x1a\n\0\0",
            "truncated-payload": png_with_chunks([(b"IHDR", rgba8_ihdr())])[:-2],
            "chunk-before-ihdr": png_with_chunks(
                [(b"tEXt", b"early"), (b"IHDR", rgba8_ihdr()), (b"IDAT", zlib.compress(valid_rows)), (b"IEND", b"")]
            ),
            "wrong-native-size": png_with_chunks(
                [(b"IHDR", rgba8_ihdr(65, 64)), (b"IDAT", zlib.compress(valid_rows)), (b"IEND", b"")]
            ),
            "truncated-zlib": png_with_chunks(
                [(b"IHDR", rgba8_ihdr()), (b"IDAT", zlib.compress(valid_rows)[:-2]), (b"IEND", b"")]
            ),
            "trailing-zlib": png_with_chunks(
                [(b"IHDR", rgba8_ihdr()), (b"IDAT", zlib.compress(valid_rows) + b"trailing"), (b"IEND", b"")]
            ),
            "decoded-bomb": png_with_chunks(
                [(b"IHDR", rgba8_ihdr()), (b"IDAT", zlib.compress(bytes(filtered_size + 4096))), (b"IEND", b"")]
            ),
            "trailing-png": png_with_chunks(
                [(b"IHDR", rgba8_ihdr()), (b"IDAT", zlib.compress(valid_rows)), (b"IEND", b"")]
            ) + b"trailing",
        }
        for name, data in malformed.items():
            with self.subTest(name=name):
                with self.assertRaises(importer.ImportFailure):
                    importer.decode_rgba_png(data)

    def test_anatomy_authoring_requires_projection_policy_and_all_lod_audits(self) -> None:
        asset = json.loads(json.dumps(self.assets["norn-head"]))
        mutations = (
            lambda candidate: candidate["anatomy_authoring"].__setitem__("schema", "unreviewed"),
            lambda candidate: candidate["anatomy_authoring"]["projection"].__setitem__(
                "triangle_tie_break", "caller-order"
            ),
            lambda candidate: candidate["anatomy_authoring"]["projection"][
                "detail_group_channels"
            ].__setitem__("head.teeth", "belly"),
            lambda candidate: candidate["anatomy_authoring"].__setitem__(
                "required_channels", ["primary"]
            ),
            lambda candidate: candidate["anatomy_authoring"][
                "source_projection_audit"
            ]["lods"].pop("compact"),
            lambda candidate: candidate["anatomy_authoring"][
                "source_projection_audit"
            ]["lods"]["full"].__setitem__("projection_sha256", "invalid"),
        )
        for mutation in mutations:
            candidate = json.loads(json.dumps(asset))
            mutation(candidate)
            with self.subTest(mutation=mutation):
                with self.assertRaises(importer.ImportFailure):
                    importer.validate_anatomy_authoring(candidate)
                    importer.validate_anatomy_source_audit(candidate)

    def test_source_projected_anatomy_profile_requires_owned_channel(self) -> None:
        asset = self.assets["norn-head"]
        feature_name = "left-ear"
        source_channel = asset["anatomy_authoring"]["projection"]["source_geometry"][
            "feature_landmarks"
        ][feature_name]["channel"]
        self.assertEqual(
            asset["anatomy_authoring"]["source_projection_audit"]["lods"]["full"][
                "feature_anchor_ownership"
            ][feature_name]["owned_channel"],
            source_channel,
        )
        importer.validate_anatomy_source_audit(asset)

        def remove_owned_channel(ownership: dict) -> None:
            ownership.pop("owned_channel")

        mutations = {
            "missing": remove_owned_channel,
            "mismatched": lambda ownership: ownership.__setitem__(
                "owned_channel", "muzzle"
            ),
            "non-string": lambda ownership: ownership.__setitem__(
                "owned_channel", 7
            ),
        }
        for name, mutation in mutations.items():
            candidate = json.loads(json.dumps(asset))
            ownership = candidate["anatomy_authoring"]["source_projection_audit"][
                "lods"
            ]["full"]["feature_anchor_ownership"][feature_name]
            mutation(ownership)
            with self.subTest(mutation=name):
                with self.assertRaises(importer.ImportFailure):
                    importer.validate_anatomy_source_audit(candidate)

    def test_augmented_tree_requires_both_mask_dimensions_to_be_64(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        for width, height in ((64, 63), (63, 64)):
            staging = TEST_OUTPUT / f"augment-rectangular-mask-{width}x{height}"
            if staging.exists():
                shutil.rmtree(staging)
            shutil.copytree(source, staging)
            recipe = json.loads(json.dumps(self.recipe))
            lod = recipe["part_assets"][0]["lods"][0]
            for field in ("semantic_mask", "anatomy_mask"):
                path = staging / lod[field]
                original_width, original_height, pixels = importer.decode_rgba_png(
                    path.read_bytes()
                )
                self.assertEqual((original_width, original_height), (64, 64))
                resized = bytearray(width * height * 4)
                for row in range(height):
                    source_row = min(row, original_height - 1)
                    copied_width = min(width, original_width)
                    resized[row * width * 4 : (row * width + copied_width) * 4] = pixels[
                        source_row * original_width * 4 : (source_row * original_width + copied_width) * 4
                    ]
                path.write_bytes(importer.png_bytes(width, height, bytes(resized)))
            with self.subTest(dimensions=(width, height)):
                with self.assertRaisesRegex(importer.ImportFailure, "64x64"):
                    importer._validate_augmented_tree(staging, recipe)

    def test_receipt_sources_are_exactly_bound_to_donor_owned_outputs(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        receipt = json.loads((staging / "build_receipt.json").read_text(encoding="utf-8"))

        def stale_count(candidate: dict) -> None:
            candidate["sources"][0]["asset_count"] += 1

        def missing_path(candidate: dict) -> None:
            candidate["sources"][0]["outputs"].pop()
            candidate["sources"][0]["output_count"] -= 1

        def wrong_donor(candidate: dict) -> None:
            candidate["sources"][0]["donor"] = candidate["sources"][1]["donor"]

        def duplicate_path(candidate: dict) -> None:
            candidate["sources"][0]["outputs"].append(candidate["sources"][0]["outputs"][0])
            candidate["sources"][0]["output_count"] += 1

        def union_drift(candidate: dict) -> None:
            candidate["sources"][0]["outputs"][0] = candidate["sources"][1]["outputs"][0]

        for label, mutation in (
            ("stale count", stale_count),
            ("missing path", missing_path),
            ("wrong donor", wrong_donor),
            ("duplicate path", duplicate_path),
            ("union drift", union_drift),
        ):
            candidate = json.loads(json.dumps(receipt))
            mutation(candidate)
            with self.subTest(label=label):
                with self.assertRaisesRegex(importer.ImportFailure, "receipt source accounting"):
                    importer._verify_receipt_outputs(staging, candidate, self.recipe)

    def test_legacy_126_output_receipt_uses_the_same_source_accounting_contract(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        staging = TEST_OUTPUT / "legacy-receipt-tree"
        if staging.exists():
            shutil.rmtree(staging)
        shutil.copytree(source, staging)
        receipt = json.loads((staging / "build_receipt.json").read_text(encoding="utf-8"))
        anatomy_paths = {
            lod["anatomy_mask"]
            for asset in self.recipe["part_assets"]
            for lod in asset["lods"]
        }
        receipt["outputs"] = {
            path: digest
            for path, digest in receipt["outputs"].items()
            if path not in anatomy_paths
        }
        for source in receipt["sources"]:
            source["outputs"] = [
                path for path in source["outputs"] if path not in anatomy_paths
            ]
            source["output_count"] = len(source["outputs"])
        for relative in anatomy_paths:
            (staging / relative).unlink()
        self.assertFalse(importer._verify_receipt_outputs(staging, receipt, self.recipe))

        receipt["sources"][0]["output_count"] += 1
        with self.assertRaisesRegex(importer.ImportFailure, "receipt source accounting"):
            importer._verify_receipt_outputs(staging, receipt, self.recipe)

    def test_staging_containment_helpers_reject_external_and_reparse_paths(self) -> None:
        root = TEST_OUTPUT / "containment-root"
        external = TEST_OUTPUT / "containment-external"
        shutil.rmtree(root, ignore_errors=True)
        shutil.rmtree(external, ignore_errors=True)
        root.mkdir(parents=True)
        external.mkdir(parents=True)
        external_file = external / "outside.bin"
        external_file.write_bytes(b"outside remains unchanged")
        self.assertFalse(
            importer.canonical_path_is_within(root.resolve(), external_file.resolve())
        )

        linked_file = root / "linked.bin"
        linked_directory = root / "linked-directory"
        try:
            os.symlink(external_file, linked_file)
            os.symlink(external, linked_directory, target_is_directory=True)
        except OSError:
            return

        with self.assertRaisesRegex(importer.ImportFailure, "symlink|reparse"):
            importer.confined_existing_staged_path(root, "linked.bin", "test output")
        with self.assertRaisesRegex(importer.ImportFailure, "symlink|reparse"):
            importer.staged_output_path(root, "linked-directory/new.bin")
        self.assertEqual(external_file.read_bytes(), b"outside remains unchanged")
        self.assertFalse((external / "new.bin").exists())

    def test_staged_output_path_accepts_an_importer_worker_mkdir_race(self) -> None:
        root = TEST_OUTPUT / "worker-mkdir-race"
        shutil.rmtree(root, ignore_errors=True)
        root.mkdir(parents=True)
        raced_parent = root / "shared"
        original_mkdir = os.mkdir
        injected = False

        def create_then_report_existing(path, mode=0o777, *, dir_fd=None):
            nonlocal injected
            if Path(path) == raced_parent and not injected:
                injected = True
                if dir_fd is None:
                    original_mkdir(path, mode)
                else:
                    original_mkdir(path, mode, dir_fd=dir_fd)
                raise FileExistsError(183, "simulated concurrent mkdir", str(path))
            if dir_fd is None:
                return original_mkdir(path, mode)
            return original_mkdir(path, mode, dir_fd=dir_fd)

        with mock.patch.object(importer.os, "mkdir", side_effect=create_then_report_existing):
            output = importer.staged_output_path(root, "shared/nested/output.bin")

        self.assertTrue(injected)
        self.assertEqual(output, root / "shared/nested/output.bin")
        self.assertTrue(output.parent.is_dir())

    def test_source_projection_is_bound_to_same_lod_obj_bytes(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        asset = self.assets["norn-head"]
        lod = asset["lods"][0]
        semantic = (staging / lod["semantic_mask"]).read_bytes()
        obj = (staging / lod["generated_obj"]).read_bytes()
        profile = json.loads(json.dumps(asset["anatomy_authoring"]))
        before, audit = importer.anatomy_mask_with_audit(semantic, obj, profile, "head")
        self.assertEqual(audit["obj_sha256"], hashlib.sha256(obj).hexdigest())
        tampered = obj.replace(b"v -0.820633054", b"v -0.800633054", 1)
        self.assertNotEqual(tampered, obj)
        after, changed_audit = importer.anatomy_mask_with_audit(
            semantic, tampered, profile, "head"
        )
        self.assertNotEqual(audit["obj_sha256"], changed_audit["obj_sha256"])
        _, _, before_pixels = importer.decode_rgba_png(before)
        _, _, after_pixels = importer.decode_rgba_png(after)
        self.assertEqual(before_pixels[3::4], after_pixels[3::4])
        self.assertEqual(semantic, (staging / asset["lods"][0]["semantic_mask"]).read_bytes())

    def test_source_landmark_audit_reprojects_each_landmark_with_independent_barycentrics(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        for asset in self.recipe["part_assets"]:
            self.assertIn("source_geometry", asset["anatomy_authoring"]["projection"])
            source_geometry = asset["anatomy_authoring"]["projection"]["source_geometry"]
            for lod in asset["lods"]:
                triangles = []
                positions = []
                uvs = []
                group = None
                for line in (staging / lod["generated_obj"]).read_text(encoding="ascii").splitlines():
                    fields = line.split()
                    if not fields:
                        continue
                    if fields[0] == "v":
                        positions.append(tuple(float(value) for value in fields[1:4]))
                    elif fields[0] == "vt":
                        uvs.append(tuple(float(value) for value in fields[1:3]))
                    elif fields[0] == "g":
                        group = fields[1]
                    elif fields[0] == "f":
                        references = [field.split("/") for field in fields[1:4]]
                        triangles.append(
                            {
                                "group": group,
                                "positions": tuple(positions[int(ref[0]) - 1] for ref in references),
                                "uvs": tuple(uvs[int(ref[1]) - 1] for ref in references),
                            }
                        )
                evidence = asset["anatomy_authoring"]["source_projection_audit"]["lods"][lod["lod"]]
                for name, landmark in evidence["source_landmark_projections"].items():
                    triangle = triangles[landmark["face"]]
                    weights = landmark["weights"]
                    self.assertAlmostEqual(sum(weights), 1.0, places=5)
                    self.assertTrue(all(0.0 <= value <= 1.0 for value in weights))
                    projected = [
                        sum(weights[index] * triangle["positions"][index][axis] for index in range(3))
                        for axis in range(3)
                    ]
                    self.assertEqual(landmark["group"], triangle["group"])
                    for actual, recorded in zip(projected, landmark["projected"]):
                        self.assertAlmostEqual(actual, recorded, places=5)
                    self.assertEqual(len(landmark["source"]), 3)
                    self.assertTrue(all(math.isfinite(value) for value in landmark["source"]))
                    self.assertIn(name, source_geometry["landmarks"])

    def test_production_augmentation_matches_normal_build_rasterizer_for_all_lods(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        for asset in self.recipe["part_assets"]:
            for lod in asset["lods"]:
                expected = importer.anatomy_mask(
                    (staging / lod["semantic_mask"]).read_bytes(),
                    (staging / lod["generated_obj"]).read_bytes(),
                    asset["anatomy_authoring"],
                    asset["logical_slot"],
                )
                self.assertEqual(expected, (staging / lod["anatomy_mask"]).read_bytes())

    def test_every_family_and_independent_slot_combination_has_all_channels(self) -> None:
        channels_by_asset = {
            asset["id"]: set(asset["anatomy_authoring"]["required_channels"])
            for asset in self.recipe["part_assets"]
        }
        all_channels = set(EXPECTED_ANATOMY_COLORS)
        for family in self.recipe["families"]:
            union = set().union(*(channels_by_asset[part["asset_id"]] for part in family["parts"].values()))
            self.assertEqual(union, all_channels)
        by_slot = {
            slot: [channels_by_asset[asset["id"]] for asset in self.recipe["part_assets"] if asset["logical_slot"] == slot]
            for slot in REQUIRED_ANATOMY_CHANNELS
        }
        for slot, variants in by_slot.items():
            for channels in variants:
                self.assertTrue(REQUIRED_ANATOMY_CHANNELS[slot] <= channels)

    def test_source_microdetail_roots_are_explicit_and_audited(self) -> None:
        self.assertEqual(
            {
                source["donor"]: source["microdetail_root"]
                for source in self.recipe["sources"]
            },
            EXPECTED_MICRODETAIL_ROOTS,
        )

    def test_marker_map_is_exactly_one_through_fourteen(self) -> None:
        self.assertEqual(
            {int(key): value for key, value in self.recipe["marker_map"].items()},
            EXPECTED_MARKERS,
        )

    def test_audited_non_marker_property_exception_is_exact(self) -> None:
        self.assertEqual(
            {
                source["donor"]: source["audited_non_marker_properties"]
                for source in self.recipe["sources"]
            },
            {
                "norn": {},
                "ettin": {"head1_Ettin_angry": 0},
                "grendel": {},
            },
        )

    def test_recipe_digest_uses_zeroed_canonical_compact_json(self) -> None:
        self.assertEqual(self.recipe["recipe_sha256"], canonical_recipe_digest(self.recipe))

    def test_all_selectors_are_explicit_and_deterministic(self) -> None:
        for asset in self.assets.values():
            selector = asset["selector"]
            with self.subTest(asset=asset["id"]):
                self.assertTrue(selector["include_objects"])
                self.assertEqual(len(selector["include_objects"]), len(set(selector["include_objects"])))
                self.assertTrue(selector["marker_ids"])
                self.assertEqual(selector["selection_policy"], "exact-case-sensitive-names")
                self.assertEqual(selector["geometry_policy"], "evaluated-depsgraph")
                self.assertEqual(
                    set(selector["object_visscripts"]),
                    set(selector["include_objects"]),
                )
                self.assertNotIn("PPU", json.dumps(selector))

    def test_bridge_and_seam_contract_is_explicit_and_bounded(self) -> None:
        self.assertIn("assembly_contract", self.recipe)
        contract = self.recipe["assembly_contract"]
        self.assertEqual(contract["schema"], "alife.geneforge_family_assembly.v1")
        self.assertEqual(contract["attachment_error_limit"], 0.025)
        self.assertGreaterEqual(contract["default_overlap_depth"], 0.005)
        self.assertEqual(
            set(contract["slot_sockets"]),
            {"head", "torso", "arms", "legs", "tail"},
        )
        for family in self.recipe["families"]:
            for slot, part in family["parts"].items():
                with self.subTest(family=family["label"], slot=slot):
                    self.assertTrue(contract["slot_sockets"][slot])
                    self.assertTrue(
                        all(
                            abs(value) <= contract["attachment_error_limit"]
                            for value in part["seam_offset"]
                        )
                    )

    def test_all_founders_use_all_three_donors_and_have_a_tweak(self) -> None:
        for family in self.recipe["families"]:
            donors = {
                self.assets[part["asset_id"]]["donor"]
                for part in family["parts"].values()
            }
            with self.subTest(family=family["label"]):
                self.assertEqual(donors, {"norn", "ettin", "grendel"})
                self.assertTrue(any(part["variant_label"] for part in family["parts"].values()))
                self.assertTrue(
                    any(
                        part["fit"] != {
                            "translation": [0, 0, 0],
                            "rotation_xyzw": [0, 0, 0, 1],
                            "scale": [1, 1, 1],
                        }
                        or part["seam_offset"] != [0, 0, 0]
                        for part in family["parts"].values()
                    )
                )

    def test_no_ettin_tail_asset_or_selection_exists(self) -> None:
        tails = [asset for asset in self.assets.values() if asset["logical_slot"] == "tail"]
        self.assertTrue(tails)
        self.assertNotIn("ettin", {asset["donor"] for asset in tails})
        for family in self.recipe["families"]:
            self.assertFalse(family["parts"]["tail"]["asset_id"].startswith("ettin-"))

    def test_paths_are_relative_and_confined_to_declared_roots(self) -> None:
        allowed_outputs = (
            PurePosixPath("production_voxel_v1/creature_parts/generated/geneforge"),
            PurePosixPath("production_voxel_v1/models/geneforge"),
        )
        for source in self.recipe["sources"]:
            for key in ("blend_file", "texture_root", "microdetail_root"):
                path = PurePosixPath(source[key])
                self.assertFalse(path.is_absolute())
                self.assertNotIn("..", path.parts)
        for asset in self.assets.values():
            for lod in asset["lods"]:
                for key in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask"):
                    path = PurePosixPath(lod[key])
                    self.assertFalse(path.is_absolute())
                    self.assertNotIn("..", path.parts)
                    self.assertTrue(any(path.is_relative_to(root) for root in allowed_outputs))

    def test_all_fourteen_shared_assets_have_unique_lod_outputs(self) -> None:
        self.assertEqual(len(self.assets), 14)
        for key in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask"):
            paths = [lod[key] for asset in self.assets.values() for lod in asset["lods"]]
            with self.subTest(output=key):
                self.assertEqual(len(paths), 14 * 3)
                self.assertEqual(len(paths), len(set(paths)))

    def test_audited_never_and_invalid_objects_are_not_selected(self) -> None:
        selected = {
            name
            for asset in self.assets.values()
            for name in asset["selector"]["include_objects"]
        }
        self.assertFalse(selected & AUDITED_NEVER_OR_INVALID)

    def test_norn_whiskers_are_repaired_or_excluded(self) -> None:
        selector = self.assets["norn-head"]["selector"]
        repairs = selector.get("topology_repairs", {})
        for whisker in (
            "extra whiskers mL-bengal",
            "extra whiskers mR-bengal",
        ):
            self.assertTrue(
                whisker not in selector["include_objects"]
                or repairs.get(whisker) == ["remove-zero-area-faces"]
            )

    def test_ettin_empty_evaluated_lids_have_explicit_raw_fallback(self) -> None:
        selector = self.assets["ettin-head"]["selector"]
        self.assertEqual(
            selector["evaluated_empty_policy"],
            {"Eyelid L": "validated-raw-mesh", "Eyelid R": "validated-raw-mesh"},
        )

    def test_uvless_details_have_explicit_semantic_region_fallbacks(self) -> None:
        expected = {
            "ettin-head": {
                "Eye L",
                "Eye R",
                "Eyelid L",
                "Eyelid R",
                "Teeth B",
                "Teeth T",
                "tongue",
            },
            "grendel-head": {
                "Eye L",
                "Eye_R",
                "Lid_L_Closed",
                "Lid_R_Closed",
                "teeth B",
                "teeth T",
                "extra teeth",
                "tongue.001",
                "Hair m",
            },
        }
        for asset_id, names in expected.items():
            with self.subTest(asset=asset_id):
                self.assertEqual(
                    self.assets[asset_id]["selector"]["uv_fallbacks"],
                    {name: "semantic-detail-region" for name in names},
                )

    def test_grendel_and_ettin_declared_topology_hazards_have_repairs(self) -> None:
        expected = {
            "ettin-arms": {
                "radius L": ["repair-declared-non-manifold-edges"],
                "radius R": ["repair-declared-non-manifold-edges"],
            },
            "grendel-head": {
                "Head1_Grendel": [
                    "remove-loose-vertices",
                    "repair-declared-boundary-edges",
                ],
            },
            "grendel-arms": {
                "radius L": ["repair-declared-non-manifold-edges"],
                "radius R": ["repair-declared-non-manifold-edges"],
            },
        }
        for asset_id, required in expected.items():
            repairs = self.assets[asset_id]["selector"]["topology_repairs"]
            for object_name, tags in required.items():
                with self.subTest(asset=asset_id, object=object_name):
                    self.assertEqual(repairs[object_name], tags)

    def test_head_details_remain_declared_groups(self) -> None:
        required_roles = {"eyes", "lids", "hair", "teeth", "tongue"}
        for asset_id in ("norn-head", "ettin-head", "grendel-head"):
            with self.subTest(asset=asset_id):
                self.assertTrue(required_roles <= self.assets[asset_id]["detail_groups"].keys())


def tree_digest(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def upgrade_fixture_anatomy_contracts() -> None:
    production_assets = {asset["id"]: asset for asset in load_recipe()["part_assets"]}
    for path in FIXTURE_ROOT.rglob("fixture_recipes.json"):
        recipe = json.loads(path.read_text(encoding="utf-8"))
        for asset in recipe["part_assets"]:
            asset["anatomy_authoring"] = production_assets[asset["id"]]["anatomy_authoring"]
            for lod in asset["lods"]:
                lod["anatomy_mask"] = lod["semantic_mask"].replace("_semantic.png", "_anatomy.png")
                lod["anatomy_mask_sha256"] = "0" * 64
        recipe["recipe_sha256"] = canonical_recipe_digest(recipe)
        path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


class GeneForgeImporterSubprocessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        subprocess.run(
            [
                sys.executable,
                str(FIXTURE_GENERATOR),
                "--output",
                str(FIXTURE_ROOT),
            ],
            cwd=WORKSPACE,
            check=True,
        )
        upgrade_fixture_anatomy_contracts()
        if TEST_OUTPUT.exists():
            shutil.rmtree(TEST_OUTPUT)
        TEST_OUTPUT.mkdir(parents=True)

    def run_importer(
        self,
        command: str,
        *,
        variant: str = "valid",
        extra: tuple[str, ...] = (),
        blender_exe: Path | None = None,
        recipes: Path | None = None,
    ) -> subprocess.CompletedProcess[str]:
        root = FIXTURE_ROOT / variant
        invocation = [
            sys.executable,
            str(IMPORTER),
            command,
            "--source-root",
            str(root),
            "--recipes",
            str(recipes or root / "fixture_recipes.json"),
        ]
        if blender_exe is not None:
            invocation.extend(("--blender-exe", str(blender_exe)))
        if command == "inventory" and "--output" not in extra:
            invocation.extend(("--output", str(TEST_OUTPUT / "automatic-inventory.json")))
        invocation.extend(extra)
        return subprocess.run(
            invocation,
            cwd=WORKSPACE,
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONUTF8": "1"},
        )

    def assert_success(self, completed: subprocess.CompletedProcess[str]) -> None:
        self.assertEqual(
            completed.returncode,
            0,
            msg=f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}",
        )

    def valid_staging(self) -> Path:
        staging = TEST_OUTPUT / "staging-a"
        if not (staging / "build_receipt.json").is_file():
            self.assert_success(
                self.run_importer("build", extra=("--staging", str(staging)))
            )
        return staging

    def test_inventory_reports_fixture_features_and_exact_names(self) -> None:
        output = TEST_OUTPUT / "inventory.json"
        completed = self.run_importer("inventory", extra=("--output", str(output)))
        self.assert_success(completed)
        inventory = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual({entry["donor"] for entry in inventory["sources"]}, {"norn", "ettin", "grendel"})
        for source in inventory["sources"]:
            expected_markers = list(range(1, 13 if source["donor"] == "ettin" else 15))
            self.assertEqual(source["marker_ids"], expected_markers)
            self.assertTrue(source["has_constraint"])
            self.assertTrue(source["has_geometry_nodes"])
            self.assertTrue(source["has_armature"])
            self.assertEqual(
                source["has_declared_non_manifold"],
                source["donor"] in {"ettin", "grendel"},
            )
            self.assertIn(source["primary_uv"], {"UVMap", "UVChannel_1"})
            self.assertTrue(source["evaluated_transform_objects"])
            self.assertTrue(source["evaluated_deformation_objects"])
            self.assertEqual(
                source["audited_non_marker_properties"],
                ["head1_Ettin_angry"] if source["donor"] == "ettin" else [],
            )

    def test_validate_sources_relinks_stale_images_without_mutating_blends(self) -> None:
        source_root = FIXTURE_ROOT / "valid"
        before = {
            path.relative_to(source_root): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in source_root.rglob("*.blend")
        }
        completed = self.run_importer("validate-sources")
        self.assert_success(completed)
        after = {
            path.relative_to(source_root): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in source_root.rglob("*.blend")
        }
        self.assertEqual(before, after)
        self.assertIn("relinked_images=3", completed.stdout)
        self.assertIn(
            "marker_ids=norn:1..14,ettin:1..12,grendel:1..14",
            completed.stdout,
        )

    def test_wrong_blender_version_is_rejected_exactly(self) -> None:
        fake = TEST_OUTPUT / "wrong-blender.cmd"
        fake.write_text("@echo Blender 4.3.2\r\n", encoding="ascii")
        completed = self.run_importer("inventory", blender_exe=fake)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("requires Blender 5.1.0; found 4.3.2", completed.stderr)

    def test_broken_texture_fails_with_basename(self) -> None:
        completed = self.run_importer("validate-sources", variant="broken-texture")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("missing texture basename fixture_fur.png", completed.stderr)

    def test_invalid_marker_fails_with_exact_marker_contract(self) -> None:
        completed = self.run_importer("validate-sources", variant="invalid-marker")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("marker IDs must be exactly 1..14", completed.stderr)

    def test_duplicate_marker_ids_are_rejected_as_ambiguous(self) -> None:
        completed = self.run_importer("validate-sources", variant="duplicate-marker")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("duplicate marker ID 1", completed.stderr)

    def test_marker_id_zero_is_rejected(self) -> None:
        completed = self.run_importer("validate-sources", variant="zero-marker")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("marker ID 0", completed.stderr)

    def test_marker_property_on_nonempty_object_is_rejected(self) -> None:
        completed = self.run_importer("validate-sources", variant="nonempty-marker")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("marker property requires EMPTY", completed.stderr)

    def test_selector_property_mismatch_is_rejected_exactly(self) -> None:
        completed = self.run_importer("validate-sources", variant="selector-mismatch")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("kc3dsbpy_visscript mismatch", completed.stderr)

    def test_source_and_texture_paths_are_confined_at_execution(self) -> None:
        root = FIXTURE_ROOT / "valid"
        for field, escaped in (
            ("blend_file", "../outside.blend"),
            ("texture_root", str((root.parent / "outside-textures").resolve())),
            ("microdetail_root", "../../outside-microdetail"),
        ):
            recipe = json.loads((root / "fixture_recipes.json").read_text(encoding="utf-8"))
            recipe["sources"][0][field] = escaped
            recipe["recipe_sha256"] = canonical_recipe_digest(recipe)
            path = TEST_OUTPUT / f"unsafe-{field}.json"
            path.write_text(json.dumps(recipe), encoding="utf-8")
            completed = self.run_importer("validate-sources", recipes=path)
            with self.subTest(field=field):
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn("escapes --source-root", completed.stderr)

    def test_unapproved_evaluated_empty_geometry_is_never_silently_accepted(self) -> None:
        completed = self.run_importer("validate-sources", variant="evaluated-empty")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("evaluated geometry is empty", completed.stderr)
        self.assertIn("ettin", completed.stderr)

    def test_build_emits_three_lods_masks_sockets_and_repair_receipt(self) -> None:
        staging = TEST_OUTPUT / "staging-a"
        completed = self.run_importer("build", extra=("--staging", str(staging)))
        self.assert_success(completed)
        receipt = json.loads((staging / "build_receipt.json").read_text(encoding="utf-8"))
        self.assertEqual(receipt["lods"], ["full", "compact", "impostor"])
        self.assertEqual(receipt["donor_count"], 3)
        self.assertEqual(receipt["asset_count"], 14)
        self.assertEqual(len(receipt["outputs"]), 14 * 3 * 4)
        self.assertGreater(receipt["topology"]["removed_degenerate_faces"], 0)
        self.assertGreater(receipt["topology"]["removed_loose_vertices"], 0)
        self.assertGreater(receipt["topology"]["repaired_non_manifold_edges"], 0)
        self.assertGreater(receipt["topology"]["filled_boundary_edges"], 0)
        self.assertEqual(receipt["schema"], "alife.geneforge_build_receipt.v2")
        self.assertEqual(
            receipt["worker_execution"],
            {"max_workers": 3, "strategy": "bounded-parallel-donor-workers"},
        )
        recipe = json.loads(
            (FIXTURE_ROOT / "valid/fixture_recipes.json").read_text(encoding="utf-8")
        )
        self.assertEqual(receipt["importer_version"], recipe["importer_version"])
        self.assertEqual(receipt["recipe_sha256"], recipe["recipe_sha256"])
        self.assertEqual(
            receipt["source_sha256"],
            {source["donor"]: source["sha256"] for source in recipe["sources"]},
        )
        staged_files = [path for path in staging.rglob("*") if path.is_file()]
        self.assertTrue(all(path.stat().st_size <= 512 * 1024 for path in staged_files))
        self.assertLessEqual(sum(path.stat().st_size for path in staged_files), 8 * 1024 * 1024)
        for asset in load_recipe()["part_assets"]:
            for lod in asset["lods"]:
                for key in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask"):
                    self.assertTrue((staging / lod[key]).is_file())

    def test_bind_output_digests_writes_external_contract_and_updates_receipt(self) -> None:
        staging = self.valid_staging()
        source_recipe = FIXTURE_ROOT / "valid/fixture_recipes.json"
        output_recipe = TEST_OUTPUT / "bound-output-recipes.json"
        source_before = source_recipe.read_bytes()
        completed = subprocess.run(
            [
                sys.executable,
                str(IMPORTER),
                "bind-output-digests",
                "--recipes",
                str(source_recipe),
                "--staging",
                str(staging),
                "--output",
                str(output_recipe),
            ],
            cwd=WORKSPACE,
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONUTF8": "1"},
        )
        self.assert_success(completed)
        self.assertEqual(source_recipe.read_bytes(), source_before)
        bound = json.loads(output_recipe.read_text(encoding="utf-8"))
        self.assertEqual(bound["recipe_sha256"], canonical_recipe_digest(bound))
        for asset in bound["part_assets"]:
            for lod in asset["lods"]:
                for path_field, digest_field in (
                    ("generated_obj", "generated_obj_sha256"),
                    ("socket_manifest", "socket_manifest_sha256"),
                    ("semantic_mask", "semantic_mask_sha256"),
                    ("anatomy_mask", "anatomy_mask_sha256"),
                ):
                    self.assertEqual(
                        lod[digest_field],
                        hashlib.sha256((staging / lod[path_field]).read_bytes()).hexdigest(),
                    )
        receipt = json.loads((staging / "build_receipt.json").read_text(encoding="utf-8"))
        self.assertEqual(receipt["recipe_sha256"], bound["recipe_sha256"])

    def test_augment_anatomy_is_atomic_idempotent_and_preserves_existing_outputs(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        staging = TEST_OUTPUT / "augment-idempotent-staging"
        if staging.exists():
            shutil.rmtree(staging)
        shutil.copytree(source, staging)
        recipe = TEST_OUTPUT / "augment-idempotent-recipes.json"
        shutil.copy2(RECIPE_PATH, recipe)
        before = tree_digest(staging)
        completed = subprocess.run(
            [
                sys.executable,
                str(IMPORTER),
                "augment-anatomy",
                "--recipes",
                str(recipe),
                "--staging",
                str(staging),
                "--output",
                str(recipe),
            ],
            cwd=WORKSPACE,
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONUTF8": "1"},
        )
        self.assert_success(completed)
        self.assertEqual(before, tree_digest(staging))
        self.assertIn("outputs=168", completed.stdout)
        self.assertIn("unchanged_obj_semantic=84", completed.stdout)

    def test_augment_rejects_incomplete_receipt_without_mutating_staging(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        staging = TEST_OUTPUT / "augment-incomplete-receipt"
        if staging.exists():
            shutil.rmtree(staging)
        shutil.copytree(source, staging)
        recipe = TEST_OUTPUT / "augment-incomplete-recipes.json"
        shutil.copy2(RECIPE_PATH, recipe)
        receipt_path = staging / "build_receipt.json"
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        receipt["outputs"].pop(next(iter(receipt["outputs"])))
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        before = tree_digest(staging)
        completed = subprocess.run(
            [
                sys.executable,
                str(IMPORTER),
                "augment-anatomy",
                "--recipes",
                str(recipe),
                "--staging",
                str(staging),
                "--output",
                str(recipe),
            ],
            cwd=WORKSPACE,
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONUTF8": "1"},
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("exactly 126 legacy or 168 augmented outputs", completed.stderr)
        self.assertEqual(before, tree_digest(staging))

    def test_augment_preflight_binds_receipt_and_every_existing_recipe_digest(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        for name, mutation in (
            ("receipt-recipe", "receipt"),
            ("rebound-obj", "obj"),
            ("rebound-socket", "socket"),
            ("rebound-semantic", "semantic"),
            ("rebound-anatomy", "anatomy"),
        ):
            with self.subTest(name=name):
                staging = TEST_OUTPUT / f"augment-authority-{name}"
                if staging.exists():
                    shutil.rmtree(staging)
                shutil.copytree(source, staging)
                recipe = load_recipe()
                receipt_path = staging / "build_receipt.json"
                receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
                receipt["recipe_sha256"] = recipe["recipe_sha256"]
                if mutation == "receipt":
                    receipt["recipe_sha256"] = "0" * 64
                else:
                    lod = recipe["part_assets"][0]["lods"][0]
                    field = {
                        "obj": "generated_obj",
                        "socket": "socket_manifest",
                        "semantic": "semantic_mask",
                        "anatomy": "anatomy_mask",
                    }[mutation]
                    path = staging / lod[field]
                    path.write_bytes(path.read_bytes() + b"tampered")
                    receipt["outputs"][lod[field]] = hashlib.sha256(path.read_bytes()).hexdigest()
                receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
                with self.assertRaises(importer.ImportFailure):
                    importer._verify_receipt_outputs(staging, receipt, recipe)

    def test_authority_handoff_preserves_every_preexisting_identity_path_and_digest(self) -> None:
        authority = load_recipe()
        candidate = json.loads(json.dumps(authority))
        candidate["part_assets"][0]["anatomy_authoring"] = {
            "schema": "replacement-authoring-evidence"
        }
        candidate["part_assets"][0]["lods"][0]["anatomy_mask_sha256"] = "1" * 64
        candidate["part_assets"][0]["lods"][0]["socket_manifest_sha256"] = "2" * 64
        candidate["recipe_sha256"] = canonical_recipe_digest(candidate)
        importer._validate_authority_handoff(authority, candidate)

        mutations = {
            "asset-id": lambda recipe: recipe["part_assets"][0].__setitem__("id", "alternate-id"),
            "extra-asset": lambda recipe: recipe["part_assets"].append(
                json.loads(json.dumps(recipe["part_assets"][0]))
            ),
            "obj-path": lambda recipe: recipe["part_assets"][0]["lods"][0].__setitem__(
                "generated_obj", "production_voxel_v1/alternate.obj"
            ),
            "obj-digest": lambda recipe: recipe["part_assets"][0]["lods"][0].__setitem__(
                "generated_obj_sha256", "3" * 64
            ),
            "socket-path": lambda recipe: recipe["part_assets"][0]["lods"][0].__setitem__(
                "socket_manifest", "production_voxel_v1/alternate.json"
            ),
            "semantic-path": lambda recipe: recipe["part_assets"][0]["lods"][0].__setitem__(
                "semantic_mask", "production_voxel_v1/alternate_semantic.png"
            ),
            "semantic-digest": lambda recipe: recipe["part_assets"][0]["lods"][0].__setitem__(
                "semantic_mask_sha256", "4" * 64
            ),
            "anatomy-path": lambda recipe: recipe["part_assets"][0]["lods"][0].__setitem__(
                "anatomy_mask", "production_voxel_v1/alternate_anatomy.png"
            ),
        }
        for name, mutation in mutations.items():
            with self.subTest(name=name):
                altered = json.loads(json.dumps(candidate))
                mutation(altered)
                altered["recipe_sha256"] = canonical_recipe_digest(altered)
                with self.assertRaises(importer.ImportFailure):
                    importer._validate_authority_handoff(authority, altered)

    def test_authority_must_be_the_live_prior_output_recipe(self) -> None:
        root = TEST_OUTPUT / "augment-live-authority"
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)
        output = root / "live-recipes.json"
        alternate = root / "review-copy.json"
        shutil.copy2(RECIPE_PATH, output)
        shutil.copy2(RECIPE_PATH, alternate)
        loaded = importer._load_live_authority_recipe(output, output)
        self.assertEqual(loaded["recipe_sha256"], load_recipe()["recipe_sha256"])
        with self.assertRaisesRegex(importer.ImportFailure, "live prior output"):
            importer._load_live_authority_recipe(alternate, output)

    def test_receipt_validation_rejects_unreceipted_staged_paths(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        staging = TEST_OUTPUT / "augment-unreceipted-extra"
        if staging.exists():
            shutil.rmtree(staging)
        shutil.copytree(source, staging)
        extra = staging / "production_voxel_v1/models/geneforge/alternate_semantic.png"
        extra.parent.mkdir(parents=True, exist_ok=True)
        extra.write_bytes(b"alternate staged bytes")
        receipt = json.loads((staging / "build_receipt.json").read_text(encoding="utf-8"))
        with self.assertRaisesRegex(importer.ImportFailure, "unreceipted staged path"):
            importer._verify_receipt_outputs(staging, receipt, load_recipe())

    def test_interrupted_augmentation_promotion_recovers_a_matched_pair_at_every_phase(self) -> None:
        for phase in ("prepared", "staging-backed-up", "staging-promoted", "recipe-promoted"):
            with self.subTest(phase=phase):
                root = TEST_OUTPUT / f"augment-recovery-{phase}"
                if root.exists():
                    shutil.rmtree(root)
                staging = root / "staging"
                temporary = root / "staging.augment-tmp-test"
                backup = root / "staging.augment-rollback-test"
                output = root / "recipe.json"
                recipe_temporary = root / ".recipe.json.augment-tmp-test"
                staging.mkdir(parents=True)
                temporary.mkdir()
                old_digest = "1" * 64
                new_digest = "2" * 64
                (staging / "build_receipt.json").write_text(
                    json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
                )
                (temporary / "build_receipt.json").write_text(
                    json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
                )
                output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
                recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")

                def interrupt(observed: str) -> None:
                    if observed == phase:
                        raise KeyboardInterrupt(observed)

                with self.assertRaises(KeyboardInterrupt):
                    importer._promote_augmented_pair(
                        staging,
                        temporary,
                        backup,
                        recipe_temporary,
                        output,
                        phase_observer=interrupt,
                    )
                importer._recover_augmentation_transaction(staging, output)
                receipt_digest = json.loads(
                    (staging / "build_receipt.json").read_text(encoding="utf-8")
                )["recipe_sha256"]
                recipe_digest = json.loads(output.read_text(encoding="utf-8"))["recipe_sha256"]
                self.assertEqual(receipt_digest, recipe_digest)
                self.assertFalse(importer._augmentation_transaction_path(staging).exists())

    def test_cli_recovers_transaction_before_loading_the_live_recipe(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        root = TEST_OUTPUT / "augment-cli-startup-recovery"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        shutil.copytree(source, staging)
        shutil.copytree(source, temporary)
        shutil.copy2(RECIPE_PATH, output)
        candidate = json.loads(output.read_text(encoding="utf-8"))
        candidate["recipe_sha256"] = "2" * 64
        recipe_temporary.write_text(json.dumps(candidate), encoding="utf-8")
        receipt_path = temporary / "build_receipt.json"
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        receipt["recipe_sha256"] = "2" * 64
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "staging-promoted"
                else None,
            )
        output.write_text("{unreadable live recipe", encoding="utf-8")
        completed = subprocess.run(
            [
                sys.executable,
                str(IMPORTER),
                "augment-anatomy",
                "--recipes",
                str(output),
                "--staging",
                str(staging),
                "--output",
                str(output),
            ],
            cwd=WORKSPACE,
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONUTF8": "1"},
        )
        self.assert_success(completed)
        self.assertFalse(importer._augmentation_transaction_path(staging).exists())
        restored = json.loads(output.read_text(encoding="utf-8"))
        restored_receipt = json.loads(
            (staging / "build_receipt.json").read_text(encoding="utf-8")
        )
        self.assertEqual(restored["recipe_sha256"], restored_receipt["recipe_sha256"])

    def test_augmentation_recovery_rejects_marker_controlled_external_paths(self) -> None:
        root = TEST_OUTPUT / "augment-recovery-unsafe-marker"
        external = TEST_OUTPUT / "augment-recovery-external"
        for path in (root, external):
            if path.exists():
                shutil.rmtree(path)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        external.mkdir()
        sentinel = external / "sentinel.txt"
        sentinel.write_text("preserve\n", encoding="ascii")
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
        )
        (temporary / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "prepared"
                else None,
            )
        marker = importer._augmentation_transaction_path(staging)
        transaction = json.loads(marker.read_text(encoding="utf-8"))
        transaction["temporary"] = str(external.resolve())
        marker.write_text(json.dumps(transaction), encoding="utf-8")
        with self.assertRaisesRegex(importer.ImportFailure, "unsafe recovery path"):
            importer._recover_augmentation_transaction(staging, output)
        self.assertEqual(sentinel.read_text(encoding="ascii"), "preserve\n")
        marker.unlink()

    def test_augmentation_recovery_maps_every_malformed_marker_field_to_import_failure(self) -> None:
        root = TEST_OUTPUT / "augment-recovery-malformed-marker"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
        )
        (temporary / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "prepared"
                else None,
            )
        marker = importer._augmentation_transaction_path(staging)
        valid = json.loads(marker.read_text(encoding="utf-8"))
        mutations = {
            "missing-path": lambda value: value.pop("temporary"),
            "path-type": lambda value: value.__setitem__("backup", ["not", "a", "path"]),
            "phase-type": lambda value: value.__setitem__("phase", 7),
            "missing-digest": lambda value: value.pop("old_recipe_sha256"),
            "digest-type": lambda value: value.__setitem__("new_recipe_sha256", 2),
            "digest-format": lambda value: value.__setitem__("new_recipe_sha256", "not-sha256"),
        }
        for name, mutation in mutations.items():
            with self.subTest(name=name):
                malformed = json.loads(json.dumps(valid))
                mutation(malformed)
                marker.write_text(json.dumps(malformed), encoding="utf-8")
                with self.assertRaises(importer.ImportFailure):
                    importer._recover_augmentation_transaction(staging, output)
        marker.write_text(json.dumps(valid), encoding="utf-8")
        importer._recover_augmentation_transaction(staging, output)

    def test_augmentation_recovery_rejects_reparse_operands_before_tree_access(self) -> None:
        root = TEST_OUTPUT / "augment-recovery-reparse-marker"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
        )
        (temporary / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "prepared"
                else None,
            )
        with mock.patch.object(
            importer,
            "_is_symlink_or_reparse",
            side_effect=lambda path: Path(path) == temporary,
        ):
            with self.assertRaisesRegex(importer.ImportFailure, "reparse"):
                importer._recover_augmentation_transaction(staging, output)
        importer._augmentation_transaction_path(staging).unlink()

    def test_augmentation_recovery_rejects_real_sibling_symlink_when_supported(self) -> None:
        root = TEST_OUTPUT / "augment-recovery-real-reparse"
        external = TEST_OUTPUT / "augment-recovery-real-reparse-external"
        for path in (root, external):
            if path.exists():
                shutil.rmtree(path)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        external.mkdir()
        sentinel = external / "sentinel.txt"
        sentinel.write_text("preserve", encoding="ascii")
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
        )
        (temporary / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "prepared"
                else None,
            )
        shutil.rmtree(temporary)
        try:
            os.symlink(external, temporary, target_is_directory=True)
        except OSError:
            importer._augmentation_transaction_path(staging).unlink()
            return
        try:
            with self.assertRaisesRegex(importer.ImportFailure, "symlink|reparse"):
                importer._recover_augmentation_transaction(staging, output)
            self.assertEqual(sentinel.read_text(encoding="ascii"), "preserve")
        finally:
            temporary.unlink()
            importer._augmentation_transaction_path(staging).unlink()

    def test_augmentation_recovery_rejects_a_tampered_rollback_tree(self) -> None:
        root = TEST_OUTPUT / "augment-recovery-tampered-backup"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
        )
        (temporary / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        with self.assertRaises(KeyboardInterrupt):
            importer._promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                phase_observer=lambda phase: (_ for _ in ()).throw(KeyboardInterrupt())
                if phase == "staging-promoted"
                else None,
            )
        (backup / "build_receipt.json").write_bytes(b"tampered rollback")
        with self.assertRaisesRegex(importer.ImportFailure, "verified old staging"):
            importer._recover_augmentation_transaction(staging, output)
        importer._augmentation_transaction_path(staging).unlink()

    def test_promotion_phases_cross_file_and_directory_durability_barriers(self) -> None:
        self.assertTrue(hasattr(importer, "_flush_file"))
        self.assertTrue(hasattr(importer, "_flush_directory"))
        self.assertTrue(hasattr(importer, "_durable_replace"))
        root = TEST_OUTPUT / "augment-durability-probe"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        temporary = root / "staging.augment-tmp-test"
        backup = root / "staging.augment-rollback-test"
        output = root / "recipe.json"
        recipe_temporary = root / ".recipe.json.augment-tmp-test"
        staging.mkdir(parents=True)
        temporary.mkdir()
        old_digest, new_digest = "1" * 64, "2" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": old_digest}), encoding="utf-8"
        )
        (temporary / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": new_digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": old_digest}), encoding="utf-8")
        recipe_temporary.write_text(json.dumps({"recipe_sha256": new_digest}), encoding="utf-8")
        file_barriers = []
        directory_barriers = []
        original_file_flush = importer._flush_file
        original_directory_flush = importer._flush_directory

        def observe_file(path):
            file_barriers.append(Path(path).resolve())
            return original_file_flush(path)

        def observe_directory(path):
            directory_barriers.append(Path(path).resolve())
            return original_directory_flush(path)

        with mock.patch.object(importer, "_flush_file", side_effect=observe_file), mock.patch.object(
            importer, "_flush_directory", side_effect=observe_directory
        ):
            importer._promote_augmented_pair(
                staging, temporary, backup, recipe_temporary, output
            )
        self.assertIn(output.resolve(), file_barriers)
        self.assertGreaterEqual(directory_barriers.count(root.resolve()), 8)

    def test_startup_cleans_only_confined_orphans_after_a_verified_live_pair(self) -> None:
        root = TEST_OUTPUT / "augment-orphan-cleanup"
        if root.exists():
            shutil.rmtree(root)
        staging = root / "staging"
        staging.mkdir(parents=True)
        output = root / "recipe.json"
        digest = "a" * 64
        (staging / "build_receipt.json").write_text(
            json.dumps({"recipe_sha256": digest}), encoding="utf-8"
        )
        output.write_text(json.dumps({"recipe_sha256": digest}), encoding="utf-8")
        orphan_directory = root / "staging.augment-tmp-orphan"
        orphan_directory.mkdir()
        orphan_file = root / ".recipe.json.augment-rollback-orphan"
        orphan_file.write_text("orphan", encoding="ascii")
        unrelated = root / "preserve.txt"
        unrelated.write_text("preserve", encoding="ascii")
        importer._cleanup_augmentation_orphans(staging, output)
        self.assertFalse(orphan_directory.exists())
        self.assertFalse(orphan_file.exists())
        self.assertEqual(unrelated.read_text(encoding="ascii"), "preserve")

    def test_augment_rejects_64x63_semantic_without_mutating_staging_or_recipe(self) -> None:
        source = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        staging = TEST_OUTPUT / "augment-rectangular-no-mutation"
        if staging.exists():
            shutil.rmtree(staging)
        shutil.copytree(source, staging)
        recipe_path = TEST_OUTPUT / "augment-rectangular-recipes.json"
        recipe = json.loads(RECIPE_PATH.read_text(encoding="utf-8"))
        lod = recipe["part_assets"][0]["lods"][0]
        semantic_path = staging / lod["semantic_mask"]
        width, height, pixels = importer.decode_rgba_png(semantic_path.read_bytes())
        self.assertEqual((width, height), (64, 64))
        semantic_path.write_bytes(importer.png_bytes(64, 63, pixels[: 64 * 63 * 4]))
        semantic_digest = hashlib.sha256(semantic_path.read_bytes()).hexdigest()
        lod["semantic_mask_sha256"] = semantic_digest
        recipe["recipe_sha256"] = canonical_recipe_digest(recipe)
        recipe_path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")

        receipt_path = staging / "build_receipt.json"
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        receipt["outputs"][lod["semantic_mask"]] = semantic_digest
        receipt["recipe_sha256"] = recipe["recipe_sha256"]
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        before_staging = tree_digest(staging)
        before_recipe = recipe_path.read_bytes()

        completed = subprocess.run(
            [
                sys.executable,
                str(IMPORTER),
                "augment-anatomy",
                "--recipes",
                str(recipe_path),
                "--staging",
                str(staging),
                "--output",
                str(recipe_path),
            ],
            cwd=WORKSPACE,
            text=True,
            capture_output=True,
            check=False,
            env={**os.environ, "PYTHONUTF8": "1"},
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("64x64", completed.stderr)
        self.assertEqual(tree_digest(staging), before_staging)
        self.assertEqual(recipe_path.read_bytes(), before_recipe)

    def test_atomic_pair_replace_rolls_back_first_destination_on_second_failure(self) -> None:
        root = TEST_OUTPUT / "atomic-pair-replace"
        root.mkdir(parents=True, exist_ok=True)
        first_destination = root / "recipe.json"
        second_destination = root / "build_receipt.json"
        first_temporary = root / ".recipe.new"
        second_temporary = root / ".receipt.new"
        first_destination.write_text("old recipe\n", encoding="ascii")
        second_destination.write_text("old receipt\n", encoding="ascii")
        first_temporary.write_text("new recipe\n", encoding="ascii")
        second_temporary.write_text("new receipt\n", encoding="ascii")
        real_replace = os.replace
        calls = 0

        def fail_second(source, destination):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("injected second replacement failure")
            return real_replace(source, destination)

        with mock.patch.object(importer.os, "replace", side_effect=fail_second):
            with self.assertRaisesRegex(OSError, "injected second replacement failure"):
                importer.atomic_replace_pair(
                    first_temporary,
                    first_destination,
                    second_temporary,
                    second_destination,
                )
        self.assertEqual(first_destination.read_text(encoding="ascii"), "old recipe\n")
        self.assertEqual(second_destination.read_text(encoding="ascii"), "old receipt\n")
        self.assertFalse(first_temporary.exists())
        self.assertFalse(second_temporary.exists())

    def test_repaired_geometry_has_smooth_normals_and_connected_lods(self) -> None:
        staging = self.valid_staging()
        for asset in load_recipe()["part_assets"]:
            metrics = []
            for lod in asset["lods"]:
                metrics.append(obj_topology(staging / lod["generated_obj"]))
            with self.subTest(asset=asset["id"]):
                self.assertGreater(metrics[0]["smooth_shared_vertices"], 0)
                self.assertEqual(metrics[0]["non_manifold_edges"], 0)
                self.assertGreater(metrics[0]["faces"], metrics[1]["faces"])
                self.assertGreater(metrics[1]["faces"], metrics[2]["faces"])
                expected_object_count = len(set(asset["selector"]["include_objects"]))
                self.assertEqual(len(metrics[0]["component_ids"]), expected_object_count)
                self.assertEqual(
                    [item["component_ids"] for item in metrics],
                    [metrics[0]["component_ids"]] * 3,
                )
                for item in metrics:
                    self.assertEqual(
                        sum(item["component_connected_counts"].values()),
                        item["components"],
                    )
                    self.assertEqual(
                        set(item["component_connected_counts"]),
                        metrics[0]["component_ids"],
                    )
                for item in metrics[1:]:
                    self.assertTrue(
                        all(
                            1 <= count <= metrics[0]["component_connected_counts"][component]
                            for component, count in item["component_connected_counts"].items()
                        )
                    )
                self.assertTrue(
                    all(
                        count > 0
                        for item in metrics
                        for count in item["component_triangle_counts"].values()
                    )
                )
                self.assertTrue(
                    all(item["boundary_edges"] <= metrics[0]["boundary_edges"] for item in metrics[1:])
                )

    def test_semantic_masks_bake_nonuniform_source_microdetail(self) -> None:
        staging = self.valid_staging()
        donor_hashes = {}
        recipe = load_recipe()
        for asset in recipe["part_assets"]:
            obj_path = staging / asset["lods"][0]["generated_obj"]
            groups = {
                line[2:]
                for line in obj_path.read_text(encoding="ascii").splitlines()
                if line.startswith("g ")
            }
            path = staging / asset["lods"][0]["semantic_mask"]
            width, height, pixels = read_rgba_png(path)
            occupied = [
                pixels[index : index + 4]
                for index in range(0, len(pixels), 4)
                if pixels[index + 3] > 0
            ]
            with self.subTest(asset=asset["id"]):
                self.assertEqual((width, height), (64, 64))
                self.assertEqual(
                    {tuple(pixel[:3]) for pixel in occupied},
                    {EXPECTED_GROUP_COLORS[group] for group in groups},
                )
                self.assertGreater(len({pixel[3] for pixel in occupied}), 8)
        for donor in ("norn", "ettin", "grendel"):
            asset = next(
                item
                for item in load_recipe()["part_assets"]
                if item["donor"] == donor and item["logical_slot"] == "head"
            )
            path = staging / asset["lods"][0]["semantic_mask"]
            width, height, pixels = read_rgba_png(path)
            self.assertEqual((width, height), (64, 64))
            rgba = [
                pixels[index : index + 4]
                for index in range(0, len(pixels), 4)
                if pixels[index + 3] > 0
            ]
            with self.subTest(donor=donor):
                self.assertGreater(len({pixel[3] for pixel in rgba}), 8)
            donor_hashes[donor] = hashlib.sha256(pixels).hexdigest()
        self.assertEqual(len(set(donor_hashes.values())), 3)

        alternate = TEST_OUTPUT / "alternate-texture-staging"
        self.assert_success(
            self.run_importer(
                "build",
                variant="alternate-texture",
                extra=("--staging", str(alternate)),
            )
        )
        norn_head = next(
            item for item in load_recipe()["part_assets"] if item["id"] == "norn-head"
        )
        valid_mask = staging / norn_head["lods"][0]["semantic_mask"]
        alternate_mask = alternate / norn_head["lods"][0]["semantic_mask"]
        self.assertNotEqual(hashlib.sha256(valid_mask.read_bytes()).digest(), hashlib.sha256(alternate_mask.read_bytes()).digest())
        valid_obj = staging / norn_head["lods"][0]["generated_obj"]
        alternate_obj = alternate / norn_head["lods"][0]["generated_obj"]
        self.assertEqual(hashlib.sha256(valid_obj.read_bytes()).digest(), hashlib.sha256(alternate_obj.read_bytes()).digest())

    def test_binary_alpha_maps_still_preserve_linked_color_texture_detail(self) -> None:
        staging = TEST_OUTPUT / "binary-microdetail-staging"
        self.assert_success(
            self.run_importer(
                "build",
                variant="binary-microdetail",
                extra=("--staging", str(staging)),
            )
        )
        recipe = load_recipe()
        for asset in recipe["part_assets"]:
            for lod in asset["lods"]:
                path = staging / lod["semantic_mask"]
                _, _, pixels = read_rgba_png(path)
                occupied_alpha = {
                    pixels[index + 3]
                    for index in range(0, len(pixels), 4)
                    if pixels[index + 3] > 0
                }
                with self.subTest(asset=asset["id"], lod=lod["lod"]):
                    self.assertGreater(len(occupied_alpha), 8)

    def test_parallel_build_failure_is_atomic(self) -> None:
        staging = TEST_OUTPUT / "atomic-existing-staging"
        staging.mkdir(parents=True, exist_ok=True)
        sentinel = staging / "preserve-me.txt"
        sentinel.write_text("existing production candidate\n", encoding="ascii")
        before = tree_digest(staging)

        completed = self.run_importer(
            "build",
            variant="broken-texture",
            extra=("--staging", str(staging)),
        )

        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(tree_digest(staging), before)
        self.assertFalse(
            list(staging.parent.glob(staging.name + ".tmp-*")),
            msg="failed parallel build left temporary staging behind",
        )

    def test_socket_manifests_emit_recipe_derived_bridge_preparation(self) -> None:
        staging = self.valid_staging()
        recipe = load_recipe()
        expected = {
            (family["id"], slot)
            for family in recipe["families"]
            for slot in family["parts"]
        }
        observed = set()
        for asset in recipe["part_assets"]:
            path = staging / asset["lods"][0]["socket_manifest"]
            manifest = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["asset_id"], asset["id"])
            self.assertEqual(
                manifest["assembly_preparation_schema"],
                "alife.geneforge_family_assembly.v1",
            )
            for prepared in manifest["assembly_preparations"]:
                observed.add((prepared["family_id"], prepared["logical_slot"]))
                expected_mode = (
                    "per-group-socket-transforms"
                    if prepared["logical_slot"] in {"arms", "legs"}
                    else "slot-transform"
                )
                self.assertEqual(prepared["transform_mode"], expected_mode)
                self.assertTrue(
                    all(math.isfinite(value) for value in prepared["prepared_translation"])
                )
                self.assertTrue(prepared["bridge_sockets"])
                self.assertTrue(prepared["bridge_kind"])
                self.assertTrue(prepared["join_cover_kind"])
                self.assertLessEqual(
                    prepared["attachment_error_bound"],
                    recipe["assembly_contract"]["attachment_error_limit"],
                )
                self.assertEqual(len(prepared["prepared_matrix"]), 16)
                self.assertEqual(prepared["prepared_matrix"][12:], [0.0, 0.0, 0.0, 1.0])
                self.assertEqual(
                    [
                        prepared["prepared_matrix"][3],
                        prepared["prepared_matrix"][7],
                        prepared["prepared_matrix"][11],
                    ],
                    prepared["prepared_translation"],
                )
                self.assertTrue(prepared["bridge_geometry"])
                residuals = []
                for bridge in prepared["bridge_geometry"]:
                    self.assertIn(bridge["socket"], prepared["bridge_sockets"])
                    self.assertGreater(bridge["prepared_vertex_count"], 0)
                    self.assertGreater(bridge["applied_overlap_depth"], 0.0)
                    self.assertLessEqual(
                        bridge["applied_overlap_depth"], prepared["overlap_depth"]
                    )
                    self.assertEqual(len(bridge["prepared_anchor"]), 3)
                    self.assertEqual(len(bridge["source_anchor"]), 3)
                    self.assertEqual(len(bridge["target_anchor"]), 3)
                    self.assertEqual(len(bridge["transformed_source_anchor"]), 3)
                    self.assertEqual(len(bridge["prepared_matrix"]), 16)
                    self.assertEqual(
                        bridge["prepared_matrix"][12:], [0.0, 0.0, 0.0, 1.0]
                    )
                    expected_group = {
                        "neck": "head",
                        "left-shoulder": "left-arm",
                        "right-shoulder": "right-arm",
                        "left-hip": "left-leg",
                        "right-hip": "right-leg",
                        "tail-base": "tail-back",
                    }.get(bridge["socket"], "torso")
                    if prepared["logical_slot"] == "torso":
                        expected_group = "torso"
                    self.assertEqual(bridge["runtime_group"], expected_group)
                    self.assertIn(bridge["runtime_group"], manifest["expected_groups"])
                    transformed = [
                        sum(
                            bridge["prepared_matrix"][row * 4 + axis]
                            * bridge["source_anchor"][axis]
                            for axis in range(3)
                        )
                        + bridge["prepared_matrix"][row * 4 + 3]
                        for row in range(3)
                    ]
                    for actual, expected_coordinate in zip(
                        bridge["transformed_source_anchor"], transformed
                    ):
                        self.assertAlmostEqual(actual, expected_coordinate, places=7)
                    residual = math.sqrt(
                        sum(
                            (transformed[axis] - bridge["target_anchor"][axis]) ** 2
                            for axis in range(3)
                        )
                    )
                    self.assertAlmostEqual(bridge["residual"], residual, places=7)
                    residuals.append(residual)
                self.assertAlmostEqual(
                    prepared["predicted_attachment_error"], max(residuals), places=7
                )
                self.assertLessEqual(
                    prepared["predicted_attachment_error"],
                    prepared["attachment_error_bound"],
                )
        self.assertEqual(observed, expected)

    def test_bridge_preparation_moves_source_geometry_within_overlap_bound(self) -> None:
        source = {
            "torso": [
                (((0.0, 0.0, 0.0), (0.0, 0.0), 0.5),
                 ((1.0, 0.0, 0.0), (1.0, 0.0), 0.5),
                 ((0.0, 1.0, 0.0), (0.0, 1.0), 0.5)),
            ]
        }
        prepared, evidence = importer.prepare_bridge_overlap_geometry(
            source,
            {"neck": {"translation": [0.0, 0.0, 1.0]}},
            ["neck"],
            0.02,
        )
        before = {corner[0] for triangle in source["torso"] for corner in triangle}
        after = {corner[0] for triangle in prepared["torso"] for corner in triangle}
        self.assertNotEqual(after, before)
        self.assertEqual(evidence[0]["socket"], "neck")
        self.assertGreater(evidence[0]["prepared_vertex_count"], 0)
        self.assertGreater(evidence[0]["applied_overlap_depth"], 0.0)
        self.assertLessEqual(evidence[0]["applied_overlap_depth"], 0.02)
        self.assertIn(tuple(evidence[0]["prepared_anchor"]), after)

    def test_preview_emits_nonempty_pngs_for_every_donor_and_lod(self) -> None:
        staging = TEST_OUTPUT / "staging-preview"
        self.assert_success(self.run_importer("build", extra=("--staging", str(staging))))
        preview_root = TEST_OUTPUT / "previews"
        preview_root.mkdir(parents=True, exist_ok=True)
        (preview_root / "stale-preview.png").write_bytes(b"stale")
        completed = self.run_importer(
            "preview",
            extra=("--staging", str(staging), "--output", str(preview_root)),
        )
        self.assert_success(completed)
        self.assertFalse((preview_root / "stale-preview.png").exists())
        previews = sorted(preview_root.glob("*.png"))
        self.assertEqual(len(previews), 14 * 3)
        for path in previews:
            width, height, pixels = read_rgba_png(path)
            with self.subTest(preview=path.name):
                self.assertEqual((width, height), (256, 256))
                self.assertGreater(len(set(pixels[index : index + 4] for index in range(0, len(pixels), 4))), 3)

    def test_build_is_byte_identical_across_two_clean_outputs(self) -> None:
        first = TEST_OUTPUT / "determinism-first"
        second = TEST_OUTPUT / "determinism-second"
        self.assert_success(self.run_importer("build", extra=("--staging", str(first))))
        self.assert_success(self.run_importer("build", extra=("--staging", str(second))))
        self.assertEqual(tree_digest(first), tree_digest(second))


if __name__ == "__main__":
    unittest.main(verbosity=2)
