#!/usr/bin/env python3
"""Generate the committed A-Life alpha art v1 PNG pack.

The pack is deterministic, original, and generated with Python's standard
library. The direction is "production-alpha": small enough for the repo, but
with layered silhouettes, soft shadows, and terrain detail that is closer to a
real game art pass than programmer rectangles.
"""

from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates" / "alife_game_app" / "assets" / "alpha_art_v1"
SIZE = 128
SCALE = 2
CANVAS = SIZE * SCALE


def rgba(hex_color: str, alpha: int = 255) -> tuple[int, int, int, int]:
    hex_color = hex_color.lstrip("#")
    return (
        int(hex_color[0:2], 16),
        int(hex_color[2:4], 16),
        int(hex_color[4:6], 16),
        alpha,
    )


def blend(dst: tuple[int, int, int, int], src: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    sa = src[3] / 255.0
    if sa <= 0.0:
        return dst
    da = dst[3] / 255.0
    out_a = sa + da * (1.0 - sa)
    if out_a <= 0.0:
        return (0, 0, 0, 0)
    r = (src[0] * sa + dst[0] * da * (1.0 - sa)) / out_a
    g = (src[1] * sa + dst[1] * da * (1.0 - sa)) / out_a
    b = (src[2] * sa + dst[2] * da * (1.0 - sa)) / out_a
    return (round(r), round(g), round(b), round(out_a * 255.0))


class Image:
    def __init__(self, transparent: bool = True, bg: tuple[int, int, int, int] | None = None):
        color = (0, 0, 0, 0) if transparent else (28, 58, 35, 255)
        if bg is not None:
            color = bg
        self.pixels = [[color for _ in range(CANVAS)] for _ in range(CANVAS)]

    def set(self, x: int, y: int, color: tuple[int, int, int, int]) -> None:
        if 0 <= x < CANVAS and 0 <= y < CANVAS:
            self.pixels[y][x] = blend(self.pixels[y][x], color)

    def ellipse(self, cx: float, cy: float, rx: float, ry: float, color: tuple[int, int, int, int]) -> None:
        cx *= SCALE
        cy *= SCALE
        rx *= SCALE
        ry *= SCALE
        for y in range(max(0, int(cy - ry - 2)), min(CANVAS, int(cy + ry + 3))):
            for x in range(max(0, int(cx - rx - 2)), min(CANVAS, int(cx + rx + 3))):
                dx = (x + 0.5 - cx) / max(rx, 1.0)
                dy = (y + 0.5 - cy) / max(ry, 1.0)
                if dx * dx + dy * dy <= 1.0:
                    self.set(x, y, color)

    def polygon(self, points: list[tuple[float, float]], color: tuple[int, int, int, int]) -> None:
        pts = [(x * SCALE, y * SCALE) for x, y in points]
        min_x = max(0, int(min(x for x, _ in pts)) - 2)
        max_x = min(CANVAS, int(max(x for x, _ in pts)) + 3)
        min_y = max(0, int(min(y for _, y in pts)) - 2)
        max_y = min(CANVAS, int(max(y for _, y in pts)) + 3)
        for y in range(min_y, max_y):
            for x in range(min_x, max_x):
                inside = False
                j = len(pts) - 1
                for i in range(len(pts)):
                    xi, yi = pts[i]
                    xj, yj = pts[j]
                    if ((yi > y) != (yj > y)) and (
                        x < (xj - xi) * (y - yi) / ((yj - yi) or 1.0) + xi
                    ):
                        inside = not inside
                    j = i
                if inside:
                    self.set(x, y, color)

    def line(self, x1: float, y1: float, x2: float, y2: float, width: float, color: tuple[int, int, int, int]) -> None:
        x1 *= SCALE
        y1 *= SCALE
        x2 *= SCALE
        y2 *= SCALE
        width *= SCALE
        dx = x2 - x1
        dy = y2 - y1
        length_sq = dx * dx + dy * dy
        for y in range(max(0, int(min(y1, y2) - width - 2)), min(CANVAS, int(max(y1, y2) + width + 3))):
            for x in range(max(0, int(min(x1, x2) - width - 2)), min(CANVAS, int(max(x1, x2) + width + 3))):
                if length_sq == 0:
                    dist = math.hypot(x - x1, y - y1)
                else:
                    t = max(0.0, min(1.0, ((x - x1) * dx + (y - y1) * dy) / length_sq))
                    px = x1 + t * dx
                    py = y1 + t * dy
                    dist = math.hypot(x - px, y - py)
                if dist <= width * 0.5:
                    self.set(x, y, color)

    def shadow(self, cx: float, cy: float, rx: float, ry: float, alpha: int = 90) -> None:
        self.ellipse(cx, cy, rx, ry, (0, 0, 0, alpha))

    def downsample(self) -> bytes:
        out = bytearray()
        for y in range(SIZE):
            out.append(0)
            for x in range(SIZE):
                acc = [0, 0, 0, 0]
                for yy in range(SCALE):
                    for xx in range(SCALE):
                        p = self.pixels[y * SCALE + yy][x * SCALE + xx]
                        for i in range(4):
                            acc[i] += p[i]
                samples = SCALE * SCALE
                out.extend(round(v / samples) for v in acc)
        return bytes(out)

    def save(self, path: Path) -> None:
        raw = self.downsample()

        def chunk(kind: bytes, data: bytes) -> bytes:
            return (
                struct.pack(">I", len(data))
                + kind
                + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
            )

        png = (
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b"")
        )
        path.write_bytes(png)


def deterministic_dot(img: Image, seed: int, palette: list[tuple[int, int, int, int]], count: int, min_r: float = 1.0, max_r: float = 4.0) -> None:
    value = seed
    for _ in range(count):
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
        x = 4 + (value % (SIZE - 8))
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
        y = 4 + (value % (SIZE - 8))
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
        r = min_r + (value % 100) / 100.0 * (max_r - min_r)
        img.ellipse(x, y, r, r * (0.55 + ((value >> 8) % 30) / 100.0), palette[value % len(palette)])


def leaf(img: Image, cx: float, cy: float, angle: float, length: float, width: float, color: str, alpha: int = 235) -> None:
    dx = math.cos(angle) * length * 0.5
    dy = math.sin(angle) * length * 0.5
    px = -math.sin(angle) * width * 0.5
    py = math.cos(angle) * width * 0.5
    img.polygon(
        [
            (cx - dx, cy - dy),
            (cx + px, cy + py),
            (cx + dx, cy + dy),
            (cx - px, cy - py),
        ],
        rgba(color, alpha),
    )
    img.line(cx - dx * 0.65, cy - dy * 0.65, cx + dx * 0.65, cy + dy * 0.65, 1.1, rgba("1d5b2b", 120))


def creature_body(img: Image, base: str, shade: str, accent: str, hurt: bool = False) -> None:
    img.shadow(64, 92, 31, 9, 95)
    img.ellipse(62, 68, 31, 24, rgba(shade, 245))
    img.ellipse(57, 62, 30, 23, rgba(base, 255))
    img.ellipse(43, 57, 14, 13, rgba("a8f4ee" if not hurt else "f2aaa5", 230))
    img.ellipse(74, 56, 13, 12, rgba(accent, 215))
    for x, y in [(38, 86), (78, 88), (34, 77), (88, 76)]:
        img.ellipse(x, y, 7, 5, rgba(shade, 235))
    img.line(77, 42, 93, 26, 3.4, rgba(shade, 230))
    img.line(45, 42, 33, 25, 3.2, rgba(shade, 230))
    img.ellipse(95, 24, 4.5, 4.5, rgba("b4fbff", 235))
    img.ellipse(31, 23, 4.2, 4.2, rgba("b4fbff", 235))
    if hurt:
        img.ellipse(58, 64, 34, 25, rgba("ef6657", 95))
        for x in [45, 72]:
            img.line(x - 5, 51, x + 5, 61, 2.4, rgba("fff2df", 255))
            img.line(x + 5, 51, x - 5, 61, 2.4, rgba("fff2df", 255))
        img.polygon([(92, 34), (101, 62), (83, 59)], rgba("ff3834", 220))
    else:
        for x in [45, 72]:
            img.ellipse(x, 56, 5.0, 6.0, rgba("082b35", 255))
            img.ellipse(x - 1.5, 53.5, 1.5, 1.9, rgba("f3fffb", 255))
        img.line(53, 72, 64, 76, 1.7, rgba("11636c", 170))
        img.line(64, 76, 72, 72, 1.7, rgba("11636c", 170))
    img.ellipse(44, 42, 14, 6, rgba("d7fffb", 45))


def creature_idle() -> Image:
    img = Image()
    creature_body(img, "57d3e1", "2a98bb", "3db2ca", False)
    return img


def creature_hurt() -> Image:
    img = Image()
    creature_body(img, "95b8b8", "6f8d95", "a7c9c5", True)
    return img


def creature_moving() -> Image:
    img = Image()
    img.shadow(68, 94, 35, 8, 88)
    img.ellipse(66, 68, 32, 21, rgba("55d7e3", 255))
    img.ellipse(53, 61, 24, 18, rgba("8af5ef", 230))
    img.ellipse(82, 60, 12, 11, rgba("43b7d0", 230))
    for x, y in [(37, 86), (73, 89), (90, 78)]:
        img.ellipse(x, y, 8, 4, rgba("237b95", 210))
    img.line(43, 43, 30, 25, 3.0, rgba("237b95", 230))
    img.line(83, 42, 102, 31, 3.2, rgba("237b95", 230))
    img.ellipse(28, 24, 4.0, 4.0, rgba("d7fffb", 230))
    img.ellipse(104, 30, 4.2, 4.2, rgba("d7fffb", 230))
    img.ellipse(47, 55, 4.6, 5.4, rgba("092d35", 255))
    img.ellipse(72, 55, 4.4, 5.2, rgba("092d35", 255))
    for y, alpha in [(50, 70), (62, 55), (75, 42)]:
        img.line(19, y, 5, y + 4, 2.1, rgba("b8fff8", alpha))
    return img


def creature_eat() -> Image:
    img = Image()
    creature_body(img, "58d6dd", "2d9ab8", "71dacb", False)
    img.ellipse(83, 66, 10, 7, rgba("0b3b3f", 210))
    img.line(87, 69, 107, 76, 3.0, rgba("44ae52", 230))
    leaf(img, 111, 73, -0.2, 22, 10, "8de45f", 245)
    img.ellipse(102, 65, 5, 4, rgba("ff6d96", 235))
    img.ellipse(70, 84, 12, 4, rgba("3bcf7d", 125))
    return img


def creature_sleep() -> Image:
    img = Image()
    img.shadow(64, 92, 34, 9, 85)
    img.ellipse(64, 72, 34, 21, rgba("58cbd5", 255))
    img.ellipse(52, 69, 20, 15, rgba("a8f4ee", 225))
    img.ellipse(82, 74, 18, 14, rgba("2d98b6", 230))
    img.ellipse(50, 67, 4, 1.6, rgba("0b3740", 230))
    img.ellipse(73, 66, 4, 1.6, rgba("0b3740", 230))
    img.line(41, 50, 28, 40, 2.5, rgba("2d98b6", 190))
    img.line(86, 52, 99, 42, 2.5, rgba("2d98b6", 190))
    for i, (x, y) in enumerate([(95, 31), (106, 22), (113, 13)]):
        img.line(x - 6, y, x + 5, y, 2.0, rgba("eaffff", 155 + i * 25))
        img.line(x + 5, y, x - 5, y + 7, 2.0, rgba("eaffff", 155 + i * 25))
        img.line(x - 5, y + 7, x + 6, y + 7, 2.0, rgba("eaffff", 155 + i * 25))
    return img


def creature_signal() -> Image:
    img = Image()
    creature_body(img, "5dd9dd", "2c96b0", "70decf", False)
    for radius, alpha in [(24, 72), (36, 45), (49, 28)]:
        for a in range(205, 336, 9):
            rad = math.radians(a)
            x = 65 + math.cos(rad) * radius
            y = 52 + math.sin(rad) * radius * 0.62
            img.ellipse(x, y, 1.8, 1.4, rgba("fbf2b8", alpha))
    img.polygon([(88, 30), (115, 27), (108, 45), (93, 44)], rgba("fff0b2", 190))
    img.line(93, 44, 85, 53, 2.0, rgba("fff0b2", 160))
    return img


def selection_ring() -> Image:
    img = Image()
    img.shadow(64, 74, 42, 10, 50)
    for radius, alpha, width in [(45, 105, 2.0), (39, 210, 2.8), (33, 88, 1.6)]:
        for a in range(0, 360, 3):
            if a % 24 in (0, 3, 6):
                continue
            rad = math.radians(a)
            x = 64 + math.cos(rad) * radius
            y = 66 + math.sin(rad) * radius * 0.48
            img.ellipse(x, y, width, width * 0.65, rgba("ffe86a", alpha))
    for x, y in [(20, 66), (108, 66), (64, 43), (64, 88)]:
        img.ellipse(x, y, 5, 3, rgba("fff8b0", 220))
    return img


def selection_pulse() -> Image:
    img = Image()
    for radius, alpha, width in [(50, 60, 3.2), (42, 120, 2.6), (34, 92, 1.8)]:
        for a in range(0, 360, 4):
            rad = math.radians(a)
            x = 64 + math.cos(rad) * radius
            y = 66 + math.sin(rad) * radius * 0.50
            img.ellipse(x, y, width, width * 0.58, rgba("7fffd8", alpha))
    for x, y in [(30, 50), (98, 50), (42, 83), (86, 83)]:
        img.ellipse(x, y, 4.2, 2.2, rgba("fff7a2", 170))
    return img


def food() -> Image:
    img = Image()
    img.shadow(65, 94, 21, 7, 78)
    img.line(64, 99, 64, 55, 7, rgba("5bc44b", 255))
    for angle, color in [(-2.8, "367f3b"), (-0.55, "77e05a"), (2.8, "4db44b"), (0.35, "a0ef73")]:
        leaf(img, 64, 70, angle, 42, 20, color, 245)
    img.ellipse(60, 42, 13, 12, rgba("d44583", 255))
    img.ellipse(71, 38, 10, 9, rgba("ff6d96", 255))
    img.ellipse(52, 45, 8, 8, rgba("b12f70", 255))
    img.ellipse(63, 40, 2.5, 2.5, rgba("fff3ce", 220))
    return img


def food_bloom() -> Image:
    img = food()
    for angle in [0.0, 1.25, 2.45, 3.75, 5.0]:
        leaf(img, 64 + math.cos(angle) * 9, 43 + math.sin(angle) * 5, angle, 20, 9, "f8d36b", 215)
    img.ellipse(64, 43, 8, 7, rgba("ff7aa7", 245))
    img.ellipse(61, 40, 2, 2, rgba("fff3ce", 240))
    return img


def hazard() -> Image:
    img = Image()
    img.shadow(66, 106, 30, 9, 110)
    img.polygon([(64, 12), (93, 61), (70, 111), (42, 110), (31, 58)], rgba("ed1f39", 248))
    img.polygon([(64, 12), (74, 64), (59, 110), (42, 110), (31, 58)], rgba("941029", 245))
    img.polygon([(88, 31), (117, 59), (91, 73)], rgba("ff5b37", 228))
    img.polygon([(34, 37), (9, 73), (42, 69)], rgba("ff433c", 228))
    img.polygon([(64, 28), (77, 63), (63, 96), (51, 66)], rgba("ffd08e", 200))
    for x, h in [(26, 21), (103, 28), (79, 18), (42, 16)]:
        img.polygon([(x, 87), (x + 7, 113), (x - 9, 113)], rgba("cf2732", 215))
    return img


def hazard_glow() -> Image:
    img = Image()
    for r, alpha in [(48, 35), (38, 50), (27, 65)]:
        img.ellipse(64, 70, r, r * 0.72, rgba("ff3136", alpha))
    base = hazard()
    for y in range(CANVAS):
        for x in range(CANVAS):
            src = base.pixels[y][x]
            if src[3] > 0:
                img.set(x, y, src)
    img.polygon([(65, 18), (76, 62), (63, 91), (53, 63)], rgba("fff2b0", 235))
    return img


def rock() -> Image:
    img = Image()
    img.shadow(65, 99, 39, 12, 105)
    img.polygon([(22, 87), (34, 49), (61, 23), (96, 43), (105, 82), (80, 105), (42, 105)], rgba("7d8275", 255))
    img.polygon([(34, 49), (61, 23), (57, 72), (22, 87)], rgba("a9ae9c", 245))
    img.polygon([(61, 23), (96, 43), (76, 70), (57, 72)], rgba("8f9588", 245))
    img.polygon([(57, 72), (105, 82), (80, 105), (42, 105)], rgba("565e54", 255))
    for x, y, r in [(25, 99, 6), (96, 94, 7), (42, 109, 5), (83, 37, 3)]:
        img.ellipse(x, y, r, r * 0.7, rgba("464d45", 150))
    img.line(57, 72, 81, 104, 2.5, rgba("374038", 150))
    img.line(62, 25, 76, 70, 2.0, rgba("d8dcc9", 95))
    return img


def tile(base: str, accents: list[str], seed: int, mood: str) -> Image:
    img = Image(False, rgba(base, 255))
    deterministic_dot(img, seed, [rgba(c, 90) for c in accents], 170, 0.8, 4.5)
    for i in range(10):
        offset = (seed + i * 17) % (SIZE + 18) - 9
        img.line(-8, offset, SIZE + 8, offset + ((seed + i * 11) % 27) - 13, 1.0, rgba(accents[i % len(accents)], 62))
    if mood == "grass":
        for i in range(18):
            x = 8 + ((seed * (i + 3) + i * 19) % 112)
            y = 10 + ((seed * (i + 7) + i * 29) % 108)
            leaf(img, x, y, (i % 7) * 0.7, 13 + i % 8, 5, accents[i % len(accents)], 120)
    elif mood == "hazard":
        for i in range(8):
            x = 10 + ((seed * (i + 5) + i * 31) % 105)
            y = 12 + ((seed * (i + 11) + i * 17) % 102)
            img.polygon([(x, y - 8), (x + 6, y + 6), (x - 5, y + 8)], rgba("dd3b34", 95))
    elif mood == "stone":
        for i in range(12):
            x = 8 + ((seed * (i + 13) + i * 23) % 110)
            y = 8 + ((seed * (i + 17) + i * 21) % 110)
            img.ellipse(x, y, 6 + (i % 5), 3 + (i % 3), rgba(accents[i % len(accents)], 86))
    return img


def prop_grass() -> Image:
    img = Image()
    img.shadow(64, 104, 24, 6, 70)
    for i, x in enumerate([35, 43, 52, 61, 70, 80, 90]):
        img.line(x, 104, x + (64 - x) * 0.45, 43 + (i % 4) * 6, 5, rgba("72d956", 230))
        img.line(x + 1, 100, x + (64 - x) * 0.25 + 6, 54 + (i % 3) * 4, 2, rgba("c9f68d", 120))
    return img


def prop_pebble() -> Image:
    img = Image()
    img.shadow(64, 99, 25, 7, 75)
    for x, y, r, c in [(44, 77, 13, "8c8f82"), (62, 69, 18, "aaa894"), (80, 82, 12, "6d7168"), (52, 91, 8, "5c6258")]:
        img.ellipse(x, y, r, r * 0.7, rgba(c, 235))
        img.ellipse(x - r * 0.25, y - r * 0.35, r * 0.35, r * 0.18, rgba("d8dcc9", 80))
    return img


def prop_warning() -> Image:
    img = Image()
    img.shadow(64, 101, 23, 7, 78)
    img.polygon([(64, 24), (94, 99), (34, 99)], rgba("d93b31", 228))
    img.polygon([(64, 36), (82, 91), (46, 91)], rgba("ff9a3d", 210))
    img.line(64, 53, 64, 78, 5, rgba("fff0c4", 240))
    img.ellipse(64, 88, 4.0, 4.0, rgba("fff0c4", 240))
    return img


def prop_leaf() -> Image:
    img = Image()
    img.shadow(64, 95, 28, 7, 70)
    for cx, cy, ang, col in [(45, 70, -0.35, "3b9447"), (67, 64, -0.7, "60bb55"), (82, 76, 0.2, "428f37"), (56, 85, 0.45, "7bcf5b")]:
        leaf(img, cx, cy, ang, 38, 16, col, 220)
    img.line(33, 91, 93, 60, 2.0, rgba("1c642d", 180))
    return img


ASSETS = [
    ("creature_idle", "creature-idle", "sprite", creature_idle),
    ("creature_hurt", "creature-hurt", "sprite", creature_hurt),
    ("creature_moving", "creature-moving", "sprite", creature_moving),
    ("creature_eat", "creature-eat", "sprite", creature_eat),
    ("creature_sleep", "creature-sleep", "sprite", creature_sleep),
    ("creature_signal", "creature-signal", "sprite", creature_signal),
    ("selection_ring", "selection-ring", "selection", selection_ring),
    ("selection_pulse", "selection-pulse", "selection", selection_pulse),
    ("food_sprout", "food", "sprite", food),
    ("food_bloom", "food-variant", "sprite", food_bloom),
    ("hazard_crystal", "hazard", "sprite", hazard),
    ("hazard_glow", "hazard-active", "sprite", hazard_glow),
    ("rock_cluster", "rock-obstacle", "sprite", rock),
    ("terrain_safe_grass", "terrain-safe-grass", "terrain-tile", lambda: tile("275d2e", ["3d7839", "173d22", "6da448", "9dbe57"], 11, "grass")),
    ("terrain_soil_path", "terrain-soil-path", "terrain-tile", lambda: tile("6a4a2d", ["8a6237", "4d3625", "a57b49", "c09a5a"], 23, "soil")),
    ("terrain_resource_grove", "terrain-resource-grove", "terrain-tile", lambda: tile("2d7134", ["4ba64a", "1d4d29", "83d85f", "a6e66c"], 37, "grass")),
    ("terrain_hazard_pressure", "terrain-hazard-pressure", "terrain-tile", lambda: tile("64302b", ["8b392e", "2f2920", "c04b31", "e75c45"], 41, "hazard")),
    ("terrain_stone_rough", "terrain-stone-rough", "terrain-tile", lambda: tile("555a50", ["73776b", "343a34", "8e927f", "b2b19c"], 53, "stone")),
    ("prop_grass_tuft", "prop-dressing", "prop", prop_grass),
    ("prop_pebble_cluster", "prop-dressing", "prop", prop_pebble),
    ("prop_warning_shard", "prop-dressing", "prop", prop_warning),
    ("prop_leaf_patch", "prop-dressing", "prop", prop_leaf),
]


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    entries = []
    for asset_id, role, kind, factory in ASSETS:
        path = OUT / f"{asset_id}.png"
        factory().save(path)
        size = path.stat().st_size
        entries.append(
            {
                "id": asset_id,
                "role": role,
                "kind": kind,
                "relative_path": str(path.relative_to(ROOT)).replace("\\", "/"),
                "width": SIZE,
                "height": SIZE,
                "file_size_bytes": size,
            }
        )
    manifest = {
        "schema": "alife.ca44a.alpha_art_manifest.v1",
        "schema_version": 1,
        "pack_id": "alpha-art-v1",
        "art_direction": "production-alpha-organic-topdown-v3",
        "entries": entries,
    }
    (OUT / "alpha_art_manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
