"""Bake a single inward-facing coastal day envelope for close and orthographic views.

One unlit opaque draw without a collision, shadow, per-frame motion or post pass.
Clouds are painted in continuous 3D direction space, so the azimuth seam closes.
"""

import argparse
import io
import json
import math
from pathlib import Path
import struct

import numpy as np
from PIL import Image
from scipy.ndimage import map_coordinates

ROOT = Path(__file__).resolve().parents[1]
ISLAND = ROOT / "crates/alife_game_app/assets/landscape/island"
ELEVATION_MIN = math.radians(-2.0)
RADIUS = 2800.0


def ease(a, b, v):
    t = np.clip((v - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def mix(a, b, t):
    return a + (b - a) * t[..., None]


def noise(x, y, z, salt):
    volume = np.random.default_rng(salt).uniform(-1, 1, (24, 24, 24)).astype("f")
    return map_coordinates(volume, [x + 10, y + 8, z + 12], order=3, mode="grid-wrap")


def paint(palette):
    w, h = 1536, 384
    y, x = np.indices((h, w), dtype="f")
    a = (x + 0.5) / w * math.tau - math.pi
    elev = math.pi / 2 + (y + 0.5) / h * (ELEVATION_MIN - math.pi / 2)
    u, v, q = np.cos(a) * np.cos(elev), np.sin(elev), np.sin(a) * np.cos(elev)
    sun = -np.array(palette["sun_ray_direction"])
    sun /= np.linalg.norm(sun)
    facing = np.clip(
        (u * sun[0] + q * sun[2]) / (np.hypot(sun[0], sun[2]) + 0.0001), -1, 1
    )
    warmth = (facing * 0.5 + 0.5) ** 2
    # Celadon atmospheric mixing at the horizon gives way to blue overhead.
    horizon = mix(np.array([191, 210, 212]), np.array([248, 208, 159]), warmth)
    upper = mix(np.array([77, 136, 175]), np.array([116, 157, 184]), warmth * 0.60)
    sky = mix(horizon, upper, np.clip(v, 0, 1) ** 0.62)
    # Low cumulus is gathered into broad cells with a flat, warm-lit underside.
    wobble = noise(u * 3.2, v * 5.5, q * 3.2, 583)
    texture = (
        0.56 * noise(u * 8.2 + v * 1.7, v * 23, q * 8.2, 146)
        + 0.28 * noise(u * 19 + v * 2, v * 46, q * 19, 863)
        + 0.12 * noise(u * 39, v * 84, q * 39, 212)
        + 0.04 * noise(u * 67, v * 147, q * 67, 181)
    )
    low = 0.066 + 0.023 * wobble + 0.018 * noise(u * 6, v * 4, q * 6, 478)
    thick = 0.07 + 0.13 * ease(-0.67, 0.67, wobble * 0.68 + texture * 0.65)
    profile = np.clip((v - low) / np.maximum(thick, 0.025), 0, 1)
    volume = np.maximum(0, np.sin(profile * math.pi)) ** 0.75
    mask = ease(0.34, 0.59, volume * (0.78 + 0.30 * wobble) + texture * 0.64)
    mask *= ease(0.023, 0.073, v) * (1 - ease(0.25, 0.54, v))
    light = ease(0.02, 0.95, profile * 0.82 + texture * 0.48 + warmth * 0.24)
    shade = mix(np.array([143, 161, 174]), np.array([193, 170, 153]), warmth * 0.67)
    top = mix(np.array([232, 234, 219]), np.array([254, 226, 179]), warmth)
    sky = mix(sky, mix(shade, top, light), mask * 0.82)
    # Thin wind-sheared cirrus is high and intermittent. Keep generous blue gaps.
    wisps = noise(u * 6.0 + q * 2, v * 41 + q * 3, q * 5.3, 531) + 0.28 * noise(
        u * 17, v * 73, q * 14, 245
    )
    veil = (
        ease(0.33, 1.00, wisps) * ease(0.19, 0.4, v) * (1 - ease(0.55, 0.94, v)) * 0.28
    )
    sky = mix(
        sky, mix(np.array([222, 227, 216]), np.array([247, 217, 185]), warmth), veil
    )
    # All colour approaches one zenith at the pinched pole.
    sky = mix(sky, np.array([86, 142, 178]), ease(0.79, 1, v))
    if not np.isfinite(sky).all():
        raise ValueError("Non-finite sky radiance")
    if np.max(np.abs(sky[:, 0] - sky[:, -1])) > 20:
        raise ValueError("Visible seam in coastal panorama")
    buf = io.BytesIO()
    Image.fromarray(np.uint8(np.clip(np.rint(sky), 0, 255))).save(
        buf, "PNG", optimize=True
    )
    return buf.getvalue()


def build(path, palette):
    img = paint(json.loads(palette.read_text()))
    # Each vertex is in real native X/Y/Z metres, not camera or projected space.
    n, m = 96, 22
    pos = []
    uv = []
    idx = []
    for j in range(m):
        e = ELEVATION_MIN + (math.pi / 2 - ELEVATION_MIN) * j / m
        for i in range(n + 1):
            a = math.tau * i / n - math.pi
            pos.append(
                [
                    RADIUS * math.cos(a) * math.cos(e),
                    RADIUS * math.sin(e),
                    RADIUS * math.sin(a) * math.cos(e),
                ]
            )
            uv.append([i / n, 1 - j / m])
    for j in range(m - 1):
        for i in range(n):
            a = j * (n + 1) + i
            b = a + n + 1
            idx.extend((a, a + 1, b + 1, a, b + 1, b))
    for i in range(n):
        c = len(pos)
        pos.append([0, RADIUS, 0])
        uv.append([(i + 0.5) / n, 0])
        a = (m - 1) * (n + 1) + i
        idx.extend((a, a + 1, c))
    pp = np.asarray(pos, dtype="<f4")
    ix = np.asarray(idx, dtype="<u2").reshape(-1, 3)
    direction = np.cross(pp[ix[:, 1]] - pp[ix[:, 0]], pp[ix[:, 2]] - pp[ix[:, 0]])
    assert np.all((direction * pp[ix[:, 0]]).sum(axis=1) < 0)
    parts = [pp.tobytes(), np.asarray(uv, "<f4").tobytes(), ix.tobytes(), img]
    views = []
    data = bytearray()
    for i, p in enumerate(parts):
        data.extend(bytes(-len(data) % 4))
        views.append(dict(buffer=0, byteOffset=len(data), byteLength=len(p)))
        if i < 3:
            views[-1]["target"] = 34963 if i == 2 else 34962
        data.extend(p)
    mesh = dict(
        name="Marine sky",
        primitives=[
            dict(attributes=dict(POSITION=0, TEXCOORD_0=1), indices=2, material=0)
        ],
    )
    doc = dict(
        asset=dict(version="2.0", generator="Qualia marine-layer sky"),
        extensionsUsed=["KHR_materials_unlit"],
        scene=0,
        scenes=[dict(nodes=[0])],
        nodes=[dict(name="Marine sky", mesh=0)],
        meshes=[mesh],
        materials=[
            dict(
                name="Marine clear and cloud",
                doubleSided=False,
                extensions=dict(KHR_materials_unlit={}),
                pbrMetallicRoughness=dict(
                    baseColorTexture=dict(index=0), metallicFactor=0, roughnessFactor=1
                ),
            )
        ],
        images=[
            dict(bufferView=3, mimeType="image/png", name="Directional marine day")
        ],
        samplers=[dict(wrapS=10497, wrapT=33071, magFilter=9729, minFilter=9987)],
        textures=[dict(source=0, sampler=0)],
        accessors=[
            dict(
                bufferView=0,
                componentType=5126,
                type="VEC3",
                count=len(pos),
                min=pp.min(0).tolist(),
                max=pp.max(0).tolist(),
            ),
            dict(bufferView=1, componentType=5126, type="VEC2", count=len(pos)),
            dict(bufferView=2, componentType=5123, type="SCALAR", count=ix.size),
        ],
        bufferViews=views,
        buffers=[dict(byteLength=len(data))],
    )
    raw = json.dumps(doc, separators=(",", ":")).encode()
    raw += b" " * (-len(raw) % 4)
    data += bytes(-len(data) % 4)
    with path.open("wb") as out:
        out.write(
            struct.pack(
                "<4sIII4s", b"glTF", 2, len(raw) + len(data) + 28, len(raw), b"JSON"
            )
        )
        out.write(raw)
        out.write(struct.pack("<I4s", len(data), b"BIN\0"))
        out.write(data)
    print(
        dict(
            triangles=ix.shape[0],
            bytes=path.stat().st_size,
            radius_m=RADIUS,
            detail="unlit, no dynamic cloud computation",
        )
    )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--out", type=Path, default=ISLAND / "marine-day.glb")
    cli.add_argument("--palette", type=Path, default=ISLAND / "coastal-daylight.json")
    a = cli.parse_args()
    build(a.out, a.palette)
