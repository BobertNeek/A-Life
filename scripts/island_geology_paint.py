"""Bake the island's salt-marsh, broken escarpment and alpine mantle in world metres.

Offline only. Pigments are sRGB triplets chosen as a family; calculations never
write to the source heightfield. The ground texture spans all chunks and all LODs.
"""

import io
import struct

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, map_coordinates, shift


def ramp(lo, hi, x):
    t = np.clip((x - lo) / (hi - lo), 0, 1)
    return t * t * (3 - 2 * t)


def pigment(base, tint, wet):
    return base + (tint - base) * wet[..., None]


def rgb(*v):
    return np.array(v, dtype=np.float32)


def grain(x, z, period, salt):
    """Continuous hashed value noise: tile-independent and stable across strip sizes."""
    u, v = x / period + 0.345, z / period + 0.678
    i, j = np.floor(u).astype(np.int32), np.floor(v).astype(np.int32)
    a, b = u - i, v - j
    a, b = a * a * (3 - 2 * a), b * b * (3 - 2 * b)

    def sample(dx, dz):
        q = (
            (i + dx).astype(np.uint32) * np.uint32(0x9E3779B1)
            + (j + dz).astype(np.uint32) * np.uint32(0x85EBCA77)
            + np.uint32(salt)
        )
        q ^= q >> 16
        q *= np.uint32(0x7FEB352D)
        q ^= q >> 15
        return (q >> 8).astype(np.float32) / (1 << 23) - 1

    t, r = sample(0, 0), sample(0, 1)
    top = t + (sample(1, 0) - t) * a
    bottom = r + (sample(1, 1) - r) * a
    return top + (bottom - top) * b


def fault_cells(u, v):
    """Offset fracture panels on a dipping cliff face, not an XY grid of bricks."""
    i, j = np.floor(u).astype(np.int32), np.floor(v).astype(np.int32)
    near = np.full_like(u, 10.0)
    nxt = near.copy()
    tint = np.zeros_like(u)
    for dz in (-1, 0, 1):
        for dx in (-1, 0, 1):
            a, b = i + dx, j + dz
            # Deterministic panel centres avoid discontinuities at bake-strip boundaries.
            q = a.astype(np.uint32) * np.uint32(1597334677) + b.astype(
                np.uint32
            ) * np.uint32(3812015801)
            q ^= q >> 16
            q *= np.uint32(2246822519)
            fx = ((q & 255).astype(np.float32) + 0.5) / 256
            fz = (((q >> 8) & 255).astype(np.float32) + 0.5) / 256
            dist = (u - a - 0.12 - 0.76 * fx) ** 2 + (v - b - 0.12 - 0.76 * fz) ** 2
            wins = dist < near
            nxt = np.minimum(nxt, np.where(wins, near, dist))
            near = np.minimum(near, dist)
            tint = np.where(wins, fx - fz, tint)
    return np.sqrt(nxt) - np.sqrt(near), tint


class Island:
    def __init__(self, source):
        data = source.read_bytes()
        if data[:8] != b"HLAND001":
            raise ValueError(
                "Unknown terrain geometry; do not repaint a different world."
            )
        w, d, self.x, self.z, self.spacing, _ = struct.unpack_from("<IIfffI", data, 8)
        self.h = np.frombuffer(data, "<f4", w * d, 32).reshape(d, w)
        self.size = ((w - 1) * self.spacing, (d - 1) * self.spacing)
        dz, dx = np.gradient(gaussian_filter(self.h, 0.75), self.spacing)
        self.dx, self.dz = dx, dz
        self.grade = np.hypot(dx, dz)
        self.aspect = (0.78 * dx + 0.63 * dz) / (0.18 + self.grade)
        self.hollow = gaussian_filter(self.h, 4.0) - self.h
        self.shelter = self.sky_shelter()
        self.drains = self.drainage()

    def sky_shelter(self):
        """Static diffuse sky visibility in ravines, no painted directional sun."""
        cover = np.zeros_like(self.h)
        for di, dj in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ]:
            horizon = np.zeros_like(self.h)
            for step in (2, 5, 12, 28):
                surface = shift(self.h, (dj * step, di * step), order=0, mode="nearest")
                horizon = np.maximum(
                    horizon,
                    (surface - self.h) / (step * self.spacing * np.hypot(di, dj)),
                )
            cover += horizon / np.sqrt(1 + horizon * horizon)
        return cover / 8

    def drainage(self):
        """Catchments follow the real downhill gradient; no cosmetic rivers painted uphill."""
        h = self.h
        d, w = h.shape
        cells = np.arange(w * d, dtype=np.int32).reshape(d, w)
        padded = np.pad(h, 1, mode="edge")
        best, target = np.zeros_like(h), cells.copy()
        for dj in (-1, 0, 1):
            for di in (-1, 0, 1):
                if not (di or dj):
                    continue
                hood = padded[1 + dj : 1 + dj + d, 1 + di : 1 + di + w]
                drop = (h - hood) / np.hypot(di, dj)
                mask = drop > best
                # Clamped heights mean an edge cell can only fall to an in-bounds neighbour.
                valid = (
                    (cells // w + dj >= 0)
                    & (cells // w + dj < d)
                    & (cells % w + di >= 0)
                    & (cells % w + di < w)
                )
                mask &= valid
                best[mask] = drop[mask]
                target[mask] = cells[mask] + dj * w + di
        area = np.ones(h.size, dtype=np.float32)
        target = target.ravel()
        for i in np.argsort(h.ravel(), kind="stable")[::-1]:
            if target[i] != i:
                area[target[i]] += area[i]
        return gaussian_filter(np.log1p(area.reshape(d, w)), 0.65)

    def sample(self, field, x, z):
        # Small periodic detail must not displace the world-aligned coast or hills.
        u, v = np.broadcast_arrays(
            (x - self.x) / self.spacing, (z - self.z) / self.spacing
        )
        return map_coordinates(field, [v, u], order=1, mode="nearest", prefilter=False)

    def paint(self, x, z):
        h, g, face, hollow, shelter, drainage = [
            self.sample(f, x, z)
            for f in (
                self.h,
                self.grade,
                self.aspect,
                self.hollow,
                self.shelter,
                self.drains,
            )
        ]
        broad = grain(x, z, 77, 173)
        knot = grain(x, z, 25, 529)
        warp_x = x + 12 * knot + 2.4 * grain(x, z, 8, 310)
        warp_z = z + 10 * grain(x, z, 32, 831)
        field = grain(warp_x, warp_z, 42, 210)
        brush = grain(
            warp_x * 0.81 + warp_z * 0.57, warp_z * 1.7 - warp_x * 0.44, 8, 533
        )
        broken = grain(x, z, 2.7, 337)
        grit = grain(x, z, 0.9, 971)
        cool = ramp(
            -0.9, 0.85, 0.57 * field + 0.25 * brush - 0.32 * face + 0.64 * shelter
        )
        turf = pigment(rgb(150, 162, 110), rgb(94, 138, 103), cool)
        sedge = ramp(
            0.15, 0.85, knot * 0.54 + broad * 0.62 + brush * 0.19 + face * 0.15
        )
        turf = pigment(turf, rgb(185, 171, 126), sedge * 0.67)
        runoff = ramp(2.2, 4.9, drainage + brush * 0.55)
        turf = pigment(turf, rgb(72, 121, 100), runoff * 0.41)
        # Small islands of silver-green growth, not a regular contour at the treeline.
        alpine = ramp(49, 140, h + 12 * broad + 6 * brush)
        turf = pigment(turf, rgb(133, 151, 119), alpine * 0.67)
        turf *= (1 + 0.077 * brush + 0.059 * broken + 0.024 * grit)[..., None]

        # Alternating beds dip across the island; faults offset them on large faces.
        fault = (
            4.8 * grain(x, z, 80, 411)
            + 1.55 * grain(warp_x, warp_z, 21, 127)
            + 0.35 * broken
        )
        bed = h + 0.17 * x - 0.085 * z + fault
        # Long coherent parent units with a few thin resistant seams. A straight
        # sine ramp here turns a mountain into an enormous striped cone.
        unit = grain(bed, bed * 0.026, 18.5, 3209)
        ledge = grain(bed + 1.75 * brush, bed * 0.07, 4.1, 1069)
        intact = ramp(
            -0.48, 0.61, grain(warp_x * 0.60, warp_z, 32, 1461) + 0.23 * brush
        )
        beds = unit * 0.73 + ledge * 0.27
        lamina = ramp(0.65, 0.94, np.cos(bed * 1.43 + 0.72 * broken + 2.8 * unit))
        flank = ramp(
            -0.74,
            0.85,
            0.67 * grain(warp_x, warp_z, 38, 729) + 0.21 * face + 0.32 * beds,
        )
        cliff = pigment(rgb(108, 121, 125), rgb(164, 151, 132), flank)
        cliff = pigment(
            cliff, rgb(194, 180, 151), lamina * (0.09 + 0.16 * flank) * intact
        )
        # Broken vertical joints only read in exposed masses.
        joints = np.abs(
            np.sin((x * 0.64 + z * 0.76 + 3.4 * grain(x, z, 11, 147)) * 0.37)
        )
        joint = (1 - ramp(0.015, 0.155, joints)) * ramp(-0.4, 0.46, brush)
        u = (0.70 * x + 0.62 * z + 0.18 * h + 0.76 * broken) / 10.8
        v = (h + 0.19 * x - 0.07 * z + 1.32 * knot + 0.38 * broken) / 4.3
        seam, panel = fault_cells(u, v)
        cleavage = (1 - ramp(0.012, 0.135, seam)) * (0.22 + 0.78 * intact)
        cliff *= (
            1
            + 0.10 * beds
            - 0.07 * joint
            + 0.074 * broken
            + 0.036 * grit
            - 0.20 * cleavage
            + 0.10 * panel * intact
        )[..., None]
        # Light mineral edges stop and pick up again instead of ringing the island.
        lip = (
            ramp(0.08, 0.16, seam)
            * (1 - ramp(0.16, 0.31, seam))
            * ramp(0.17, 0.75, ledge + intact * 0.34)
        )
        cliff = pigment(cliff, rgb(191, 183, 160), lip * 0.30)
        rock = ramp(
            0.38, 1.12, g + 0.22 * brush + 0.21 * knot + 0.08 * broken + 0.10 * alpine
        )
        # Low-lying rocky headlands also get their own geological read.
        talus = (
            ramp(0.17, 0.69, g)
            * ramp(17, 76, h)
            * (1 - rock)
            * ramp(-0.56, 0.55, knot + hollow * 0.13)
        )
        turf = pigment(turf, rgb(144, 144, 119), talus * 0.66)
        ground = pigment(turf, cliff, rock)
        snow = (
            ramp(128, 200, h - 17 * face + 12 * broad + 7 * brush + 3 * broken)
            * (1 - ramp(1.05, 2.30, g - 0.18 * brush))
            * (0.56 + 0.44 * ramp(-0.8, 0.4, knot + 0.28 * hollow + 0.32))
        )
        ground = pigment(ground, rgb(215, 225, 215), (snow * 0.96))
        # Different deposition on low sheltered sand, cut riverbanks and rock shelves.
        sand = pigment(
            rgb(205, 183, 144),
            rgb(219, 200, 162),
            ramp(-0.58, 0.71, field + 0.29 * brush),
        )
        sand *= (1 + 0.036 * broken + 0.026 * grit)[..., None]
        wet = 1 - ramp(0.25, 2.5, h + 0.28 * broken)
        sand = pigment(sand, rgb(147, 147, 125), wet * 0.85)
        shore_rock = ramp(0.24, 0.84, g + 0.18 * knot + 0.1 * brush)
        tidal = pigment(sand, cliff * 0.87, shore_rock)
        beach = 1 - ramp(
            1.8, 7.4, h + g * 1.75 + 1.3 * field + 0.82 * brush + 0.29 * broken
        )
        ground = pigment(ground, tidal, beach)
        ground *= (1 - 0.23 * shelter)[..., None]
        # Keep the unseen seabed quiet and the compressed atlas economical.
        ground = pigment(ground, rgb(98, 130, 120), 1 - ramp(-9, -1, h))
        # A shallow relief field for tangent-space normals, *not* displaced vertices.
        # Stone has anisotropic joints and partly eroded sills. Turf is fine and
        # soft; sandy shelves have low ripples normal to the actual tideline.
        facets = 1 - abs(grain(warp_x + 1.7 * broken, warp_z * 0.75, 5.5, 2107))
        stone_relief = (
            0.35 * unit
            + 0.45 * ledge
            + 0.15 * lamina * intact
            - 0.16 * joint
            - 0.63 * cleavage
            + 0.32 * panel * intact
            + 0.18 * lip
            + 0.27 * facets
            + 0.13 * broken
            + 0.045 * grit
        )
        turf_relief = 0.075 * brush + 0.053 * broken + 0.031 * grit
        tide_relief = (
            0.065 * np.sin(3.4 * h + 1.4 * brush + 0.35 * broken) + 0.033 * grit
        )
        relief = turf_relief + rock * (stone_relief - turf_relief)
        relief += beach * (1 - shore_rock) * (tide_relief - relief)
        relief *= (1 - 0.62 * snow) * ramp(-1, 0, h)
        return np.clip(ground, 0, 255).astype(np.uint8), relief.astype(np.float32)

    def atlas(self, width=2048):
        pixels = np.empty((width, width, 3), np.uint8)
        relief = np.empty((width, width), np.float32)
        x = (
            self.x
            + (np.arange(width, dtype=np.float32)[None, :] + 0.5) * self.size[0] / width
        )
        for start in range(0, width, 40):
            rows = np.arange(start, min(width, start + 40), dtype=np.float32)
            z = self.z + (rows[:, None] + 0.5) * self.size[1] / width
            pixels[start : start + len(rows)], relief[start : start + len(rows)] = (
                self.paint(x, z)
            )

        def png(pixels):
            out = io.BytesIO()
            Image.fromarray(pixels).save(out, "PNG", optimize=False, compress_level=8)
            return out.getvalue()

        color = png(pixels)
        rz, rx = np.gradient(relief, self.size[1] / width, self.size[0] / width)
        for start in range(0, width, 40):
            rows = np.arange(start, min(width, start + 40))
            z = self.z + (rows[:, None] + 0.5) * self.size[1] / width
            gx, gz = self.sample(self.dx, x, z), self.sample(self.dz, x, z)
            # Tangent follows increasing U; Gram-Schmidt the other surface axis.
            tx = np.sqrt(1 + gx * gx)
            bx = gx * gz / (tx * tx)
            bl = np.sqrt(1 + gz * gz - bx * bx * tx * tx)
            nx, ny = -rx[rows] / tx, -(rz[rows] - bx * rx[rows]) / bl
            length = np.sqrt(nx * nx + ny * ny + 1)
            normal = np.stack((nx / length, ny / length, 1 / length), axis=-1)
            pixels[rows] = np.rint(127.5 * (1 + normal)).clip(0, 255).astype(np.uint8)
        return color, png(pixels)


def ground_micro(size=256):
    """One seamless close-range sample for mineral pores, soil crumbs and roughness.

    The R/G channels have physical meaning (ambient cavity and roughness);
    nothing is stamped into the macro land colour or displaced into the world.
    """
    rng = np.random.default_rng(17011)
    noise = rng.normal(size=(size, size))
    kernel = gaussian_filter(noise, 1.2, mode="wrap") + 0.5 * gaussian_filter(
        noise, 3, mode="wrap"
    )
    kernel = kernel / kernel.std()
    coarse = gaussian_filter(noise, 4, mode="wrap")
    coarse /= coarse.std()
    pockets = ramp(0.32, 1.9, kernel + 0.35 * coarse)
    surface = np.zeros((size, size, 3), dtype=np.uint8)
    surface[:, :, 0] = np.rint(255 * (1 - 0.23 * pockets))
    surface[:, :, 1] = np.rint(255 * (0.90 - 0.18 * pockets + 0.04 * np.tanh(coarse)))
    # glTF B is metalness, correctly zero even on wet sand or mica-like inclusions.
    out = io.BytesIO()
    Image.fromarray(surface).save(out, "PNG")
    return out.getvalue()
