"""Paint the fixed 25 m nursery ground wash; no tiled noise, letters or item symbols."""

from math import cos, pi, sin
from pathlib import Path
from random import Random

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target/artifacts/nursery-understory/floor.png"
SIZE = 1536


def smooth(a, b, value):
    t = np.clip((value - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def wash():
    rng = np.random.default_rng(18317)
    v = (np.arange(SIZE, dtype=np.float32) + 0.5) * 25 / SIZE - 12.5
    x, z = np.meshgrid(v, v)

    def grain(n):
        field = Image.fromarray(rng.integers(0, 256, (n, n), dtype=np.uint8))
        return (
            np.asarray(field.resize((SIZE, SIZE), Image.Resampling.BICUBIC)) / 255 - 0.5
        )

    pools = grain(9) * 0.34 + grain(31) * 0.19
    grain_mid = grain(149)
    grain_fine = grain(570)
    # A walkable seam of sun-warmed earth between the existing stone drifts.
    trail = abs(x + 0.63 * z - 0.74 * np.sin(z * 0.64) - 0.22)
    clay = (1 - smooth(0.22, 1.72, trail + 1.45 * pools)) * 0.92
    shade = grain_mid * 14 + grain_fine * 8 + pools * 27
    greens = np.array([111, 132, 92]) + (pools + 0.2)[:, :, None] * [52, 30, -8]
    earth = np.array([175, 142, 111]) + grain_mid[:, :, None] * [12, 8, 6]
    rgb = greens + (earth - greens) * clay[:, :, None] + shade[:, :, None]
    # Brush cupped cushions into the banks as overlapping, scalloped pieces;
    # their umber contact edge and pale upper rim supply near-camera structure.
    brush = Random(3371)
    for _ in range(218):
        cx, cz = brush.uniform(-9.5, 9.5), brush.uniform(-9.5, 9.5)
        if abs(cx + 0.63 * cz - 0.74 * sin(cz * 0.64) - 0.22) < 1.0:
            continue
        ox, oz = (cx / 25 + 0.5) * SIZE, (cz / 25 + 0.5) * SIZE
        radius = brush.uniform(10, 34)
        phase = brush.uniform(0, 2 * pi)
        reach = int(radius * 1.6)
        sy = slice(max(0, int(oz - reach)), min(SIZE, int(oz + reach + 1)))
        sx = slice(max(0, int(ox - reach)), min(SIZE, int(ox + reach + 1)))
        yy, xx = np.mgrid[sy, sx]
        dx, dz = (xx - ox) / radius, (yy - oz) / (radius * brush.uniform(0.61, 0.94))
        theta = np.arctan2(dz, dx)
        lobes = 1 + 0.16 * np.sin(3 * theta + phase) + 0.11 * np.cos(5 * theta - phase)
        shell = np.sqrt(dx * dx + dz * dz) / lobes + grain_fine[sy, sx] * 0.07
        cover = (1 - smooth(0.79, 1.03, shell)) * brush.uniform(0.47, 0.75)
        rim = np.exp(-(((shell - 0.83) / 0.14) ** 2)) * np.clip(
            -0.4 * dx - 0.8 * dz, 0, 1
        )
        shade = np.exp(-(((shell - 0.98) / 0.12) ** 2)) * np.clip(0.2 + 0.8 * dz, 0, 1)
        c = brush.choice(
            ((115, 140, 111), (137, 154, 105), (127, 146, 122), (145, 151, 108))
        )
        tint = (
            np.array(c)
            + (11 * grain_fine[sy, sx] + 7 * grain_mid[sy, sx] + 25 * rim)[:, :, None]
        )
        rgb[sy, sx] += (tint - rgb[sy, sx]) * cover[:, :, None] - 16 * shade[:, :, None]
    edge = np.sqrt((x / 1.05) ** 2 + (z * 1.04) ** 2) + pools * 3
    alpha = (1 - smooth(7.6, 11.5, edge)) * (0.83 + grain_mid * 0.12)
    rgba = np.dstack((rgb, alpha * 255)).clip(0, 255).astype(np.uint8)
    result = Image.fromarray(rgba)
    # Overpaint fine leaf shadows, narrow blade tips and mineral chips. Their
    # original local wash opacity is preserved, including the invisible border.
    pen = ImageDraw.Draw(result)
    ink = Random(20261118)
    for _ in range(11500):
        u, v = ink.randrange(2, SIZE - 16), ink.randrange(2, SIZE - 16)
        color = rgba[v, u].astype(int)
        if not color[3]:
            continue
        angle = (
            -0.66 + 0.63 * sin(u / 68) + 0.41 * cos(v / 131) + ink.uniform(-1.0, 1.0)
        )
        length = ink.uniform(1.5, 7.0)
        dx, dy = length * cos(angle), length * sin(angle)
        light = ink.choice((-22, -14, -9, 11, 18, 24))
        pigment = tuple(int(np.clip(c + light, 0, 255)) for c in color[:3]) + (
            int(color[3]),
        )
        pen.line((u, v, u + dx, v + dy), fill=pigment, width=ink.choice((1, 1, 2)))
        if length > 6 and clay[v, u] < 0.4:
            pen.line(
                (u, v, u + length * cos(angle - pi / 4), v + dy * 0.34), fill=pigment
            )
    return result


if __name__ == "__main__":
    OUT.parent.mkdir(parents=True, exist_ok=True)
    wash().save(OUT)
