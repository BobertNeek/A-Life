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

    def test_outer_v2_contracts_remain_stable_with_anatomy_authoring_v1(self) -> None:
        self.assertEqual(self.recipe["schema"], "alife.geneforge_creature_part_catalog.v2")
        self.assertEqual(self.recipe["schema_version"], 2)
        self.assertEqual(self.recipe["importer_version"], "alife.geneforge_importer.v2")
        self.assertEqual(len(self.recipe["part_assets"]), 14)
        for asset in self.recipe["part_assets"]:
            authoring = asset["anatomy_authoring"]
            self.assertEqual(authoring["schema"], "alife.geneforge_anatomy_authoring.v1")
            self.assertEqual(authoring["coordinate_space"], "semantic-group-local-uv")
            self.assertEqual(authoring["default_channel"], "primary")
            channels = {zone["channel"] for zone in authoring["zones"]} | {"primary"}
            self.assertTrue(REQUIRED_ANATOMY_CHANNELS[asset["logical_slot"]] <= channels)

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

    def test_anatomy_rasterizer_rejects_conflicting_equal_priority_overlap(self) -> None:
        semantic = importer.png_bytes(64, 64, bytes([230, 92, 88, 255]) * (64 * 64))
        profile = {
            "schema": "alife.geneforge_anatomy_authoring.v1",
            "coordinate_space": "semantic-group-local-uv",
            "default_channel": "primary",
            "zones": [
                {"id": "a", "channel": "muzzle", "semantic_groups": ["head"], "shape": {"kind": "ellipse", "center": [0.5, 0.5], "radius": [0.4, 0.4]}, "strength": 255, "priority": 10},
                {"id": "b", "channel": "inner-ear", "semantic_groups": ["head"], "shape": {"kind": "ellipse", "center": [0.5, 0.5], "radius": [0.4, 0.4]}, "strength": 255, "priority": 10},
                {"id": "c", "channel": "keratin-skin", "semantic_groups": ["head"], "shape": {"kind": "ellipse", "center": [0.05, 0.05], "radius": [0.02, 0.02]}, "strength": 255, "priority": 20},
                {"id": "d", "channel": "secondary-marking", "semantic_groups": ["head"], "shape": {"kind": "ellipse", "center": [0.95, 0.95], "radius": [0.02, 0.02]}, "strength": 255, "priority": 20},
            ],
        }
        with self.assertRaisesRegex(importer.ImportFailure, "equal-priority"):
            importer.anatomy_mask(semantic, profile, "head")

    def test_anatomy_rasterizer_is_deterministic_and_preserves_occupancy(self) -> None:
        semantic_pixels = bytearray(64 * 64 * 4)
        for y in range(1, 21):
            for x in range(1, 15):
                offset = (y * 64 + x) * 4
                semantic_pixels[offset : offset + 4] = bytes((*EXPECTED_GROUP_COLORS["head"], 37 + ((x + y) % 200)))
        semantic = importer.png_bytes(64, 64, bytes(semantic_pixels))
        profile = self.assets["norn-head"]["anatomy_authoring"]
        first = importer.anatomy_mask(semantic, profile, "head")
        second = importer.anatomy_mask(semantic, profile, "head")
        self.assertEqual(first, second)
        _, _, anatomy_pixels = importer.decode_rgba_png(first)
        for semantic_alpha, anatomy_alpha in zip(semantic_pixels[3::4], anatomy_pixels[3::4]):
            self.assertEqual(semantic_alpha > 0, anatomy_alpha > 0)
        used = {tuple(anatomy_pixels[i : i + 3]) for i in range(0, len(anatomy_pixels), 4) if anatomy_pixels[i + 3]}
        self.assertTrue(set(EXPECTED_ANATOMY_COLORS.values()) >= used)

    def test_anatomy_authoring_rejects_malformed_shapes_groups_and_channels(self) -> None:
        asset = json.loads(json.dumps(self.assets["norn-head"]))
        mutations = (
            ("malformed polygon", lambda zone: zone["shape"].update({"points": [[-0.1, 0.0], [0.2, 0.0], [0.2, 0.2]]})),
            ("unknown semantic group", lambda zone: zone.update({"semantic_groups": ["unknown"]})),
            ("not owned", lambda zone: zone.update({"channel": "belly"})),
            ("unknown shape", lambda zone: zone.update({"shape": {"kind": "rectangle"}})),
        )
        for expected, mutation in mutations:
            candidate = json.loads(json.dumps(asset))
            mutation(candidate["anatomy_authoring"]["zones"][0])
            with self.subTest(expected=expected):
                with self.assertRaisesRegex(importer.ImportFailure, expected):
                    importer.validate_anatomy_authoring(candidate)

    def test_zone_change_changes_only_anatomy_output(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        asset = self.assets["norn-head"]
        semantic = (staging / asset["lods"][0]["semantic_mask"]).read_bytes()
        profile = json.loads(json.dumps(asset["anatomy_authoring"]))
        before = importer.anatomy_mask(semantic, profile, "head")
        profile["zones"][2]["shape"]["center"][1] += 0.08
        after = importer.anatomy_mask(semantic, profile, "head")
        self.assertNotEqual(before, after)
        self.assertEqual(semantic, (staging / asset["lods"][0]["semantic_mask"]).read_bytes())

    def test_production_augmentation_matches_normal_build_rasterizer_for_all_lods(self) -> None:
        staging = WORKSPACE / "target/artifacts/creature_parts/geneforge-staging"
        for asset in self.recipe["part_assets"]:
            for lod in asset["lods"]:
                expected = importer.anatomy_mask(
                    (staging / lod["semantic_mask"]).read_bytes(),
                    asset["anatomy_authoring"],
                    asset["logical_slot"],
                )
                self.assertEqual(expected, (staging / lod["anatomy_mask"]).read_bytes())

    def test_every_family_and_independent_slot_combination_has_all_channels(self) -> None:
        channels_by_asset = {
            asset["id"]: {"primary"} | {zone["channel"] for zone in asset["anatomy_authoring"]["zones"]}
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
