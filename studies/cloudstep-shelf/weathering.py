"""Three *non-repeating*, differently dipping rock contacts on the real shelf face.

A local optical surface proposal. No displaced island vertices or new rocks.
The dark/rust massing attaches to the actual surface normal and sea-side drainage;
the three local contacts cross it in 3D rather than marching across height bands.
"""

from pathlib import Path
import csv, json, math, sys
import numpy as np

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[1] / "scripts"), str(H.parent / "lanternback-rim")]
from island_inlet_ground import Inlet, ease
from strata import nz

BOX = (14.0, 115.0, -437.0, -316.0)
# x/z anchor, world-space plane normal (x,y,z), extent and open-side direction.
# The principal beds are anchored to measured native summit-frame witnesses.
CONTACTS = (
    (71.74, -391.07, [0.76, 0.26, 0.60], 18.0, 1.45),
    (65.12, -385.10, [-0.48, 0.31, 0.82], 16.5, 1.03),
    (86.70, -357.20, [0.79, -0.20, 0.57], 21.0, 1.25),
)


def surface(b, x, z):
    y, gx, gz = b.floor(x, z)
    grade = np.hypot(gx, gz)
    a, d, c, e = BOX
    edge = ease(0.0, 9.0, np.minimum(x - a, d - x)) * ease(
        0.0, 9.0, np.minimum(z - c, e - z)
    )
    multi = nz(x, z, 12.1, 530) - 0.5
    mid = nz(x, z, 3.55, 455) - 0.5
    grain = nz(x, z, 0.60, 706) - 0.5
    # Native upslope drapes increasingly reveal the rocky middle wall.
    # Little of this optical layer reaches the quiet interior terrace.
    mantle = ease(0.43, 1.02, grade + 0.30 * multi + 0.14 * mid) * edge
    normal_depression = np.zeros_like(y)
    lens = np.zeros_like(y)
    iron = np.zeros_like(y)
    fissure = np.zeros_like(y)
    for k, (cx, cz, n, span, hw) in enumerate(CONTACTS):
        cy = b.floor(cx, cz)[0]
        signed = (x - cx) * n[0] + (y - cy) * n[1] + (z - cz) * n[2]
        along = (x - cx) * n[2] - (z - cz) * n[0]
        warp = 1.92 * (nz(x, z, 9.2, 121 + k * 2) - 0.5) + 0.61 * (
            nz(x, z, 1.93, 191 + k) - 0.5
        )
        across = signed + warp
        extent = 1 - ease(
            span * 0.63, span * 1.24, np.abs(along) + np.abs(y - cy) * 0.29
        )
        open_side = 1.0 if k != 1 else -1.0
        shoulder = (
            1 - ease(0.05, hw * 2.6, np.abs(across - open_side * hw * 0.75))
        ) * extent
        intermittent = ease(
            0.11, 0.60, 0.49 + 0.82 * (nz(x, z, 4.35, 964 + k) - 0.5) + 0.45 * grain
        )
        lens = np.maximum(lens, shoulder * intermittent)
        cut = (1 - ease(0.055, 0.34, np.abs(across + open_side * 0.19))) * extent
        fissure = np.maximum(fissure, cut * (0.30 + 0.70 * intermittent))
        ochre = (1 - ease(0.13, hw * 0.97, np.abs(across + open_side * 0.71))) * extent
        iron = np.maximum(
            iron, ochre * ease(0.24, 0.70, 0.52 + 0.94 * mid - 0.23 * grain)
        )
    # An explicitly local rain shadow derived from the native convexity. No
    # regularly spaced rills, no wrapping striped triplanar mountains.
    north = b.floor(x, z + 2.8)[0]
    south = b.floor(x, z - 2.8)[0]
    east = b.floor(x + 2.8, z)[0]
    west = b.floor(x - 2.8, z)[0]
    trough = ease(0.035, 0.43, (north + south + east + west - 4 * y) / 2.8)
    wet = trough * ease(-0.06, 0.36, 0.27 + multi + 0.5 * mid)
    stony = mantle * (0.33 + 0.44 * lens + 0.24 * wet + 0.18 * fissure)
    warm = iron * mantle
    seam = fissure * mantle
    bed = lens * mantle
    # Neutral linear pigment multiplier plus small signed offset: preserve
    # original material's hue and lichen variation everywhere else.
    strength = np.clip(stony, 0, 0.85)
    shade = -0.20 * wet - 0.14 * fissure + 0.11 * multi + 0.08 * mid + 0.04 * grain
    pigment = np.stack(
        (
            0.79 + shade + 0.50 * bed + 0.27 * warm,
            0.83 + shade + 0.44 * bed + 0.08 * warm,
            0.86 + shade + 0.36 * bed - 0.09 * warm,
        ),
        axis=-1,
    )
    gain = 1 + (pigment - 1) * strength[..., None]
    rough = np.clip(0.77 + 0.05 * grain - 0.14 * wet + 0.08 * bed, 0.45, 0.93)
    height = np.clip(
        0.5 + 0.095 * mid + 0.055 * grain + 0.12 * bed - 0.22 * seam, 0.16, 0.86
    )
    return gain, rough, height, strength, bed, warm, seam, wet


def register(native, folder):
    b = Inlet(native / "island-v1.bin")
    r = {}
    for pose in ("summit-window", "sheltered-boles", "shelf-section"):
        with (folder / (pose + "-footprint.csv")).open() as f:
            q = list(csv.DictReader(f))
        x, z = [np.array([float(k[t]) for k in q]) for t in ("x", "z")]
        gain, rough, height, k, bed, iron, seam, wet = surface(b, x, z)
        r[pose] = dict(
            registered_ground_hits=len(x),
            percent_with_material_delta=round(100 * float((k > 0.14).mean()), 2),
            percent_with_broken_light_bed=round(100 * float((bed > 0.22).mean()), 2),
            percent_with_narrow_iron_margin=round(100 * float((iron > 0.26).mean()), 2),
            percent_with_contact_cut=round(100 * float((seam > 0.16).mean()), 2),
        )
    meta = dict(
        bounds=BOX,
        non_repeating_contact_count=3,
        actual_V1_geometry_changes=0,
        paint_is_optical_only=True,
        ground_projection_only=r,
    )
    (folder / "weathering-register.json").write_text(json.dumps(meta, indent=2) + "\n")
    return r


if __name__ == "__main__":
    print(register(*(Path(p) for p in sys.argv[1:])))
