#!/usr/bin/env python3
"""Generate deterministic low-poly biped part packs for the eight launch families.

The runtime still consumes the normal named-part OBJ contract. This authoring
step replaces malformed launch-family slices while leaving the generic source
mesh slicer available for future catalog additions.
"""

from __future__ import annotations

import argparse
import json
import math
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from PIL import Image, ImageDraw


SLOTS = (
    "head",
    "torso",
    "left_arm",
    "right_arm",
    "left_leg",
    "right_leg",
    "tail_back",
)


@dataclass(frozen=True)
class FamilyProfile:
    label: str
    head: tuple[float, float, float]
    muzzle: tuple[float, float, float]
    ear_kind: str
    crest_kind: str
    torso: tuple[float, float, float]
    torso_kind: str
    shoulder_mantle: float
    hip_width: float
    arm_length: float
    arm_radius: float
    arm_kind: str
    hand_size: tuple[float, float, float]
    leg_length: float
    leg_radius: float
    leg_kind: str
    stance: float
    foot_size: tuple[float, float, float]
    tail_kind: str
    tail_length: float


PROFILES = (
    FamilyProfile(
        "colobus", (0.53, 0.45, 0.52), (0.27, 0.19, 0.17), "round", "crown",
        (0.58, 0.40, 0.76), "ape", 0.15, 0.43, 0.70, 0.105, "furry",
        (0.15, 0.12, 0.11), 0.58, 0.115, "plantigrade", 0.025,
        (0.25, 0.30, 0.13), "plume", 0.55,
    ),
    FamilyProfile(
        "gecko", (0.50, 0.47, 0.47), (0.30, 0.22, 0.15), "fin", "ridge",
        (0.48, 0.38, 0.72), "tapered", 0.07, 0.38, 0.61, 0.090, "webbed",
        (0.17, 0.09, 0.10), 0.64, 0.100, "webbed", 0.040,
        (0.24, 0.32, 0.12), "tapered", 0.76,
    ),
    FamilyProfile(
        "herring", (0.48, 0.44, 0.46), (0.28, 0.22, 0.14), "fin", "fan",
        (0.54, 0.42, 0.68), "barrel", 0.11, 0.42, 0.58, 0.095, "fin",
        (0.16, 0.09, 0.09), 0.57, 0.115, "webbed", 0.055,
        (0.27, 0.34, 0.13), "dorsal", 0.38,
    ),
    FamilyProfile(
        "inkfish", (0.60, 0.49, 0.56), (0.30, 0.21, 0.18), "droop", "crown",
        (0.60, 0.44, 0.72), "pear", 0.08, 0.46, 0.72, 0.100, "soft",
        (0.16, 0.13, 0.13), 0.53, 0.120, "soft", 0.030,
        (0.28, 0.34, 0.14), "soft", 0.45,
    ),
    FamilyProfile(
        "muskrat", (0.56, 0.48, 0.50), (0.32, 0.22, 0.19), "round", "none",
        (0.63, 0.46, 0.74), "barrel", 0.16, 0.50, 0.58, 0.120, "furry",
        (0.16, 0.14, 0.13), 0.56, 0.135, "stocky", 0.040,
        (0.30, 0.36, 0.15), "tapered", 0.68,
    ),
    FamilyProfile(
        "pudu", (0.46, 0.42, 0.52), (0.27, 0.20, 0.18), "tall", "brow",
        (0.48, 0.36, 0.78), "slim", 0.06, 0.38, 0.60, 0.090, "furry",
        (0.12, 0.09, 0.10), 0.72, 0.095, "digitigrade", 0.020,
        (0.22, 0.30, 0.12), "nub", 0.30,
    ),
    FamilyProfile(
        "sparrow", (0.52, 0.45, 0.48), (0.25, 0.23, 0.15), "fan", "swept",
        (0.58, 0.43, 0.70), "wing", 0.14, 0.40, 0.60, 0.105, "wing",
        (0.15, 0.08, 0.10), 0.54, 0.105, "talon", 0.040,
        (0.24, 0.31, 0.12), "plume", 0.48,
    ),
    FamilyProfile(
        "taipan", (0.49, 0.45, 0.54), (0.31, 0.20, 0.17), "round", "ridge",
        (0.46, 0.35, 0.84), "tapered", 0.08, 0.36, 0.72, 0.082, "webbed",
        (0.13, 0.08, 0.10), 0.68, 0.090, "webbed", 0.025,
        (0.22, 0.31, 0.11), "tapered", 0.84,
    ),
)


@dataclass
class Mesh:
    vertices: list[tuple[float, float, float]]
    uvs: list[tuple[float, float]]
    normals: list[tuple[float, float, float]]
    faces: list[tuple[int, int, int]]

    @classmethod
    def empty(cls) -> "Mesh":
        return cls([], [], [], [])

    def append(self, other: "Mesh") -> None:
        offset = len(self.vertices)
        self.vertices.extend(other.vertices)
        self.uvs.extend(other.uvs)
        self.normals.extend(other.normals)
        self.faces.extend(tuple(index + offset for index in face) for face in other.faces)


def normalize(value: tuple[float, float, float]) -> tuple[float, float, float]:
    length = math.sqrt(sum(axis * axis for axis in value))
    if length <= 1.0e-9:
        return (0.0, 0.0, 1.0)
    return tuple(axis / length for axis in value)


def ellipsoid(
    center: tuple[float, float, float],
    radii: tuple[float, float, float],
    segments: int,
    rings: int,
) -> Mesh:
    mesh = Mesh.empty()
    for ring in range(rings + 1):
        v = ring / rings
        phi = math.pi * v
        sin_phi = math.sin(phi)
        cos_phi = math.cos(phi)
        for segment in range(segments + 1):
            u = segment / segments
            theta = math.tau * u
            unit = (math.cos(theta) * sin_phi, math.sin(theta) * sin_phi, cos_phi)
            mesh.vertices.append(
                tuple(center[axis] + radii[axis] * unit[axis] for axis in range(3))
            )
            mesh.uvs.append((u, 1.0 - v))
            mesh.normals.append(
                normalize(tuple(unit[axis] / max(radii[axis], 1.0e-6) for axis in range(3)))
            )
    stride = segments + 1
    for ring in range(rings):
        for segment in range(segments):
            a = ring * stride + segment
            b = a + 1
            c = a + stride
            d = c + 1
            if ring > 0:
                mesh.faces.append((a, c, b))
            if ring < rings - 1:
                mesh.faces.append((b, c, d))
    return mesh


def add_ellipsoid(
    target: Mesh,
    center: tuple[float, float, float],
    radii: tuple[float, float, float],
    segments: int,
    rings: int,
) -> None:
    target.append(ellipsoid(center, radii, segments, rings))


def head_mesh(profile: FamilyProfile, segments: int, rings: int) -> Mesh:
    mesh = Mesh.empty()
    width, depth, height = profile.head
    add_ellipsoid(mesh, (0.0, 0.0, height * 0.48), (width / 2, depth / 2, height / 2), segments, rings)
    muzzle_w, muzzle_d, muzzle_h = profile.muzzle
    add_ellipsoid(
        mesh,
        (0.0, -depth * 0.43, height * 0.34),
        (muzzle_w / 2, muzzle_d / 2, muzzle_h / 2),
        max(6, segments - 2),
        max(4, rings - 2),
    )
    cheek_radius = 0.105 if profile.ear_kind in ("round", "droop", "tall") else 0.082
    for side in (-1.0, 1.0):
        add_ellipsoid(
            mesh,
            (side * width * 0.21, -depth * 0.31, height * 0.35),
            (cheek_radius, cheek_radius * 0.72, cheek_radius * 0.92),
            max(6, segments - 4),
            max(4, rings - 3),
        )
    ear_x = width * (0.43 if profile.ear_kind == "droop" else 0.47)
    if profile.ear_kind == "tall":
        ear_radii = (0.075, 0.065, 0.18)
        ear_z = height * 0.88
    elif profile.ear_kind in ("fin", "fan"):
        ear_radii = (0.12, 0.045, 0.14)
        ear_z = height * 0.65
    elif profile.ear_kind == "droop":
        ear_radii = (0.10, 0.07, 0.10)
        ear_z = height * 0.62
    else:
        ear_radii = (0.105, 0.075, 0.12)
        ear_z = height * 0.70
    for side in (-1.0, 1.0):
        add_ellipsoid(mesh, (side * ear_x, 0.01, ear_z), ear_radii, max(6, segments - 3), max(4, rings - 2))
    crest_components = {
        "none": (),
        "brow": ((0.0, 0.00, height * 0.94, 0.12, 0.07, 0.09),),
        "crown": ((-0.10, 0.01, height * 1.02, 0.07, 0.06, 0.12), (0.10, 0.01, height * 1.02, 0.07, 0.06, 0.12)),
        "fan": ((0.0, 0.03, height * 1.02, 0.19, 0.045, 0.13),),
        "ridge": ((0.0, 0.08, height * 1.02, 0.075, 0.15, 0.11),),
        "swept": ((0.0, 0.10, height * 1.00, 0.12, 0.18, 0.10),),
    }[profile.crest_kind]
    for x, y, z, rx, ry, rz in crest_components:
        add_ellipsoid(mesh, (x, y, z), (rx, ry, rz), max(6, segments - 3), max(4, rings - 2))
    return mesh


def torso_mesh(profile: FamilyProfile, segments: int, rings: int) -> Mesh:
    mesh = Mesh.empty()
    width, depth, height = profile.torso
    chest_width, waist_width, chest_depth = {
        "ape": (1.00, 0.61, 1.00),
        "barrel": (0.98, 0.84, 1.02),
        "pear": (0.82, 0.91, 1.00),
        "slim": (0.78, 0.62, 0.90),
        "wing": (1.04, 0.70, 0.88),
        "tapered": (0.90, 0.64, 0.94),
    }[profile.torso_kind]
    add_ellipsoid(
        mesh,
        (0.0, 0.0, height * 0.17),
        (width * 0.50 * chest_width, depth * 0.50 * chest_depth, height * 0.29),
        segments,
        rings,
    )
    add_ellipsoid(
        mesh,
        (0.0, 0.01, -height * 0.10),
        (width * 0.50 * waist_width, depth * 0.40, height * 0.25),
        max(6, segments - 2),
        max(4, rings - 2),
    )
    add_ellipsoid(
        mesh,
        (0.0, 0.015, -height * 0.30),
        (profile.hip_width / 2, depth * 0.43, height * 0.24),
        max(6, segments - 2),
        max(4, rings - 2),
    )
    if profile.shoulder_mantle > 0.0:
        for side in (-1.0, 1.0):
            add_ellipsoid(
                mesh,
                (side * width * 0.42, 0.01, height * 0.24),
                (profile.shoulder_mantle, depth * 0.30, height * 0.15),
                max(6, segments - 4),
                max(4, rings - 3),
            )
    return mesh


def arm_mesh(profile: FamilyProfile, side: float, segments: int, rings: int) -> Mesh:
    mesh = Mesh.empty()
    length = profile.arm_length
    radius = profile.arm_radius
    lateral = 0.025 * side
    depth = -0.025 if profile.arm_kind in ("webbed", "fin", "wing") else 0.0
    upper_radii = (radius * (1.25 if profile.arm_kind == "wing" else 1.0), radius, length * 0.27)
    lower_radii = (radius * 0.86, radius * (0.72 if profile.arm_kind == "fin" else 0.82), length * 0.29)
    add_ellipsoid(mesh, (lateral, depth, -length * 0.25), upper_radii, max(6, segments - 2), max(4, rings - 2))
    add_ellipsoid(mesh, (-lateral * 0.35, depth - 0.015, -length * 0.70), lower_radii, max(6, segments - 2), max(4, rings - 2))
    if profile.arm_kind in ("fin", "webbed", "wing"):
        add_ellipsoid(
            mesh,
            (side * radius * 0.35, depth + 0.015, -length * 0.48),
            (radius * 1.35, radius * 0.34, length * 0.24),
            max(6, segments - 3),
            max(4, rings - 2),
        )
    hand_w, hand_d, hand_h = profile.hand_size
    add_ellipsoid(
        mesh,
        (side * 0.015, depth - 0.035, -length * 0.96),
        (hand_w / 2, hand_d / 2, hand_h / 2),
        max(6, segments - 4),
        max(4, rings - 3),
    )
    return mesh


def leg_mesh(profile: FamilyProfile, side: float, segments: int, rings: int) -> Mesh:
    mesh = Mesh.empty()
    length = profile.leg_length
    radius = profile.leg_radius
    upper_width, lower_width, lower_depth, ankle_push = {
        "plantigrade": (1.08, 0.82, 0.82, -0.015),
        "webbed": (1.00, 0.72, 0.66, -0.035),
        "soft": (1.16, 0.94, 0.90, 0.0),
        "stocky": (1.28, 1.02, 0.98, -0.01),
        "digitigrade": (0.92, 0.62, 0.68, 0.055),
        "talon": (0.96, 0.67, 0.62, 0.035),
    }[profile.leg_kind]
    add_ellipsoid(
        mesh,
        (profile.stance * side, 0.0, -length * 0.24),
        (radius * upper_width, radius, length * 0.27),
        max(6, segments - 2),
        max(4, rings - 2),
    )
    add_ellipsoid(
        mesh,
        (-profile.stance * 0.35 * side, ankle_push, -length * 0.67),
        (radius * lower_width, radius * lower_depth, length * 0.28),
        max(6, segments - 2),
        max(4, rings - 2),
    )
    foot_w, foot_d, foot_h = profile.foot_size
    if profile.leg_kind == "webbed":
        foot_w *= 1.14
    elif profile.leg_kind == "talon":
        foot_d *= 1.12
    add_ellipsoid(
        mesh,
        (0.0, -foot_d * 0.20, -length + foot_h * 0.44),
        (foot_w / 2, foot_d / 2, foot_h / 2),
        max(6, segments - 2),
        max(4, rings - 2),
    )
    return mesh


def tail_mesh(profile: FamilyProfile, segments: int, rings: int) -> Mesh:
    mesh = Mesh.empty()
    length = profile.tail_length
    if profile.tail_kind == "nub":
        add_ellipsoid(mesh, (0.0, 0.13, 0.02), (0.10, 0.16, 0.10), max(6, segments - 3), max(4, rings - 2))
        return mesh
    count = 2 if profile.tail_kind == "dorsal" else 3
    for index in range(count):
        t = (index + 0.5) / count
        radius = (0.105 if profile.tail_kind in ("plume", "soft") else 0.075) * (1.0 - t * 0.48)
        vertical = 0.10 - t * (0.24 if profile.tail_kind != "dorsal" else 0.05)
        add_ellipsoid(
            mesh,
            (0.0, length * t, vertical),
            (radius * (1.45 if profile.tail_kind == "plume" else 1.0), length / count * 0.62, radius),
            max(6, segments - 3),
            max(4, rings - 2),
        )
    return mesh


def family_parts(profile: FamilyProfile, segments: int, rings: int) -> dict[str, Mesh]:
    return {
        "head": head_mesh(profile, segments, rings),
        "torso": torso_mesh(profile, segments, rings),
        "left_arm": arm_mesh(profile, -1.0, segments, rings),
        "right_arm": arm_mesh(profile, 1.0, segments, rings),
        "left_leg": leg_mesh(profile, -1.0, segments, rings),
        "right_leg": leg_mesh(profile, 1.0, segments, rings),
        "tail_back": tail_mesh(profile, segments, rings),
    }


def emit_obj(parts: dict[str, Mesh]) -> str:
    lines = ["# A-Life deterministic canonical biped part pack v2"]
    vertex_base = 1
    for slot in SLOTS:
        mesh = parts[slot]
        lines.append(f"o part_{slot}")
        for x, y, z in mesh.vertices:
            lines.append(f"v {x:.6f} {y:.6f} {z:.6f}")
        for u, v in mesh.uvs:
            lines.append(f"vt {u:.6f} {v:.6f}")
        for x, y, z in mesh.normals:
            lines.append(f"vn {x:.6f} {y:.6f} {z:.6f}")
        for a, b, c in mesh.faces:
            indices = (a + vertex_base, b + vertex_base, c + vertex_base)
            lines.append("f " + " ".join(f"{i}/{i}/{i}" for i in indices))
        vertex_base += len(mesh.vertices)
    return "\n".join(lines) + "\n"


def generate(output_root: Path) -> Iterable[Path]:
    lods = {
        "full": (14, 8),
        "compact": (10, 6),
        "impostor": (8, 4),
    }
    socket_slots = {
        "neck": "head",
        "left-shoulder": "left_arm",
        "right-shoulder": "right_arm",
        "left-hip": "left_leg",
        "right-hip": "right_leg",
        "tail-base": "tail_back",
    }
    for family_id, profile in enumerate(PROFILES):
        for lod, (segments, rings) in lods.items():
            output = output_root / f"{profile.label}_{lod}_parts.obj"
            output.write_text(emit_obj(family_parts(profile, segments, rings)), encoding="ascii")
            yield output
            sockets = {
                name: {
                    "translation": list(ASSEMBLY_TRANSLATIONS[slot]),
                    "rotation_xyzw": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0],
                }
                for name, slot in socket_slots.items()
            }
            socket_output = output_root / f"{profile.label}_{lod}_sockets.json"
            socket_output.write_text(
                json.dumps(
                    {
                        "schema": "alife.creature_part_sockets.v1",
                        "schema_version": 1,
                        "family_id": family_id,
                        "lod": lod,
                        "sockets": sockets,
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="ascii",
            )
            yield socket_output


def texture_value(label: str, x: int, y: int, noise: float) -> tuple[int, int, int, int]:
    u = x / 256.0
    v = y / 256.0
    tau = math.tau
    patterns = {
        "colobus": 0.48 * math.sin(tau * (u * 5.0 + 0.16 * math.sin(tau * v * 3.0))),
        "gecko": 0.34 * math.sin(tau * (u * 4.0 + 0.24 * math.sin(tau * v * 3.0)))
        + 0.24 * math.cos(tau * (v * 5.0 + 0.18 * math.sin(tau * u * 2.0))),
        "herring": 0.52 * math.sin(tau * (v * 8.0 + 0.22 * math.sin(tau * u * 3.0))),
        "inkfish": 0.28 * math.sin(tau * (u * 3.0 + 0.30 * math.sin(tau * v * 2.0)))
        + 0.28 * math.cos(tau * (v * 4.0 + 0.22 * math.sin(tau * u * 3.0))),
        "muskrat": 0.46 * math.sin(tau * (u * 13.0 + v * 2.0 + 0.08 * math.sin(tau * v * 5.0))),
        "pudu": 0.32 * math.cos(tau * (u * 4.0 + 0.20 * math.sin(tau * v * 3.0)))
        + 0.30 * math.cos(tau * (v * 5.0 + 0.16 * math.sin(tau * u * 4.0))),
        "sparrow": 0.54 * math.cos(tau * (abs((u * 8.0) % 2.0 - 1.0) + v * 7.0)),
        "taipan": 0.60 * math.cos(tau * u * 8.0) * math.cos(tau * v * 8.0),
    }
    pattern = patterns[label]
    grain = (noise - 0.5) * 0.08
    mask = 1.0 / (1.0 + math.exp(-pattern * 8.0))
    value = max(0.28, min(1.0, 0.34 + mask * 0.64 + grain))
    pigment = {
        "colobus": (1.00, 0.88, 0.68),
        "gecko": (0.72, 1.00, 0.84),
        "herring": (0.66, 0.88, 1.00),
        "inkfish": (0.86, 0.67, 1.00),
        "muskrat": (1.00, 0.76, 0.52),
        "pudu": (1.00, 0.87, 0.64),
        "sparrow": (0.82, 0.78, 1.00),
        "taipan": (0.66, 1.00, 0.78),
    }[label]
    channels = tuple(round(255.0 * max(0.0, min(1.0, value * channel))) for channel in pigment)
    return channels[0], channels[1], channels[2], 255


def generate_textures(texture_root: Path) -> Iterable[Path]:
    texture_root.mkdir(parents=True, exist_ok=True)
    for family_id, profile in enumerate(PROFILES):
        rng = random.Random(0xA11F_2026 + family_id * 7919)
        pixels = [
            texture_value(profile.label, x, y, rng.random())
            for y in range(256)
            for x in range(256)
        ]
        image = Image.new("RGBA", (256, 256))
        image.putdata(pixels)
        output = texture_root / f"T_{profile.label.capitalize()}.png"
        image.save(output, format="PNG", optimize=True)
        yield output


ASSEMBLY_TRANSLATIONS = {
    "head": (0.0, 0.0, 0.43),
    "torso": (0.0, 0.0, 0.0),
    "left_arm": (-0.27, 0.0, 0.27),
    "right_arm": (0.27, 0.0, 0.27),
    "left_leg": (-0.15, 0.0, -0.10),
    "right_leg": (0.15, 0.0, -0.10),
    "tail_back": (0.0, 0.22, -0.18),
}


def render_preview(profile: FamilyProfile, output: Path) -> None:
    parts = family_parts(profile, 12, 7)
    camera_triangles = []
    assembled_vertices = []
    base_colors = {
        "head": (185, 137, 92),
        "torso": (114, 82, 59),
        "left_arm": (98, 68, 49),
        "right_arm": (106, 73, 52),
        "left_leg": (83, 58, 44),
        "right_leg": (90, 63, 46),
        "tail_back": (132, 89, 60),
    }
    for slot, mesh in parts.items():
        translation = ASSEMBLY_TRANSLATIONS[slot]
        vertices = [
            tuple(vertex[axis] + translation[axis] for axis in range(3))
            for vertex in mesh.vertices
        ]
        assembled_vertices.extend(vertices)
        for face in mesh.faces:
            triangle = tuple(vertices[index] for index in face)
            depth = sum(point[1] - point[0] * 0.18 for point in triangle) / 3.0
            a, b, c = triangle
            ab = tuple(b[axis] - a[axis] for axis in range(3))
            ac = tuple(c[axis] - a[axis] for axis in range(3))
            normal = normalize(
                (
                    ab[1] * ac[2] - ab[2] * ac[1],
                    ab[2] * ac[0] - ab[0] * ac[2],
                    ab[0] * ac[1] - ab[1] * ac[0],
                )
            )
            light = max(0.52, min(1.16, 0.78 + normal[0] * -0.18 + normal[1] * -0.24 + normal[2] * 0.26))
            color = tuple(max(0, min(255, round(channel * light))) for channel in base_colors[slot])
            camera_triangles.append((depth, triangle, color))

    projected = [
        (point[0] + point[1] * 0.34, -point[2] + point[1] * 0.16)
        for point in assembled_vertices
    ]
    min_x = min(point[0] for point in projected)
    max_x = max(point[0] for point in projected)
    min_y = min(point[1] for point in projected)
    max_y = max(point[1] for point in projected)
    canvas_size = 1024
    margin = 90
    scale = min(
        (canvas_size - margin * 2) / max(max_x - min_x, 1.0e-6),
        (canvas_size - margin * 2) / max(max_y - min_y, 1.0e-6),
    )

    image = Image.new("RGB", (canvas_size, canvas_size), (25, 29, 30))
    draw = ImageDraw.Draw(image)
    for _, triangle, color in sorted(camera_triangles, key=lambda item: item[0], reverse=True):
        polygon = [
            (
                margin + (point[0] + point[1] * 0.34 - min_x) * scale,
                margin + (-point[2] + point[1] * 0.16 - min_y) * scale,
            )
            for point in triangle
        ]
        draw.polygon(polygon, fill=color)
    draw.text((24, 22), profile.label, fill=(244, 241, 230))
    output.parent.mkdir(parents=True, exist_ok=True)
    image.resize((512, 512), Image.Resampling.LANCZOS).save(output, format="PNG")


def render_contact_sheet(images: list[Path], output: Path, columns: int = 4) -> None:
    tile_size = 512
    rows = math.ceil(len(images) / columns)
    sheet = Image.new("RGB", (tile_size * columns, tile_size * rows), (25, 29, 30))
    for index, path in enumerate(images):
        with Image.open(path) as tile:
            normalized = tile.convert("RGB").resize(
                (tile_size, tile_size), Image.Resampling.NEAREST
            )
            sheet.paste(normalized, ((index % columns) * tile_size, (index // columns) * tile_size))
    output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output, format="PNG")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-root",
        type=Path,
        default=Path("crates/alife_game_app/assets/production_voxel_v1/creature_parts/generated"),
    )
    parser.add_argument(
        "--texture-root",
        type=Path,
        default=Path("crates/alife_game_app/assets/production_voxel_v1/models"),
    )
    parser.add_argument("--preview-root", type=Path)
    args = parser.parse_args()
    args.output_root.mkdir(parents=True, exist_ok=True)
    for output in generate(args.output_root):
        print(output)
    texture_outputs = list(generate_textures(args.texture_root))
    for output in texture_outputs:
        print(output)
    if args.preview_root is not None:
        previews = []
        for profile in PROFILES:
            output = args.preview_root / f"{profile.label}_canonical.png"
            render_preview(profile, output)
            previews.append(output)
            print(output)
        contact_sheet = args.preview_root.parent / f"{args.preview_root.name}_contact_sheet.png"
        render_contact_sheet(previews, contact_sheet)
        print(contact_sheet)
        texture_sheet = args.preview_root.parent / f"{args.preview_root.name}_textures.png"
        render_contact_sheet(texture_outputs, texture_sheet)
        print(texture_sheet)


if __name__ == "__main__":
    main()
