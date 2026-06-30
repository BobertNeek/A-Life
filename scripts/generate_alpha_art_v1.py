#!/usr/bin/env python3
"""Generate the committed A-Life alpha art v1 PNG pack.

The pack is deterministic, original, and generated with Python's standard
library. The direction is "production-alpha": small enough for the repo, but
with layered silhouettes, soft shadows, organic terrain alpha, and readable
entity shapes that are closer to a real game art pass than programmer
rectangles.
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
ART_DIRECTION = "production-alpha-generated-world-atlas-v22-opaque-ground-tiles"


def rgba(hex_color: str, alpha: int = 255) -> tuple[int, int, int, int]:
    hex_color = hex_color.lstrip("#")
    return (
        int(hex_color[0:2], 16),
        int(hex_color[2:4], 16),
        int(hex_color[4:6], 16),
        alpha,
    )


def clamp_byte(value: float) -> int:
    return max(0, min(255, round(value)))


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

    def multiply_alpha_mask(
        self,
        cx: float = 64.0,
        cy: float = 64.0,
        rx: float = 58.0,
        ry: float = 54.0,
        softness: float = 0.25,
        wobble_seed: int = 0,
    ) -> None:
        """Fade tile edges into organic patches instead of square cards."""

        seed_phase = wobble_seed * 0.137
        for y in range(CANVAS):
            py = y / SCALE
            for x in range(CANVAS):
                r, g, b, a = self.pixels[y][x]
                if a == 0:
                    continue
                px = x / SCALE
                nx = (px - cx) / max(rx, 1.0)
                ny = (py - cy) / max(ry, 1.0)
                angle = math.atan2(ny, nx)
                wobble = (
                    1.0
                    + math.sin(angle * 3.0 + seed_phase) * 0.075
                    + math.sin(angle * 7.0 + seed_phase * 0.41) * 0.045
                )
                dist = math.sqrt(nx * nx + ny * ny) / max(wobble, 0.7)
                if dist >= 1.0:
                    alpha = 0.0
                elif dist > 1.0 - softness:
                    alpha = (1.0 - dist) / max(softness, 0.01)
                    alpha = alpha * alpha * (3.0 - 2.0 * alpha)
                else:
                    alpha = 1.0
                if alpha <= 0.0:
                    self.pixels[y][x] = (r, g, b, 0)
                elif alpha < 1.0:
                    self.pixels[y][x] = (r, g, b, clamp_byte(a * alpha))

    def apply_texture_noise(self, seed: int, color_strength: float = 5.0, alpha_strength: float = 0.04) -> None:
        """Subtle deterministic noise keeps large terrain patches painterly."""

        for y in range(CANVAS):
            for x in range(CANVAS):
                r, g, b, a = self.pixels[y][x]
                if a == 0:
                    continue
                value = (
                    (x * 73856093)
                    ^ (y * 19349663)
                    ^ (seed * 83492791)
                    ^ ((x + y) * 2654435761)
                ) & 0xFF
                delta = (value - 128) / 128.0
                alpha_delta = 1.0 + delta * alpha_strength
                self.pixels[y][x] = (
                    clamp_byte(r + delta * color_strength),
                    clamp_byte(g + delta * color_strength),
                    clamp_byte(b + delta * color_strength),
                    clamp_byte(a * alpha_delta),
                )

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
        write_rgba_png(path, SIZE, SIZE, raw)


def write_rgba_png(path: Path, width: int, height: int, raw_scanlines: bytes) -> None:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw_scanlines, 9))
        + chunk(b"IEND", b"")
    )
    path.write_bytes(png)


def read_png_dimensions(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a valid PNG")
    return (
        struct.unpack(">I", data[16:20])[0],
        struct.unpack(">I", data[20:24])[0],
    )


class WideImage:
    def __init__(self, width: int, height: int, bg: tuple[int, int, int, int]):
        self.width = width
        self.height = height
        self.pixels = [[bg for _ in range(width)] for _ in range(height)]

    def set(self, x: int, y: int, color: tuple[int, int, int, int]) -> None:
        if 0 <= x < self.width and 0 <= y < self.height:
            self.pixels[y][x] = blend(self.pixels[y][x], color)

    def ellipse(self, cx: float, cy: float, rx: float, ry: float, color: tuple[int, int, int, int]) -> None:
        for y in range(max(0, int(cy - ry - 2)), min(self.height, int(cy + ry + 3))):
            for x in range(max(0, int(cx - rx - 2)), min(self.width, int(cx + rx + 3))):
                dx = (x + 0.5 - cx) / max(rx, 1.0)
                dy = (y + 0.5 - cy) / max(ry, 1.0)
                if dx * dx + dy * dy <= 1.0:
                    self.set(x, y, color)

    def polygon(self, points: list[tuple[float, float]], color: tuple[int, int, int, int]) -> None:
        min_x = max(0, int(min(x for x, _ in points)) - 2)
        max_x = min(self.width, int(max(x for x, _ in points)) + 3)
        min_y = max(0, int(min(y for _, y in points)) - 2)
        max_y = min(self.height, int(max(y for _, y in points)) + 3)
        for y in range(min_y, max_y):
            for x in range(min_x, max_x):
                inside = False
                j = len(points) - 1
                for i in range(len(points)):
                    xi, yi = points[i]
                    xj, yj = points[j]
                    if ((yi > y) != (yj > y)) and (
                        x < (xj - xi) * (y - yi) / ((yj - yi) or 1.0) + xi
                    ):
                        inside = not inside
                    j = i
                if inside:
                    self.set(x, y, color)

    def line(self, x1: float, y1: float, x2: float, y2: float, width: float, color: tuple[int, int, int, int]) -> None:
        dx = x2 - x1
        dy = y2 - y1
        length_sq = dx * dx + dy * dy
        pad = width + 2
        for y in range(max(0, int(min(y1, y2) - pad)), min(self.height, int(max(y1, y2) + pad))):
            for x in range(max(0, int(min(x1, x2) - pad)), min(self.width, int(max(x1, x2) + pad))):
                if length_sq == 0:
                    dist = math.hypot(x - x1, y - y1)
                else:
                    t = max(0.0, min(1.0, ((x - x1) * dx + (y - y1) * dy) / length_sq))
                    px = x1 + t * dx
                    py = y1 + t * dy
                    dist = math.hypot(x - px, y - py)
                if dist <= width * 0.5:
                    self.set(x, y, color)

    def save(self, path: Path) -> None:
        raw = bytearray()
        for row in self.pixels:
            raw.append(0)
            for pixel in row:
                raw.extend(pixel)
        write_rgba_png(path, self.width, self.height, bytes(raw))


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


def ambient_canopy_shadow() -> Image:
    img = Image()
    for x, y, rx, ry, alpha in [
        (20, 20, 22, 15, 38),
        (48, 12, 25, 18, 31),
        (91, 22, 27, 16, 34),
        (18, 76, 31, 20, 28),
        (66, 68, 42, 25, 42),
        (103, 92, 29, 20, 31),
        (49, 109, 35, 17, 24),
    ]:
        img.ellipse(x, y, rx, ry, rgba("0a2213", alpha))
    for i in range(34):
        x = 8 + ((i * 29) % 114)
        y = 5 + ((i * 47) % 118)
        angle = (i % 9) * 0.42
        leaf(img, x, y, angle, 13 + (i % 5), 5 + (i % 3), "102a19", 32 + (i % 4) * 9)
    return img


def ambient_light_pool() -> Image:
    img = Image()
    for r, alpha in [(58, 18), (44, 26), (30, 34), (16, 25)]:
        img.ellipse(64, 65, r, r * 0.62, rgba("ffe7a7", alpha))
    for angle in [0.2, 1.4, 2.7, 4.0, 5.1]:
        leaf(img, 64 + math.cos(angle) * 22, 65 + math.sin(angle) * 12, angle, 18, 7, "f7f1b8", 32)
    return img


def entity_shadow() -> Image:
    img = Image()
    for rx, ry, alpha in [(44, 17, 50), (34, 12, 44), (22, 7, 32)]:
        img.ellipse(64, 75, rx, ry, rgba("031006", alpha))
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
    """Seam-friendly production alpha terrain tile.

    Ground needs to be actual tile material, not transparent daubs. Every
    terrain tile is fully opaque and has edge-to-edge texture so generated
    chunks can be stitched into a continuous world surface.
    """

    img = Image(bg=rgba(base, 255))
    # Low-frequency material washes. These are clipped by the full tile, so
    # they do not create the black holes that made earlier alpha-mask terrain
    # look broken.
    for i in range(18):
        value = (seed * 1103515245 + i * 2654435761 + 12345) & 0x7FFFFFFF
        cx = -12 + (value % 152)
        cy = -12 + ((value >> 8) % 152)
        rx = 14 + ((value >> 16) % 26)
        ry = 8 + ((value >> 22) % 22)
        alpha = 24 + ((value >> 5) % 36)
        img.ellipse(cx, cy, rx, ry, rgba(accents[(value >> 12) % len(accents)], alpha))

    deterministic_dot(img, seed, [rgba(c, 72) for c in accents], 220, 0.35, 1.95)

    # Fine scratches and grass/stone grain align loosely across tile borders.
    for i in range(34):
        cx = (seed * (i + 19) + i * 31) % 128
        cy = (seed * (i + 11) + i * 41) % 128
        angle = ((seed + i * 31) % 360) * math.pi / 180.0
        length = 5 + ((seed + i * 13) % 21)
        dx = math.cos(angle) * length * 0.5
        dy = math.sin(angle) * length * 0.5
        img.line(cx - dx, cy - dy, cx + dx, cy + dy, 0.55, rgba(accents[i % len(accents)], 34))
    if mood == "grass":
        for i in range(34):
            x = 9 + ((seed * (i + 3) + i * 19) % 110)
            y = 10 + ((seed * (i + 7) + i * 29) % 108)
            leaf(img, x, y, (i % 11) * 0.43, 6 + i % 6, 2.4, accents[i % len(accents)], 86)
            if i % 7 == 0:
                img.ellipse(x + 4, y - 3, 1.8, 1.4, rgba("f4e77d", 118))
    elif mood == "resource":
        for i in range(30):
            x = 8 + ((seed * (i + 5) + i * 23) % 112)
            y = 8 + ((seed * (i + 9) + i * 31) % 112)
            leaf(img, x, y, (i % 13) * 0.39, 7 + (i % 8), 3.4, accents[i % len(accents)], 112)
            if i % 5 == 0:
                img.ellipse(x - 3, y - 4, 2.0, 1.5, rgba("df5e5a", 128))
    elif mood == "hazard":
        for i in range(7):
            x = 10 + ((seed * (i + 5) + i * 31) % 105)
            y = 12 + ((seed * (i + 11) + i * 17) % 102)
            img.polygon([(x, y - 7), (x + 6, y + 8), (x - 6, y + 8)], rgba("dd3b34", 88))
            img.ellipse(x + 2, y + 3, 9, 4, rgba("ff7844", 34))
        for i in range(9):
            x = 3 + ((seed * (i + 17) + i * 37) % 122)
            y = 5 + ((seed * (i + 23) + i * 29) % 118)
            img.line(x - 8, y + 4, x + 10, y - 5, 1.2, rgba("5f231f", 72))
    elif mood == "stone":
        for i in range(24):
            x = 8 + ((seed * (i + 13) + i * 23) % 110)
            y = 8 + ((seed * (i + 17) + i * 21) % 110)
            img.ellipse(x, y, 5 + (i % 7), 2.0 + (i % 4), rgba(accents[i % len(accents)], 78))
            img.line(x - 6, y - 1, x + 7, y + 1, 0.6, rgba("d6dcc9", 42))
    elif mood == "soil":
        for i in range(24):
            x = 6 + ((seed * (i + 23) + i * 13) % 112)
            y = 8 + ((seed * (i + 19) + i * 27) % 108)
            img.line(x - 8, y, x + 9, y + ((i % 5) - 2), 0.85, rgba(accents[i % len(accents)], 58))
    elif mood == "water":
        for i in range(18):
            y = 8 + ((seed * (i + 13) + i * 17) % 112)
            phase = ((seed + i * 19) % 29) - 14
            img.line(0, y, 40 + phase, y + 3, 0.9, rgba("a7efff", 52))
            img.line(42 + phase, y + 4, 92 + phase, y - 1, 0.9, rgba("6fd3ef", 48))
            img.line(94 + phase, y, 128, y + 4, 0.9, rgba("e0fbff", 36))
        img.ellipse(42, 38, 18, 7, rgba("d9fbff", 30))
        img.ellipse(90, 92, 22, 8, rgba("b7efff", 26))
    elif mood == "sand":
        for i in range(20):
            x = -8 + ((seed * (i + 31) + i * 17) % 144)
            y = 6 + ((seed * (i + 29) + i * 21) % 116)
            img.line(x - 16, y, x + 20, y + ((i % 7) - 3), 0.75, rgba("f2d98c", 54))
            if i % 4 == 0:
                img.ellipse(x + 8, y + 4, 2.4, 1.3, rgba("b99045", 52))
    img.apply_texture_noise(seed, color_strength=8.0, alpha_strength=0.0)
    return img


def terrain_edge_blend() -> Image:
    img = Image()
    for x, y, rx, ry, col, alpha in [
        (22, 47, 28, 10, "1f3f20", 54),
        (54, 38, 39, 12, "9c7c43", 48),
        (87, 63, 36, 13, "384f2a", 55),
        (54, 90, 45, 12, "16341d", 44),
        (101, 93, 22, 8, "6b4e2f", 42),
    ]:
        img.ellipse(x, y, rx, ry, rgba(col, alpha))
    for i in range(24):
        x = 9 + ((i * 37) % 111)
        y = 15 + ((i * 53) % 96)
        leaf(img, x, y, (i % 8) * 0.62, 9 + (i % 6), 3.5, "82bd56", 36 + (i % 4) * 8)
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


def prop_mushroom() -> Image:
    img = Image()
    img.shadow(64, 101, 30, 8, 75)
    for x, y, s, cap in [
        (45, 79, 1.0, "e55f78"),
        (66, 72, 1.25, "f0b95b"),
        (86, 84, 0.82, "d5536e"),
    ]:
        img.line(x, y + 4, x, y + 23 * s, 5.0 * s, rgba("f2dfb6", 235))
        img.ellipse(x, y, 18 * s, 10 * s, rgba(cap, 245))
        img.ellipse(x - 5 * s, y - 2 * s, 5 * s, 2.3 * s, rgba("fff1bd", 150))
        img.ellipse(x + 7 * s, y + 1 * s, 3.4 * s, 1.8 * s, rgba("fff1bd", 140))
    for i in range(5):
        leaf(img, 35 + i * 13, 101 - (i % 2) * 3, -0.45 + i * 0.18, 21, 7, "65ba55", 155)
    return img


def prop_glow_spore() -> Image:
    img = Image()
    img.shadow(64, 104, 24, 6, 56)
    for r, alpha in [(38, 28), (26, 48), (15, 70)]:
        img.ellipse(64, 70, r, r * 0.58, rgba("8affd0", alpha))
    for angle in [0.0, 0.9, 1.7, 2.9, 4.2, 5.1]:
        x = 64 + math.cos(angle) * 24
        y = 72 + math.sin(angle) * 16
        img.line(64, 96, x, y, 2.2, rgba("4cbf7d", 160))
        img.ellipse(x, y, 6, 5, rgba("9bffca", 230))
        img.ellipse(x - 1, y - 1, 2, 1.8, rgba("f3ffe9", 230))
    img.ellipse(64, 96, 12, 5, rgba("368c55", 185))
    return img


def prop_thorn_scrub() -> Image:
    img = Image()
    img.shadow(64, 101, 31, 8, 82)
    for angle in [-1.2, -0.82, -0.45, 0.0, 0.42, 0.83, 1.25]:
        base_x = 64 + math.sin(angle) * 14
        img.line(base_x, 102, 64 + math.sin(angle) * 30, 45 + abs(angle) * 11, 4.4, rgba("5a322a", 220))
        tip_x = 64 + math.sin(angle) * 30
        tip_y = 45 + abs(angle) * 11
        img.polygon([(tip_x, tip_y - 9), (tip_x + 5, tip_y + 7), (tip_x - 5, tip_y + 6)], rgba("e64a39", 180))
    for x in [40, 52, 74, 88]:
        img.ellipse(x, 92 + (x % 3), 9, 4, rgba("8d4a2e", 130))
    return img


def ui_panel_frame() -> Image:
    img = Image()
    img.ellipse(64, 70, 60, 45, rgba("06130c", 205))
    img.ellipse(64, 67, 55, 39, rgba("12271a", 228))
    img.ellipse(64, 64, 49, 33, rgba("1b3a25", 190))
    for x, y, rx, ry, col in [
        (23, 32, 18, 9, "6f8f47"),
        (107, 35, 16, 8, "8aa34c"),
        (25, 101, 18, 8, "425d33"),
        (104, 99, 19, 9, "7c8443"),
    ]:
        img.ellipse(x, y, rx, ry, rgba(col, 150))
    for x1, y1, x2, y2 in [(16, 38, 112, 36), (17, 91, 111, 92), (23, 27, 21, 101), (106, 27, 108, 101)]:
        img.line(x1, y1, x2, y2, 3.0, rgba("d3c783", 115))
        img.line(x1, y1 + 2, x2, y2 + 2, 1.0, rgba("fff2b0", 55))
    return img


def ui_inspector_frame() -> Image:
    img = Image()
    img.ellipse(64, 68, 58, 43, rgba("06101a", 210))
    img.ellipse(64, 64, 51, 36, rgba("0f2630", 226))
    img.ellipse(64, 62, 44, 29, rgba("173845", 168))
    for angle in [0.2, 1.0, 2.1, 3.3, 4.4, 5.3]:
        leaf(img, 64 + math.cos(angle) * 48, 64 + math.sin(angle) * 31, angle + 1.2, 24, 8, "56b6a5", 115)
    for x1, y1, x2, y2 in [(19, 35, 109, 35), (19, 92, 109, 92), (27, 25, 25, 103), (101, 25, 103, 103)]:
        img.line(x1, y1, x2, y2, 2.4, rgba("86e0cf", 92))
    return img


def ui_status_chip() -> Image:
    img = Image()
    img.shadow(64, 75, 45, 10, 70)
    img.ellipse(64, 64, 50, 22, rgba("0a2a1a", 230))
    img.ellipse(64, 61, 44, 16, rgba("1f6939", 188))
    img.ellipse(31, 59, 9, 9, rgba("7dffa0", 210))
    img.ellipse(32, 59, 4, 4, rgba("e8ffcf", 230))
    img.line(43, 55, 98, 55, 2.0, rgba("c9ffd2", 105))
    img.line(42, 68, 92, 68, 1.5, rgba("08150e", 120))
    return img


def ui_meter_bar() -> Image:
    img = Image()
    img.shadow(64, 78, 48, 8, 55)
    img.polygon([(17, 52), (27, 42), (104, 42), (114, 52), (105, 82), (25, 82)], rgba("08140d", 220))
    img.polygon([(25, 55), (32, 50), (99, 50), (104, 56), (98, 72), (30, 72)], rgba("20331d", 235))
    for i, col in enumerate(["5ee06d", "d7e16d", "e97654"]):
        img.ellipse(40 + i * 18, 61, 10, 5, rgba(col, 120))
    img.line(25, 48, 104, 48, 2.0, rgba("e6d98b", 110))
    return img


def ui_control_keycap() -> Image:
    img = Image()
    img.shadow(64, 84, 35, 8, 62)
    img.polygon([(27, 49), (41, 36), (92, 36), (104, 50), (97, 82), (35, 84)], rgba("10251a", 232))
    img.polygon([(38, 48), (47, 42), (86, 42), (94, 51), (89, 70), (43, 72)], rgba("375137", 224))
    img.line(45, 47, 87, 47, 2.0, rgba("f2e7a2", 130))
    img.line(45, 65, 88, 65, 1.8, rgba("07100b", 115))
    for x in [49, 64, 79]:
        img.ellipse(x, 58, 3, 3, rgba("d4ffc3", 150))
    return img


def world_backdrop_gpu_alpha() -> WideImage:
    """Compact painted alpha map used as the default Player View backdrop."""

    width = 640
    height = 360
    img = WideImage(width, height, rgba("7fb045", 255))

    def rnd(seed: int) -> int:
        return (seed * 1664525 + 1013904223) & 0xFFFFFFFF

    def paint_cluster(cx: float, cy: float, rx: float, ry: float, color: str, alpha: int, seed: int, count: int) -> None:
        img.ellipse(cx, cy, rx, ry, rgba(color, alpha))
        value = seed
        for _ in range(count):
            value = rnd(value)
            x = cx + ((value % 2000) / 1000.0 - 1.0) * rx
            value = rnd(value)
            y = cy + ((value % 2000) / 1000.0 - 1.0) * ry
            if ((x - cx) / max(rx, 1.0)) ** 2 + ((y - cy) / max(ry, 1.0)) ** 2 > 1.08:
                continue
            value = rnd(value)
            rr = 2.0 + (value % 42) / 8.0
            img.ellipse(x, y, rr, rr * (0.46 + ((value >> 7) % 28) / 100.0), rgba(color, max(22, alpha // 3)))

    # Broad target-like biome plate: safe green center, ochre left, gray
    # highlands, red hazard right, and water/fog edges.
    for args in [
        (160, 120, 190, 110, "a2c85b", 170, 101, 90),
        (320, 188, 250, 130, "83b84e", 190, 102, 130),
        (458, 115, 170, 96, "727c68", 170, 103, 95),
        (520, 210, 154, 92, "b64e38", 230, 104, 115),
        (108, 245, 170, 86, "b69948", 165, 105, 86),
        (62, 300, 118, 58, "578d82", 115, 106, 48),
        (570, 306, 100, 54, "4b8183", 115, 107, 52),
        (404, 288, 128, 56, "4f9641", 150, 108, 70),
    ]:
        paint_cluster(*args)

    # Curving dirt paths, thin and layered like the mockup rather than giant
    # slabs. These are visual-only brush strokes.
    paths = [
        [(8, 248), (86, 223), (172, 194), (260, 173), (365, 148), (504, 116), (632, 83)],
        [(190, 354), (228, 292), (272, 226), (318, 164), (354, 93), (372, 6)],
        [(78, 116), (154, 133), (230, 156), (316, 183), (424, 205), (594, 226)],
    ]
    for path in paths:
        for width_px, alpha, color in [(20, 44, "593c24"), (13, 70, "8f6539"), (6, 108, "bd8a4a"), (2, 76, "e1bd70")]:
            for (x1, y1), (x2, y2) in zip(path, path[1:]):
                img.line(x1, y1, x2, y2, width_px, rgba(color, alpha))

    # Dense painterly terrain speckle.
    value = 424242
    palettes = [
        ("b7d668", 54),
        ("6ea13f", 50),
        ("3e7133", 42),
        ("d0ae58", 46),
        ("8b7d55", 42),
        ("b44a38", 42),
        ("52615d", 36),
    ]
    for _ in range(3150):
        value = rnd(value)
        x = value % width
        value = rnd(value)
        y = value % height
        color, alpha = palettes[(value >> 8) % len(palettes)]
        r = 0.35 + ((value >> 18) % 20) / 13.0
        img.ellipse(x, y, r, r * (0.42 + ((value >> 4) % 34) / 100.0), rgba(color, alpha))

    # Fine grass, scratches, and contour strokes.
    value = 99331
    for _ in range(650):
        value = rnd(value)
        x = 6 + (value % (width - 12))
        value = rnd(value)
        y = 6 + (value % (height - 12))
        value = rnd(value)
        angle = (value % 628) / 100.0
        length = 3.0 + ((value >> 9) % 13)
        dx = math.cos(angle) * length * 0.5
        dy = math.sin(angle) * length * 0.5
        color, alpha = palettes[(value >> 17) % len(palettes)]
        img.line(x - dx, y - dy, x + dx, y + dy, 0.75, rgba(color, min(105, alpha + 22)))

    # Small forests/groves.
    for cx, cy, seed, count in [(300, 82, 7001, 85), (350, 94, 7002, 70), (258, 270, 7003, 58), (112, 170, 7004, 52)]:
        value = seed
        for _ in range(count):
            value = rnd(value)
            x = cx + ((value % 1000) / 500.0 - 1.0) * 46
            value = rnd(value)
            y = cy + ((value % 1000) / 500.0 - 1.0) * 31
            img.ellipse(x, y + 2, 4.5, 2.4, rgba("14391f", 72))
            img.ellipse(x, y, 3.4, 3.1, rgba("2f7a32", 150))
            img.ellipse(x - 1.0, y - 1.0, 1.4, 1.1, rgba("9bd768", 96))
            if value % 7 == 0:
                img.ellipse(x + 1.8, y - 1.2, 1.2, 1.0, rgba("d34b44", 145))

    # Rock fields and red crystal field are baked small so the foreground
    # selectable sprites do not need to become giant labels.
    for x, y, s in [
        (34, 62, 1.0), (58, 83, 0.75), (108, 68, 0.9), (468, 50, 1.15), (510, 66, 0.85),
        (536, 92, 1.0), (392, 52, 0.8), (425, 72, 0.7), (590, 160, 0.82), (214, 96, 0.78),
        (332, 245, 0.86), (374, 262, 0.75), (116, 290, 0.88), (152, 274, 0.7),
    ]:
        img.ellipse(x, y + 5 * s, 8 * s, 4 * s, rgba("30352e", 58))
        img.ellipse(x, y, 7 * s, 5 * s, rgba("757d72", 205))
        img.ellipse(x - 2 * s, y - 2 * s, 2.8 * s, 1.6 * s, rgba("d7dbc9", 95))
        img.ellipse(x + 4 * s, y + 1 * s, 3.2 * s, 2.1 * s, rgba("4b534b", 130))
    for x, y, s in [
        (458, 184, 1.1), (492, 168, 0.92), (540, 212, 1.25), (574, 194, 0.9),
        (516, 250, 1.0), (604, 235, 0.86), (438, 234, 0.75), (486, 278, 0.7),
    ]:
        img.line(x, y + 8 * s, x - 5 * s, y - 8 * s, 3.2 * s, rgba("c73735", 205))
        img.line(x, y + 8 * s, x + 5 * s, y - 6 * s, 3.2 * s, rgba("ff7250", 158))
        img.ellipse(x, y + 9 * s, 5 * s, 2 * s, rgba("51211f", 85))

    # Food/flower/grove hints at much smaller scale than the selectable object
    # sprites.
    for x, y, col in [
        (178, 70, "f6d95b"), (232, 78, "e96868"), (286, 136, "ffdf6e"), (126, 224, "f29654"),
        (402, 138, "f7dd72"), (352, 206, "ea6375"), (224, 232, "e6cc4d"), (84, 260, "f2de76"),
        (418, 284, "f39c6b"), (572, 102, "e86264"), (60, 150, "ffdf72"),
    ]:
        img.ellipse(x, y, 2.0, 1.2, rgba(col, 170))
        img.line(x, y + 4, x, y - 3, 1.0, rgba("4faa38", 150))
        img.ellipse(x - 2, y + 2, 2, 1.2, rgba("72c84d", 112))
        img.ellipse(x + 2, y + 2, 2, 1.2, rgba("72c84d", 112))

    # Cloud/fog edges, deliberately subtle and not gameplay-significant.
    for cx, cy, rx, ry, alpha in [
        (20, 334, 55, 17, 74), (82, 350, 70, 14, 62), (602, 326, 76, 18, 72),
        (633, 288, 42, 22, 50), (8, 24, 36, 15, 38), (622, 18, 46, 14, 36),
    ]:
        img.ellipse(cx, cy, rx, ry, rgba("e9eddf", alpha))

    return img


def world_backdrop_gpu_alpha_v13() -> WideImage:
    """Painted top-down alpha map that matches the target game-world mockup.

    The previous preserved v12 image had giant baked creatures and high-contrast
    blob fields. This version keeps the art as one coherent terrain plate:
    green center, traversable dirt paths, resource groves, gray highlands, red
    hazard pressure, and tiny world dressing. Foreground creatures remain small
    Bevy sprites instead of being baked into the backdrop.
    """

    width = 1280
    height = 720
    img = WideImage(width, height, rgba("8fb34f", 255))

    def rnd(seed: int) -> int:
        return (seed * 1664525 + 1013904223) & 0xFFFFFFFF

    def patch(cx: float, cy: float, rx: float, ry: float, color: str, alpha: int, seed: int) -> None:
        value = seed
        for _ in range(72):
            value = rnd(value)
            x = cx + ((value % 2000) / 1000.0 - 1.0) * rx * 0.95
            value = rnd(value)
            y = cy + ((value % 2000) / 1000.0 - 1.0) * ry * 0.95
            if ((x - cx) / max(rx, 1.0)) ** 2 + ((y - cy) / max(ry, 1.0)) ** 2 > 1.0:
                continue
            value = rnd(value)
            local_rx = 18 + (value % 74)
            local_ry = 9 + ((value >> 7) % 42)
            img.ellipse(x, y, local_rx, local_ry, rgba(color, max(28, alpha // 4)))

    def path_line(points: list[tuple[float, float]]) -> None:
        for width_px, alpha, color in [
            (34, 38, "4a341f"),
            (24, 70, "7b5632"),
            (15, 120, "a97843"),
            (7, 126, "d4ad63"),
        ]:
            for (x1, y1), (x2, y2) in zip(points, points[1:]):
                img.line(x1, y1, x2, y2, width_px, rgba(color, alpha))

    # Broad hand-authored composition, closer to the target mockup than a noisy
    # tiled field. Keep every region connected; no black voids or giant cards.
    for args in [
        (260, 160, 330, 170, "aac65a", 118, 1001),
        (530, 260, 430, 215, "8cb34d", 140, 1002),
        (470, 138, 190, 95, "5f9b39", 100, 1003),
        (760, 170, 250, 120, "6f866e", 122, 1004),
        (950, 150, 255, 120, "69736f", 136, 1005),
        (1032, 322, 260, 150, "b84e3b", 178, 1006),
        (1008, 520, 305, 152, "b35a3e", 118, 1007),
        (185, 398, 250, 140, "b69547", 118, 1008),
        (350, 600, 330, 95, "698f48", 102, 1009),
        (710, 570, 300, 120, "4d963f", 100, 1010),
        (70, 554, 160, 108, "6fa19a", 78, 1011),
        (1202, 570, 125, 112, "527f83", 80, 1012),
    ]:
        patch(*args)

    # A few low-alpha irregular washes establish large biomes without the
    # obvious circular overlays that made v13.0 read like a debug heatmap.
    for points, color, alpha in [
        ([(746, 54), (1114, 42), (1238, 142), (1180, 250), (956, 236), (792, 178)], "566262", 58),
        ([(820, 246), (1192, 210), (1278, 388), (1174, 612), (880, 548), (756, 384)], "b54835", 70),
        ([(78, 254), (318, 292), (448, 434), (334, 584), (68, 590), (0, 476)], "b79a47", 50),
        ([(432, 430), (730, 378), (894, 476), (776, 660), (466, 664), (306, 552)], "4d943e", 48),
        ([(378, 66), (650, 58), (782, 146), (648, 230), (418, 210), (280, 142)], "4f9438", 42),
    ]:
        img.polygon(points, rgba(color, alpha))

    path_line([(20, 500), (160, 455), (292, 390), (450, 335), (620, 282), (805, 250), (1025, 232), (1260, 190)])
    path_line([(180, 704), (255, 606), (338, 520), (438, 440), (544, 350), (606, 228), (642, 24)])
    path_line([(44, 194), (196, 220), (360, 254), (520, 298), (704, 352), (930, 412), (1225, 448)])
    path_line([(422, 690), (512, 608), (622, 520), (730, 444), (840, 356), (936, 288)])

    value = 204771
    palette = [
        ("c6d96d", 35),
        ("7aa346", 42),
        ("3d7433", 36),
        ("d5b55f", 38),
        ("855a35", 34),
        ("a24336", 34),
        ("626d67", 34),
    ]
    for _ in range(1850):
        value = rnd(value)
        x = value % width
        value = rnd(value)
        y = value % height
        color, alpha = palette[(value >> 9) % len(palette)]
        r = 1.0 + ((value >> 19) % 10) / 5.0
        img.ellipse(x, y, r, r * (0.45 + ((value >> 5) % 26) / 100.0), rgba(color, alpha))

    for _ in range(680):
        value = rnd(value)
        x = 12 + value % (width - 24)
        value = rnd(value)
        y = 14 + value % (height - 28)
        value = rnd(value)
        angle = (value % 628) / 100.0
        length = 4.0 + ((value >> 8) % 11)
        dx = math.cos(angle) * length * 0.5
        dy = math.sin(angle) * length * 0.5
        color = ["477735", "a6c764", "6e9a46", "c8ad5a"][(value >> 18) % 4]
        img.line(x - dx, y - dy, x + dx, y + dy, 0.9, rgba(color, 92))

    def tiny_tree(x: float, y: float, s: float = 1.0, berry: bool = False) -> None:
        img.ellipse(x, y + 7 * s, 10 * s, 4 * s, rgba("13230f", 62))
        for dx, dy, r, col in [
            (-7, 2, 8, "326f30"),
            (4, -3, 10, "43853a"),
            (10, 4, 7, "2d662b"),
            (-1, -10, 8, "5f9b44"),
        ]:
            img.ellipse(x + dx * s, y + dy * s, r * s, r * 0.72 * s, rgba(col, 210))
        if berry:
            img.ellipse(x + 5 * s, y - 7 * s, 2.0 * s, 1.7 * s, rgba("df4f49", 190))
            img.ellipse(x - 5 * s, y + 1 * s, 1.7 * s, 1.4 * s, rgba("ef6f59", 180))

    def tiny_rock(x: float, y: float, s: float = 1.0) -> None:
        img.ellipse(x, y + 5 * s, 12 * s, 4 * s, rgba("263028", 60))
        img.ellipse(x - 4 * s, y, 7 * s, 5 * s, rgba("8b9183", 220))
        img.ellipse(x + 5 * s, y + 1 * s, 8 * s, 5 * s, rgba("666f66", 210))
        img.ellipse(x - 6 * s, y - 2 * s, 2.8 * s, 1.5 * s, rgba("d7ddc8", 100))

    def tiny_crystal(x: float, y: float, s: float = 1.0) -> None:
        img.line(x, y + 13 * s, x - 7 * s, y - 12 * s, 5.2 * s, rgba("be2833", 230))
        img.line(x, y + 13 * s, x + 6 * s, y - 10 * s, 4.4 * s, rgba("ff7250", 190))
        img.line(x + 12 * s, y + 10 * s, x + 18 * s, y - 5 * s, 3.7 * s, rgba("e7443f", 185))
        img.ellipse(x + 2 * s, y + 13 * s, 10 * s, 3 * s, rgba("411d1b", 70))

    def tiny_flower(x: float, y: float, color: str = "f56c86") -> None:
        img.line(x, y + 9, x, y - 4, 1.4, rgba("459a35", 180))
        img.ellipse(x, y - 4, 3.0, 2.2, rgba(color, 220))
        img.ellipse(x - 3, y - 1, 3.0, 1.9, rgba("ffd86c", 160))
        img.ellipse(x + 3, y - 1, 3.0, 1.9, rgba("ffe27d", 160))

    def tiny_creature(x: float, y: float, color: str = "54d1e2") -> None:
        img.ellipse(x, y + 6, 9, 3.5, rgba("112820", 55))
        img.ellipse(x, y, 8, 6, rgba(color, 228))
        img.ellipse(x - 4, y - 5, 2, 2, rgba("ecffff", 220))
        img.ellipse(x + 4, y - 5, 2, 2, rgba("ecffff", 220))
        img.line(x - 4, y - 8, x - 9, y - 14, 1.3, rgba(color, 190))
        img.line(x + 4, y - 8, x + 9, y - 13, 1.3, rgba(color, 190))

    for x, y, s in [
        (66, 88, 1.2), (105, 122, 0.9), (174, 96, 1.0), (760, 74, 1.1), (814, 112, 0.92),
        (886, 86, 1.0), (948, 136, 1.3), (640, 414, 0.95), (698, 454, 0.85), (262, 528, 1.15),
        (334, 566, 0.86), (430, 184, 0.9), (520, 126, 0.8), (1188, 276, 0.92), (1132, 386, 0.75),
    ]:
        tiny_rock(x, y, s)

    for x, y, s in [
        (914, 256, 1.1), (996, 280, 1.3), (1088, 314, 1.05), (1158, 252, 0.9),
        (1048, 420, 1.0), (1186, 454, 0.78), (940, 456, 0.85), (1114, 544, 0.75),
    ]:
        tiny_crystal(x, y, s)

    for x, y, s, berry in [
        (458, 110, 1.1, True), (504, 95, 0.95, True), (538, 136, 1.0, False),
        (610, 116, 0.9, False), (706, 592, 1.05, False), (748, 562, 0.9, True),
        (378, 474, 1.0, False), (318, 436, 0.82, False), (176, 320, 0.95, False),
        (246, 300, 0.78, True), (552, 494, 0.72, False), (592, 520, 0.86, False),
    ]:
        tiny_tree(x, y, s, berry)

    for x, y, c in [
        (236, 120, "f4d65e"), (350, 150, "e85764"), (428, 248, "ffe06e"), (548, 228, "f7b84f"),
        (658, 316, "df6e85"), (302, 408, "f4d55a"), (186, 584, "f28c62"), (780, 352, "f1db6b"),
        (836, 524, "ef6d7d"), (1130, 168, "f1d964"), (1034, 604, "f09c58"), (91, 238, "ead763"),
    ]:
        tiny_flower(x, y, c)

    for x, y, c in [(530, 332, "56d7e5"), (742, 286, "48bdd1"), (842, 396, "58d1e0"), (392, 338, "54cde3")]:
        tiny_creature(x, y, c)

    # Soft edge atmosphere like the target image, without hiding gameplay.
    for cx, cy, rx, ry, alpha in [
        (46, 675, 120, 32, 50),
        (158, 704, 160, 28, 38),
        (1170, 672, 150, 36, 48),
        (1255, 92, 75, 30, 26),
        (22, 24, 70, 22, 24),
        (628, 712, 165, 22, 24),
    ]:
        img.ellipse(cx, cy, rx, ry, rgba("edf1df", alpha))

    # Gentle vignette keeps the HUD readable but avoids the black checkerboard
    # appearance of the rejected backdrop.
    for cx, cy, rx, ry, alpha in [
        (0, 360, 80, 360, 14),
        (1280, 360, 90, 360, 16),
        (640, -20, 600, 70, 12),
        (640, 740, 560, 75, 14),
    ]:
        img.ellipse(cx, cy, rx, ry, rgba("0b1b10", alpha))

    return img


def world_backdrop_gpu_alpha_v15() -> WideImage:
    """Dense painted top-down alpha map matching the game-world mockup.

    v15 keeps the dense terrain plate but removes the last rail-like path
    strokes from v14. The map now uses broken, irregular dirt footpaths with
    small readable dressing: grass clearings, rock fields, resource groves,
    red hazard crystals, tiny creatures, and edge atmosphere. It remains a
    visual backdrop only; foreground gameplay objects are still rendered as
    normal sprites.
    """

    width = 1280
    height = 720
    img = WideImage(width, height, rgba("6f983b", 255))

    def rnd(seed: int) -> int:
        return (seed * 1664525 + 1013904223) & 0xFFFFFFFF

    def jitter(seed: int, scale: float) -> tuple[float, float, int]:
        seed = rnd(seed)
        x = ((seed & 0xFFFF) / 32767.5 - 1.0) * scale
        seed = rnd(seed)
        y = ((seed & 0xFFFF) / 32767.5 - 1.0) * scale
        return x, y, seed

    def organic_poly(cx: float, cy: float, rx: float, ry: float, sides: int, seed: int) -> list[tuple[float, float]]:
        points: list[tuple[float, float]] = []
        value = seed
        for i in range(sides):
            angle = (math.tau * i / sides) + 0.10 * math.sin(i * 1.7)
            value = rnd(value)
            r = 0.78 + ((value >> 8) % 48) / 100.0
            points.append((cx + math.cos(angle) * rx * r, cy + math.sin(angle) * ry * r))
        return points

    def wash(cx: float, cy: float, rx: float, ry: float, color: str, alpha: int, seed: int, sides: int = 12) -> None:
        img.polygon(organic_poly(cx, cy, rx, ry, sides, seed), rgba(color, alpha))

    def daub_cluster(cx: float, cy: float, rx: float, ry: float, color: str, alpha: int, seed: int, count: int) -> None:
        value = seed
        for _ in range(count):
            dx, dy, value = jitter(value, 1.0)
            x = cx + dx * rx
            y = cy + dy * ry
            if ((x - cx) / max(rx, 1.0)) ** 2 + ((y - cy) / max(ry, 1.0)) ** 2 > 1.05:
                continue
            value = rnd(value)
            local_rx = 8 + (value % 34)
            local_ry = 5 + ((value >> 7) % 20)
            img.ellipse(x, y, local_rx, local_ry, rgba(color, alpha))

    def trail(points: list[tuple[float, float]], seed: int) -> None:
        # Broken organic footpaths, not roads. A jittered centerline and many
        # short low-alpha strokes create a dirt-track read without the straight
        # rail impression that made v14 feel unlike the target mockup.
        samples: list[tuple[float, float]] = []
        value = seed
        for segment_index, ((x1, y1), (x2, y2)) in enumerate(zip(points, points[1:])):
            steps = 12
            dx = x2 - x1
            dy = y2 - y1
            length = max(math.hypot(dx, dy), 1.0)
            nx = -dy / length
            ny = dx / length
            for step in range(steps):
                t = step / steps
                value = rnd(value + segment_index * 97 + step * 13)
                wobble = math.sin((segment_index + t) * 3.8) * 7.0 + (((value >> 8) % 100) - 50) * 0.06
                samples.append((x1 + dx * t + nx * wobble, y1 + dy * t + ny * wobble))
        samples.append(points[-1])

        for width_px, alpha, color in [
            (9, 46, "3b2819"),
            (6, 86, "7f5730"),
            (3, 126, "be8646"),
        ]:
            for (x1, y1), (x2, y2) in zip(samples, samples[1:]):
                img.line(x1, y1, x2, y2, width_px, rgba(color, alpha))

        value = seed ^ 0x56A33
        for i, (x, y) in enumerate(samples[::3]):
            value = rnd(value + i * 31)
            if value % 5 == 0:
                continue
            rx = 5.0 + ((value >> 9) % 9)
            ry = 2.2 + ((value >> 15) % 5)
            img.ellipse(x, y, rx, ry, rgba("d4ad65", 58 + (value % 45)))

    def small_tree(x: float, y: float, s: float, berry: bool = False) -> None:
        img.ellipse(x, y + 7 * s, 12 * s, 4.0 * s, rgba("152712", 70))
        for dx, dy, r, col in [
            (-7, 2, 8, "1f5c2a"),
            (2, -5, 10, "347d34"),
            (9, 2, 7, "285f2c"),
            (-2, -10, 7, "609d3f"),
        ]:
            img.ellipse(x + dx * s, y + dy * s, r * s, r * 0.75 * s, rgba(col, 225))
        if berry:
            img.ellipse(x + 4 * s, y - 8 * s, 1.8 * s, 1.5 * s, rgba("d84043", 210))
            img.ellipse(x - 5 * s, y + 1 * s, 1.6 * s, 1.3 * s, rgba("ef7060", 190))

    def small_rock(x: float, y: float, s: float) -> None:
        img.ellipse(x, y + 6 * s, 14 * s, 4.5 * s, rgba("213126", 66))
        img.ellipse(x - 6 * s, y + 1 * s, 7 * s, 5 * s, rgba("7b8375", 225))
        img.ellipse(x + 3 * s, y - 2 * s, 10 * s, 7 * s, rgba("9a9f8f", 225))
        img.ellipse(x + 10 * s, y + 2 * s, 6 * s, 4.5 * s, rgba("59665d", 215))
        img.ellipse(x - 8 * s, y - 2 * s, 2.5 * s, 1.4 * s, rgba("dce1c9", 120))

    def small_crystal(x: float, y: float, s: float) -> None:
        img.line(x - 2 * s, y + 14 * s, x - 8 * s, y - 12 * s, 4.5 * s, rgba("b72033", 235))
        img.line(x + 2 * s, y + 14 * s, x + 6 * s, y - 15 * s, 4.0 * s, rgba("ff604c", 220))
        img.line(x + 12 * s, y + 12 * s, x + 18 * s, y - 5 * s, 3.0 * s, rgba("dc3b39", 200))
        img.ellipse(x + 2 * s, y + 14 * s, 10 * s, 2.8 * s, rgba("421918", 85))

    def small_flower(x: float, y: float, color: str) -> None:
        img.line(x, y + 9, x, y - 4, 1.1, rgba("3d8e31", 175))
        img.ellipse(x, y - 4, 2.3, 1.7, rgba(color, 225))
        img.ellipse(x - 2.4, y - 1, 2.3, 1.5, rgba("ffe16b", 178))
        img.ellipse(x + 2.4, y - 1, 2.3, 1.5, rgba("fff08b", 150))

    def small_creature(x: float, y: float, color: str) -> None:
        img.ellipse(x, y + 7, 9, 3.2, rgba("0b251f", 70))
        img.ellipse(x, y, 8, 6, rgba(color, 230))
        img.ellipse(x - 3, y - 4, 2.0, 1.9, rgba("eefcff", 230))
        img.ellipse(x + 4, y - 4, 2.0, 1.9, rgba("eefcff", 230))
        img.line(x - 4, y - 7, x - 9, y - 13, 1.2, rgba(color, 190))
        img.line(x + 4, y - 7, x + 9, y - 12, 1.2, rgba(color, 190))

    # Biomes: target-style lush center, ochre left, gray highlands, red hazard
    # right, and damp edge color. These are irregular and low enough alpha that
    # the world reads as terrain instead of UI overlays.
    for args in [
        (314, 160, 340, 150, "a7c85a", 108, 1101, 14),
        (565, 285, 380, 210, "79ad43", 118, 1102, 16),
        (310, 452, 300, 170, "af9145", 78, 1103, 14),
        (835, 130, 285, 120, "69766e", 92, 1104, 14),
        (1020, 220, 250, 136, "5f6a68", 74, 1105, 13),
        (1044, 390, 290, 186, "ad4639", 112, 1106, 15),
        (1085, 560, 285, 125, "a34a35", 74, 1107, 12),
        (92, 572, 190, 112, "578f83", 54, 1108, 12),
        (1216, 610, 150, 88, "4e8387", 60, 1109, 11),
    ]:
        wash(*args)

    # Painterly but compact micro-daubs, more like dense ground texture than
    # soft giant circles.
    for args in [
        (430, 184, 430, 170, "bcd36c", 38, 2101, 220),
        (540, 330, 420, 220, "578e37", 42, 2102, 260),
        (255, 466, 320, 165, "b79248", 40, 2103, 180),
        (868, 126, 300, 138, "717c76", 42, 2104, 175),
        (1038, 390, 310, 190, "b94d3e", 46, 2105, 205),
        (736, 560, 290, 120, "3f8738", 38, 2106, 165),
    ]:
        daub_cluster(*args)

    trail([(35, 500), (148, 462), (286, 402), (422, 344), (574, 302), (748, 262), (954, 236), (1230, 188)], 9011)
    trail([(158, 700), (224, 608), (322, 522), (420, 452), (520, 372), (580, 270), (618, 150), (646, 32)], 9012)
    trail([(64, 220), (214, 230), (360, 262), (530, 312), (704, 356), (904, 414), (1168, 450)], 9013)
    trail([(492, 680), (584, 602), (690, 514), (800, 424), (918, 326)], 9014)

    value = 320711
    palette = [
        ("d2df75", 58),
        ("89b84c", 56),
        ("456f31", 48),
        ("d0ad55", 46),
        ("72502e", 38),
        ("c44b3f", 42),
        ("667067", 42),
        ("f1dc86", 34),
    ]
    for _ in range(4200):
        value = rnd(value)
        x = value % width
        value = rnd(value)
        y = value % height
        color, alpha = palette[(value >> 9) % len(palette)]
        r = 0.75 + ((value >> 19) % 13) / 5.5
        img.ellipse(x, y, r, r * (0.38 + ((value >> 4) % 28) / 100.0), rgba(color, alpha))

    # Grass blades and scratches break up any remaining smoothness.
    for _ in range(1300):
        value = rnd(value)
        x = 8 + value % (width - 16)
        value = rnd(value)
        y = 8 + value % (height - 16)
        value = rnd(value)
        angle = (value % 628) / 100.0
        length = 3.0 + ((value >> 8) % 13)
        dx = math.cos(angle) * length * 0.5
        dy = math.sin(angle) * length * 0.5
        color = ["405f2b", "7fac43", "bace65", "a98041", "59665a"][(value >> 18) % 5]
        img.line(x - dx, y - dy, x + dx, y + dy, 0.75, rgba(color, 108))

    # Dense object dressing, with higher concentration around the areas the
    # player sees first. These silhouettes are small enough to feel like world
    # art instead of debug labels.
    for x, y, s in [
        (70, 92, 1.2), (108, 132, 0.85), (168, 82, 1.0), (236, 132, 0.75),
        (750, 70, 1.1), (802, 116, 0.95), (872, 84, 1.15), (930, 138, 1.3),
        (1002, 86, 0.9), (1102, 154, 0.85), (656, 410, 1.0), (710, 462, 0.86),
        (276, 540, 1.12), (348, 576, 0.92), (430, 184, 0.9), (520, 124, 0.82),
        (1188, 278, 0.92), (1132, 388, 0.75), (980, 548, 0.8), (842, 572, 0.75),
    ]:
        small_rock(x, y, s)

    for x, y, s in [
        (912, 254, 1.05), (992, 282, 1.25), (1086, 314, 1.02), (1158, 250, 0.92),
        (1048, 420, 1.05), (1184, 456, 0.86), (938, 456, 0.82), (1114, 544, 0.78),
        (1038, 604, 0.66), (1246, 356, 0.70),
    ]:
        small_crystal(x, y, s)

    grove_specs = [
        (420, 120, 180, 70, 3001, 52, True),
        (548, 136, 150, 74, 3002, 48, False),
        (676, 600, 180, 82, 3003, 46, True),
        (330, 464, 160, 88, 3004, 42, False),
        (188, 330, 120, 78, 3005, 34, True),
        (604, 500, 120, 62, 3006, 28, False),
    ]
    for cx, cy, rx, ry, seed, count, berries in grove_specs:
        local = seed
        for _ in range(count):
            dx, dy, local = jitter(local, 1.0)
            if dx * dx + dy * dy > 1.0:
                continue
            local = rnd(local)
            small_tree(cx + dx * rx, cy + dy * ry, 0.55 + ((local >> 7) % 50) / 100.0, berries and local % 5 == 0)

    flower_colors = ["f05f7b", "f4d85f", "f7a15a", "e95d5f", "f1df78"]
    for i, (x, y) in enumerate([
        (236, 120), (350, 150), (428, 248), (548, 228), (658, 316), (302, 408),
        (186, 584), (780, 352), (836, 524), (1130, 168), (1034, 604), (91, 238),
        (472, 402), (612, 206), (726, 174), (254, 300), (410, 606), (696, 442),
    ]):
        small_flower(x, y, flower_colors[i % len(flower_colors)])

    for x, y, c in [
        (530, 332, "56d7e5"),
        (744, 286, "48bdd1"),
        (842, 396, "58d1e0"),
        (392, 338, "54cde3"),
        (680, 188, "65dce8"),
        (610, 458, "49bfd8"),
    ]:
        small_creature(x, y, c)

    # Target-like edge fog and UI-friendly atmosphere without covering the map.
    for cx, cy, rx, ry, alpha in [
        (34, 680, 136, 36, 58),
        (176, 710, 172, 28, 44),
        (1170, 674, 158, 36, 54),
        (1260, 96, 70, 28, 30),
        (18, 24, 74, 20, 28),
        (640, 712, 190, 22, 28),
        (36, 344, 42, 170, 20),
    ]:
        img.ellipse(cx, cy, rx, ry, rgba("edf1df", alpha))

    for cx, cy, rx, ry, alpha in [
        (0, 360, 74, 350, 16),
        (1280, 360, 80, 350, 18),
        (640, -20, 620, 64, 10),
        (640, 738, 590, 70, 13),
    ]:
        img.ellipse(cx, cy, rx, ry, rgba("07150d", alpha))

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
    ("ambient_canopy_shadow", "ambient-canopy-shadow", "overlay", ambient_canopy_shadow),
    ("ambient_light_pool", "ambient-light-pool", "overlay", ambient_light_pool),
    ("entity_shadow", "entity-shadow", "overlay", entity_shadow),
    ("rock_cluster", "rock-obstacle", "sprite", rock),
    ("terrain_safe_grass", "terrain-safe-grass", "terrain-tile", lambda: tile("6f9a42", ["94bf58", "4f7e35", "b7d76e", "d4df7a"], 11, "grass")),
    ("terrain_soil_path", "terrain-soil-path", "terrain-tile", lambda: tile("8f6336", ["bc8649", "6d492c", "d2a25a", "e0bb76"], 23, "soil")),
    ("terrain_resource_grove", "terrain-resource-grove", "terrain-tile", lambda: tile("5aa33f", ["78cb51", "347832", "a3de68", "d0f185"], 37, "resource")),
    ("terrain_hazard_pressure", "terrain-hazard-pressure", "terrain-tile", lambda: tile("b24d38", ["d26244", "73352f", "ef7752", "f29b65"], 41, "hazard")),
    ("terrain_stone_rough", "terrain-stone-rough", "terrain-tile", lambda: tile("777e72", ["9fa68f", "556158", "c0c5a9", "d5d1b7"], 53, "stone")),
    ("terrain_water", "terrain-water", "terrain-tile", lambda: tile("277b91", ["49aac0", "1f5f78", "72d4e8", "b1eff8"], 59, "water")),
    ("terrain_sand", "terrain-sand", "terrain-tile", lambda: tile("c9a75b", ["efcc77", "a58042", "f5df99", "dcb665"], 61, "sand")),
    ("terrain_edge_blend", "terrain-edge-blend", "overlay", terrain_edge_blend),
    ("world_backdrop_gpu_alpha", "world-backdrop", "backdrop", world_backdrop_gpu_alpha_v15),
    ("prop_grass_tuft", "prop-dressing", "prop", prop_grass),
    ("prop_pebble_cluster", "prop-dressing", "prop", prop_pebble),
    ("prop_warning_shard", "prop-dressing", "prop", prop_warning),
    ("prop_leaf_patch", "prop-dressing", "prop", prop_leaf),
    ("prop_mushroom_cluster", "prop-dressing", "prop", prop_mushroom),
    ("ui_panel_frame", "ui-panel-frame", "ui-skin", ui_panel_frame),
    ("ui_inspector_frame", "ui-inspector-frame", "ui-skin", ui_inspector_frame),
    ("ui_status_chip", "ui-status-chip", "ui-skin", ui_status_chip),
    ("ui_meter_bar", "ui-meter-bar", "ui-skin", ui_meter_bar),
    ("ui_control_keycap", "ui-control-keycap", "ui-skin", ui_control_keycap),
]


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    entries = []
    for asset_id, role, kind, factory in ASSETS:
        path = OUT / f"{asset_id}.png"
        image = factory()
        image.save(path)
        width = getattr(image, "width", SIZE)
        height = getattr(image, "height", SIZE)
        size = path.stat().st_size
        entries.append(
            {
                "id": asset_id,
                "role": role,
                "kind": kind,
                "relative_path": str(path.relative_to(ROOT)).replace("\\", "/"),
                "width": width,
                "height": height,
                "file_size_bytes": size,
            }
        )
    manifest = {
        "schema": "alife.ca44a.alpha_art_manifest.v1",
        "schema_version": 1,
        "pack_id": "alpha-art-v1",
        "art_direction": ART_DIRECTION,
        "entries": entries,
    }
    (OUT / "alpha_art_manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
