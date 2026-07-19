#!/usr/bin/env python3
"""Deterministic Blender 5.1 importer for staged GeneForge creature parts.

Path confinement assumes one importer/validator owns a staging tree at a time.
Callers must not run a concurrent filesystem mutator that can swap checked path
components between validation and open; symlink and Windows reparse components
are rejected at each normal access boundary.
"""

from __future__ import annotations

import argparse
from collections import deque
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import sys
import tempfile
import zlib


WORKSPACE = Path(__file__).resolve().parents[1]
ARTIFACT_ROOT = (WORKSPACE / "target/artifacts").resolve()
DEFAULT_BLENDER = Path(r"C:\Program Files\Blender Foundation\Blender 5.1\blender.exe")
REQUIRED_BLENDER_VERSION = "5.1.0"
REQUIRED_IMPORTER_VERSION = "alife.geneforge_importer.v2"
MAX_DONOR_WORKERS = 3
FILE_ATTRIBUTE_REPARSE_POINT = 0x0400
MAX_STAGED_FILE_BYTES = 512 * 1024
PNG_WIDTH = 64
PNG_HEIGHT = 64
PNG_FILTERED_BYTES = PNG_HEIGHT * (1 + PNG_WIDTH * 4)
ANATOMY_AUTHORING_SCHEMA = "alife.geneforge_anatomy_authoring.v2"
ANATOMY_PROJECTION_SCHEMA = "alife.geneforge_anatomy_projection.v1"
ANATOMY_SOURCE_GEOMETRY_SCHEMA = "alife.geneforge_source_geometry_classifier.v2"
ANATOMY_AUDIT_SCHEMA = "alife.geneforge_source_projection_audit.v1"
ANATOMY_TRIANGLE_TIE_BREAK = (
    "inside-max-min-barycentric-then-face-index;nearest-uv-then-face-index"
)
ANATOMY_CLASSIFIER = "source-geometry-feature-anchors.v3"
ANATOMY_TRANSACTION_SCHEMA = "alife.geneforge_anatomy_transaction.v2"
PREPARATION_SCHEMA = "alife.geneforge_assembly_preparation.v2"
PREPARATION_VERSION = 2
PREPARATION_AUGMENTOR_VERSION = "alife.geneforge_assembly_augmentor.v1"
PREPARATION_TRANSFORM_SPACE = "alife.creature.canonical.rhs-y-up-neg-z-forward.v1"
PREPARATION_MATRIX_LAYOUT = (
    "row-major-4x4-affine;point=[x,y,z,1];translation=[3,7,11];bottom-row=[0,0,0,1]"
)
PREPARATION_KEY_FIELDS = [
    "source_family_id",
    "source_asset_id",
    "target_torso_asset_id",
    "lod",
    "runtime_group",
    "socket",
]
PREPARATION_RECORD_FIELDS = PREPARATION_KEY_FIELDS + [
    "transform_space",
    "schema_digest",
    "prepared_matrix",
    "residual",
]
PREPARATION_LOD_ORDER = ["full", "compact", "impostor"]
PREPARATION_RUNTIME_GROUP_ORDER = [
    "head",
    "torso",
    "left-arm",
    "right-arm",
    "left-leg",
    "right-leg",
    "tail-back",
]
PREPARATION_SOCKET_ORDER = [
    "neck",
    "left-shoulder",
    "right-shoulder",
    "left-hip",
    "right-hip",
    "tail-base",
    "torso-frame",
]
PREPARATION_TORSO_ASSETS = ["norn-torso", "ettin-torso", "grendel-torso"]
PREPARATION_GROUP_SOCKET = {
    "head": "neck",
    "torso": "torso-frame",
    "left-arm": "left-shoulder",
    "right-arm": "right-shoulder",
    "left-leg": "left-hip",
    "right-leg": "right-hip",
    "tail-back": "tail-base",
}
PREPARATION_GROUP_FOR_SOCKET = {
    "neck": "head",
    "left-shoulder": "left-arm",
    "right-shoulder": "right-arm",
    "left-hip": "left-leg",
    "right-hip": "right-leg",
    "tail-base": "tail-back",
}
PREPARATION_SLOT_GROUPS = {
    "head": ["head"],
    "torso": ["torso"],
    "arms": ["left-arm", "right-arm"],
    "legs": ["left-leg", "right-leg"],
    "tail": ["tail-back"],
}
LODS = (("full", 1.0, 1200), ("compact", 0.65, 800), ("impostor", 0.35, 400))
GROUP_COLORS = {
    "head": (230, 92, 88, 255),
    "torso": (64, 166, 184, 255),
    "left-arm": (244, 177, 76, 255),
    "right-arm": (244, 177, 76, 255),
    "left-leg": (95, 177, 104, 255),
    "right-leg": (95, 177, 104, 255),
    "tail": (154, 108, 180, 255),
    "tail-back": (154, 108, 180, 255),
    "head.eyes": (238, 238, 224, 255),
    "head.lids": (184, 80, 96, 255),
    "head.hair": (114, 84, 145, 255),
    "head.teeth": (235, 222, 188, 255),
    "head.tongue": (213, 92, 126, 255),
}
GROUP_REGIONS = {
    group: (index % 4, index // 4)
    for index, group in enumerate(
        (
            "head",
            "head.eyes",
            "head.lids",
            "head.hair",
            "head.teeth",
            "head.tongue",
            "torso",
            "left-arm",
            "right-arm",
            "left-leg",
            "right-leg",
            "tail-back",
        )
    )
}
ANATOMY_COLORS = {
    "primary": (248, 248, 248),
    "belly": (232, 176, 72),
    "muzzle": (226, 112, 128),
    "inner-ear": (238, 86, 154),
    "hands-feet": (72, 174, 218),
    "keratin-skin": (64, 52, 72),
    "secondary-marking": (84, 92, 214),
}
ALLOWED_ANATOMY_CHANNELS = {
    "head": {"primary", "muzzle", "inner-ear", "keratin-skin", "secondary-marking"},
    "torso": {"primary", "belly", "keratin-skin", "secondary-marking"},
    "arms": {"primary", "hands-feet", "keratin-skin", "secondary-marking"},
    "legs": {"primary", "hands-feet", "keratin-skin", "secondary-marking"},
    "tail": {"primary", "keratin-skin", "secondary-marking"},
}
REQUIRED_ANATOMY_CHANNELS = {
    "head": {"primary", "muzzle", "inner-ear", "keratin-skin", "secondary-marking"},
    "torso": {"primary", "belly", "secondary-marking"},
    "arms": {"primary", "hands-feet", "secondary-marking"},
    "legs": {"primary", "hands-feet", "secondary-marking"},
    "tail": {"primary", "keratin-skin", "secondary-marking"},
}
ANATOMY_DETAIL_GROUP_CHANNELS = {
    "head.hair": "secondary-marking",
    "head.teeth": "keratin-skin",
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


def preparation_schema_digest(contract: dict) -> str:
    descriptor = dict(contract)
    descriptor["schema_digest"] = "0" * 64
    payload = json.dumps(
        descriptor, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(payload).hexdigest()


def preparation_contract() -> dict:
    contract = {
        "schema": PREPARATION_SCHEMA,
        "version": PREPARATION_VERSION,
        "augmentor_version": PREPARATION_AUGMENTOR_VERSION,
        "transform_space": PREPARATION_TRANSFORM_SPACE,
        "matrix_layout": PREPARATION_MATRIX_LAYOUT,
        "key_fields": list(PREPARATION_KEY_FIELDS),
        "required_record_fields": list(PREPARATION_RECORD_FIELDS),
        "lod_order": list(PREPARATION_LOD_ORDER),
        "runtime_group_order": list(PREPARATION_RUNTIME_GROUP_ORDER),
        "socket_order": list(PREPARATION_SOCKET_ORDER),
        "residual_limit": 0.025,
        "schema_digest": "0" * 64,
    }
    contract["schema_digest"] = preparation_schema_digest(contract)
    return contract


def preparation_key(record: dict) -> str:
    return "|".join(str(record[field]) for field in PREPARATION_KEY_FIELDS)


def preparation_sort_key(record: dict) -> tuple:
    target_order = {asset: index for index, asset in enumerate(PREPARATION_TORSO_ASSETS)}
    lod_order = {lod: index for index, lod in enumerate(PREPARATION_LOD_ORDER)}
    group_order = {
        group: index for index, group in enumerate(PREPARATION_RUNTIME_GROUP_ORDER)
    }
    socket_order = {socket: index for index, socket in enumerate(PREPARATION_SOCKET_ORDER)}
    return (
        int(record["source_family_id"]),
        str(record["source_asset_id"]),
        target_order.get(record["target_torso_asset_id"], 999),
        lod_order.get(record["lod"], 999),
        group_order.get(record["runtime_group"], 999),
        socket_order.get(record["socket"], 999),
    )


def validate_preparation_contract(recipe: dict) -> dict:
    contract = recipe.get("assembly_preparation_contract")
    if not isinstance(contract, dict) or contract != preparation_contract():
        raise ImportFailure("recipe assembly preparation contract is missing or invalid")
    if recipe.get("group_key_counts") != {"canonical": 252, "cross_torso": 432, "total": 684}:
        raise ImportFailure("recipe assembly preparation group-key counts are invalid")
    return contract


def load_recipe(path: Path, *, validate_current_anatomy: bool = True) -> dict:
    recipe = json.loads(path.read_text(encoding="utf-8"))
    actual = canonical_recipe_digest(recipe)
    if recipe.get("recipe_sha256", "").lower() != actual:
        raise ImportFailure(
            f"recipe digest mismatch: expected {recipe.get('recipe_sha256')}, calculated {actual}"
        )
    if recipe.get("blender_version") != REQUIRED_BLENDER_VERSION:
        raise ImportFailure("recipe does not pin Blender 5.1.0")
    if recipe.get("importer_version") != REQUIRED_IMPORTER_VERSION:
        raise ImportFailure(f"recipe does not pin {REQUIRED_IMPORTER_VERSION}")
    validate_preparation_contract(recipe)
    for asset in recipe.get("part_assets", []):
        selector = asset.get("selector", {})
        if selector.get("selection_policy") != "exact-case-sensitive-names":
            raise ImportFailure(f"{asset.get('id')} has an unsupported selection policy")
        if selector.get("geometry_policy") != "evaluated-depsgraph":
            raise ImportFailure(f"{asset.get('id')} has an unsupported geometry policy")
        if set(selector.get("object_visscripts", {})) != set(selector.get("include_objects", [])):
            raise ImportFailure(f"{asset.get('id')} lacks exact kc3dsbpy_visscript contracts")
        if validate_current_anatomy:
            validate_anatomy_authoring(asset)
            validate_anatomy_source_audit(asset)
        for lod in asset.get("lods", []):
            for field in ("anatomy_mask", "anatomy_mask_sha256"):
                if field not in lod:
                    raise ImportFailure(f"{asset.get('id')} {lod.get('lod')} lacks {field}")
    return recipe


def validate_anatomy_authoring(asset: dict) -> None:
    profile = asset.get("anatomy_authoring")
    slot = asset.get("logical_slot")
    if not isinstance(profile, dict) or profile.get("schema") != ANATOMY_AUTHORING_SCHEMA:
        raise ImportFailure(f"{asset.get('id')} has invalid anatomy authoring schema")
    if (
        profile.get("coordinate_space") != "same-lod-staged-obj"
        or profile.get("default_channel") != "primary"
        or "zones" in profile
    ):
        raise ImportFailure(f"{asset.get('id')} has invalid anatomy authoring coordinates/default")
    if slot not in ALLOWED_ANATOMY_CHANNELS:
        raise ImportFailure(f"{asset.get('id')} has unsupported anatomy slot {slot}")
    required = profile.get("required_channels")
    if (
        not isinstance(required, list)
        or len(required) != len(set(required))
        or set(required) != REQUIRED_ANATOMY_CHANNELS[slot]
    ):
        raise ImportFailure(f"{asset.get('id')} has invalid required anatomy channels")
    projection = profile.get("projection")
    source_geometry = projection.get("source_geometry") if isinstance(projection, dict) else None
    if (
        not isinstance(projection, dict)
        or projection.get("schema") != ANATOMY_PROJECTION_SCHEMA
        or projection.get("texel_sample") != "pixel-center"
        or projection.get("triangle_tie_break") != ANATOMY_TRIANGLE_TIE_BREAK
        or projection.get("classifier") != ANATOMY_CLASSIFIER
        or projection.get("detail_group_channels") != ANATOMY_DETAIL_GROUP_CHANNELS
        or not isinstance(source_geometry, dict)
        or source_geometry.get("schema") != ANATOMY_SOURCE_GEOMETRY_SCHEMA
        or not isinstance(source_geometry.get("groups"), list)
        or not source_geometry["groups"]
        or len(source_geometry["groups"]) != len(set(source_geometry["groups"]))
        or any(not isinstance(group, str) or group not in GROUP_REGIONS for group in source_geometry["groups"])
        or not isinstance(source_geometry.get("landmarks"), dict)
        or not source_geometry["landmarks"]
        or any(
            not isinstance(name, str)
            or not name
            or not _finite_vector(point, 3)
            for name, point in source_geometry["landmarks"].items()
        )
        or not isinstance(source_geometry.get("canonical_bounds"), dict)
        or not _finite_vector(source_geometry["canonical_bounds"].get("min"), 3)
        or not _finite_vector(source_geometry["canonical_bounds"].get("max"), 3)
        or any(
            lower >= upper
            for lower, upper in zip(
                source_geometry["canonical_bounds"]["min"],
                source_geometry["canonical_bounds"]["max"],
            )
        )
    ):
        raise ImportFailure(f"{asset.get('id')} has invalid OBJ projection policy")
    if "groups" in asset:
        expected_groups = set(asset["groups"].values())
        for role in asset.get("detail_groups", {}):
            expected_groups.add(f"head.{role}")
        if set(source_geometry["groups"]) != expected_groups:
            raise ImportFailure(f"{asset.get('id')} source geometry groups are not source-bound")
    expected_features = REQUIRED_FEATURE_ANCHORS[slot]
    feature_landmarks = source_geometry.get("feature_landmarks")
    if (
        not isinstance(feature_landmarks, dict)
        or set(feature_landmarks) != set(expected_features)
        or any(
            not isinstance(anchor, dict)
            or anchor.get("channel") != channel
            or anchor.get("runtime_group") != runtime_group
            or anchor.get("source_group") != runtime_group
            or anchor.get("method") != "source-geometry-anchor-v1"
            or not isinstance(anchor.get("source_basis"), list)
            or not anchor["source_basis"]
            or any(not isinstance(basis, str) or not basis for basis in anchor["source_basis"])
            or not _finite_vector(anchor.get("point"), 3)
            or not _finite_vector(anchor.get("source_position"), 3)
            or runtime_group not in source_geometry["groups"]
            for name, (channel, runtime_group) in expected_features.items()
            for anchor in [feature_landmarks.get(name)]
        )
    ):
        raise ImportFailure(f"{asset.get('id')} has invalid source feature landmarks")
    if "landmarks" in asset and source_geometry["landmarks"] != asset["landmarks"]:
        raise ImportFailure(f"{asset.get('id')} source geometry landmarks are not source-bound")
    if "canonical_bounds" in asset and source_geometry["canonical_bounds"] != asset["canonical_bounds"]:
        raise ImportFailure(f"{asset.get('id')} source geometry bounds are not source-bound")


def validate_anatomy_source_audit(asset: dict) -> None:
    audit = asset.get("anatomy_authoring", {}).get("source_projection_audit")
    if not isinstance(audit, dict) or audit.get("schema") != ANATOMY_AUDIT_SCHEMA:
        raise ImportFailure(f"{asset.get('id')} has invalid source anatomy audit evidence")
    source_geometry = (
        asset.get("anatomy_authoring", {}).get("projection", {}).get("source_geometry")
    )
    source_feature_landmarks = (
        source_geometry.get("feature_landmarks")
        if isinstance(source_geometry, dict)
        else None
    )
    lods = audit.get("lods")
    if not isinstance(lods, dict) or set(lods) != {lod for lod, _, _ in LODS}:
        raise ImportFailure(f"{asset.get('id')} source anatomy audit lacks every LOD")
    required_channels = REQUIRED_ANATOMY_CHANNELS[asset["logical_slot"]]
    for lod_name, evidence in lods.items():
        bounds = evidence.get("source_bounds") if isinstance(evidence, dict) else None
        if (
            not isinstance(evidence, dict)
            or any(
                not isinstance(evidence.get(field), str)
                or re.fullmatch(r"[0-9a-f]{64}", evidence[field]) is None
                for field in ("obj_sha256", "semantic_sha256", "projection_sha256")
            )
            or not isinstance(evidence.get("projected_texels"), int)
            or evidence["projected_texels"] <= 0
            or not isinstance(evidence.get("inside_texels"), int)
            or not isinstance(evidence.get("nearest_texels"), int)
            or evidence["projected_texels"]
            != evidence["inside_texels"] + evidence["nearest_texels"]
            or not isinstance(evidence.get("overlap_texels"), int)
            or not 0 <= evidence["overlap_texels"] <= evidence["inside_texels"]
            or not isinstance(evidence.get("runtime_group_counts"), dict)
            or not evidence["runtime_group_counts"]
            or any(
                not isinstance(group, str)
                or group not in GROUP_REGIONS
                or not isinstance(count, int)
                or count <= 0
                for group, count in evidence["runtime_group_counts"].items()
            )
            or not isinstance(evidence.get("channel_counts"), dict)
            or set(evidence["channel_counts"]) != required_channels
            or any(
                not isinstance(count, int) or count <= 0
                for count in evidence["channel_counts"].values()
            )
            or not isinstance(bounds, dict)
            or not _finite_vector(bounds.get("min"), 3)
            or not _finite_vector(bounds.get("max"), 3)
            or any(lower > upper for lower, upper in zip(bounds["min"], bounds["max"]))
            or not isinstance(evidence.get("derived_landmarks"), dict)
            or not evidence["derived_landmarks"]
            or any(
                not isinstance(name, str) or not name or not _finite_vector(point, 3)
                for name, point in evidence["derived_landmarks"].items()
            )
            or not isinstance(evidence.get("source_landmark_projections"), dict)
            or not evidence["source_landmark_projections"]
            or any(
                not isinstance(name, str)
                or not isinstance(projection, dict)
                or not _finite_vector(projection.get("source"), 3)
                or not _finite_vector(projection.get("projected"), 3)
                or not isinstance(projection.get("face"), int)
                or projection["face"] < 0
                or not isinstance(projection.get("group"), str)
                or projection["group"] not in GROUP_REGIONS
                or not isinstance(projection.get("weights"), list)
                or len(projection["weights"]) != 3
                or any(
                    not isinstance(weight, (int, float))
                    or isinstance(weight, bool)
                    or not math.isfinite(weight)
                    or weight < -1.0e-6
                    or weight > 1.0 + 1.0e-6
                    for weight in projection["weights"]
                )
                or abs(sum(projection["weights"]) - 1.0) > 1.0e-5
                or not isinstance(projection.get("distance"), (int, float))
                or isinstance(projection["distance"], bool)
                or not math.isfinite(projection["distance"])
                or projection["distance"] < 0.0
                for name, projection in evidence["source_landmark_projections"].items()
            )
            or not isinstance(evidence.get("feature_anchor_ownership"), dict)
            or set(evidence["feature_anchor_ownership"]) != set(
                REQUIRED_FEATURE_ANCHORS[asset["logical_slot"]]
            )
            or any(
                not isinstance(name, str)
                or not isinstance(ownership, dict)
                or not isinstance(source_anchor, dict)
                or ownership.get("channel") != channel
                or not isinstance(ownership.get("owned_channel"), str)
                or ownership["owned_channel"] != ownership["channel"]
                or ownership["owned_channel"] != channel
                or ownership["owned_channel"] != source_anchor.get("channel")
                or ownership.get("runtime_group") != runtime_group
                or ownership.get("group") != runtime_group
                or not isinstance(ownership.get("x"), int)
                or not 0 <= ownership["x"] < PNG_WIDTH
                or not isinstance(ownership.get("y"), int)
                or not 0 <= ownership["y"] < PNG_HEIGHT
                or not _finite_vector(ownership.get("canonical"), 3)
                or not _finite_vector(ownership.get("source"), 3)
                or not _finite_vector(ownership.get("projected"), 3)
                or not isinstance(ownership.get("face"), int)
                or ownership["face"] < 0
                or not isinstance(ownership.get("weights"), list)
                or len(ownership["weights"]) != 3
                or any(
                    not isinstance(weight, (int, float))
                    or isinstance(weight, bool)
                    or not math.isfinite(weight)
                    or weight < -1.0e-6
                    or weight > 1.0 + 1.0e-6
                    for weight in ownership["weights"]
                )
                or abs(sum(ownership["weights"]) - 1.0) > 1.0e-5
                or not isinstance(ownership.get("distance"), (int, float))
                or isinstance(ownership["distance"], bool)
                or not math.isfinite(ownership["distance"])
                or ownership["distance"] < 0.0
                for name, (channel, runtime_group) in REQUIRED_FEATURE_ANCHORS[
                    asset["logical_slot"]
                ].items()
                for ownership in [evidence["feature_anchor_ownership"].get(name)]
                for source_anchor in [
                    source_feature_landmarks.get(name)
                    if isinstance(source_feature_landmarks, dict)
                    else None
                ]
            )
            or not isinstance(evidence.get("geometry_classification"), dict)
            or any(
                channel not in ALLOWED_ANATOMY_CHANNELS[asset["logical_slot"]]
                or not isinstance(classification, dict)
                or not isinstance(classification.get("groups"), list)
                or not classification["groups"]
                or any(group not in GROUP_REGIONS for group in classification["groups"])
                or not isinstance(classification.get("landmarks"), list)
                or any(not isinstance(name, str) or not name for name in classification["landmarks"])
                for channel, classification in evidence["geometry_classification"].items()
            )
        ):
            raise ImportFailure(
                f"{asset.get('id')} {lod_name} has invalid source projection audit evidence"
            )


def _finite_vector(value, length: int) -> bool:
    return (
        isinstance(value, list)
        and len(value) == length
        and all(
            isinstance(component, (int, float))
            and not isinstance(component, bool)
            and math.isfinite(component)
            for component in value
        )
    )


def ensure_artifact_path(path: Path, label: str) -> Path:
    resolved = path.resolve()
    try:
        resolved.relative_to(ARTIFACT_ROOT)
    except ValueError as error:
        raise ImportFailure(f"{label} must stay under {ARTIFACT_ROOT}") from error
    return resolved


def canonical_path_is_within(canonical_root: Path, canonical_candidate: Path) -> bool:
    try:
        canonical_candidate.relative_to(canonical_root)
    except ValueError:
        return False
    return True


def _relative_staged_path(relative: str | Path) -> Path:
    raw = Path(relative)
    if (
        raw.is_absolute()
        or not raw.parts
        or any(part == ".." for part in raw.parts)
    ):
        raise ImportFailure(f"generated output escapes staging: {relative}")
    return raw


def _is_symlink_or_reparse(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return False
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & FILE_ATTRIBUTE_REPARSE_POINT
    )


def _canonical_staging_root(staging: Path) -> Path:
    try:
        root = staging.resolve(strict=True)
    except OSError as error:
        raise ImportFailure(f"staging root is missing or inaccessible: {staging}") from error
    if not root.is_dir():
        raise ImportFailure(f"staging root is not a directory: {staging}")
    return root


def confined_existing_staged_path(staging: Path, relative: str | Path, label: str) -> Path:
    raw = _relative_staged_path(relative)
    root = _canonical_staging_root(staging)
    candidate = root
    for component in raw.parts:
        candidate = candidate / component
        if _is_symlink_or_reparse(candidate):
            raise ImportFailure(
                f"{label} contains a symlink or reparse point: {raw.as_posix()}"
            )
        try:
            candidate.lstat()
        except OSError as error:
            raise ImportFailure(f"missing {label}: {raw.as_posix()}") from error
    try:
        canonical = candidate.resolve(strict=True)
    except OSError as error:
        raise ImportFailure(f"missing {label}: {raw.as_posix()}") from error
    if not canonical_path_is_within(root, canonical):
        raise ImportFailure(f"{label} escapes canonical staging root: {raw.as_posix()}")
    if not canonical.is_file():
        raise ImportFailure(f"{label} is not a regular file: {raw.as_posix()}")
    return canonical


def _assert_tree_has_no_reparse_entries(staging: Path) -> None:
    root = _canonical_staging_root(staging)
    pending = [root]
    while pending:
        directory = pending.pop()
        try:
            entries = sorted(os.scandir(directory), key=lambda entry: entry.name)
        except OSError as error:
            raise ImportFailure(f"cannot inspect staged directory: {directory}") from error
        for entry in entries:
            path = Path(entry.path)
            if _is_symlink_or_reparse(path):
                raise ImportFailure(
                    f"staged tree contains a symlink or reparse point: {path.relative_to(root)}"
                )
            if entry.is_dir(follow_symlinks=False):
                pending.append(path)


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


def confined_source_path(source_root: Path, relative: str, label: str) -> Path:
    raw = Path(relative)
    if raw.is_absolute() or any(part == ".." for part in raw.parts):
        raise ImportFailure(f"{label} escapes --source-root: {relative}")
    root = source_root.resolve()
    resolved = (root / raw).resolve()
    try:
        resolved.relative_to(root)
    except ValueError as error:
        raise ImportFailure(f"{label} escapes --source-root: {relative}") from error
    return resolved


def validate_source_files(source_root: Path, recipe: dict) -> None:
    for source in recipe["sources"]:
        path = confined_source_path(
            source_root, source["blend_file"], f"{source['donor']} source file"
        )
        if not path.is_file():
            raise ImportFailure(f"missing {source['donor']} source file: {path}")
        actual = sha256_file(path).upper()
        if actual != source["sha256"].upper():
            raise ImportFailure(
                f"{source['donor']} source digest mismatch: expected {source['sha256']}, found {actual}"
            )
        texture_root = confined_source_path(
            source_root, source["texture_root"], f"{source['donor']} texture root"
        )
        if not texture_root.is_dir():
            raise ImportFailure(f"missing {source['donor']} texture root: {texture_root}")
        microdetail_root = confined_source_path(
            source_root,
            source["microdetail_root"],
            f"{source['donor']} microdetail root",
        )
        if not microdetail_root.is_dir():
            raise ImportFailure(
                f"missing {source['donor']} microdetail root: {microdetail_root}"
            )


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
        str(confined_source_path(source_root, source["blend_file"], f"{source['donor']} source file")),
        "--texture-root",
        str(confined_source_path(source_root, source["texture_root"], f"{source['donor']} texture root")),
        "--microdetail-root",
        str(
            confined_source_path(
                source_root,
                source["microdetail_root"],
                f"{source['donor']} microdetail root",
            )
        ),
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
    with tempfile.TemporaryDirectory(prefix="geneforge-inspect-", dir=ARTIFACT_ROOT) as temp:
        temp_root = Path(temp)

        def inspect_source(source: dict) -> dict:
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
            return json.loads(output.read_text(encoding="utf-8"))

        sources = list(recipe["sources"])
        with ThreadPoolExecutor(
            max_workers=min(MAX_DONOR_WORKERS, len(sources)),
            thread_name_prefix="geneforge-inspect",
        ) as executor:
            return list(executor.map(inspect_source, sources))


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
    marker_contracts = []
    for item in results:
        marker_ids = item["marker_ids"]
        if marker_ids == list(range(marker_ids[0], marker_ids[-1] + 1)):
            serialized = f"{marker_ids[0]}..{marker_ids[-1]}"
        else:
            serialized = ",".join(str(marker_id) for marker_id in marker_ids)
        marker_contracts.append(f"{item['donor']}:{serialized}")
    print("marker_ids=" + ",".join(marker_contracts))


def command_build(args, recipe: dict, blender: Path) -> None:
    staging = ensure_artifact_path(args.staging, "staging output")
    staging.parent.mkdir(parents=True, exist_ok=True)
    temporary = staging.with_name(staging.name + f".tmp-{os.getpid()}")
    if temporary.exists():
        shutil.rmtree(temporary)
    temporary.mkdir(parents=True)
    sources = list(recipe["sources"])
    max_workers = min(MAX_DONOR_WORKERS, len(sources))
    try:
        def build_source(source: dict) -> dict:
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
            receipt = json.loads(output.read_text(encoding="utf-8"))
            output.unlink()
            return receipt

        with ThreadPoolExecutor(
            max_workers=max_workers,
            thread_name_prefix="geneforge-build",
        ) as executor:
            receipts = list(executor.map(build_source, sources))
        postprocess_assembly_preparations(recipe, temporary)
        staged_manifests, _ = _staged_socket_manifests(temporary, recipe)
        canonical_preparation = validate_preparation_metadata(
            recipe, staged_manifests, require_cross_torso=False
        )
        _assert_tree_has_no_reparse_entries(temporary)
        outputs = {
            path.relative_to(temporary).as_posix(): sha256_file(
                confined_existing_staged_path(
                    temporary,
                    path.relative_to(temporary),
                    "newly built staged output",
                )
            )
            for path in sorted(temporary.rglob("*"))
            if path.is_file()
        }
        receipt = {
            "schema": "alife.geneforge_build_receipt.v2",
            "blender_version": REQUIRED_BLENDER_VERSION,
            "importer_version": recipe["importer_version"],
            "recipe_sha256": recipe["recipe_sha256"],
            "source_sha256": {
                source["donor"]: source["sha256"] for source in recipe["sources"]
            },
            "donor_count": len(receipts),
            "asset_count": sum(item["asset_count"] for item in receipts),
            "lods": [name for name, _, _ in LODS],
            "worker_execution": {
                "strategy": "bounded-parallel-donor-workers",
                "max_workers": max_workers,
            },
            "sources": receipts,
            "topology": {
                key: sum(item["topology"][key] for item in receipts)
                for key in (
                    "removed_degenerate_faces",
                    "removed_duplicate_vertices",
                    "removed_loose_vertices",
                    "repaired_non_manifold_edges",
                    "filled_boundary_edges",
                )
            },
            "assembly_preparation": {
                "schema": recipe["assembly_preparation_contract"]["schema"],
                "augmentor_version": recipe["assembly_preparation_contract"]["augmentor_version"],
                "schema_digest": recipe["assembly_preparation_contract"]["schema_digest"],
                "canonical_slot_records": canonical_preparation["canonical_slot_records"],
                "cross_torso_slot_records": 0,
                "canonical_group_keys": canonical_preparation["canonical_group_keys"],
                "cross_torso_group_keys": 0,
                "total_group_keys": canonical_preparation["total_group_keys"],
                "stable_hashes": _stable_output_hashes(temporary, recipe),
            },
            "outputs": outputs,
        }
        staged_output_path(temporary, "build_receipt.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        _verify_receipt_outputs(
            temporary, receipt, recipe, require_recipe_output_digests=False
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


def decode_rgba_png(data: bytes) -> tuple[int, int, bytes]:
    if len(data) > MAX_STAGED_FILE_BYTES:
        raise ImportFailure("anatomy source PNG exceeds 512 KiB")
    if len(data) < 8 or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ImportFailure("anatomy source is not a PNG")
    offset = 8
    width = height = None
    compressed = bytearray()
    saw_iend = False
    saw_idat = False
    while offset < len(data):
        if len(data) - offset < 12:
            raise ImportFailure("anatomy source PNG has a truncated chunk header")
        length = int.from_bytes(data[offset : offset + 4], "big")
        chunk_end = offset + 12 + length
        if chunk_end > len(data):
            raise ImportFailure("anatomy source PNG has a truncated chunk payload")
        kind = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        expected_crc = int.from_bytes(data[offset + 8 + length : chunk_end], "big")
        if zlib.crc32(kind + payload) & 0xFFFFFFFF != expected_crc:
            raise ImportFailure("anatomy source PNG has an invalid chunk checksum")
        offset = chunk_end
        if width is None and kind != b"IHDR":
            raise ImportFailure("anatomy source PNG must begin with IHDR")
        if kind == b"IHDR":
            if width is not None or length != 13 or saw_idat:
                raise ImportFailure("anatomy source PNG has invalid IHDR structure")
            width, height, depth, color, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", payload
            )
            if (width, height, depth, color, compression, filtering, interlace) != (
                PNG_WIDTH,
                PNG_HEIGHT,
                8,
                6,
                0,
                0,
                0,
            ):
                raise ImportFailure("anatomy source PNG must be exactly 64x64 native deterministic RGBA8")
        elif kind == b"IDAT":
            if width is None:
                raise ImportFailure("anatomy source PNG has IDAT before IHDR")
            saw_idat = True
            if len(compressed) + length > MAX_STAGED_FILE_BYTES:
                raise ImportFailure("anatomy source PNG has oversized compressed data")
            compressed.extend(payload)
        elif kind == b"IEND":
            if length != 0:
                raise ImportFailure("anatomy source PNG has invalid IEND structure")
            saw_iend = True
            break
    if width is None or height is None or not saw_idat or not saw_iend:
        raise ImportFailure("anatomy source PNG is missing required chunks")
    if offset != len(data):
        raise ImportFailure("anatomy source PNG has trailing bytes after IEND")
    try:
        decoder = zlib.decompressobj()
        raw = decoder.decompress(bytes(compressed), PNG_FILTERED_BYTES + 1)
    except zlib.error as error:
        raise ImportFailure(f"anatomy source PNG has invalid zlib data: {error}") from error
    if len(raw) > PNG_FILTERED_BYTES or decoder.unconsumed_tail:
        raise ImportFailure("anatomy source PNG exceeds the bounded decoded size")
    if not decoder.eof:
        raise ImportFailure("anatomy source PNG has a truncated zlib stream")
    if decoder.unused_data:
        raise ImportFailure("anatomy source PNG has trailing zlib data")
    stride = PNG_WIDTH * 4
    if len(raw) != PNG_FILTERED_BYTES:
        raise ImportFailure("anatomy source PNG has invalid decompressed length")
    pixels = bytearray()
    for row in range(PNG_HEIGHT):
        start = row * (stride + 1)
        if raw[start] != 0:
            raise ImportFailure("anatomy source PNG must use deterministic filter zero")
        pixels.extend(raw[start + 1 : start + 1 + stride])
    return PNG_WIDTH, PNG_HEIGHT, bytes(pixels)


def _parse_projection_obj(data: bytes) -> list[dict]:
    if len(data) > MAX_STAGED_FILE_BYTES:
        raise ImportFailure("anatomy source OBJ exceeds 512 KiB")
    try:
        lines = data.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        raise ImportFailure("anatomy source OBJ is not ASCII") from error
    positions = []
    uvs = []
    triangles = []
    group = None
    for line_number, line in enumerate(lines, 1):
        fields = line.split()
        if not fields or fields[0].startswith("#"):
            continue
        try:
            if fields[0] == "v":
                if len(fields) != 4:
                    raise ValueError("position arity")
                position = tuple(float(value) for value in fields[1:4])
                if not all(math.isfinite(value) for value in position):
                    raise ValueError("non-finite position")
                positions.append(position)
            elif fields[0] == "vt":
                if len(fields) < 3:
                    raise ValueError("UV arity")
                uv = tuple(float(value) for value in fields[1:3])
                if not all(math.isfinite(value) for value in uv):
                    raise ValueError("non-finite UV")
                uvs.append(uv)
            elif fields[0] == "g":
                if len(fields) != 2 or fields[1] not in GROUP_REGIONS:
                    raise ValueError("unknown runtime group")
                group = fields[1]
            elif fields[0] == "f":
                if group is None or len(fields) != 4:
                    raise ValueError("triangle/group contract")
                references = [field.split("/") for field in fields[1:]]
                if any(len(reference) < 2 for reference in references):
                    raise ValueError("missing UV reference")
                position_indices = [int(reference[0]) for reference in references]
                uv_indices = [int(reference[1]) for reference in references]
                if (
                    any(index <= 0 or index > len(positions) for index in position_indices)
                    or any(index <= 0 or index > len(uvs) for index in uv_indices)
                ):
                    raise ValueError("out-of-range OBJ index")
                triangles.append(
                    {
                        "face": len(triangles),
                        "group": group,
                        "positions": tuple(positions[index - 1] for index in position_indices),
                        "uvs": tuple(uvs[index - 1] for index in uv_indices),
                    }
                )
        except (ValueError, IndexError) as error:
            raise ImportFailure(
                f"anatomy source OBJ is malformed at line {line_number}: {error}"
            ) from error
    if not positions or not uvs or not triangles:
        raise ImportFailure("anatomy source OBJ has no projectable triangles")
    return triangles


def _closest_uv_weights(
    point: tuple[float, float], triangle: tuple[tuple[float, float], ...]
) -> tuple[float, tuple[float, float, float]]:
    candidates = []
    for first, second, opposite in ((0, 1, 2), (1, 2, 0), (2, 0, 1)):
        start = triangle[first]
        end = triangle[second]
        edge = (end[0] - start[0], end[1] - start[1])
        length_squared = edge[0] * edge[0] + edge[1] * edge[1]
        amount = (
            0.0
            if length_squared <= 1.0e-18
            else max(
                0.0,
                min(
                    1.0,
                    ((point[0] - start[0]) * edge[0] + (point[1] - start[1]) * edge[1])
                    / length_squared,
                ),
            )
        )
        projected = (start[0] + edge[0] * amount, start[1] + edge[1] * amount)
        weights = [0.0, 0.0, 0.0]
        weights[first] = 1.0 - amount
        weights[second] = amount
        weights[opposite] = 0.0
        distance_squared = sum((a - b) ** 2 for a, b in zip(point, projected))
        candidates.append((distance_squared, tuple(weights)))
    return min(candidates, key=lambda candidate: (candidate[0], candidate[1]))


def _project_semantic_texels(obj_bytes: bytes, semantic_png: bytes) -> list[dict]:
    triangles = _parse_projection_obj(obj_bytes)
    width, height, semantic = decode_rgba_png(semantic_png)
    groups_by_color = {}
    for group, color in GROUP_COLORS.items():
        if group in GROUP_REGIONS:
            groups_by_color.setdefault(tuple(color[:3]), set()).add(group)
    triangles_by_group = {}
    exact_bins = {}
    for triangle in triangles:
        triangles_by_group.setdefault(triangle["group"], []).append(triangle)
        minimum_x = max(
            0,
            math.ceil(min(uv[0] for uv in triangle["uvs"]) * width - 0.5 - 1.0e-9),
        )
        maximum_x = min(
            width - 1,
            math.floor(max(uv[0] for uv in triangle["uvs"]) * width - 0.5 + 1.0e-9),
        )
        minimum_y = max(
            0,
            math.ceil(min(uv[1] for uv in triangle["uvs"]) * height - 0.5 - 1.0e-9),
        )
        maximum_y = min(
            height - 1,
            math.floor(max(uv[1] for uv in triangle["uvs"]) * height - 0.5 + 1.0e-9),
        )
        for bin_y in range(minimum_y, maximum_y + 1):
            for bin_x in range(minimum_x, maximum_x + 1):
                exact_bins.setdefault((triangle["group"], bin_x, bin_y), []).append(
                    triangle
                )
    records = []
    for y in range(height):
        for x in range(width):
            offset = (y * width + x) * 4
            if semantic[offset + 3] == 0:
                continue
            color = tuple(semantic[offset : offset + 3])
            groups = groups_by_color.get(color)
            if not groups:
                raise ImportFailure(f"semantic mask contains unknown occupied color {color}")
            candidates = [
                triangle
                for group in sorted(groups)
                for triangle in triangles_by_group.get(group, ())
            ]
            if not candidates:
                raise ImportFailure(
                    f"semantic texel {x},{y} has no same-group OBJ triangle"
                )
            texel_uv = ((x + 0.5) / width, (y + 0.5) / height)
            exact_candidates = [
                triangle
                for group in sorted(groups)
                for triangle in exact_bins.get((group, x, y), ())
            ]
            exact = []
            nearest = []
            for triangle in exact_candidates:
                weights = barycentric_weights(texel_uv, triangle["uvs"])
                if weights is not None:
                    exact.append((-min(weights), triangle["face"], triangle, weights))
            if exact:
                _, _, triangle, weights = min(exact)
                mode = "inside"
            else:
                for triangle in candidates:
                    distance_squared, closest = _closest_uv_weights(
                        texel_uv, triangle["uvs"]
                    )
                    nearest.append(
                        (distance_squared, triangle["face"], triangle, closest)
                    )
                _, _, triangle, weights = min(nearest)
                mode = "nearest"
            point = tuple(
                sum(
                    weights[corner] * triangle["positions"][corner][axis]
                    for corner in range(3)
                )
                for axis in range(3)
            )
            records.append(
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
    if not records:
        raise ImportFailure("semantic mask has no occupied anatomy texels")
    return records


def _point_bounds(records: list[dict]) -> tuple[list[float], list[float]]:
    return (
        [min(record["point"][axis] for record in records) for axis in range(3)],
        [max(record["point"][axis] for record in records) for axis in range(3)],
    )


def _normalized_point(record: dict, bounds: tuple[list[float], list[float]]) -> tuple[float, float, float]:
    lower, upper = bounds
    return tuple(
        0.5
        if upper[axis] - lower[axis] <= 1.0e-12
        else (record["point"][axis] - lower[axis]) / (upper[axis] - lower[axis])
        for axis in range(3)
    )


def _select_records(candidates: list[dict], count: int, score) -> list[dict]:
    return sorted(
        candidates,
        key=lambda record: (score(record), record["face"], record["y"], record["x"]),
    )[: max(0, min(count, len(candidates)))]


def _centroid(records: list[dict]) -> list[float]:
    if not records:
        raise ImportFailure("cannot derive an anatomy landmark from no source points")
    return [
        round(sum(record["point"][axis] for record in records) / len(records), 9)
        for axis in range(3)
    ]


def _source_landmark_projection_data(
    records: list[dict], profile: dict
) -> tuple[dict[str, tuple[float, float, float]], dict[str, dict]]:
    source_geometry = profile["projection"]["source_geometry"]
    return _source_point_projection_data(
        records,
        source_geometry["landmarks"],
        source_geometry["canonical_bounds"],
    )


def _source_point_projection_data(
    records: list[dict],
    points: dict[str, list[float]],
    canonical: dict,
) -> tuple[dict[str, tuple[float, float, float]], dict[str, dict]]:
    actual = _point_bounds(records)
    targets = {}
    projections = {}
    for name, source in sorted(points.items()):
        if name == "canonical_bounds":
            continue
        target = tuple(
            actual[0][axis]
            + (source[axis] - canonical["min"][axis])
            / (canonical["max"][axis] - canonical["min"][axis])
            * (actual[1][axis] - actual[0][axis])
            for axis in range(3)
        )
        nearest = min(
            records,
            key=lambda record: (
                sum((record["point"][axis] - target[axis]) ** 2 for axis in range(3)),
                record["face"],
                record["y"],
                record["x"],
            ),
        )
        distance = math.sqrt(
            sum((nearest["point"][axis] - target[axis]) ** 2 for axis in range(3))
        )
        targets[name] = target
        projections[name] = {
            "x": nearest["x"],
            "y": nearest["y"],
            "source": [round(value, 9) for value in target],
            "projected": [round(value, 9) for value in nearest["point"]],
            "face": nearest["face"],
            "group": nearest["group"],
            "weights": [round(value, 9) for value in nearest["weights"]],
            "distance": round(distance, 9),
        }
    return targets, projections


def _feature_anchor_projection_data(
    records: list[dict], profile: dict
) -> dict[str, dict]:
    source_geometry = profile["projection"]["source_geometry"]
    canonical = source_geometry["canonical_bounds"]
    actual = _point_bounds(records)
    projections = {}
    for name, anchor in sorted(source_geometry["feature_landmarks"].items()):
        source = anchor["point"]
        target = tuple(
            actual[0][axis]
            + (source[axis] - canonical["min"][axis])
            / (canonical["max"][axis] - canonical["min"][axis])
            * (actual[1][axis] - actual[0][axis])
            for axis in range(3)
        )
        candidates = [
            record
            for record in records
            if record["group"] == anchor["runtime_group"]
        ]
        if not candidates:
            raise ImportFailure(
                f"feature anchor {name} has no {anchor['runtime_group']} source geometry"
            )
        nearest = min(
            candidates,
            key=lambda record: (
                sum((record["point"][axis] - target[axis]) ** 2 for axis in range(3)),
                record["face"],
                record["y"],
                record["x"],
            ),
        )
        distance = math.sqrt(
            sum((nearest["point"][axis] - target[axis]) ** 2 for axis in range(3))
        )
        projections[name] = {
            "channel": anchor["channel"],
            "runtime_group": anchor["runtime_group"],
            "canonical": [round(value, 9) for value in source],
            "x": nearest["x"],
            "y": nearest["y"],
            "source": [round(value, 9) for value in target],
            "projected": [round(value, 9) for value in nearest["point"]],
            "face": nearest["face"],
            "group": nearest["group"],
            "weights": [round(value, 9) for value in nearest["weights"]],
            "distance": round(distance, 9),
        }
    return projections


def _classify_projected_anatomy(
    records: list[dict], profile: dict, logical_slot: str
) -> tuple[dict[str, list[dict]], dict[str, dict], dict[str, dict], dict[str, dict]]:
    targets, landmark_projections = _source_landmark_projection_data(records, profile)
    feature_anchor_ownership = _feature_anchor_projection_data(records, profile)
    for record in records:
        record["channel"] = ANATOMY_DETAIL_GROUP_CHANNELS.get(record["group"], "primary")
    classification = {}

    def mark(channel: str, selected: list[dict], landmarks: tuple[str, ...] = ()) -> None:
        entry = classification.setdefault(channel, {"groups": set(), "landmarks": set()})
        entry["groups"].update(record["group"] for record in selected)
        entry["landmarks"].update(landmarks)

    def assign(
        channel: str,
        candidates: list[dict],
        count: int,
        target: tuple[float, float, float],
        landmarks: tuple[str, ...],
    ) -> list[dict]:
        selected = _select_records(
            [record for record in candidates if record["channel"] == "primary"],
            count,
            lambda record: sum(
                (record["point"][axis] - target[axis]) ** 2 for axis in range(3)
            ),
        )
        for record in selected:
            record["channel"] = channel
        mark(channel, selected, landmarks)
        return selected

    used_anchor_pixels = set()
    for name, ownership in sorted(feature_anchor_ownership.items()):
        anchor_candidates = [
            record
            for record in records
            if record["group"] == ownership["runtime_group"]
        ]
        if name == "tail-tip":
            tail_z = [record["point"][2] for record in anchor_candidates]
            root_z, tip_z = max(tail_z), min(tail_z)
            span = max(root_z - tip_z, 1.0e-12)
            anchor_candidates = [
                record
                for record in anchor_candidates
                if (root_z - record["point"][2]) / span >= 0.72
            ]
        candidates = [
            record
            for record in anchor_candidates
            if (record["x"], record["y"]) not in used_anchor_pixels
            and record["channel"] in {"primary", ownership["channel"]}
        ]
        if not candidates:
            raise ImportFailure(f"feature anchor {name} did not resolve to an unused source texel")
        target = ownership["source"]
        anchor_record = min(
            candidates,
            key=lambda record: (
                sum((record["point"][axis] - target[axis]) ** 2 for axis in range(3)),
                record["face"],
                record["y"],
                record["x"],
            ),
        )
        ownership.update(
            {
                "x": anchor_record["x"],
                "y": anchor_record["y"],
                "projected": [round(value, 9) for value in anchor_record["point"]],
                "face": anchor_record["face"],
                "group": anchor_record["group"],
                "weights": [round(value, 9) for value in anchor_record["weights"]],
                "distance": round(
                    math.sqrt(
                        sum(
                            (anchor_record["point"][axis] - target[axis]) ** 2
                            for axis in range(3)
                        )
                    ),
                    9,
                ),
            }
        )
        used_anchor_pixels.add((anchor_record["x"], anchor_record["y"]))
        if anchor_record["channel"] not in {"primary", ownership["channel"]}:
            raise ImportFailure(
                f"feature anchor {name} conflicts with {anchor_record['channel']} source detail"
            )
        anchor_record["channel"] = ownership["channel"]
        ownership["owned_channel"] = anchor_record["channel"]
        mark(ownership["channel"], [anchor_record], (name,))

    for record in records:
        channel = record["channel"]
        if channel != "primary":
            mark(channel, [record])

    bounds = _point_bounds(records)
    if logical_slot == "head":
        main = [record for record in records if record["group"] == "head"]
        if not main:
            raise ImportFailure("head anatomy has no source head geometry")
        eye_y = sum(targets[name][1] for name in ("left-eye", "right-eye")) / 2.0
        eye_z = sum(targets[name][2] for name in ("left-eye", "right-eye")) / 2.0
        left_target = (bounds[0][0], eye_y, eye_z)
        right_target = (bounds[1][0], eye_y, eye_z)
        ear_count = max(1, round(len(main) * 0.07))
        ear_candidates = [
            record
            for record in main
            if abs(_normalized_point(record, bounds)[0] - 0.5) >= 0.24
        ]
        left_ears = assign(
            "inner-ear", ear_candidates, ear_count, left_target, ("left-eye",)
        )
        right_ears = assign(
            "inner-ear",
            [record for record in ear_candidates if record not in left_ears],
            ear_count,
            right_target,
            ("right-eye",),
        )
        muzzle_candidates = [
            record
            for record in main
            if abs(_normalized_point(record, bounds)[0] - 0.5) <= 0.34
            and _normalized_point(record, bounds)[1] <= 0.68
            and _normalized_point(record, bounds)[2] <= 0.58
        ]
        if not muzzle_candidates:
            muzzle_candidates = main
        assign(
            "muzzle",
            muzzle_candidates,
            max(1, round(len(main) * 0.16)),
            targets["muzzle"],
            ("muzzle",),
        )
    elif logical_slot == "torso":
        torso = [record for record in records if record["group"] == "torso"]
        if not torso:
            raise ImportFailure("torso anatomy has no source torso geometry")
        hip_names = ("left-hip-attachment", "right-hip-attachment")
        y_target = sum(targets[name][1] for name in hip_names) / 2.0
        tail_target = targets["tail-root"]
        front_z = bounds[0][2] if tail_target[2] > sum(bounds[axis][2] for axis in (0, 1)) / 2.0 else bounds[1][2]
        belly_target = (0.0, y_target, front_z)
        belly_candidates = [
            record
            for record in torso
            if abs(_normalized_point(record, bounds)[0] - 0.5) <= 0.36
            and _normalized_point(record, bounds)[2] <= 0.58
        ]
        assign(
            "belly",
            belly_candidates,
            max(1, round(len(torso) * 0.28)),
            belly_target,
            hip_names,
        )
        assign(
            "secondary-marking",
            torso,
            max(1, round(len(torso) * 0.18)),
            tail_target,
            ("tail-root",),
        )
    elif logical_slot in {"arms", "legs"}:
        distal_name = "hand" if logical_slot == "arms" else "foot"
        root_name = "shoulder-attachment" if logical_slot == "arms" else "hip-attachment"
        for group in sorted({record["group"] for record in records}):
            group_records = [record for record in records if record["group"] == group]
            side = "left" if group.startswith("left-") else "right"
            distal = f"{side}-{distal_name}"
            root = f"{side}-{root_name}"
            distal_target = targets[distal]
            root_target = targets[root]
            group_bounds = _point_bounds(group_records)
            distal_candidates = [
                record
                for record in group_records
                if _normalized_point(record, group_bounds)[1] <= 0.46
            ]
            selected = assign(
                "hands-feet",
                distal_candidates,
                max(1, round(len(group_records) * 0.30)),
                distal_target,
                (distal,),
            )
            midpoint = tuple((distal_target[axis] + root_target[axis]) / 2.0 for axis in range(3))
            assign(
                "secondary-marking",
                [record for record in group_records if record not in selected],
                max(1, round(len(group_records) * 0.16)),
                midpoint,
                (distal, root),
            )
    elif logical_slot == "tail":
        tail = [record for record in records if record["group"] == "tail-back"]
        if not tail:
            raise ImportFailure("tail anatomy has no source tail geometry")
        tip_target = targets["tail-tip"]
        root_target = targets["tail-root"]
        root_z = max(record["point"][2] for record in tail)
        tip_z = min(record["point"][2] for record in tail)
        span = max(root_z - tip_z, 1.0e-12)
        tip_candidates = [
            record
            for record in tail
            if (root_z - record["point"][2]) / span >= 0.72
        ]
        assign(
            "keratin-skin",
            tip_candidates,
            max(1, round(len(tail) * 0.30)),
            tip_target,
            ("tail-tip",),
        )
        midpoint = tuple((tip_target[axis] + root_target[axis]) / 2.0 for axis in range(3))
        middle_candidates = [
            record
            for record in tail
            if record["channel"] == "primary"
            and 0.28 <= (root_z - record["point"][2]) / span <= 0.82
        ]
        assign(
            "secondary-marking",
            middle_candidates,
            max(1, round(len(tail) * 0.25)),
            midpoint,
            ("tail-root", "tail-tip"),
        )
    else:
        raise ImportFailure(f"unsupported anatomy slot {logical_slot}")
    by_channel = {}
    for record in records:
        by_channel.setdefault(record["channel"], []).append(record)
    for channel, channel_records in by_channel.items():
        mark(channel, channel_records)
    missing = REQUIRED_ANATOMY_CHANNELS[logical_slot] - set(by_channel)
    if missing:
        raise ImportFailure(
            f"source geometry cannot localize required {logical_slot} channels: {sorted(missing)}"
        )
    return by_channel, classification, landmark_projections, feature_anchor_ownership


def _derived_anatomy_landmarks(
    records: list[dict], by_channel: dict[str, list[dict]], logical_slot: str
) -> dict[str, list[float]]:
    landmarks = {
        f"channel.{channel}": _centroid(channel_records)
        for channel, channel_records in sorted(by_channel.items())
    }
    if logical_slot == "head":
        ears = by_channel["inner-ear"]
        landmarks["head.left-lateral"] = _centroid(
            [record for record in ears if record["point"][0] <= 0.0] or ears
        )
        landmarks["head.right-lateral"] = _centroid(
            [record for record in ears if record["point"][0] > 0.0] or ears
        )
        landmarks["head.muzzle"] = _centroid(by_channel["muzzle"])
    elif logical_slot in {"arms", "legs"}:
        for group in sorted({record["group"] for record in records}):
            grouped = [record for record in records if record["group"] == group]
            ordered = sorted(grouped, key=lambda record: (record["point"][1], record["face"]))
            count = max(1, len(ordered) // 10)
            landmarks[f"{group}.distal"] = _centroid(ordered[:count])
            landmarks[f"{group}.root"] = _centroid(ordered[-count:])
            if logical_slot == "legs":
                ground_y = ordered[0]["point"][1]
                contacts = [
                    record
                    for record in ordered
                    if abs(record["point"][1] - ground_y) <= 1.0e-6
                ]
                landmarks[f"{group}.ground-contact"] = _centroid(contacts)
    elif logical_slot == "tail":
        ordered = sorted(records, key=lambda record: (record["point"][2], record["face"]))
        count = max(1, len(ordered) // 10)
        landmarks["tail.tip"] = _centroid(ordered[:count])
        landmarks["tail.root"] = _centroid(ordered[-count:])
    return dict(sorted(landmarks.items()))


def _projection_digest(records: list[dict]) -> str:
    evidence = [
        {
            "x": record["x"],
            "y": record["y"],
            "group": record["group"],
            "face": record["face"],
            "weights": [round(value, 9) for value in record["weights"]],
            "point": [round(value, 9) for value in record["point"]],
            "mode": record["mode"],
        }
        for record in records
    ]
    payload = json.dumps(evidence, sort_keys=True, separators=(",", ":")).encode("ascii")
    return hashlib.sha256(payload).hexdigest()


def anatomy_mask_with_audit(
    semantic_png: bytes,
    obj_bytes: bytes,
    profile: dict,
    logical_slot: str,
    *,
    validate_profile: bool = True,
) -> tuple[bytes, dict]:
    if validate_profile:
        validate_anatomy_authoring(
            {
                "id": "raster-input",
                "logical_slot": logical_slot,
                "anatomy_authoring": profile,
            }
        )
    records = _project_semantic_texels(obj_bytes, semantic_png)
    (
        by_channel,
        geometry_classification,
        landmark_projections,
        feature_anchor_ownership,
    ) = _classify_projected_anatomy(
        records, profile, logical_slot
    )
    output = bytearray(PNG_WIDTH * PNG_HEIGHT * 4)
    for record in records:
        offset = (record["y"] * PNG_WIDTH + record["x"]) * 4
        output[offset : offset + 4] = bytes((*ANATOMY_COLORS[record["channel"]], 255))
    lower, upper = _point_bounds(records)
    runtime_group_counts = {
        group: sum(record["group"] == group for record in records)
        for group in sorted({record["group"] for record in records})
    }
    audit = {
        "obj_sha256": hashlib.sha256(obj_bytes).hexdigest(),
        "semantic_sha256": hashlib.sha256(semantic_png).hexdigest(),
        "projection_sha256": _projection_digest(records),
        "projected_texels": len(records),
        "inside_texels": sum(record["mode"] == "inside" for record in records),
        "nearest_texels": sum(record["mode"] == "nearest" for record in records),
        "overlap_texels": sum(record["overlap"] > 1 for record in records),
        "runtime_group_counts": runtime_group_counts,
        "channel_counts": {
            channel: len(channel_records)
            for channel, channel_records in sorted(by_channel.items())
        },
        "geometry_classification": {
            channel: {
                "groups": sorted(classification["groups"]),
                "landmarks": sorted(classification["landmarks"]),
            }
            for channel, classification in sorted(geometry_classification.items())
        },
        "source_landmark_projections": landmark_projections,
        "feature_anchor_ownership": feature_anchor_ownership,
        "source_bounds": {
            "min": [round(value, 9) for value in lower],
            "max": [round(value, 9) for value in upper],
        },
        "derived_landmarks": _derived_anatomy_landmarks(
            records, by_channel, logical_slot
        ),
    }
    return png_bytes(PNG_WIDTH, PNG_HEIGHT, bytes(output)), audit


def anatomy_mask(
    semantic_png: bytes, obj_bytes: bytes, profile: dict, logical_slot: str
) -> bytes:
    return anatomy_mask_with_audit(
        semantic_png, obj_bytes, profile, logical_slot
    )[0]


def _source_feature_landmarks(asset: dict, staging: Path) -> dict[str, dict]:
    full_lod = next(lod for lod in asset["lods"] if lod["lod"] == "full")
    triangles = _parse_projection_obj(
        confined_existing_staged_path(
            staging, full_lod["generated_obj"], "full source geometry"
        ).read_bytes()
    )
    points_by_group = {}
    for triangle in triangles:
        points_by_group.setdefault(triangle["group"], []).extend(triangle["positions"])
    all_points = [point for points in points_by_group.values() for point in points]
    actual_bounds = (
        [min(point[axis] for point in all_points) for axis in range(3)],
        [max(point[axis] for point in all_points) for axis in range(3)],
    )
    canonical = asset["canonical_bounds"]

    def to_actual(point: list[float] | tuple[float, float, float]) -> tuple[float, float, float]:
        return tuple(
            actual_bounds[0][axis]
            + (point[axis] - canonical["min"][axis])
            / (canonical["max"][axis] - canonical["min"][axis])
            * (actual_bounds[1][axis] - actual_bounds[0][axis])
            for axis in range(3)
        )

    def to_canonical(point: tuple[float, float, float]) -> list[float]:
        return [
            round(
                canonical["min"][axis]
                + (point[axis] - actual_bounds[0][axis])
                / (actual_bounds[1][axis] - actual_bounds[0][axis])
                * (canonical["max"][axis] - canonical["min"][axis]),
                9,
            )
            for axis in range(3)
        ]

    def nearest(group: str, canonical_target: list[float]) -> tuple[float, float, float]:
        target = to_actual(canonical_target)
        points = points_by_group.get(group, [])
        if not points:
            raise ImportFailure(f"{asset['id']} has no source geometry group {group}")
        return min(
            points,
            key=lambda point: (
                sum((point[axis] - target[axis]) ** 2 for axis in range(3)),
                point,
            ),
        )

    def feature(
        channel: str,
        runtime_group: str,
        point: tuple[float, float, float],
        basis: list[str],
    ) -> dict:
        return {
            "channel": channel,
            "runtime_group": runtime_group,
            "source_group": runtime_group,
            "point": to_canonical(point),
            "source_position": [round(value, 9) for value in point],
            "source_basis": basis,
            "method": "source-geometry-anchor-v1",
        }

    landmarks = asset["landmarks"]
    features = {}
    if asset["logical_slot"] == "head":
        head_points = points_by_group["head"]
        left_eye = to_actual(landmarks["left-eye"])
        right_eye = to_actual(landmarks["right-eye"])
        left_ear = min(
            head_points,
            key=lambda point: (point[0], abs(point[1] - left_eye[1]), abs(point[2] - left_eye[2]), point),
        )
        right_ear = min(
            head_points,
            key=lambda point: (-point[0], abs(point[1] - right_eye[1]), abs(point[2] - right_eye[2]), point),
        )
        features["left-ear"] = feature(
            "inner-ear", "head", left_ear, ["source-geometry:head", "landmark:left-eye"]
        )
        features["right-ear"] = feature(
            "inner-ear", "head", right_ear, ["source-geometry:head", "landmark:right-eye"]
        )
        features["muzzle"] = feature(
            "muzzle",
            "head",
            nearest("head", landmarks["muzzle"]),
            ["source-geometry:head", "landmark:muzzle"],
        )
    elif asset["logical_slot"] == "torso":
        hip_y = (landmarks["left-hip-attachment"][1] + landmarks["right-hip-attachment"][1]) / 2.0
        belly_target = [0.0, hip_y, canonical["min"][2]]
        features["belly"] = feature(
            "belly",
            "torso",
            nearest("torso", belly_target),
            [
                "source-geometry:torso",
                "landmark:left-hip-attachment",
                "landmark:right-hip-attachment",
                "landmark:tail-root",
            ],
        )
    elif asset["logical_slot"] == "arms":
        for side in ("left", "right"):
            name = f"{side}-hand"
            features[name] = feature(
                "hands-feet",
                f"{side}-arm",
                nearest(f"{side}-arm", landmarks[name]),
                [f"source-geometry:{side}-arm", f"landmark:{name}"],
            )
    elif asset["logical_slot"] == "legs":
        for side in ("left", "right"):
            name = f"{side}-foot"
            features[name] = feature(
                "hands-feet",
                f"{side}-leg",
                nearest(f"{side}-leg", landmarks[name]),
                [f"source-geometry:{side}-leg", f"landmark:{name}"],
            )
    elif asset["logical_slot"] == "tail":
        tail_tip = min(
            points_by_group["tail-back"],
            key=lambda point: (point[2], point[0], point[1], point),
        )
        features["tail-tip"] = feature(
            "keratin-skin",
            "tail-back",
            tail_tip,
            ["source-geometry:tail-back", "landmark:tail-tip", "derived:distal-z-extreme"],
        )
    return features


def source_projected_anatomy_authoring(asset: dict, staging: Path) -> dict:
    source_groups = set(asset["groups"].values())
    for role in asset.get("detail_groups", {}):
        source_groups.add(f"head.{role}")
    profile = {
        "schema": ANATOMY_AUTHORING_SCHEMA,
        "coordinate_space": "same-lod-staged-obj",
        "default_channel": "primary",
        "required_channels": sorted(REQUIRED_ANATOMY_CHANNELS[asset["logical_slot"]]),
        "projection": {
            "schema": ANATOMY_PROJECTION_SCHEMA,
            "texel_sample": "pixel-center",
            "triangle_tie_break": ANATOMY_TRIANGLE_TIE_BREAK,
            "classifier": ANATOMY_CLASSIFIER,
            "detail_group_channels": ANATOMY_DETAIL_GROUP_CHANNELS,
            "source_geometry": {
                "schema": ANATOMY_SOURCE_GEOMETRY_SCHEMA,
                "groups": sorted(source_groups),
                "landmarks": json.loads(json.dumps(asset["landmarks"])),
                "canonical_bounds": json.loads(json.dumps(asset["canonical_bounds"])),
                "feature_landmarks": _source_feature_landmarks(asset, staging),
            },
        },
    }
    lod_audits = {}
    for lod in asset["lods"]:
        obj_bytes = confined_existing_staged_path(
            staging, lod["generated_obj"], "generated OBJ"
        ).read_bytes()
        semantic_bytes = confined_existing_staged_path(
            staging, lod["semantic_mask"], "semantic mask"
        ).read_bytes()
        _, lod_audits[lod["lod"]] = anatomy_mask_with_audit(
            semantic_bytes,
            obj_bytes,
            profile,
            asset["logical_slot"],
            validate_profile=False,
        )
    profile["source_projection_audit"] = {
        "schema": ANATOMY_AUDIT_SCHEMA,
        "lods": lod_audits,
    }
    return profile


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
    draw_line(pixels, width, height, (12, 12), (243, 12), (58, 63, 68, 255))
    draw_line(pixels, width, height, (12, 243), (243, 243), (58, 63, 68, 255))
    draw_line(pixels, width, height, (12, 12), (12, 243), (48, 53, 58, 255))
    draw_line(pixels, width, height, (243, 12), (243, 243), (48, 53, 58, 255))
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
        base = GROUP_COLORS.get(semantic, (210, 210, 215, 255))
        for edge_index, (first, second) in enumerate(((0, 1), (1, 2), (2, 0))):
            shade = (0.78, 0.9, 1.0)[edge_index]
            color = tuple(round(channel * shade) for channel in base[:3]) + (255,)
            draw_line(pixels, width, height, projected[indices[first]], projected[indices[second]], color)
    output.write_bytes(png_bytes(width, height, bytes(pixels)))


def command_preview(args, recipe: dict) -> None:
    staging = ensure_artifact_path(args.staging, "preview staging input")
    _assert_tree_has_no_reparse_entries(staging)
    output = ensure_artifact_path(args.output, "preview output")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(
        tempfile.mkdtemp(prefix=f".{output.name}.tmp-", dir=output.parent)
    )
    count = 0
    try:
        for asset in recipe["part_assets"]:
            for lod in asset["lods"]:
                obj = confined_existing_staged_path(
                    staging, lod["generated_obj"], "staged OBJ for preview"
                )
                render_obj_preview(
                    obj,
                    temporary / f"{asset['id']}_{lod['lod']}.png",
                )
                count += 1
        if output.exists():
            shutil.rmtree(output)
        temporary.replace(output)
    except BaseException:
        if temporary.exists():
            shutil.rmtree(temporary)
        raise
    print(f"previews={count}")
    print(f"preview_root={output}")


def command_bind_output_digests(args, recipe: dict) -> None:
    staging = ensure_artifact_path(args.staging, "digest-binding staging input")
    _assert_tree_has_no_reparse_entries(staging)
    receipt_path = confined_existing_staged_path(
        staging, "build_receipt.json", "staged build receipt"
    )
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    _verify_receipt_outputs(
        staging, receipt, recipe, require_recipe_output_digests=False
    )
    bound_outputs = {}
    for asset in recipe["part_assets"]:
        for lod in asset["lods"]:
            for path_field, digest_field in (
                ("generated_obj", "generated_obj_sha256"),
                ("socket_manifest", "socket_manifest_sha256"),
                ("semantic_mask", "semantic_mask_sha256"),
                ("anatomy_mask", "anatomy_mask_sha256"),
            ):
                relative = lod[path_field]
                raw = _relative_staged_path(relative)
                path = confined_existing_staged_path(
                    staging, raw, "generated output for digest binding"
                )
                digest = sha256_file(path)
                lod[digest_field] = digest
                bound_outputs[raw.as_posix()] = digest
    for asset in recipe["part_assets"]:
        source_geometry = asset.get("anatomy_authoring", {}).get("projection", {}).get(
            "source_geometry", {}
        )
        if "feature_landmarks" not in source_geometry:
            asset["anatomy_authoring"] = source_projected_anatomy_authoring(
                asset, staging
            )
        for lod in asset["lods"]:
            expected = anatomy_mask(
                confined_existing_staged_path(
                    staging, lod["semantic_mask"], "bound semantic mask"
                ).read_bytes(),
                confined_existing_staged_path(
                    staging, lod["generated_obj"], "bound generated OBJ"
                ).read_bytes(),
                asset["anatomy_authoring"],
                asset["logical_slot"],
            )
            actual = confined_existing_staged_path(
                staging, lod["anatomy_mask"], "bound anatomy mask"
            ).read_bytes()
            if actual != expected:
                raise ImportFailure(
                    f"bound anatomy mask is not projected from {asset['id']} {lod['lod']}"
                )
    if set(receipt.get("outputs", {})) != set(bound_outputs):
        raise ImportFailure(
            "build receipt outputs do not exactly match the recipe digest-binding paths"
        )
    recipe["recipe_sha256"] = canonical_recipe_digest(recipe)
    receipt["recipe_sha256"] = recipe["recipe_sha256"]
    receipt["outputs"] = bound_outputs

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    recipe_temporary = output.with_name(f".{output.name}.tmp-{os.getpid()}")
    receipt_temporary = receipt_path.with_name(
        f".{receipt_path.name}.tmp-{os.getpid()}"
    )
    recipe_temporary.write_text(
        json.dumps(recipe, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
    )
    receipt_temporary.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    atomic_replace_pair(
        recipe_temporary,
        output,
        receipt_temporary,
        receipt_path,
    )
    print(f"bound_outputs={len(bound_outputs)}")
    print(f"recipe_sha256={recipe['recipe_sha256']}")
    print(f"recipe_output={output}")


def _expected_receipt_outputs(recipe: dict, include_anatomy: bool) -> dict[str, set[str]]:
    fields = ["generated_obj", "socket_manifest", "semantic_mask"]
    if include_anatomy:
        fields.append("anatomy_mask")
    expected = {source["donor"]: set() for source in recipe["sources"]}
    for asset in recipe["part_assets"]:
        donor = asset["donor"]
        if donor not in expected:
            raise ImportFailure(f"recipe asset has unknown donor: {donor}")
        for lod in asset["lods"]:
            for field in fields:
                expected[donor].add(_relative_staged_path(lod[field]).as_posix())
    return expected


def _staging_file_set(staging: Path) -> set[str]:
    _assert_tree_has_no_reparse_entries(staging)
    root = _canonical_staging_root(staging)
    files = set()
    pending = [root]
    while pending:
        directory = pending.pop()
        for entry in sorted(os.scandir(directory), key=lambda candidate: candidate.name):
            path = Path(entry.path)
            if entry.is_dir(follow_symlinks=False):
                pending.append(path)
            elif entry.is_file(follow_symlinks=False):
                files.add(path.relative_to(root).as_posix())
            else:
                raise ImportFailure(
                    f"staged tree contains a non-regular path: {path.relative_to(root)}"
                )
    return files


def _authority_frozen_contract(recipe: dict) -> dict:
    frozen = json.loads(json.dumps(recipe))
    frozen.pop("recipe_sha256", None)
    for asset in frozen.get("part_assets", []):
        asset["anatomy_authoring"] = "<allowed-anatomy-authoring-change>"
        for lod in asset.get("lods", []):
            lod.pop("anatomy_mask_sha256", None)
            lod.pop("socket_manifest_sha256", None)
    return frozen


def _validate_authority_handoff(authority: dict, candidate: dict) -> None:
    if _authority_frozen_contract(authority) != _authority_frozen_contract(candidate):
        raise ImportFailure(
            "candidate recipe changes pre-existing asset identities, paths, or path/digest tuples"
        )


def _load_live_authority_recipe(authority_path: Path, output: Path) -> dict:
    authority_path = Path(authority_path)
    output = Path(output)
    if _is_symlink_or_reparse(authority_path) or _is_symlink_or_reparse(output):
        raise ImportFailure("live prior output recipe is a symlink or reparse point")
    authority_path = authority_path.resolve(strict=False)
    output = output.resolve(strict=False)
    if authority_path != output:
        raise ImportFailure("authority recipe must be the live prior output recipe")
    try:
        return load_recipe(output, validate_current_anatomy=False)
    except OSError as error:
        raise ImportFailure(f"live prior output recipe is unavailable: {output}") from error


def _verify_receipt_outputs(
    staging: Path,
    receipt: dict,
    recipe: dict,
    *,
    require_recipe_output_digests: bool = True,
) -> bool:
    if receipt.get("schema") != "alife.geneforge_build_receipt.v2":
        raise ImportFailure("augment-anatomy requires a v2 build receipt")
    if receipt.get("recipe_sha256", "").lower() != recipe.get("recipe_sha256", "").lower():
        raise ImportFailure("build receipt recipe digest does not match the input recipe")
    outputs = receipt.get("outputs")
    if not isinstance(outputs, dict) or any(
        not isinstance(relative, str) or not isinstance(digest, str)
        for relative, digest in outputs.items()
    ):
        raise ImportFailure("build receipt outputs are missing")
    output_set = set(outputs)
    legacy_by_donor = _expected_receipt_outputs(recipe, False)
    augmented_by_donor = _expected_receipt_outputs(recipe, True)
    legacy_outputs = set().union(*legacy_by_donor.values())
    augmented_outputs = set().union(*augmented_by_donor.values())
    expected_lod_count = len(recipe["part_assets"]) * len(LODS)
    if (
        len(legacy_outputs) != expected_lod_count * 3
        or len(augmented_outputs) != expected_lod_count * 4
        or len(legacy_by_donor) != len(recipe["sources"])
    ):
        raise ImportFailure("receipt source accounting recipe paths or donors are not unique")
    if output_set == legacy_outputs:
        expected_by_donor = legacy_by_donor
        include_anatomy = False
    elif output_set == augmented_outputs:
        expected_by_donor = augmented_by_donor
        include_anatomy = True
    else:
        raise ImportFailure(
            "existing build receipt must contain exactly 126 legacy or 168 augmented outputs"
        )
    actual_files = _staging_file_set(staging)
    expected_files = output_set | {"build_receipt.json"}
    extra_files = sorted(actual_files - expected_files)
    missing_files = sorted(expected_files - actual_files)
    if extra_files:
        raise ImportFailure(f"unreceipted staged path: {extra_files[0]}")
    if missing_files:
        raise ImportFailure(f"receipt references a missing staged path: {missing_files[0]}")

    expected_donors = set(expected_by_donor)
    sources = receipt.get("sources")
    if not isinstance(sources, list) or len(sources) != len(expected_donors):
        raise ImportFailure("receipt source accounting has an invalid donor set")
    by_donor = {}
    union = set()
    for source in sources:
        if not isinstance(source, dict) or not isinstance(source.get("donor"), str):
            raise ImportFailure("receipt source accounting has invalid source metadata")
        donor = source["donor"]
        if donor in by_donor:
            raise ImportFailure("receipt source accounting contains duplicate donors")
        source_outputs = source.get("outputs")
        if not isinstance(source_outputs, list) or any(
            not isinstance(relative, str) for relative in source_outputs
        ):
            raise ImportFailure("receipt source accounting has invalid output paths")
        source_set = set(source_outputs)
        expected_asset_count = sum(
            asset["donor"] == donor for asset in recipe["part_assets"]
        )
        if (
            donor not in expected_by_donor
            or source.get("asset_count") != expected_asset_count
            or source.get("output_count") != len(source_outputs)
            or len(source_set) != len(source_outputs)
            or source_set != expected_by_donor.get(donor)
            or union.intersection(source_set)
        ):
            raise ImportFailure(
                f"receipt source accounting does not match donor-owned outputs: {donor}"
            )
        by_donor[donor] = source
        union.update(source_set)
    if set(by_donor) != expected_donors or union != output_set:
        raise ImportFailure("receipt source accounting union does not match top-level outputs")
    if (
        receipt.get("donor_count") != len(expected_donors)
        or receipt.get("asset_count") != len(recipe["part_assets"])
    ):
        raise ImportFailure("receipt source accounting has stale top-level counts")

    recipe_digests = {}
    for asset in recipe["part_assets"]:
        for lod in asset["lods"]:
            for path_field, digest_field in (
                ("generated_obj", "generated_obj_sha256"),
                ("socket_manifest", "socket_manifest_sha256"),
                ("semantic_mask", "semantic_mask_sha256"),
                ("anatomy_mask", "anatomy_mask_sha256"),
            ):
                relative = _relative_staged_path(lod[path_field]).as_posix()
                if relative in output_set:
                    recipe_digests[relative] = lod[digest_field].lower()
    if set(recipe_digests) != output_set:
        raise ImportFailure("input recipe does not bind every existing receipt output")
    for relative, expected in outputs.items():
        path = confined_existing_staged_path(staging, relative, "staged receipt output")
        if path.stat().st_size > MAX_STAGED_FILE_BYTES:
            raise ImportFailure(f"existing staged output exceeds 512 KiB: {relative}")
        actual = sha256_file(path)
        if actual != expected.lower():
            raise ImportFailure(f"existing staged output digest mismatch: {relative}")
        if require_recipe_output_digests and actual != recipe_digests[relative]:
            raise ImportFailure(f"input recipe output digest mismatch: {relative}")
    return include_anatomy


def _validate_augmented_tree(staging: Path, recipe: dict) -> dict[str, str]:
    outputs = {}
    output_sizes = {}
    _assert_tree_has_no_reparse_entries(staging)
    if len(recipe["part_assets"]) != 14:
        raise ImportFailure("anatomy augmentation requires 14 production assets")
    for asset in recipe["part_assets"]:
        validate_anatomy_authoring(asset)
        validate_anatomy_source_audit(asset)
        for lod in asset["lods"]:
            obj_path = confined_existing_staged_path(
                staging, lod["generated_obj"], "generated anatomy source OBJ"
            )
            semantic_path = confined_existing_staged_path(
                staging, lod["semantic_mask"], "semantic staging mask"
            )
            anatomy_path = confined_existing_staged_path(
                staging, lod["anatomy_mask"], "anatomy staging mask"
            )
            semantic_width, semantic_height, semantic = decode_rgba_png(
                semantic_path.read_bytes()
            )
            anatomy_width, anatomy_height, anatomy = decode_rgba_png(
                anatomy_path.read_bytes()
            )
            expected_anatomy, expected_audit = anatomy_mask_with_audit(
                semantic_path.read_bytes(),
                obj_path.read_bytes(),
                asset["anatomy_authoring"],
                asset["logical_slot"],
            )
            if anatomy_path.read_bytes() != expected_anatomy:
                raise ImportFailure(
                    f"anatomy mask is not derived from its same-LOD OBJ: {lod['anatomy_mask']}"
                )
            if (
                asset["anatomy_authoring"]["source_projection_audit"]["lods"].get(
                    lod["lod"]
                )
                != expected_audit
            ):
                raise ImportFailure(
                    f"source projection audit does not match {asset['id']} {lod['lod']}"
                )
            if (
                (semantic_width, semantic_height) != (64, 64)
                or (anatomy_width, anatomy_height) != (64, 64)
            ):
                raise ImportFailure("semantic/anatomy staging masks must be 64x64")
            used = set()
            for offset in range(0, len(semantic), 4):
                semantic_occupied = semantic[offset + 3] > 0
                anatomy_occupied = anatomy[offset + 3] > 0
                if semantic_occupied != anatomy_occupied:
                    raise ImportFailure(f"semantic/anatomy occupancy mismatch: {lod['anatomy_mask']}")
                if not anatomy_occupied:
                    if anatomy[offset : offset + 4] != b"\0\0\0\0":
                        raise ImportFailure(f"transparent anatomy pixel is nonzero: {lod['anatomy_mask']}")
                    continue
                rgb = tuple(anatomy[offset : offset + 3])
                channel = next((name for name, color in ANATOMY_COLORS.items() if color == rgb), None)
                if channel is None:
                    raise ImportFailure(f"unknown anatomy color {rgb}: {lod['anatomy_mask']}")
                if channel not in ALLOWED_ANATOMY_CHANNELS[asset["logical_slot"]]:
                    raise ImportFailure(f"anatomy channel {channel} is not owned by {asset['logical_slot']}")
                used.add(channel)
            missing = REQUIRED_ANATOMY_CHANNELS[asset["logical_slot"]] - used
            if missing:
                raise ImportFailure(f"{asset['id']} {lod['lod']} anatomy coverage lacks {sorted(missing)}")
            socket_path = confined_existing_staged_path(
                staging, lod["socket_manifest"], "socket staging manifest"
            )
            socket = json.loads(socket_path.read_text(encoding="utf-8"))
            if socket.get("schema") != "alife.creature_part_sockets.v2" or socket.get("anatomy_mask") != lod["anatomy_mask"]:
                raise ImportFailure(f"socket anatomy metadata mismatch: {lod['socket_manifest']}")
            for field in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask"):
                relative = _relative_staged_path(lod[field])
                path = confined_existing_staged_path(
                    staging, relative, "augmented output"
                )
                size = path.stat().st_size
                if size > 512 * 1024:
                    raise ImportFailure(f"augmented output exceeds 512 KiB: {lod[field]}")
                outputs[relative.as_posix()] = sha256_file(path)
                output_sizes[relative.as_posix()] = size
    if len(outputs) != 168:
        raise ImportFailure(f"augmented output set must contain 168 files; found {len(outputs)}")
    if sum(output_sizes.values()) > 8 * 1024 * 1024:
        raise ImportFailure("augmented production pack exceeds 8 MiB")
    return outputs


def _final_receipt_sources(receipt: dict, recipe: dict) -> list[dict]:
    existing = {source["donor"]: source for source in receipt["sources"]}
    expected = _expected_receipt_outputs(recipe, True)
    sources = []
    for source_contract in recipe["sources"]:
        donor = source_contract["donor"]
        source = dict(existing[donor])
        donor_outputs = [
            lod[field]
            for asset in recipe["part_assets"]
            if asset["donor"] == donor
            for lod in asset["lods"]
            for field in (
                "generated_obj",
                "socket_manifest",
                "semantic_mask",
                "anatomy_mask",
            )
        ]
        if set(donor_outputs) != expected[donor]:
            raise ImportFailure(
                f"receipt source accounting generation drifted for donor: {donor}"
            )
        source["donor"] = donor
        source["asset_count"] = sum(
            asset["donor"] == donor for asset in recipe["part_assets"]
        )
        source["output_count"] = len(donor_outputs)
        source["outputs"] = donor_outputs
        sources.append(source)
    return sources


def _augmentation_transaction_path(staging: Path) -> Path:
    return staging.with_name(f".{staging.name}.augment-transaction.json")


def _flush_file(path: Path) -> None:
    try:
        with Path(path).open("r+b") as stream:
            os.fsync(stream.fileno())
    except OSError as error:
        raise ImportFailure(f"cannot flush durable file data: {path}") from error


def _flush_windows_directory(path: Path) -> bool:
    import ctypes
    from ctypes import wintypes

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel32.CreateFileW
    create_file.argtypes = [
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    ]
    create_file.restype = wintypes.HANDLE
    handle = create_file(
        str(path),
        0x40000000,
        0x00000001 | 0x00000002 | 0x00000004,
        None,
        3,
        0x02000000,
        None,
    )
    invalid_handle = wintypes.HANDLE(-1).value
    if handle == invalid_handle:
        error = ctypes.get_last_error()
        if error in {1, 5, 50, 87}:
            return False
        raise OSError(error, f"CreateFileW directory flush failed for {path}")
    try:
        if kernel32.FlushFileBuffers(handle):
            return True
        error = ctypes.get_last_error()
        if error in {1, 5, 6, 50, 87}:
            return False
        raise OSError(error, f"FlushFileBuffers directory flush failed for {path}")
    finally:
        kernel32.CloseHandle(handle)


def _flush_directory(path: Path) -> bool:
    path = Path(path)
    try:
        if os.name == "nt":
            # Some Windows filesystems reject directory FlushFileBuffers. File data is
            # still flushed; only this documented unsupported-handle case is best effort.
            return _flush_windows_directory(path)
        flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        descriptor = os.open(path, flags)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
        return True
    except OSError as error:
        if getattr(error, "errno", None) in {1, 5, 13, 22, 50, 95}:
            return False
        raise ImportFailure(f"cannot flush containing directory: {path}") from error


def _require_directory_flush(path: Path) -> None:
    if not _flush_directory(path):
        raise ImportFailure(f"directory durability barrier unavailable: {path}")


def _durable_replace(source: Path, target: Path) -> None:
    source = Path(source)
    target = Path(target)
    source_parent = source.parent
    target_parent = target.parent
    if source.is_file():
        _flush_file(source)
    os.replace(source, target)
    if target.is_file():
        _flush_file(target)
    _require_directory_flush(target_parent)
    if source_parent != target_parent:
        _require_directory_flush(source_parent)


def _durable_remove(path: Path) -> None:
    path = Path(path)
    if not path.exists() and not _is_symlink_or_reparse(path):
        return
    if _is_symlink_or_reparse(path):
        raise ImportFailure(f"refusing to remove a symlink or reparse recovery operand: {path}")
    if path.is_dir():
        shutil.rmtree(path)
    else:
        path.unlink()
    _require_directory_flush(path.parent)


def _flush_tree(root: Path) -> None:
    _assert_tree_has_no_reparse_entries(root)
    directories = []
    for directory, child_directories, files in os.walk(root, topdown=True, followlinks=False):
        current = Path(directory)
        directories.append(current)
        child_directories.sort()
        for name in sorted(files):
            path = current / name
            if _is_symlink_or_reparse(path) or not path.is_file():
                raise ImportFailure(f"cannot durably flush non-regular staged path: {path}")
            _flush_file(path)
    for directory in reversed(directories):
        _require_directory_flush(directory)
    _require_directory_flush(root.parent)


def _durable_json_write(path: Path, payload: dict) -> None:
    temporary = path.with_name(f".{path.name}.write-{os.getpid()}")
    with temporary.open("w", encoding="utf-8", newline="\n") as stream:
        json.dump(payload, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    _flush_file(temporary)
    _durable_replace(temporary, path)


def _pair_recipe_digest(path: Path, label: str) -> str:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ImportFailure(f"{label} is unreadable: {path}") from error
    if not isinstance(payload, dict):
        raise ImportFailure(f"{label} must contain a JSON object: {path}")
    digest = payload.get("recipe_sha256")
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", digest):
        raise ImportFailure(f"{label} has no valid recipe digest: {path}")
    return digest.lower()


def _staging_tree_digest(staging: Path) -> str:
    root = _canonical_staging_root(staging)
    digest = hashlib.sha256()
    for relative in sorted(_staging_file_set(root)):
        path = confined_existing_staged_path(root, relative, "transaction staged file")
        encoded = relative.encode("utf-8")
        digest.update(len(encoded).to_bytes(4, "big"))
        digest.update(encoded)
        digest.update(path.stat().st_size.to_bytes(8, "big"))
        digest.update(bytes.fromhex(sha256_file(path)))
    return digest.hexdigest()


def _valid_marker_digest(value) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def _validate_transaction_marker(
    transaction, staging: Path, output: Path
) -> dict[str, object]:
    path_fields = {
        "staging",
        "temporary",
        "backup",
        "output",
        "recipe_temporary",
        "recipe_backup",
    }
    digest_fields = {
        "old_recipe_sha256",
        "new_recipe_sha256",
        "old_recipe_file_sha256",
        "new_recipe_file_sha256",
        "old_receipt_file_sha256",
        "new_receipt_file_sha256",
        "old_staging_tree_sha256",
        "new_staging_tree_sha256",
    }
    required_fields = {"schema", "phase"} | path_fields | digest_fields
    if not isinstance(transaction, dict) or set(transaction) != required_fields:
        raise ImportFailure("anatomy transaction marker has missing or unknown fields")
    if transaction["schema"] != ANATOMY_TRANSACTION_SCHEMA:
        raise ImportFailure("anatomy transaction marker has an unsupported schema")
    if transaction["phase"] not in {
        "prepared",
        "staging-backed-up",
        "staging-promoted",
        "recipe-promoted",
    }:
        raise ImportFailure("anatomy transaction marker has an invalid phase")
    if any(not isinstance(transaction[field], str) for field in path_fields):
        raise ImportFailure("anatomy transaction marker has a non-string path field")
    if any(not _valid_marker_digest(transaction[field]) for field in digest_fields):
        raise ImportFailure("anatomy transaction marker has an invalid SHA-256 field")
    expected_staging = str(staging.resolve(strict=False))
    expected_output = str(output.resolve(strict=False))
    if (
        transaction["staging"] != expected_staging
        or transaction["output"] != expected_output
    ):
        raise ImportFailure("anatomy transaction marker does not match requested staging/output")
    return transaction


def _validate_recovery_operands(transaction: dict, staging: Path, output: Path) -> dict[str, Path]:
    paths = {
        field: Path(transaction[field])
        for field in (
            "staging",
            "temporary",
            "backup",
            "output",
            "recipe_temporary",
            "recipe_backup",
        )
    }
    expected_paths = (
        (paths["staging"], staging.parent.resolve(), staging.name, False),
        (
            paths["temporary"],
            staging.parent.resolve(),
            f"{staging.name}.augment-tmp-",
            True,
        ),
        (
            paths["backup"],
            staging.parent.resolve(),
            f"{staging.name}.augment-rollback-",
            True,
        ),
        (paths["output"], output.parent.resolve(), output.name, False),
        (
            paths["recipe_temporary"],
            output.parent.resolve(),
            f".{output.name}.augment-tmp-",
            True,
        ),
        (
            paths["recipe_backup"],
            output.parent.resolve(),
            f".{output.name}.augment-rollback-",
            True,
        ),
    )
    for path, parent, expected_name, is_prefix in expected_paths:
        name_matches = (
            path.name.startswith(expected_name) if is_prefix else path.name == expected_name
        )
        if path.parent.resolve(strict=False) != parent or not name_matches:
            raise ImportFailure("anatomy transaction marker contains an unsafe recovery path")
        if _is_symlink_or_reparse(path):
            raise ImportFailure(
                f"anatomy transaction recovery operand is a symlink or reparse point: {path}"
            )
    return paths


def _recipe_file_matches(path: Path, recipe_digest: str, file_digest: str) -> bool:
    if _is_symlink_or_reparse(path) or not path.is_file():
        return False
    return (
        sha256_file(path) == file_digest
        and _pair_recipe_digest(path, "transaction recipe") == recipe_digest
    )


def _staging_generation_matches(
    path: Path,
    recipe_digest: str,
    receipt_file_digest: str,
    tree_digest: str,
) -> bool:
    if _is_symlink_or_reparse(path) or not path.is_dir():
        return False
    receipt = path / "build_receipt.json"
    return (
        not _is_symlink_or_reparse(receipt)
        and receipt.is_file()
        and sha256_file(receipt) == receipt_file_digest
        and _pair_recipe_digest(receipt, "transaction staging receipt") == recipe_digest
        and _staging_tree_digest(path) == tree_digest
    )


def _write_augmentation_phase(marker: Path, transaction: dict, phase: str) -> None:
    transaction["phase"] = phase
    _durable_json_write(marker, transaction)


def _cleanup_augmentation_orphans(staging: Path, output: Path) -> None:
    if _is_symlink_or_reparse(staging) or _is_symlink_or_reparse(output):
        raise ImportFailure("live augmentation pair contains a symlink or reparse point")
    receipt = staging / "build_receipt.json"
    if not staging.is_dir() or not output.is_file() or not receipt.is_file():
        return
    if _is_symlink_or_reparse(receipt):
        raise ImportFailure("live staging receipt is a symlink or reparse point")
    if _pair_recipe_digest(receipt, "live staging receipt") != _pair_recipe_digest(
        output, "live recipe"
    ):
        return
    marker = _augmentation_transaction_path(staging)
    candidates = []
    for pattern in (
        f"{staging.name}.augment-tmp-*",
        f"{staging.name}.augment-rollback-*",
    ):
        candidates.extend(staging.parent.glob(pattern))
    for pattern in (
        f".{output.name}.augment-tmp-*",
        f".{output.name}.augment-rollback-*",
        f".{marker.name}.write-*",
    ):
        candidates.extend(output.parent.glob(pattern))
    for candidate in sorted(set(candidates), key=lambda path: str(path)):
        if candidate.parent.resolve(strict=False) not in {
            staging.parent.resolve(),
            output.parent.resolve(),
        }:
            raise ImportFailure("augmentation orphan escaped its confined parent")
        if _is_symlink_or_reparse(candidate):
            raise ImportFailure(
                f"augmentation orphan is a symlink or reparse point: {candidate}"
            )
        _durable_remove(candidate)


def _recover_augmentation_transaction(staging: Path, output: Path) -> None:
    marker = _augmentation_transaction_path(staging)
    if _is_symlink_or_reparse(marker):
        raise ImportFailure("anatomy transaction marker is a symlink or reparse point")
    if not marker.exists():
        _cleanup_augmentation_orphans(staging, output)
        return
    try:
        transaction = json.loads(marker.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ImportFailure(f"anatomy transaction marker is unreadable: {marker}") from error
    transaction = _validate_transaction_marker(transaction, staging, output)
    paths = _validate_recovery_operands(transaction, staging, output)
    temporary = paths["temporary"]
    backup = paths["backup"]
    recipe_temporary = paths["recipe_temporary"]
    recipe_backup = paths["recipe_backup"]

    new_staging = _staging_generation_matches(
        staging,
        transaction["new_recipe_sha256"],
        transaction["new_receipt_file_sha256"],
        transaction["new_staging_tree_sha256"],
    )
    new_recipe = _recipe_file_matches(
        output,
        transaction["new_recipe_sha256"],
        transaction["new_recipe_file_sha256"],
    )
    old_staging = _staging_generation_matches(
        staging,
        transaction["old_recipe_sha256"],
        transaction["old_receipt_file_sha256"],
        transaction["old_staging_tree_sha256"],
    )
    old_recipe = _recipe_file_matches(
        output,
        transaction["old_recipe_sha256"],
        transaction["old_recipe_file_sha256"],
    )

    if not (new_staging and new_recipe) and not (old_staging and old_recipe):
        backup_matches = _staging_generation_matches(
            backup,
            transaction["old_recipe_sha256"],
            transaction["old_receipt_file_sha256"],
            transaction["old_staging_tree_sha256"],
        )
        recipe_backup_matches = _recipe_file_matches(
            recipe_backup,
            transaction["old_recipe_sha256"],
            transaction["old_recipe_file_sha256"],
        )
        if not old_staging:
            if not backup_matches:
                raise ImportFailure(
                    "anatomy transaction recovery has no verified old staging generation"
                )
            if staging.exists():
                _durable_remove(staging)
            _durable_replace(backup, staging)
        if not old_recipe:
            if not recipe_backup_matches:
                raise ImportFailure(
                    "anatomy transaction recovery has no verified old recipe generation"
                )
            _durable_replace(recipe_backup, output)
        if not _staging_generation_matches(
            staging,
            transaction["old_recipe_sha256"],
            transaction["old_receipt_file_sha256"],
            transaction["old_staging_tree_sha256"],
        ) or not _recipe_file_matches(
            output,
            transaction["old_recipe_sha256"],
            transaction["old_recipe_file_sha256"],
        ):
            raise ImportFailure("anatomy transaction recovery could not verify the restored tree")

    for orphan in (temporary, backup, recipe_temporary, recipe_backup):
        if orphan.exists() or _is_symlink_or_reparse(orphan):
            _durable_remove(orphan)
    _durable_remove(marker)


def _promote_augmented_pair(
    staging: Path,
    temporary: Path,
    backup: Path,
    recipe_temporary: Path,
    output: Path,
    phase_observer=None,
    *,
    verified_live_output: Path | None = None,
) -> None:
    if verified_live_output is not None and _is_symlink_or_reparse(verified_live_output):
        raise ImportFailure(
            f"augmentation promotion operand is a symlink or reparse point: {verified_live_output}"
        )
    marker = _augmentation_transaction_path(staging)
    recipe_backup = output.with_name(f".{output.name}.augment-rollback-{os.getpid()}")
    for operand in (
        staging,
        temporary,
        backup,
        output,
        recipe_temporary,
        recipe_backup,
        marker,
    ):
        if _is_symlink_or_reparse(operand):
            raise ImportFailure(
                f"augmentation promotion operand is a symlink or reparse point: {operand}"
            )
    _flush_tree(staging)
    _flush_tree(temporary)
    _flush_file(output)
    _flush_file(recipe_temporary)
    shutil.copy2(output, recipe_backup)
    _flush_file(recipe_backup)
    _require_directory_flush(recipe_backup.parent)
    old_receipt = staging / "build_receipt.json"
    new_receipt = temporary / "build_receipt.json"
    transaction = {
        "schema": ANATOMY_TRANSACTION_SCHEMA,
        "phase": "prepared",
        "staging": str(staging.resolve(strict=False)),
        "temporary": str(temporary.resolve(strict=False)),
        "backup": str(backup.resolve(strict=False)),
        "output": str(output.resolve(strict=False)),
        "recipe_temporary": str(recipe_temporary.resolve(strict=False)),
        "recipe_backup": str(recipe_backup.resolve(strict=False)),
        "old_recipe_sha256": _pair_recipe_digest(output, "existing recipe"),
        "new_recipe_sha256": _pair_recipe_digest(recipe_temporary, "new recipe"),
        "old_recipe_file_sha256": sha256_file(output),
        "new_recipe_file_sha256": sha256_file(recipe_temporary),
        "old_receipt_file_sha256": sha256_file(old_receipt),
        "new_receipt_file_sha256": sha256_file(new_receipt),
        "old_staging_tree_sha256": _staging_tree_digest(staging),
        "new_staging_tree_sha256": _staging_tree_digest(temporary),
    }
    if (
        _pair_recipe_digest(old_receipt, "old staging receipt")
        != transaction["old_recipe_sha256"]
        or _pair_recipe_digest(new_receipt, "new staging receipt")
        != transaction["new_recipe_sha256"]
    ):
        raise ImportFailure("staging receipt and recipe generations are not matched pairs")
    try:
        _write_augmentation_phase(marker, transaction, "prepared")
        if phase_observer is not None:
            phase_observer("prepared")

        _durable_replace(staging, backup)
        _write_augmentation_phase(marker, transaction, "staging-backed-up")
        if phase_observer is not None:
            phase_observer("staging-backed-up")

        _durable_replace(temporary, staging)
        _write_augmentation_phase(marker, transaction, "staging-promoted")
        if phase_observer is not None:
            phase_observer("staging-promoted")

        _durable_replace(recipe_temporary, output)
        _write_augmentation_phase(marker, transaction, "recipe-promoted")
        if phase_observer is not None:
            phase_observer("recipe-promoted")
        _recover_augmentation_transaction(staging, output)
    except Exception:
        if marker.exists():
            _recover_augmentation_transaction(staging, output)
        raise


def _staged_socket_manifests(staging: Path, recipe: dict) -> tuple[dict, dict]:
    manifests = {}
    paths = {}
    for asset in recipe["part_assets"]:
        for lod in asset["lods"]:
            relative = lod["socket_manifest"]
            path = confined_existing_staged_path(staging, relative, "socket manifest")
            manifest = json.loads(path.read_text(encoding="utf-8"))
            key = (asset["id"], lod["lod"])
            if key in manifests or manifest.get("asset_id") != asset["id"] or manifest.get("lod") != lod["lod"]:
                raise ImportFailure("staging socket manifest identity does not match the recipe")
            manifests[key] = manifest
            paths[key] = path
    if len(manifests) != 42:
        raise ImportFailure(f"staging must contain exactly 42 socket manifests; found {len(manifests)}")
    return manifests, paths


def _stable_output_hashes(staging: Path, recipe: dict) -> dict[str, dict[str, str]]:
    stable = {"obj": {}, "semantic": {}, "anatomy": {}}
    fields = {
        "obj": "generated_obj",
        "semantic": "semantic_mask",
        "anatomy": "anatomy_mask",
    }
    for asset in recipe["part_assets"]:
        for lod in asset["lods"]:
            for kind, field in fields.items():
                relative = lod[field]
                stable[kind][relative] = sha256_file(
                    confined_existing_staged_path(staging, relative, f"stable {kind} output")
                )
    if any(len(values) != 42 for values in stable.values()):
        raise ImportFailure("stable output capture must contain 42 OBJ, semantic, and anatomy hashes")
    return stable


def _all_staged_output_hashes(staging: Path, recipe: dict) -> dict[str, str]:
    outputs = {}
    for asset in recipe["part_assets"]:
        for lod in asset["lods"]:
            for field in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask"):
                relative = lod[field]
                outputs[relative] = sha256_file(
                    confined_existing_staged_path(staging, relative, "augmented output")
                )
    if len(outputs) != 168:
        raise ImportFailure(f"augmented output set must contain 168 files; found {len(outputs)}")
    return outputs


def _materialize_cross_torso_preparations(recipe: dict, staging: Path) -> dict:
    manifests, manifest_paths = _staged_socket_manifests(staging, recipe)
    assets = {asset["id"]: asset for asset in recipe["part_assets"]}
    contract = validate_preparation_contract(recipe)
    for key in sorted(manifests):
        manifest = manifests[key]
        asset = assets[manifest["asset_id"]]
        manifest["assembly_preparation_schema"] = contract["schema"]
        manifest["assembly_preparation_schema_digest"] = contract["schema_digest"]
        manifest["assembly_preparations"] = assembly_preparations(
            recipe, asset, manifest, manifests
        )
        manifest["cross_torso_preparations"] = []
        if asset["logical_slot"] != "torso":
            for family in recipe["families"]:
                if family["parts"][asset["logical_slot"]]["asset_id"] != asset["id"]:
                    continue
                canonical_target = family["parts"]["torso"]["asset_id"]
                for target in PREPARATION_TORSO_ASSETS:
                    if target == canonical_target:
                        continue
                    manifest["cross_torso_preparations"].extend(
                        assembly_preparations(
                            recipe,
                            asset,
                            manifest,
                            manifests,
                            family_filter=family["id"],
                            target_torso_asset_id=target,
                        )
                    )
        manifest["assembly_preparation_population"] = "canonical-plus-cross-torso"
        manifest["cross_torso_preparations"] = sorted(
            manifest["cross_torso_preparations"],
            key=lambda item: (
                item["family_id"],
                item["source_asset_id"],
                PREPARATION_TORSO_ASSETS.index(item["target_torso_asset_id"]),
                PREPARATION_LOD_ORDER.index(item["lod"]),
            ),
        )
    summary = validate_preparation_metadata(recipe, manifests)
    for key in sorted(manifests):
        manifest_paths[key].write_text(
            json.dumps(manifests[key], indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    return summary


def command_augment_cross_torso(args) -> None:
    staging = ensure_artifact_path(args.staging, "cross-torso staging input")
    live_output_argument = Path(args.output)
    if _is_symlink_or_reparse(live_output_argument):
        raise ImportFailure("cross-torso recipe output is a symlink or reparse point")
    output = live_output_argument.resolve(strict=False)
    output.parent.mkdir(parents=True, exist_ok=True)
    output_created = False
    if not output.exists():
        shutil.copy2(args.recipes, output)
        output_created = True
    try:
        _recover_augmentation_transaction(staging, output)
        recipe = load_recipe(args.recipes, validate_current_anatomy=False)
        _assert_tree_has_no_reparse_entries(staging)
        receipt_path = confined_existing_staged_path(
            staging, "build_receipt.json", "staged build receipt"
        )
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        _verify_receipt_outputs(staging, receipt, recipe)
        old_stable = _stable_output_hashes(staging, recipe)
        old_socket = {
            lod["socket_manifest"]: sha256_file(
                confined_existing_staged_path(
                    staging, lod["socket_manifest"], "stable socket manifest"
                )
            )
            for asset in recipe["part_assets"]
            for lod in asset["lods"]
        }
    except BaseException:
        if output_created:
            output.unlink(missing_ok=True)
        raise
    temporary = staging.with_name(staging.name + f".augment-tmp-{os.getpid()}")
    backup = staging.with_name(staging.name + f".augment-rollback-{os.getpid()}")
    recipe_temporary = output.with_name(f".{output.name}.augment-tmp-{os.getpid()}")
    promoted = False
    try:
        if temporary.exists():
            shutil.rmtree(temporary)
        if backup.exists():
            shutil.rmtree(backup)
        shutil.copytree(staging, temporary, symlinks=True)
        summary = _materialize_cross_torso_preparations(recipe, temporary)
        outputs = _all_staged_output_hashes(temporary, recipe)
        after_stable = _stable_output_hashes(temporary, recipe)
        if after_stable != old_stable:
            raise ImportFailure("cross-torso augmentation changed OBJ, semantic, or anatomy bytes")
        socket_after = {
            lod["socket_manifest"]: outputs[lod["socket_manifest"]]
            for asset in recipe["part_assets"]
            for lod in asset["lods"]
        }
        if set(socket_after) != set(old_socket):
            raise ImportFailure("cross-torso augmentation socket-manifest set drifted")
        changed_socket_count = sum(
            old_socket[path] != socket_after[path] for path in old_socket
        )
        if changed_socket_count not in (0, 42):
            raise ImportFailure("cross-torso augmentation did not change every socket-manifest digest")
        for asset in recipe["part_assets"]:
            for lod in asset["lods"]:
                for path_field, digest_field in (
                    ("generated_obj", "generated_obj_sha256"),
                    ("socket_manifest", "socket_manifest_sha256"),
                    ("semantic_mask", "semantic_mask_sha256"),
                    ("anatomy_mask", "anatomy_mask_sha256"),
                ):
                    lod[digest_field] = outputs[lod[path_field]]
        stable = {
            kind: dict(sorted(values.items())) for kind, values in after_stable.items()
        }
        preparation_receipt = {
            "schema": recipe["assembly_preparation_contract"]["schema"],
            "augmentor_version": recipe["assembly_preparation_contract"]["augmentor_version"],
            "schema_digest": recipe["assembly_preparation_contract"]["schema_digest"],
            "canonical_slot_records": summary["canonical_slot_records"],
            "cross_torso_slot_records": summary["cross_torso_slot_records"],
            "canonical_group_keys": summary["canonical_group_keys"],
            "cross_torso_group_keys": summary["cross_torso_group_keys"],
            "total_group_keys": summary["total_group_keys"],
            "stable_hashes": stable,
        }
        if changed_socket_count == 0:
            if (
                recipe.get("assembly_preparation") != preparation_receipt
                or receipt.get("assembly_preparation") != preparation_receipt
                or receipt.get("outputs") != outputs
            ):
                raise ImportFailure(
                    "already augmented metadata does not match the validated cross-torso result"
                )
            shutil.rmtree(temporary)
            print("cross_torso_slot_records=288")
            print("canonical_group_keys=252")
            print("cross_torso_group_keys=432")
            print("total_group_keys=684")
            print("unchanged_obj_semantic_anatomy=126")
            print("changed_socket_manifests=0")
            print(f"outputs={len(receipt['outputs'])}")
            print(f"recipe_sha256={recipe['recipe_sha256']}")
            return
        recipe["assembly_preparation"] = preparation_receipt
        recipe["recipe_sha256"] = canonical_recipe_digest(recipe)
        receipt["recipe_sha256"] = recipe["recipe_sha256"]
        receipt["outputs"] = outputs
        receipt["sources"] = _final_receipt_sources(receipt, recipe)
        receipt["assembly_preparation"] = preparation_receipt
        staged_output_path(temporary, "build_receipt.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        recipe_temporary.write_text(
            json.dumps(recipe, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
        )
        _promote_augmented_pair(
            staging,
            temporary,
            backup,
            recipe_temporary,
            output,
        )
        promoted = True
    except BaseException:
        if temporary.exists():
            shutil.rmtree(temporary)
        recipe_temporary.unlink(missing_ok=True)
        if output_created and not promoted and output.exists():
            output.unlink()
        raise
    print("cross_torso_slot_records=288")
    print("canonical_group_keys=252")
    print("cross_torso_group_keys=432")
    print("total_group_keys=684")
    print("unchanged_obj_semantic_anatomy=126")
    print("changed_socket_manifests=42")
    print(f"outputs={len(receipt['outputs'])}")
    print(f"recipe_sha256={recipe['recipe_sha256']}")


def command_augment_anatomy(args) -> None:
    staging = ensure_artifact_path(args.staging, "anatomy staging input")
    live_output_argument = Path(args.output)
    if _is_symlink_or_reparse(live_output_argument):
        raise ImportFailure("live prior output recipe is a symlink or reparse point")
    output = live_output_argument.resolve(strict=False)
    _recover_augmentation_transaction(staging, output)
    recipe = load_recipe(args.recipes, validate_current_anatomy=False)
    authority_path = (
        args.authority_recipes
        if getattr(args, "authority_recipes", None) is not None
        else output
    )
    authority_recipe = _load_live_authority_recipe(authority_path, output)
    _validate_authority_handoff(authority_recipe, recipe)
    _assert_tree_has_no_reparse_entries(staging)
    receipt_path = confined_existing_staged_path(
        staging, "build_receipt.json", "staged build receipt"
    )
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    _verify_receipt_outputs(staging, receipt, authority_recipe)
    old_stable = {
        lod[field]: sha256_file(
            confined_existing_staged_path(
                staging, lod[field], "stable staged OBJ or semantic mask"
            )
        )
        for asset in recipe["part_assets"]
        for lod in asset["lods"]
        for field in ("generated_obj", "semantic_mask")
    }
    temporary = staging.with_name(staging.name + f".augment-tmp-{os.getpid()}")
    backup = staging.with_name(staging.name + f".augment-rollback-{os.getpid()}")
    recipe_temporary = output.with_name(f".{output.name}.augment-tmp-{os.getpid()}")
    if temporary.exists():
        shutil.rmtree(temporary)
    if backup.exists():
        shutil.rmtree(backup)
    shutil.copytree(staging, temporary, symlinks=True)
    try:
        for asset in recipe["part_assets"]:
            asset["anatomy_authoring"] = source_projected_anatomy_authoring(
                asset, temporary
            )
        for asset in recipe["part_assets"]:
            lod_audits = {}
            for lod in asset["lods"]:
                obj_path = confined_existing_staged_path(
                    temporary, lod["generated_obj"], "generated anatomy source OBJ"
                )
                semantic_path = confined_existing_staged_path(
                    temporary, lod["semantic_mask"], "semantic staging mask"
                )
                anatomy_path = staged_output_path(temporary, lod["anatomy_mask"])
                anatomy_bytes, lod_audits[lod["lod"]] = anatomy_mask_with_audit(
                    semantic_path.read_bytes(),
                    obj_path.read_bytes(),
                    asset["anatomy_authoring"],
                    asset["logical_slot"],
                )
                anatomy_path.write_bytes(anatomy_bytes)
                socket_path = confined_existing_staged_path(
                    temporary, lod["socket_manifest"], "socket staging manifest"
                )
                socket = json.loads(socket_path.read_text(encoding="utf-8"))
                if socket.get("schema") != "alife.creature_part_sockets.v2" or socket.get("semantic_mask") != lod["semantic_mask"]:
                    raise ImportFailure(f"existing socket manifest is incompatible: {lod['socket_manifest']}")
                socket["anatomy_mask"] = lod["anatomy_mask"]
                socket_path.write_text(json.dumps(socket, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            asset["anatomy_authoring"]["source_projection_audit"] = {
                "schema": ANATOMY_AUDIT_SCHEMA,
                "lods": lod_audits,
            }
        outputs = _validate_augmented_tree(temporary, recipe)
        for asset in recipe["part_assets"]:
            for lod in asset["lods"]:
                for path_field, digest_field in (
                    ("generated_obj", "generated_obj_sha256"),
                    ("socket_manifest", "socket_manifest_sha256"),
                    ("semantic_mask", "semantic_mask_sha256"),
                    ("anatomy_mask", "anatomy_mask_sha256"),
                ):
                    lod[digest_field] = outputs[lod[path_field]]
        recipe["recipe_sha256"] = canonical_recipe_digest(recipe)
        receipt["recipe_sha256"] = recipe["recipe_sha256"]
        receipt["outputs"] = outputs
        receipt["sources"] = _final_receipt_sources(receipt, recipe)
        staged_output_path(temporary, "build_receipt.json").write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        after_stable = {
            relative: sha256_file(
                confined_existing_staged_path(
                    temporary, relative, "stable augmented OBJ or semantic mask"
                )
            )
            for relative in old_stable
        }
        if after_stable != old_stable:
            raise ImportFailure("augment-anatomy changed existing OBJ or semantic bytes")
        recipe_temporary.write_text(json.dumps(recipe, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
        try:
            _promote_augmented_pair(
                staging,
                temporary,
                backup,
                recipe_temporary,
                output,
                verified_live_output=live_output_argument,
            )
        except BaseException:
            _recover_augmentation_transaction(staging, output)
            raise
    except BaseException:
        if temporary.exists():
            shutil.rmtree(temporary)
        recipe_temporary.unlink(missing_ok=True)
        raise
    print("anatomy_masks=42")
    print("outputs=168")
    print("unchanged_obj_semantic=84")
    print(f"recipe_sha256={recipe['recipe_sha256']}")


def atomic_replace_pair(
    first_temporary: Path,
    first_destination: Path,
    second_temporary: Path,
    second_destination: Path,
) -> None:
    destinations = (first_destination, second_destination)
    backups = []
    for index, destination in enumerate(destinations):
        backup = destination.with_name(
            f".{destination.name}.rollback-{os.getpid()}-{index}"
        )
        backup.unlink(missing_ok=True)
        if destination.is_file():
            shutil.copy2(destination, backup)
            backups.append(backup)
        else:
            backups.append(None)
    first_replaced = False
    try:
        os.replace(first_temporary, first_destination)
        first_replaced = True
        os.replace(second_temporary, second_destination)
    except BaseException:
        if first_replaced:
            if backups[0] is None:
                first_destination.unlink(missing_ok=True)
            else:
                os.replace(backups[0], first_destination)
        raise
    finally:
        first_temporary.unlink(missing_ok=True)
        second_temporary.unlink(missing_ok=True)
        for backup in backups:
            if backup is not None:
                backup.unlink(missing_ok=True)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in (
        "inventory",
        "validate-sources",
        "build",
        "preview",
        "bind-output-digests",
        "augment-anatomy",
        "augment-cross-torso",
    ):
        child = subparsers.add_parser(command)
        child.add_argument("--recipes", type=Path, required=True)
        if command in ("bind-output-digests", "augment-anatomy", "augment-cross-torso"):
            child.add_argument("--staging", type=Path, required=True)
            child.add_argument("--output", type=Path, required=True)
            if command == "augment-anatomy":
                child.add_argument("--authority-recipes", type=Path)
            continue
        child.add_argument("--source-root", type=Path, required=True)
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
    args.recipes = args.recipes.resolve()
    if args.command == "augment-anatomy":
        command_augment_anatomy(args)
        return
    if args.command == "augment-cross-torso":
        command_augment_cross_torso(args)
        return
    recipe = load_recipe(args.recipes)
    if args.command == "bind-output-digests":
        command_bind_output_digests(args, recipe)
        return
    args.source_root = args.source_root.resolve()
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
