#!/usr/bin/env python3
"""Deterministic Blender 5.1 importer for staged GeneForge creature parts."""

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
    if recipe.get("importer_version") != REQUIRED_IMPORTER_VERSION:
        raise ImportFailure(f"recipe does not pin {REQUIRED_IMPORTER_VERSION}")
    for asset in recipe.get("part_assets", []):
        selector = asset.get("selector", {})
        if selector.get("selection_policy") != "exact-case-sensitive-names":
            raise ImportFailure(f"{asset.get('id')} has an unsupported selection policy")
        if selector.get("geometry_policy") != "evaluated-depsgraph":
            raise ImportFailure(f"{asset.get('id')} has an unsupported geometry policy")
        if set(selector.get("object_visscripts", {})) != set(selector.get("include_objects", [])):
            raise ImportFailure(f"{asset.get('id')} lacks exact kc3dsbpy_visscript contracts")
        validate_anatomy_authoring(asset)
        for lod in asset.get("lods", []):
            for field in ("anatomy_mask", "anatomy_mask_sha256"):
                if field not in lod:
                    raise ImportFailure(f"{asset.get('id')} {lod.get('lod')} lacks {field}")
    return recipe


def validate_anatomy_authoring(asset: dict) -> None:
    profile = asset.get("anatomy_authoring")
    slot = asset.get("logical_slot")
    if not isinstance(profile, dict) or profile.get("schema") != "alife.geneforge_anatomy_authoring.v1":
        raise ImportFailure(f"{asset.get('id')} has invalid anatomy authoring schema")
    if profile.get("coordinate_space") != "semantic-group-local-uv" or profile.get("default_channel") != "primary":
        raise ImportFailure(f"{asset.get('id')} has invalid anatomy authoring coordinates/default")
    if slot not in ALLOWED_ANATOMY_CHANNELS:
        raise ImportFailure(f"{asset.get('id')} has unsupported anatomy slot {slot}")
    zones = profile.get("zones")
    if not isinstance(zones, list) or not zones:
        raise ImportFailure(f"{asset.get('id')} has no anatomy zones")
    ids = set()
    authored_channels = {"primary"}
    for zone in zones:
        zone_id = zone.get("id")
        channel = zone.get("channel")
        groups = zone.get("semantic_groups")
        shape = zone.get("shape", {})
        if not isinstance(zone_id, str) or not zone_id or zone_id in ids:
            raise ImportFailure(f"{asset.get('id')} has invalid/duplicate anatomy zone id")
        ids.add(zone_id)
        if channel not in ALLOWED_ANATOMY_CHANNELS[slot]:
            raise ImportFailure(f"{asset.get('id')} anatomy channel {channel} is not owned by {slot}")
        authored_channels.add(channel)
        if not isinstance(groups, list) or not groups or any(group not in GROUP_REGIONS for group in groups):
            raise ImportFailure(f"{asset.get('id')} anatomy zone {zone_id} has unknown semantic group")
        if not isinstance(zone.get("priority"), int) or not isinstance(zone.get("strength"), int) or not 1 <= zone["strength"] <= 255:
            raise ImportFailure(f"{asset.get('id')} anatomy zone {zone_id} has invalid priority/strength")
        kind = shape.get("kind")
        if kind == "ellipse":
            center, radius = shape.get("center"), shape.get("radius")
            if not _unit_pair(center) or not _positive_unit_pair(radius):
                raise ImportFailure(f"{asset.get('id')} anatomy zone {zone_id} has malformed ellipse")
        elif kind == "polygon":
            points = shape.get("points")
            if not isinstance(points, list) or len(points) < 3 or any(not _unit_pair(point) for point in points):
                raise ImportFailure(f"{asset.get('id')} anatomy zone {zone_id} has malformed polygon")
        else:
            raise ImportFailure(f"{asset.get('id')} anatomy zone {zone_id} has unknown shape")
    missing = REQUIRED_ANATOMY_CHANNELS[slot] - authored_channels
    if missing:
        raise ImportFailure(f"{asset.get('id')} anatomy profile lacks channels: {sorted(missing)}")


def _unit_pair(value) -> bool:
    return isinstance(value, list) and len(value) == 2 and all(isinstance(v, (int, float)) and math.isfinite(v) and 0.0 <= v <= 1.0 for v in value)


def _positive_unit_pair(value) -> bool:
    return _unit_pair(value) and all(v > 0.0 for v in value)


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
        outputs = {
            path.relative_to(temporary).as_posix(): sha256_file(path)
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


def decode_rgba_png(data: bytes) -> tuple[int, int, bytes]:
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ImportFailure("anatomy source is not a PNG")
    offset = 8
    width = height = None
    compressed = bytearray()
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        if zlib.crc32(kind + payload) & 0xFFFFFFFF != struct.unpack(">I", data[offset + 8 + length : offset + 12 + length])[0]:
            raise ImportFailure("anatomy source PNG has an invalid chunk checksum")
        offset += 12 + length
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise ImportFailure("anatomy source PNG must be deterministic RGBA8")
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
    if width is None or height is None:
        raise ImportFailure("anatomy source PNG is missing IHDR")
    raw = zlib.decompress(bytes(compressed))
    stride = width * 4
    if len(raw) != height * (stride + 1):
        raise ImportFailure("anatomy source PNG has invalid decompressed length")
    pixels = bytearray()
    for row in range(height):
        start = row * (stride + 1)
        if raw[start] != 0:
            raise ImportFailure("anatomy source PNG must use deterministic filter zero")
        pixels.extend(raw[start + 1 : start + 1 + stride])
    return width, height, bytes(pixels)


def _point_in_polygon(point: tuple[float, float], vertices: list[list[float]]) -> bool:
    x, y = point
    inside = False
    previous = vertices[-1]
    for current in vertices:
        x0, y0 = previous
        x1, y1 = current
        if (y0 > y) != (y1 > y):
            crossing = (x1 - x0) * (y - y0) / (y1 - y0) + x0
            if x <= crossing:
                inside = not inside
        previous = current
    return inside


def _zone_contains(shape: dict, uv: tuple[float, float]) -> bool:
    if shape["kind"] == "ellipse":
        dx = (uv[0] - shape["center"][0]) / shape["radius"][0]
        dy = (uv[1] - shape["center"][1]) / shape["radius"][1]
        return dx * dx + dy * dy <= 1.0
    return _point_in_polygon(uv, shape["points"])


def anatomy_mask(semantic_png: bytes, profile: dict, logical_slot: str) -> bytes:
    validate_anatomy_authoring({"id": "raster-input", "logical_slot": logical_slot, "anatomy_authoring": profile})
    width, height, semantic = decode_rgba_png(semantic_png)
    if (width, height) != (64, 64):
        raise ImportFailure("semantic mask for anatomy must be exactly 64x64 RGBA8")
    color_to_group = {tuple(color[:3]): group for group, color in GROUP_COLORS.items()}
    zones = sorted(profile["zones"], key=lambda zone: (zone["priority"], zone["id"]))
    output = bytearray(width * height * 4)
    used_channels = set()
    for y in range(height):
        for x in range(width):
            offset = (y * width + x) * 4
            alpha = semantic[offset + 3]
            if alpha == 0:
                continue
            semantic_color = tuple(semantic[offset : offset + 3])
            group = color_to_group.get(semantic_color)
            if group is None:
                raise ImportFailure(f"semantic mask contains unknown occupied color {semantic_color}")
            # Detail cells stay Primary; the coat baker treats their semantic roles.
            channel = "primary"
            strength = 255
            selected_priority = None
            atlas_uv = ((x + 0.5) / width, (y + 0.5) / height)
            local_uv = semantic_source_uv(group, atlas_uv)
            if not group.startswith("head."):
                for zone in zones:
                    if group not in zone["semantic_groups"] or not _zone_contains(zone["shape"], local_uv):
                        continue
                    if selected_priority == zone["priority"] and channel != zone["channel"]:
                        raise ImportFailure(f"equal-priority anatomy zones conflict at pixel {x},{y}")
                    channel = zone["channel"]
                    strength = zone["strength"]
                    selected_priority = zone["priority"]
            output[offset : offset + 4] = bytes((*ANATOMY_COLORS[channel], strength))
            used_channels.add(channel)
    missing = REQUIRED_ANATOMY_CHANNELS[logical_slot] - used_channels
    if missing:
        raise ImportFailure(f"anatomy raster lacks required {logical_slot} channels: {sorted(missing)}")
    return png_bytes(width, height, bytes(output))


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
    output = ensure_artifact_path(args.output, "preview output")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(
        tempfile.mkdtemp(prefix=f".{output.name}.tmp-", dir=output.parent)
    )
    count = 0
    try:
        for asset in recipe["part_assets"]:
            for lod in asset["lods"]:
                obj = staging / lod["generated_obj"]
                if not obj.is_file():
                    raise ImportFailure(f"missing staged OBJ for preview: {obj}")
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
    receipt_path = staging / "build_receipt.json"
    if not receipt_path.is_file():
        raise ImportFailure(f"missing staged build receipt: {receipt_path}")
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
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
                raw = Path(relative)
                if raw.is_absolute() or any(part == ".." for part in raw.parts):
                    raise ImportFailure(f"generated output escapes staging: {relative}")
                path = staging / raw
                if not path.is_file():
                    raise ImportFailure(f"missing generated output for digest binding: {relative}")
                digest = sha256_file(path)
                lod[digest_field] = digest
                bound_outputs[raw.as_posix()] = digest
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


def _verify_receipt_outputs(staging: Path, receipt: dict) -> None:
    if receipt.get("schema") != "alife.geneforge_build_receipt.v2":
        raise ImportFailure("augment-anatomy requires a v2 build receipt")
    outputs = receipt.get("outputs")
    if not isinstance(outputs, dict):
        raise ImportFailure("build receipt outputs are missing")
    for relative, expected in outputs.items():
        path = staging / Path(relative)
        if not path.is_file() or sha256_file(path) != expected:
            raise ImportFailure(f"existing staged output digest mismatch: {relative}")


def _validate_augmented_tree(staging: Path, recipe: dict) -> dict[str, str]:
    outputs = {}
    if len(recipe["part_assets"]) != 14:
        raise ImportFailure("anatomy augmentation requires 14 production assets")
    for asset in recipe["part_assets"]:
        for lod in asset["lods"]:
            semantic_path = staging / Path(lod["semantic_mask"])
            anatomy_path = staging / Path(lod["anatomy_mask"])
            semantic_size, _, semantic = decode_rgba_png(semantic_path.read_bytes())
            anatomy_size, _, anatomy = decode_rgba_png(anatomy_path.read_bytes())
            if semantic_size != 64 or anatomy_size != 64:
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
            socket = json.loads((staging / Path(lod["socket_manifest"])).read_text(encoding="utf-8"))
            if socket.get("schema") != "alife.creature_part_sockets.v2" or socket.get("anatomy_mask") != lod["anatomy_mask"]:
                raise ImportFailure(f"socket anatomy metadata mismatch: {lod['socket_manifest']}")
            for field in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask"):
                relative = Path(lod[field])
                path = staging / relative
                if not path.is_file():
                    raise ImportFailure(f"missing augmented output: {lod[field]}")
                if path.stat().st_size > 512 * 1024:
                    raise ImportFailure(f"augmented output exceeds 512 KiB: {lod[field]}")
                outputs[relative.as_posix()] = sha256_file(path)
    if len(outputs) != 168:
        raise ImportFailure(f"augmented output set must contain 168 files; found {len(outputs)}")
    if sum((staging / Path(relative)).stat().st_size for relative in outputs) > 8 * 1024 * 1024:
        raise ImportFailure("augmented production pack exceeds 8 MiB")
    return outputs


def command_augment_anatomy(args, recipe: dict) -> None:
    staging = ensure_artifact_path(args.staging, "anatomy staging input")
    receipt_path = staging / "build_receipt.json"
    if not receipt_path.is_file():
        raise ImportFailure(f"missing staged build receipt: {receipt_path}")
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    _verify_receipt_outputs(staging, receipt)
    legacy_outputs = {
        lod[field]
        for asset in recipe["part_assets"]
        for lod in asset["lods"]
        for field in ("generated_obj", "socket_manifest", "semantic_mask")
    }
    augmented_outputs = legacy_outputs | {
        lod["anatomy_mask"]
        for asset in recipe["part_assets"]
        for lod in asset["lods"]
    }
    receipt_outputs = set(receipt["outputs"])
    if receipt_outputs not in (legacy_outputs, augmented_outputs):
        raise ImportFailure(
            "existing build receipt must contain exactly 126 legacy or 168 augmented outputs"
        )
    old_stable = {
        lod[field]: sha256_file(staging / Path(lod[field]))
        for asset in recipe["part_assets"]
        for lod in asset["lods"]
        for field in ("generated_obj", "semantic_mask")
    }
    temporary = staging.with_name(staging.name + f".augment-tmp-{os.getpid()}")
    backup = staging.with_name(staging.name + f".augment-rollback-{os.getpid()}")
    output = args.output.resolve()
    recipe_temporary = output.with_name(f".{output.name}.augment-tmp-{os.getpid()}")
    if temporary.exists():
        shutil.rmtree(temporary)
    if backup.exists():
        shutil.rmtree(backup)
    shutil.copytree(staging, temporary)
    try:
        for asset in recipe["part_assets"]:
            for lod in asset["lods"]:
                semantic_path = temporary / Path(lod["semantic_mask"])
                anatomy_path = staged_output_path(temporary, lod["anatomy_mask"])
                anatomy_path.write_bytes(anatomy_mask(semantic_path.read_bytes(), asset["anatomy_authoring"], asset["logical_slot"]))
                socket_path = temporary / Path(lod["socket_manifest"])
                socket = json.loads(socket_path.read_text(encoding="utf-8"))
                if socket.get("schema") != "alife.creature_part_sockets.v2" or socket.get("semantic_mask") != lod["semantic_mask"]:
                    raise ImportFailure(f"existing socket manifest is incompatible: {lod['socket_manifest']}")
                socket["anatomy_mask"] = lod["anatomy_mask"]
                socket_path.write_text(json.dumps(socket, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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
        for source in receipt.get("sources", []):
            donor_outputs = [
                lod[field]
                for asset in recipe["part_assets"] if asset["donor"] == source["donor"]
                for lod in asset["lods"]
                for field in ("generated_obj", "socket_manifest", "semantic_mask", "anatomy_mask")
            ]
            source["outputs"] = donor_outputs
            source["output_count"] = len(donor_outputs)
        (temporary / "build_receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        after_stable = {relative: sha256_file(temporary / Path(relative)) for relative in old_stable}
        if after_stable != old_stable:
            raise ImportFailure("augment-anatomy changed existing OBJ or semantic bytes")
        recipe_temporary.write_text(json.dumps(recipe, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
        staging.replace(backup)
        try:
            temporary.replace(staging)
            os.replace(recipe_temporary, output)
        except BaseException:
            if staging.exists():
                shutil.rmtree(staging)
            backup.replace(staging)
            raise
        shutil.rmtree(backup)
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
    ):
        child = subparsers.add_parser(command)
        child.add_argument("--recipes", type=Path, required=True)
        if command in ("bind-output-digests", "augment-anatomy"):
            child.add_argument("--staging", type=Path, required=True)
            child.add_argument("--output", type=Path, required=True)
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
    recipe = load_recipe(args.recipes)
    if args.command == "bind-output-digests":
        command_bind_output_digests(args, recipe)
        return
    if args.command == "augment-anatomy":
        command_augment_anatomy(args, recipe)
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


def assembly_preparations(
    recipe: dict,
    asset: dict,
    manifest: dict,
    manifests: dict,
) -> list[dict]:
    contract = recipe["assembly_contract"]
    preparations = []
    for family in recipe["families"]:
        for slot, part in family["parts"].items():
            if part["asset_id"] != asset["id"]:
                continue
            sockets = contract["slot_sockets"][slot]
            source_anchors = [manifest["sockets"][name]["translation"] for name in sockets]
            torso_asset_id = family["parts"]["torso"]["asset_id"]
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
            preparations.append(
                {
                    "family_id": family["id"],
                    "family_label": family["label"],
                    "logical_slot": slot,
                    "asset_id": asset["id"],
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
            )
    return preparations


def postprocess_assembly_preparations(recipe: dict, staging: Path) -> None:
    assets = {asset["id"]: asset for asset in recipe["part_assets"]}
    paths = sorted(staging.rglob("*_sockets.json"))
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
        manifest["assembly_preparations"] = assembly_preparations(
            recipe, asset, manifest, manifests
        )
        manifest_paths[key].write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )


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
        "assembly_preparation_schema": recipe["assembly_contract"]["schema"],
        "bridge_geometry": bridge_geometry,
        "assembly_preparations": [],
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
    raw = Path(relative)
    if raw.is_absolute() or any(part == ".." for part in raw.parts):
        raise ImportFailure(f"generated output escapes staging: {relative}")
    path = staging.resolve() / raw
    path.parent.mkdir(parents=True, exist_ok=True)
    return path


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
            obj_path.write_bytes(emit_obj(lod_grouped))
            semantic_bytes = semantic_mask(lod_grouped, source_samples)
            mask_path.write_bytes(semantic_bytes)
            anatomy_path.write_bytes(
                anatomy_mask(semantic_bytes, asset["anatomy_authoring"], asset["logical_slot"])
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
