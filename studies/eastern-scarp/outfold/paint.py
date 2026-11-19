"""Artist's four exposed foliation windows, not strata spanning the whole headland.

An elevation sheet on the existing native face: a pale uplift, an ochre crumbling
shoulder, a sparse exposed block and their broken joints. Actual descending
paths only control the distribution of oxidation, not the shape of the land.
"""

from pathlib import Path
import json, sys
import numpy as np
from PIL import Image

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parents[2] / "scripts"), str(H.parents[1] / "lanternback-rim")]
from island_inlet_ground import ease
from strata import nz

# The first, stepped exposure appears chiefly in the shore standing eye. A
# second, larger low lens stands alone farther along the native wall. Keeping
# their directions unrelated prevents geology from turning into terrace lines.
WINDOWS = (
    (-101.6, 7.75, 7.6, 4.2, 0.38, "pale"),
    (-123.0, 11.2, 4.7, 7.0, -0.54, "oxide"),
    (-148.3, 4.9, 8.7, 3.0, 0.17, "blue"),
    (-85.8, 15.4, 3.1, 3.0, -0.37, "pale"),
)


def paint(source, data, out):
    out.mkdir(parents=True, exist_ok=True)
    sheet = json.loads((source / "face.json").read_text())
    u0, u1, v0, v1 = sheet["side"]
    n, m = 1388, 480
    u, y = np.meshgrid(
        u0 + (np.arange(n, dtype="f") + 0.5) * (u1 - u0) / n,
        v1 - (np.arange(m, dtype="f") + 0.5) * (v1 - v0) / m,
    )
    path = json.loads(data.read_text())["catchments"]
    macro = nz(u, y, 3.57, 616) - 0.5
    mid = nz(u, y, 0.94, 627) - 0.5
    grain = nz(u, y, 0.29, 631) - 0.5
    b = u + 0.75 * macro + 0.22 * mid
    v = y + 0.52 * macro + 0.16 * mid
    oxide = np.zeros_like(v)
    for trace in path:
        row = np.array(trace["line"], "f")[::-1]
        # Interpolate only on the tracked, true declining limb. The warm
        # weathering vanishes again away from its real gravitational catchment.
        centre = np.interp(v.ravel(), row[:, 1], row[:, 2]).reshape(v.shape)
        sig = 0.28 + 0.55 * (1 - ease(2.4, 17, v))
        lateral = np.abs(b - centre)
        form = np.exp(-np.square(lateral / (sig + 0.72)))
        strength = 0.15 + 0.66 * ease(-0.08, 0.24, mid + 0.46 * macro)
        oxide = np.maximum(
            oxide, form * strength * ease(1.15, 4, v) * (1 - ease(15.0, 20, v))
        )
    strength = np.zeros_like(v)
    layer = np.zeros((m, n, 3), "f")
    kern = np.zeros_like(v)
    rough = np.zeros_like(v)
    for j, (cz, cy, wx, wy, tilt, stone) in enumerate(WINDOWS):
        a = (b - cz) / wx
        c = (v - cy + tilt * (b - cz)) / wy
        tip = np.maximum(a * 0.58 + c * 0.81, -0.87 * a - 0.33 * c)
        foot = np.maximum(-c * 0.79 + a * 0.27, a * 0.93 - c * 0.19)
        q = np.maximum(tip, foot) + 0.17 * np.abs(a + c)
        outer = 1 - ease(0.60, 1.0, q + 0.25 * macro + 0.17 * mid)
        # Abrupt exposed facets inside, softened into moss/dust at outer lip.
        # Outcrops are knocked back on one side only; they are never all rings.
        inset = ease(0.13, 0.43, (1 - q) + 0.24 * macro - 0.09 * mid)
        # The first camera board made these four islands look like swipes of
        # a wide airbrush. Erode each exposure into local interrupted fabrics;
        # leave old rock through the interior, not only at the perimeter.
        cu = u + 0.22 * y + 0.53 * mid
        cv = y - 0.20 * u + 0.36 * macro
        broad = nz(cu * 0.76, cv * 1.18, 2.27, 761 + j * 4)
        broken_bed = nz(cu * 0.68, cv * 1.43, 0.88, 762 + j * 4)
        pebbled = nz(cu, cv, 0.34, 763 + j * 4)
        gap = ease(0.34, 0.55, 0.53 * broad + 0.35 * broken_bed + 0.12 * pebbled)
        core = outer * (0.31 + 0.69 * inset) * (0.24 + 0.76 * gap)
        if stone == "pale":
            face = np.array([176.0, 166.0, 144.0])
            shadow = np.array([113.0, 115.0, 108.0])
        elif stone == "oxide":
            face = np.array([163.0, 137.0, 103.0])
            shadow = np.array([107.0, 110.0, 102.0])
        else:
            face = np.array([146.0, 152.0, 143.0])
            shadow = np.array([99.0, 113.0, 116.0])
        # Sparse changes of plane; not band frequency used as texture.
        pass1 = np.maximum(0, a * 0.76 + c * 0.32 + 0.29 * macro)
        pass2 = np.maximum(0, -a * 0.46 + c * 0.90 - 0.20 * mid)
        shine = np.clip(
            0.43 + 0.30 * (1 - pass1) + 0.38 * pass2 + 0.16 * macro + 0.10 * mid, 0, 1
        )
        local = shadow + (face - shadow) * shine[..., None]
        local += (9.6 * grain + 8.2 * mid + 10.5 * (broken_bed - 0.5))[..., None]
        # A few connected, non-parallel joint paths meet within each exposure;
        # clipped to one flank. Model is a very shallow removed recess.
        sx = 0.65 * a + 0.30 * c + 0.075 * mid
        sy = -0.19 * a + 0.73 * c + 0.04 * mid
        ax = np.abs(sx - 0.02) * wx
        bx = np.abs(sy - 0.45 * sx - 0.15) * wy
        cx = np.abs(sy + 0.25 * sx + 0.14) * wy
        long = np.exp(-((ax / 0.075) ** 2)) * (1 - ease(0.43, 0.84, -c))
        spur = np.exp(-((bx / 0.055) ** 2)) * ease(-0.10, 0.24, a)
        small = np.exp(-((cx / 0.045) ** 2)) * (1 - ease(-0.21, 0.10, a))
        ink = np.maximum.reduce((long, spur, small)) * (
            0.54 + 0.44 * ease(-0.29, 0.22, mid)
        )
        glint = (
            np.exp(-(((ax - 0.135) / 0.096) ** 2)) * (1 - ease(0.05, 0.63, a)) * 0.32
        )
        local += (-34 * ink + 22 * glint)[..., None]
        rust = oxide * core * (0.44 + 0.36 * ease(-0.24, 0.20, macro))
        local = (
            local * (1 - rust[..., None])
            + np.array([142.0, 103.0, 72.0]) * rust[..., None]
        )
        w = core * (0.92 if j == 0 else 0.88)
        contrib = w * (1 - strength)
        layer += contrib[..., None] * local
        kern += contrib * (0.19 * shine + 0.13 * macro + 0.08 * mid - 0.57 * ink)
        rough += contrib * (0.75 + 0.075 * mid - 0.18 * rust)
        strength += contrib
    col = layer / np.maximum(1e-6, strength[..., None])
    # Alpha is a coverage field. Preserve invisible RGB at zero rather than
    # letting a hidden black matte bleed into the soft patch edge on resizing.
    empty = strength < 0.0009
    col[empty] = [134, 137, 124]
    rgba = np.clip(np.dstack([col, strength * 255]), 0, 255).astype("uint8")
    Image.fromarray(rgba).save(out / "exposure-colour.png", optimize=True)
    ht = np.uint8(np.clip(125 + 126 * kern, 0, 255))
    Image.fromarray(ht).save(out / "joint-height.png", optimize=True)
    Image.fromarray(
        np.uint8(np.clip(255 * rough / np.maximum(0.004, strength), 0, 255))
    ).save(out / "face-roughness.png", optimize=True)
    (out / "outfold.json").write_text(
        json.dumps(
            dict(
                side=sheet["side"],
                plan=sheet["plan"],
                window_support=[
                    dict(
                        native_z=k[0],
                        height=k[1],
                        nominal_along_width=k[2],
                        nominal_height=k[3],
                        tilt=k[4],
                        type=k[5],
                    )
                    for k in WINDOWS
                ],
                exposure_coverage_percent=round(
                    100 * float(np.mean(strength > 0.15)), 2
                ),
                catchments=len(path),
                simulated_flow=False,
                vertex_change=0,
                art_status="Evaluate from the matched real cameras; no native registration",
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    paint(*[Path(v) for v in sys.argv[1:4]])
