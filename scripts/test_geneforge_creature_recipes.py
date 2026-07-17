#!/usr/bin/env python3
"""Contract tests for the deterministic GeneForge importer and recipe catalog."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import unittest


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

    def test_marker_map_is_exactly_one_through_fourteen(self) -> None:
        self.assertEqual(
            {int(key): value for key, value in self.recipe["marker_map"].items()},
            EXPECTED_MARKERS,
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
                self.assertNotIn("PPU", json.dumps(selector))

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
            for key in ("blend_file", "texture_root"):
                path = PurePosixPath(source[key])
                self.assertFalse(path.is_absolute())
                self.assertNotIn("..", path.parts)
        for asset in self.assets.values():
            for lod in asset["lods"]:
                for key in ("generated_obj", "socket_manifest", "semantic_mask"):
                    path = PurePosixPath(lod[key])
                    self.assertFalse(path.is_absolute())
                    self.assertNotIn("..", path.parts)
                    self.assertTrue(any(path.is_relative_to(root) for root in allowed_outputs))

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
            },
            "grendel-head": {
                "Head1_Grendel": [
                    "remove-loose-vertices",
                    "repair-declared-boundary-edges",
                ],
            },
            "grendel-arms": {
                "radius L": ["repair-declared-non-manifold-edges"],
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
    ) -> subprocess.CompletedProcess[str]:
        root = FIXTURE_ROOT / variant
        invocation = [
            sys.executable,
            str(IMPORTER),
            command,
            "--source-root",
            str(root),
            "--recipes",
            str(root / "fixture_recipes.json"),
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

    def test_inventory_reports_fixture_features_and_exact_names(self) -> None:
        output = TEST_OUTPUT / "inventory.json"
        completed = self.run_importer("inventory", extra=("--output", str(output)))
        self.assert_success(completed)
        inventory = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual({entry["donor"] for entry in inventory["sources"]}, {"norn", "ettin", "grendel"})
        for source in inventory["sources"]:
            self.assertEqual(source["marker_ids"], list(range(1, 15)))
            self.assertTrue(source["has_constraint"])
            self.assertTrue(source["has_geometry_nodes"])
            self.assertTrue(source["has_armature"])
            self.assertTrue(source["has_declared_non_manifold"])
            self.assertIn(source["primary_uv"], {"UVMap", "UVChannel_1"})

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
        self.assertGreater(receipt["topology"]["removed_degenerate_faces"], 0)
        self.assertGreater(receipt["topology"]["removed_loose_vertices"], 0)
        staged_files = [path for path in staging.rglob("*") if path.is_file()]
        self.assertTrue(all(path.stat().st_size <= 512 * 1024 for path in staged_files))
        self.assertLessEqual(sum(path.stat().st_size for path in staged_files), 8 * 1024 * 1024)
        for donor in ("norn", "ettin", "grendel"):
            for lod in ("full", "compact", "impostor"):
                self.assertTrue((staging / f"production_voxel_v1/creature_parts/generated/geneforge/{donor}_{lod}_parts.obj").is_file())
                self.assertTrue((staging / f"production_voxel_v1/creature_parts/generated/geneforge/{donor}_{lod}_sockets.json").is_file())
                self.assertTrue((staging / f"production_voxel_v1/models/geneforge/{donor}_{lod}_semantic.png").is_file())

    def test_preview_emits_nonempty_pngs_for_every_donor_and_lod(self) -> None:
        staging = TEST_OUTPUT / "staging-preview"
        self.assert_success(self.run_importer("build", extra=("--staging", str(staging))))
        preview_root = TEST_OUTPUT / "previews"
        completed = self.run_importer(
            "preview",
            extra=("--staging", str(staging), "--output", str(preview_root)),
        )
        self.assert_success(completed)
        previews = sorted(preview_root.glob("*.png"))
        self.assertEqual(len(previews), 9)
        self.assertTrue(all(path.stat().st_size > 100 for path in previews))

    def test_build_is_byte_identical_across_two_clean_outputs(self) -> None:
        first = TEST_OUTPUT / "determinism-first"
        second = TEST_OUTPUT / "determinism-second"
        self.assert_success(self.run_importer("build", extra=("--staging", str(first))))
        self.assert_success(self.run_importer("build", extra=("--staging", str(second))))
        self.assertEqual(tree_digest(first), tree_digest(second))


if __name__ == "__main__":
    unittest.main(verbosity=2)
