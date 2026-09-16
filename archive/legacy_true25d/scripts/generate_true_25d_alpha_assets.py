#!/usr/bin/env python3
"""Generate original low-poly glTF seed assets for the A-Life true 2.5D lane.

The assets are intentionally tiny and procedural, but they are authored as
native glTF geometry rather than flat sprite rectangles. They are display-only:
no physics, navigation, cognition, or action authority is encoded here.

This script now emits seed `.gltf` files under `target/artifacts/` by default.
The active product lane uses Blender-normalized `.glb` files validated by the
committed manifest; do not overwrite that lane unless intentionally regenerating
the seed pack before running the Blender normalization wrapper.
"""

from __future__ import annotations

import argparse
import base64
import json
import math
import struct
import random
import zlib
from pathlib import Path


ACTIVE_ROOT = Path("crates/alife_game_app/assets/true_25d_alpha_v1")
ACTIVE_MANIFEST = ACTIVE_ROOT / "true_25d_manifest.json"
DEFAULT_ROOT = Path("target/artifacts/true25d_seed_gltf")
SCHEMA = "alife.ca44a.true_25d_asset_manifest.v1"
ART_DIRECTION = "true-2-5d-retro-futuristic-biological-v1"
ACTIVE_GROUND_TILE = Path("crates/alife_game_app/assets/alpha_art_v1/ground_tile_repeat.png")


def pack_f32(values):
    return b"".join(struct.pack("<f", float(v)) for v in values)


def pack_u16(values):
    return b"".join(struct.pack("<H", int(v)) for v in values)


def png_chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def write_repeat_ground_tile(path: Path):
    """Write the tiny repeatable diffuse tile used by the 3D ground plane."""
    path.parent.mkdir(parents=True, exist_ok=True)
    width = height = 128
    rng = random.Random(4242)

    def clamp(value):
        return max(0, min(255, int(round(value))))

    def blend(left, right, alpha):
        return tuple(clamp(left[i] * (1.0 - alpha) + right[i] * alpha) for i in range(3))

    shadow = (45, 70, 38)
    light = (82, 120, 55)
    soil = (94, 70, 42)
    resource = (48, 116, 56)
    stone = (86, 91, 78)
    water = (34, 86, 104)
    pixels = []
    for y in range(height):
        row = []
        for x in range(width):
            u = x / width
            v = y / height
            n1 = math.sin((u * math.tau * 2.0) + math.sin(v * math.tau) * 0.55)
            n2 = math.cos((v * math.tau * 3.0) + math.sin(u * math.tau * 2.0) * 0.35)
            n3 = math.sin((u + v) * math.tau * 4.0) * math.cos((u - v) * math.tau * 3.0)
            t = max(0.0, min(1.0, (n1 * 0.35 + n2 * 0.25 + n3 * 0.18 + 1.0) * 0.5))
            color = blend(shadow, light, 0.22 + t * 0.42)
            path_field = abs(math.sin((u * 1.15 + v * 0.82) * math.tau))
            if path_field < 0.11:
                color = blend(color, soil, 0.20 * (1.0 - path_field / 0.11))
            fleck = math.sin((u * 7.0 + 0.2) * math.tau) * math.sin((v * 6.0 + 0.35) * math.tau)
            if fleck > 0.86:
                color = blend(color, resource, 0.12)
            damp = math.cos((u * 5.0 - v * 4.0) * math.tau)
            if damp > 0.86:
                color = blend(color, water, 0.06)
            if damp < -0.88:
                color = blend(color, stone, 0.08)
            micro = math.sin((u * 17.0 + 0.41) * math.tau) * math.cos((v * 19.0 - 0.27) * math.tau)
            if micro > 0.94:
                color = blend(color, (118, 146, 74), 0.16)
            elif micro < -0.94:
                color = blend(color, (58, 84, 52), 0.14)
            grain = math.sin((u * 31.0 + v * 23.0) * math.tau)
            if grain > 0.975:
                color = blend(color, (104, 90, 58), 0.15)
            row.append(tuple(clamp((channel // 4) * 4 + 2) for channel in color) + (255,))
        pixels.append(row)

    def paint_disc(cx, cy, rx, ry, color, alpha):
        for yy in range(int(cy - ry - 2), int(cy + ry + 3)):
            for xx in range(int(cx - rx - 2), int(cx + rx + 3)):
                x = xx % width
                y = yy % height
                dx = (xx - cx) / max(rx, 1)
                dy = (yy - cy) / max(ry, 1)
                distance = dx * dx + dy * dy
                if distance <= 1.0:
                    old = pixels[y][x]
                    pixels[y][x] = blend(old[:3], color, alpha * ((1.0 - distance) ** 0.6)) + (255,)

    for _ in range(26):
        cx, cy = rng.randrange(width), rng.randrange(height)
        if rng.random() < 0.70:
            paint_disc(cx, cy, rng.uniform(1.0, 1.8), rng.uniform(2.0, 4.2), (108, 150, 70), 0.24)
        else:
            paint_disc(cx, cy, rng.uniform(1.2, 2.4), rng.uniform(1.0, 2.0), (70, 76, 60), 0.22)

    for cx, cy, rx, ry, color in [
        (18, 22, 3.5, 1.6, (132, 156, 72)),
        (52, 91, 2.8, 2.2, (42, 116, 56)),
        (87, 38, 3.0, 1.5, (128, 82, 42)),
        (113, 73, 2.6, 1.8, (52, 92, 104)),
        (31, 112, 2.4, 1.7, (148, 132, 66)),
        (101, 117, 2.3, 1.9, (40, 72, 46)),
        (71, 14, 2.5, 1.4, (108, 108, 86)),
        (123, 22, 2.0, 1.8, (74, 132, 78)),
    ]:
        paint_disc(cx, cy, rx, ry, color, 0.56)

    raw = bytearray()
    for row in pixels:
        raw.append(0)
        for rgba in row:
            raw.extend(rgba)
    png = bytearray(b"\x89PNG\r\n\x1a\n")
    png.extend(png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)))
    png.extend(png_chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
    png.extend(png_chunk(b"IEND", b""))
    path.write_bytes(png)


def normal_for(vertex):
    x, y, z = vertex
    length = math.sqrt(x * x + y * y + z * z) or 1.0
    return (x / length, y / length, z / length)


def translate(vertices, dx=0.0, dy=0.0, dz=0.0):
    return [(x + dx, y + dy, z + dz) for x, y, z in vertices]


def scale_vertices(vertices, sx=1.0, sy=1.0, sz=1.0):
    return [(x * sx, y * sy, z * sz) for x, y, z in vertices]


def rebase_indices(indices, offset):
    return [index + offset for index in indices]


def make_gltf(path: Path, name: str, primitives):
    buffers = bytearray()
    buffer_views = []
    accessors = []
    mesh_primitives = []
    materials = []

    for material_index, primitive in enumerate(primitives):
        positions, normals, indices, color, material_name, alpha = primitive
        if len(positions) != len(normals):
            raise ValueError(f"{name}:{material_name} position/normal mismatch")

        pos_bytes = pack_f32([value for vertex in positions for value in vertex])
        norm_bytes = pack_f32([value for normal in normals for value in normal])
        idx_bytes = pack_u16(indices)

        pos_offset = len(buffers)
        buffers.extend(pos_bytes)
        norm_offset = len(buffers)
        buffers.extend(norm_bytes)
        idx_offset = len(buffers)
        buffers.extend(idx_bytes)

        pos_view = len(buffer_views)
        buffer_views.append(
            {
                "buffer": 0,
                "byteOffset": pos_offset,
                "byteLength": len(pos_bytes),
                "target": 34962,
            }
        )
        norm_view = len(buffer_views)
        buffer_views.append(
            {
                "buffer": 0,
                "byteOffset": norm_offset,
                "byteLength": len(norm_bytes),
                "target": 34962,
            }
        )
        idx_view = len(buffer_views)
        buffer_views.append(
            {
                "buffer": 0,
                "byteOffset": idx_offset,
                "byteLength": len(idx_bytes),
                "target": 34963,
            }
        )

        mins = [min(vertex[i] for vertex in positions) for i in range(3)]
        maxs = [max(vertex[i] for vertex in positions) for i in range(3)]
        position_accessor = len(accessors)
        accessors.append(
            {
                "bufferView": pos_view,
                "componentType": 5126,
                "count": len(positions),
                "type": "VEC3",
                "min": mins,
                "max": maxs,
            }
        )
        normal_accessor = len(accessors)
        accessors.append(
            {
                "bufferView": norm_view,
                "componentType": 5126,
                "count": len(normals),
                "type": "VEC3",
            }
        )
        index_accessor = len(accessors)
        accessors.append(
            {
                "bufferView": idx_view,
                "componentType": 5123,
                "count": len(indices),
                "type": "SCALAR",
            }
        )

        rgba = [color[0], color[1], color[2], alpha]
        material = {
            "name": f"{name}-{material_name}-toon",
            "doubleSided": True,
            "emissiveFactor": [color[0] * 0.20, color[1] * 0.20, color[2] * 0.20],
            "pbrMetallicRoughness": {
                "baseColorFactor": rgba,
                "metallicFactor": 0.0,
                "roughnessFactor": 0.82,
            },
        }
        if alpha < 0.995:
            material["alphaMode"] = "BLEND"
            material["alphaCutoff"] = 0.02
        materials.append(material)

        mesh_primitives.append(
            {
                "attributes": {"POSITION": position_accessor, "NORMAL": normal_accessor},
                "indices": index_accessor,
                "material": material_index,
            }
        )

    blob = bytes(buffers)
    gltf = {
        "asset": {"version": "2.0", "generator": "A-Life true_25d alpha generator"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": name, "mesh": 0}],
        "meshes": [{"name": f"{name}-mesh", "primitives": mesh_primitives}],
        "materials": materials,
        "buffers": [
            {
                "byteLength": len(blob),
                "uri": "data:application/octet-stream;base64,"
                + base64.b64encode(blob).decode("ascii"),
            }
        ],
        "bufferViews": buffer_views,
        "accessors": accessors,
    }
    path.write_text(json.dumps(gltf, indent=2), encoding="utf-8")


def low_poly_ellipsoid(rx=0.5, ry=0.32, rz=0.38, rings=4, segments=10):
    positions = [(0.0, ry, 0.0)]
    for ring in range(1, rings):
        phi = math.pi * ring / rings
        y = math.cos(phi) * ry
        radius = math.sin(phi)
        for index in range(segments):
            angle = math.tau * index / segments
            wobble = 1.0 + 0.06 * math.sin(angle * 3.0)
            positions.append((math.cos(angle) * rx * radius * wobble, y, math.sin(angle) * rz * radius))
    positions.append((0.0, -ry * 0.72, 0.0))
    bottom = len(positions) - 1
    normals = [normal_for((x / max(rx, 0.001), y / max(ry, 0.001), z / max(rz, 0.001))) for x, y, z in positions]
    indices = []
    first = 1
    for index in range(segments):
        indices += [0, first + index, first + ((index + 1) % segments)]
    for ring in range(rings - 2):
        start = 1 + ring * segments
        nxt = start + segments
        for index in range(segments):
            a = start + index
            b = start + ((index + 1) % segments)
            c = nxt + index
            d = nxt + ((index + 1) % segments)
            indices += [a, c, b, b, c, d]
    last = 1 + (rings - 2) * segments
    for index in range(segments):
        indices += [last + ((index + 1) % segments), last + index, bottom]
    return positions, normals, indices


def prism(radius=0.5, height=0.45, sides=8, squash_z=0.8):
    positions = []
    normals = []
    for y in (0.0, height):
        for i in range(sides):
            angle = math.tau * i / sides
            x = math.cos(angle) * radius
            z = math.sin(angle) * radius * squash_z
            positions.append((x, y, z))
            normals.append(normal_for((x, radius * 0.30, z)))
    indices = []
    for i in range(sides):
        j = (i + 1) % sides
        indices += [i, j, sides + j, i, sides + j, sides + i]
    positions.append((0.0, height + radius * 0.10, 0.0))
    normals.append((0.0, 1.0, 0.0))
    top = sides * 2
    for i in range(sides):
        indices += [sides + i, sides + ((i + 1) % sides), top]
    return positions, normals, indices


def pyramid_cluster(points):
    positions = []
    normals = []
    indices = []
    for cx, cz, radius, height, squash in points:
        base = len(positions)
        positions.extend(
            [
                (cx - radius, 0.0, cz - radius * squash),
                (cx + radius, 0.0, cz - radius * squash),
                (cx + radius * 0.82, 0.0, cz + radius * squash),
                (cx - radius * 0.75, 0.0, cz + radius * squash),
                (cx, height, cz),
            ]
        )
        normals.extend(
            [
                (-0.35, 0.65, -0.55),
                (0.55, 0.65, -0.20),
                (0.35, 0.65, 0.55),
                (-0.55, 0.65, 0.20),
                (0.0, 1.0, 0.0),
            ]
        )
        indices.extend(
            [
                base,
                base + 1,
                base + 4,
                base + 1,
                base + 2,
                base + 4,
                base + 2,
                base + 3,
                base + 4,
                base + 3,
                base,
                base + 4,
                base,
                base + 3,
                base + 2,
                base,
                base + 2,
                base + 1,
            ]
        )
    return positions, normals, indices


def leaf_blades(count=5, height=0.42, spread=0.45):
    positions = []
    normals = []
    indices = []
    for i in range(count):
        angle = math.tau * i / count + 0.35
        width = 0.04 + 0.012 * (i % 3)
        tip = (math.cos(angle) * spread * 0.36, height * (0.82 + 0.06 * (i % 2)), math.sin(angle) * spread * 0.36)
        left = (math.cos(angle + 1.85) * width, 0.02, math.sin(angle + 1.85) * width)
        right = (math.cos(angle - 1.85) * width, 0.02, math.sin(angle - 1.85) * width)
        base = len(positions)
        positions.extend([left, right, tip])
        normals.extend([(0.0, 1.0, 0.0)] * 3)
        indices.extend([base, base + 1, base + 2])
    return positions, normals, indices


def irregular_terrain(width=1.0, depth=1.0, height=0.10, bevel=0.10, seed=1):
    sides = 10
    top = [(0.0, height, 0.0)]
    bottom = []
    for i in range(sides):
        angle = math.tau * i / sides
        wobble = 1.0 + 0.09 * math.sin(seed * 0.37 + i * 1.71)
        x = math.cos(angle) * width * 0.5 * wobble
        z = math.sin(angle) * depth * 0.5 * (1.0 + 0.06 * math.cos(seed * 0.23 + i))
        top.append((x, height + 0.018 * math.sin(i * 1.9 + seed), z))
        bottom.append((x * (1.0 - bevel), 0.0, z * (1.0 - bevel)))
    positions = top + bottom
    normals = [(0.0, 1.0, 0.0)] * len(top) + [(0.0, -0.2, 0.0)] * len(bottom)
    indices = []
    for i in range(sides):
        a = 1 + i
        b = 1 + ((i + 1) % sides)
        indices += [0, a, b]
    bottom_start = len(top)
    for i in range(sides):
        a = 1 + i
        b = 1 + ((i + 1) % sides)
        c = bottom_start + i
        d = bottom_start + ((i + 1) % sides)
        indices += [a, c, d, a, d, b]
    return positions, normals, indices


def combine(parts):
    positions = []
    normals = []
    indices = []
    for part_positions, part_normals, part_indices in parts:
        offset = len(positions)
        positions.extend(part_positions)
        normals.extend(part_normals)
        indices.extend(rebase_indices(part_indices, offset))
    return positions, normals, indices


def tint_color(color, factor):
    return [max(0.0, min(1.0, channel * factor)) for channel in color]


def terrain_asset(base_color, accent_color, role_seed, feature):
    # Terrain models are authored as region-sized low-poly islands. Runtime
    # transforms still clamp scale to <= 1.0, so the authored mesh carries the
    # readable biome footprint without depending on giant unvetted transforms.
    cell_specs = [
        (0.00, 0.00, 1.22, 1.04),
        (-0.92, 0.36, 0.98, 0.82),
        (0.96, 0.32, 1.04, 0.86),
        (-0.76, -0.58, 1.08, 0.92),
        (0.72, -0.64, 1.00, 0.86),
        (-1.46, -0.04, 0.62, 0.58),
        (1.48, -0.08, 0.68, 0.60),
    ]
    primitives = []
    for index, (x, z, width, depth) in enumerate(cell_specs):
        wobble_x = 0.08 * math.sin(role_seed * 0.31 + index * 1.17)
        wobble_z = 0.08 * math.cos(role_seed * 0.41 + index * 1.33)
        cell = irregular_terrain(width, depth, 0.055 + 0.012 * (index % 3), 0.07, role_seed + index)
        shade = 0.86 + 0.07 * ((index * 37 + role_seed) % 5)
        primitives.append(
            (
                translate(cell[0], x + wobble_x, 0.0, z + wobble_z),
                cell[1],
                cell[2],
                tint_color(base_color, shade),
                f"organic-ground-{index}",
                1.0,
            )
        )
    accent_parts = []
    if feature in {"grass", "resource", "sand"}:
        for index in range(8 if feature == "resource" else 5):
            x = -1.05 + 0.52 * index + 0.08 * math.sin(index + role_seed)
            z = 0.58 * math.sin(index * 1.7 + role_seed)
            blades = leaf_blades(3 + index % 3, 0.22 + 0.05 * (index % 2), 0.34)
            accent_parts.append((translate(blades[0], x, 0.08, z), blades[1], blades[2]))
    if feature in {"stone", "hazard"}:
        accent_parts.append(
            pyramid_cluster(
                [
                    (-0.92, 0.28, 0.14, 0.30, 0.85),
                    (0.72, -0.36, 0.16, 0.34, 1.1),
                    (0.14, 0.72, 0.10, 0.25, 0.9),
                    (-0.10, -0.66, 0.09, 0.22, 1.4),
                ]
            )
        )
    if feature == "water":
        ripple = prism(0.78, 0.025, 18, 0.48)
        accent_parts.append((translate(ripple[0], 0.0, 0.09, 0.0), ripple[1], ripple[2]))
    if feature == "soil":
        ridge = irregular_terrain(2.42, 0.46, 0.02, 0.05, role_seed + 17)
        accent_parts.append((translate(ridge[0], 0.03, 0.09, -0.02), ridge[1], ridge[2]))
    if accent_parts:
        accent = combine(accent_parts)
        primitives.append((accent[0], accent[1], accent[2], accent_color, "biome-detail", 1.0))
    return primitives


def creature_asset(body_color, hurt=False):
    body = low_poly_ellipsoid(0.44, 0.30 if not hurt else 0.24, 0.35, 4, 12)
    belly = low_poly_ellipsoid(0.28, 0.08, 0.22, 3, 10)
    left_eye = low_poly_ellipsoid(0.045, 0.060, 0.045, 3, 8)
    right_eye = low_poly_ellipsoid(0.045, 0.060, 0.045, 3, 8)
    antenna_a = prism(0.025, 0.44, 5, 0.8)
    antenna_b = prism(0.025, 0.38, 5, 0.8)
    foot = low_poly_ellipsoid(0.13, 0.04, 0.07, 3, 8)
    primitives = [
        (translate(body[0], 0.0, 0.30, 0.0), body[1], body[2], body_color, "soft-body", 1.0),
        (translate(belly[0], 0.0, 0.46, -0.29), belly[1], belly[2], [0.72, 1.0, 0.95], "bio-glow", 0.78),
        (translate(left_eye[0], -0.14, 0.52, -0.32), left_eye[1], left_eye[2], [0.02, 0.07, 0.08], "left-eye", 1.0),
        (translate(right_eye[0], 0.14, 0.52, -0.32), right_eye[1], right_eye[2], [0.02, 0.07, 0.08], "right-eye", 1.0),
        (translate(antenna_a[0], -0.17, 0.54, -0.05), antenna_a[1], antenna_a[2], body_color, "antenna-a", 1.0),
        (translate(antenna_b[0], 0.19, 0.52, -0.03), antenna_b[1], antenna_b[2], body_color, "antenna-b", 1.0),
        (translate(foot[0], -0.18, 0.05, -0.10), foot[1], foot[2], [0.06, 0.52, 0.57], "foot-a", 1.0),
        (translate(foot[0], 0.18, 0.05, -0.10), foot[1], foot[2], [0.06, 0.52, 0.57], "foot-b", 1.0),
    ]
    if hurt:
        spike = pyramid_cluster([(0.0, -0.05, 0.09, 0.22, 0.8)])
        primitives.append((translate(spike[0], 0.0, 0.56, 0.05), spike[1], spike[2], [1.0, 0.45, 0.38], "pain-signal", 1.0))
    return primitives


def selection_ring_asset():
    outer = prism(0.58, 0.018, 32, 0.70)
    inner = prism(0.44, 0.020, 32, 0.70)
    halo = prism(0.68, 0.012, 32, 0.72)
    return [
        (translate(outer[0], 0.0, 0.02, 0.0), outer[1], outer[2], [0.95, 1.0, 0.28], "gold-ring", 0.82),
        (translate(inner[0], 0.0, 0.04, 0.0), inner[1], inner[2], [0.06, 0.35, 0.30], "cutout-shadow", 0.42),
        (translate(halo[0], 0.0, 0.00, 0.0), halo[1], halo[2], [0.42, 1.0, 0.82], "bio-halo", 0.32),
    ]


ASSETS = [
    ("creature-idle", "creature_idle.gltf", lambda: creature_asset([0.16, 0.82, 0.90, 1.0])),
    ("creature-hurt", "creature_hurt.gltf", lambda: creature_asset([0.52, 0.88, 0.95, 1.0], True)),
    ("selection-ring", "selection_ring.gltf", selection_ring_asset),
    (
        "food",
        "food_pod.gltf",
        lambda: [
            (*pyramid_cluster([(0.0, 0.0, 0.18, 0.34, 0.8), (-0.12, 0.10, 0.10, 0.22, 1.0)]), [0.22, 0.90, 0.25], "sprout-body", 1.0),
            (*leaf_blades(6, 0.42, 0.50), [0.52, 1.0, 0.38], "organic-leaves", 1.0),
        ],
    ),
    (
        "hazard",
        "hazard_crystal.gltf",
        lambda: [
            (*pyramid_cluster([(0.0, 0.0, 0.19, 0.72, 0.72), (-0.22, 0.10, 0.13, 0.50, 0.9), (0.22, -0.12, 0.12, 0.42, 1.1)]), [1.0, 0.12, 0.16], "red-crystal", 1.0),
            (*pyramid_cluster([(0.06, -0.04, 0.09, 0.62, 0.6)]), [1.0, 0.62, 0.34], "hot-core", 0.78),
        ],
    ),
    (
        "rock-obstacle",
        "rock_cluster.gltf",
        lambda: [
            (*pyramid_cluster([(0.0, 0.0, 0.24, 0.38, 0.9), (-0.24, 0.14, 0.18, 0.31, 1.2), (0.22, -0.12, 0.20, 0.34, 0.8)]), [0.46, 0.48, 0.45], "stone-cluster", 1.0),
            (*pyramid_cluster([(0.08, 0.02, 0.10, 0.28, 0.8)]), [0.70, 0.72, 0.66], "stone-highlight", 1.0),
        ],
    ),
    (
        "plant-prop",
        "bio_reed_prop.gltf",
        lambda: [
            (*leaf_blades(7, 0.58, 0.55), [0.34, 0.95, 0.30], "reed-leaves", 1.0),
            (*low_poly_ellipsoid(0.10, 0.08, 0.09, 3, 8), [0.78, 0.20, 0.62], "tiny-bloom", 1.0),
        ],
    ),
    ("terrain-grass-island", "terrain_grass_island.gltf", lambda: terrain_asset([0.24, 0.57, 0.22], [0.63, 0.86, 0.34], 11, "grass")),
    ("terrain-soil-island", "terrain_soil_island.gltf", lambda: terrain_asset([0.55, 0.36, 0.18], [0.75, 0.54, 0.25], 17, "soil")),
    ("terrain-resource-grove", "terrain_resource_grove.gltf", lambda: terrain_asset([0.18, 0.52, 0.20], [0.30, 0.96, 0.40], 23, "resource")),
    ("terrain-hazard-pressure", "terrain_hazard_pressure.gltf", lambda: terrain_asset([0.56, 0.18, 0.14], [1.0, 0.22, 0.20], 31, "hazard")),
    ("terrain-stone-island", "terrain_stone_island.gltf", lambda: terrain_asset([0.39, 0.42, 0.40], [0.68, 0.70, 0.63], 43, "stone")),
    ("terrain-water-cell", "terrain_water_cell.gltf", lambda: terrain_asset([0.10, 0.42, 0.56], [0.36, 0.84, 1.0], 53, "water")),
    ("terrain-sand-island", "terrain_sand_island.gltf", lambda: terrain_asset([0.70, 0.56, 0.30], [0.96, 0.82, 0.42], 61, "sand")),
    ("fog-of-war-cell", "fog_of_war_cell.gltf", lambda: [(*irregular_terrain(2.80, 2.42, 0.04, 0.09, 73), [0.03, 0.05, 0.05], "soft-fog", 0.18)]),
]


def parse_args():
    parser = argparse.ArgumentParser(
        description="Generate seed glTF assets for later Blender normalization."
    )
    parser.add_argument(
        "--output-root",
        default=str(DEFAULT_ROOT),
        help="Directory for seed glTF output. Defaults to an untracked target path.",
    )
    parser.add_argument(
        "--manifest",
        default=None,
        help="Manifest output path. Defaults to <output-root>/true_25d_seed_manifest.json.",
    )
    parser.add_argument(
        "--ground-tile",
        default=None,
        help="Ground tile output path. Defaults to <output-root>/ground_tile_repeat.png.",
    )
    parser.add_argument(
        "--overwrite-active",
        action="store_true",
        help=(
            "Allow writing into committed active asset paths. This is seed-only "
            "and must be followed by scripts/normalize_true25d_gltf_assets.ps1."
        ),
    )
    return parser.parse_args()


def reject_active_output_without_override(root: Path, manifest: Path, ground_tile: Path, allowed: bool):
    targets = [root.resolve(), manifest.resolve(), ground_tile.resolve()]
    active_targets = [ACTIVE_ROOT.resolve(), ACTIVE_MANIFEST.resolve(), ACTIVE_GROUND_TILE.resolve()]
    if allowed:
        return
    for target in targets:
        for active in active_targets:
            if target == active or active in target.parents:
                raise SystemExit(
                    "Refusing to overwrite active true 2.5D product assets. "
                    "Use --overwrite-active only for intentional seed regeneration "
                    "followed by Blender normalization."
                )


def main():
    args = parse_args()
    root = Path(args.output_root)
    manifest = Path(args.manifest) if args.manifest else root / "true_25d_seed_manifest.json"
    ground_tile = Path(args.ground_tile) if args.ground_tile else root / "ground_tile_repeat.png"
    reject_active_output_without_override(root, manifest, ground_tile, args.overwrite_active)

    root.mkdir(parents=True, exist_ok=True)
    if not ground_tile.exists():
        write_repeat_ground_tile(ground_tile)
    entries = []
    for role, filename, builder in ASSETS:
        primitives = builder()
        path = root / filename
        make_gltf(path, filename[:-5], primitives)
        vertex_count = sum(len(primitive[0]) for primitive in primitives)
        index_count = sum(len(primitive[2]) for primitive in primitives)
        entries.append(
            {
                "role": role,
                "relative_path": path.as_posix(),
                "node_count": 1,
                "mesh_count": 1,
                "material_count": len(primitives),
                "vertex_count": vertex_count,
                "index_count": index_count,
                "file_size_bytes": path.stat().st_size,
            }
        )
    manifest.write_text(
        json.dumps(
            {
                "schema": SCHEMA,
                "schema_version": 1,
                "pack_id": "true-25d-alpha-v1",
                "art_direction": ART_DIRECTION,
                "seed_only": True,
                "requires_blender_normalization": True,
                "camera": {
                    "projection": "orthographic",
                    "yaw_degrees": 0.0,
                    "pitch_degrees": -45.0,
                    "rotation_locked": True,
                },
                "shader_stack": {
                    "quantized_toon_bands": 4,
                    "sobel_outline_contract": True,
                    "low_resolution_pixel_step_filter": True,
                    "runtime_shader_contract_only": True,
                },
                "entries": entries,
            },
            indent=2,
        ),
        encoding="utf-8",
    )
    print(f"generated {len(entries)} true 2.5D seed glTF assets under {root}")
    print("active product assets still require Blender-normalized GLB manifest output")


if __name__ == "__main__":
    main()
