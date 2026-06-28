#!/usr/bin/env python3
"""Generate the CA44A alpha art v1 PNG pack.

The script uses only the Python standard library and writes small original
RGBA PNGs plus a strict manifest. It is deterministic so the committed assets
can be regenerated and audited without third-party art or tool downloads.
"""

from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates" / "alife_game_app" / "assets" / "alpha_art_v1"
SIZE = 64
SCALE = 3
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


def deterministic_dot(img: Image, seed: int, palette: list[tuple[int, int, int, int]], count: int) -> None:
    value = seed
    for _ in range(count):
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
        x = 4 + (value % 56)
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
        y = 4 + (value % 56)
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
        r = 1.5 + (value % 5) * 0.35
        img.ellipse(x, y, r, r * 0.75, palette[value % len(palette)])


def creature_idle() -> Image:
    img = Image()
    img.ellipse(31, 33, 19, 14, rgba("54cddc", 245))
    img.ellipse(24, 28, 9, 8, rgba("7ce9df", 240))
    img.ellipse(42, 30, 7, 6, rgba("42a8c8", 220))
    img.ellipse(20, 43, 4, 3, rgba("2c8aa5", 240))
    img.ellipse(39, 44, 4, 3, rgba("2c8aa5", 240))
    img.ellipse(26, 28, 2.3, 2.8, rgba("062a38", 255))
    img.ellipse(40, 28, 2.1, 2.6, rgba("062a38", 255))
    img.ellipse(26, 27, 0.8, 0.9, rgba("f0fffb", 255))
    img.ellipse(39, 27, 0.8, 0.9, rgba("f0fffb", 255))
    img.line(47, 27, 55, 21, 2.3, rgba("3faabd", 220))
    img.ellipse(56, 20, 2.5, 2.5, rgba("9cf4da", 230))
    return img


def creature_hurt() -> Image:
    img = creature_idle()
    img.ellipse(31, 35, 21, 15, rgba("ef6b5d", 92))
    img.line(20, 24, 26, 30, 2.0, rgba("ffefe4", 255))
    img.line(26, 24, 20, 30, 2.0, rgba("ffefe4", 255))
    img.line(38, 24, 44, 30, 2.0, rgba("ffefe4", 255))
    img.line(44, 24, 38, 30, 2.0, rgba("ffefe4", 255))
    img.polygon([(47, 15), (52, 29), (43, 27)], rgba("ff4b45", 230))
    return img


def selection_ring() -> Image:
    img = Image()
    for radius, alpha in [(27, 120), (24, 190), (21, 80)]:
        for a in range(0, 360, 4):
            rad = math.radians(a)
            x = 32 + math.cos(rad) * radius
            y = 32 + math.sin(rad) * radius * 0.62
            img.ellipse(x, y, 1.8, 1.2, rgba("ffe66d", alpha))
    for x, y in [(10, 32), (54, 32), (32, 15), (32, 49)]:
        img.ellipse(x, y, 3.0, 2.0, rgba("fff6a9", 210))
    return img


def food() -> Image:
    img = Image()
    img.line(32, 49, 32, 26, 5, rgba("63d947", 255))
    img.ellipse(24, 31, 12, 6, rgba("3fbf55", 245))
    img.ellipse(40, 28, 12, 6, rgba("68ed62", 245))
    img.ellipse(31, 20, 8, 8, rgba("d94b7f", 255))
    img.ellipse(37, 18, 6, 6, rgba("ff6e8c", 255))
    img.ellipse(26, 22, 5, 5, rgba("b73270", 255))
    img.ellipse(31, 20, 2, 2, rgba("fff3ce", 210))
    return img


def hazard() -> Image:
    img = Image()
    img.polygon([(32, 6), (48, 31), (36, 56), (20, 56), (16, 29)], rgba("fa2838", 245))
    img.polygon([(32, 6), (39, 34), (29, 56), (20, 56), (16, 29)], rgba("9e102b", 240))
    img.polygon([(44, 19), (58, 32), (45, 38)], rgba("ff613b", 230))
    img.polygon([(18, 22), (6, 38), (22, 36)], rgba("ff503c", 230))
    img.polygon([(31, 16), (38, 32), (31, 48), (25, 34)], rgba("ffd18a", 190))
    img.ellipse(32, 58, 16, 3, rgba("42151b", 120))
    return img


def rock() -> Image:
    img = Image()
    img.polygon([(13, 42), (20, 25), (35, 16), (52, 27), (55, 45), (42, 55), (22, 54)], rgba("7e8075", 255))
    img.polygon([(20, 25), (35, 16), (32, 39), (13, 42)], rgba("a4a99a", 245))
    img.polygon([(35, 16), (52, 27), (39, 37), (32, 39)], rgba("8f9285", 245))
    img.polygon([(32, 39), (55, 45), (42, 55), (22, 54)], rgba("5d6258", 255))
    img.line(31, 39, 43, 53, 2, rgba("3e443d", 160))
    img.line(34, 17, 39, 37, 2, rgba("d8dcc9", 100))
    return img


def tile(base: str, accents: list[str], seed: int) -> Image:
    img = Image(False, rgba(base, 255))
    deterministic_dot(img, seed, [rgba(c, 90) for c in accents], 60)
    for i in range(5):
        offset = (seed + i * 17) % 58
        img.line(-8, offset, 72, offset + ((seed + i * 11) % 21) - 10, 1.2, rgba(accents[i % len(accents)], 68))
    return img


def prop_grass() -> Image:
    img = Image()
    for x in [20, 26, 32, 38, 44]:
        img.line(x, 52, x + (32 - x) * 0.25, 25 + (x % 4) * 3, 3, rgba("6ddd50", 230))
    img.ellipse(32, 54, 18, 4, rgba("174821", 140))
    return img


def prop_pebble() -> Image:
    img = Image()
    for x, y, r, c in [(25, 39, 8, "8c8f82"), (36, 34, 11, "aaa894"), (43, 43, 7, "6d7168")]:
        img.ellipse(x, y, r, r * 0.7, rgba(c, 235))
    return img


def prop_warning() -> Image:
    img = Image()
    img.polygon([(32, 12), (48, 51), (16, 51)], rgba("dc3e31", 220))
    img.polygon([(32, 19), (42, 47), (22, 47)], rgba("ff9a3d", 205))
    img.line(32, 27, 32, 40, 3, rgba("fff0c4", 240))
    img.ellipse(32, 45, 2.3, 2.3, rgba("fff0c4", 240))
    return img


def prop_leaf() -> Image:
    img = Image()
    img.ellipse(25, 34, 13, 7, rgba("3b9447", 210))
    img.ellipse(39, 29, 15, 8, rgba("60bb55", 210))
    img.line(18, 42, 49, 25, 2, rgba("1c642d", 190))
    return img


ASSETS = [
    ("creature_idle", "creature-idle", "sprite", creature_idle),
    ("creature_hurt", "creature-hurt", "sprite", creature_hurt),
    ("selection_ring", "selection-ring", "selection", selection_ring),
    ("food_sprout", "food", "sprite", food),
    ("hazard_crystal", "hazard", "sprite", hazard),
    ("rock_cluster", "rock-obstacle", "sprite", rock),
    ("terrain_safe_grass", "terrain-safe-grass", "terrain-tile", lambda: tile("255a2c", ["3d7839", "173d22", "6da448"], 11)),
    ("terrain_soil_path", "terrain-soil-path", "terrain-tile", lambda: tile("6a4a2d", ["8a6237", "4d3625", "a57b49"], 23)),
    ("terrain_resource_grove", "terrain-resource-grove", "terrain-tile", lambda: tile("2d7134", ["4ba64a", "1d4d29", "83d85f"], 37)),
    ("terrain_hazard_pressure", "terrain-hazard-pressure", "terrain-tile", lambda: tile("64302b", ["8b392e", "2f2920", "c04b31"], 41)),
    ("terrain_stone_rough", "terrain-stone-rough", "terrain-tile", lambda: tile("555a50", ["73776b", "343a34", "8e927f"], 53)),
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
        "entries": entries,
    }
    (OUT / "alpha_art_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
