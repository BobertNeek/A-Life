"""Wrapped close PBR skin for the actual island stone props; no light baked in."""

from pathlib import Path
import numpy as np
from PIL import Image
from island_habitat_tiles import noise, panel_noise
from island_geology_paint import ramp

N = 512
METRES = 1.35


def skins(out):
    out.mkdir(parents=True, exist_ok=True)
    y, x = (np.indices((N, N), dtype="f") + 0.5) / N
    rock = noise(3, 8301, x, y)
    weather = noise(8, 2351, x, y)
    marl = noise(22, 7931, x, y)
    fine = noise(59, 7141, x, y)
    mica = noise(162, 3197, x, y)
    # Independent faults part only where a change in adjacent planes makes a
    # joint. Most rock remains quiet. Rinds weather away in broad connected
    # pieces rather than putting a black grid or white vein around each cell.
    sx = x + 0.024 * rock + 0.006 * marl
    sy = y + 0.029 * weather + 0.005 * marl
    seam, plane = panel_noise(sx, sy, 8, 5, 75329)
    fault = (1 - ramp(0.018, 0.081, seam)) * ramp(-0.35, 0.55, rock + 0.61 * weather)
    secondary, _ = panel_noise(x - 0.029 * rock, y + 0.03 * rock, 15, 11, 4106)
    hair = (1 - ramp(0.009, 0.042, secondary)) * ramp(
        0.12, 0.57, 0.62 * rock + 0.58 * marl
    )
    oxide = ramp(0.04, 0.85, 0.48 * rock + 0.62 * weather + 0.17 * marl)
    quartz = (1 - ramp(0.014, 0.048, seam)) * ramp(0.15, 0.58, -rock + 0.3 * marl)
    split = ramp(0.34, 0.67, 0.54 * weather + 0.42 * marl + 0.34 * fine)
    mica_chip = ramp(0.70, 0.94, mica + 0.11 * fine) * ramp(-0.28, 0.48, weather)
    h = (
        0.0043 * plane * (0.42 + 0.58 * ramp(-0.35, 0.35, weather))
        - 0.0052 * fault
        - 0.0023 * hair
        + 0.0019 * quartz
        - 0.0014 * split
        + 0.00024 * fine
        + 0.00017 * mica
    )
    rgb = (
        np.array((224.0, 228.0, 230.0))
        + (
            12 * plane
            + 6.2 * marl
            + 2.8 * fine
            + 2.2 * mica
            - 27 * fault
            - 12 * hair
            - 8 * split
            + 20 * mica_chip
        )[..., None]
        + oxide[..., None] * np.array((13.0, -1.5, -17.0))
        + quartz[..., None] * np.array((21.0, 21.0, 14.0))
    )
    # Here both the glTF v coordinate and image row grow together. A slope
    # rising along either tangent perturbs the normal in its negative direction.
    sx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) * (N / (2 * METRES))
    sy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) * (N / (2 * METRES))
    normal = np.stack((-sx, -sy, np.ones_like(h)), axis=-1)
    normal /= np.linalg.norm(normal, axis=-1)[..., None]
    color = lambda v: np.clip(np.rint(v), 0, 255).astype("u1")
    Image.fromarray(color(rgb)).save(out / "fracture.png", compress_level=6)
    Image.fromarray(color(127.5 * (normal + 1))).save(
        out / "fracture-normal.png", compress_level=6
    )
    rough = np.clip(
        0.74
        + 0.101 * oxide
        + 0.068 * fault
        + 0.025 * marl
        - 0.19 * quartz
        - 0.16 * mica_chip,
        0.38,
        0.96,
    )
    orm = np.stack((1 - 0.23 * fault - 0.12 * hair, rough, np.zeros_like(h)), axis=-1)
    Image.fromarray(color(orm * 255)).resize((256, 256), Image.Resampling.BOX).save(
        out / "fracture-orm.png", compress_level=6
    )
