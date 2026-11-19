"""Field art for the real southern V1 inlet. Uses actual triangulated ground, not shore height stripes."""

from pathlib import Path
import math, struct
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy.ndimage import gaussian_filter, distance_transform_edt

# The cove is a sandy margin; this is the other, genuinely steep tidal cut.
X0, X1, Z0, Z1 = 56.25, 137.50, 387.50, 459.375
W, H = 1040, 920


def ease(a, b, x):
    q = np.clip((x - a) / (b - a), 0, 1)
    return q * q * (3 - 2 * q)


class Inlet:
    def __init__(self, source):
        buf = Path(source).read_bytes()
        w, d, self.x0, self.z0, self.s, _ = struct.unpack_from("<IIfffI", buf, 8)
        self.h = np.frombuffer(buf, "<f4", w * d, 32).reshape(d, w)

    def floor(self, x, z):
        u = (x - self.x0) / self.s
        v = (z - self.z0) / self.s
        i = np.floor(u).astype(int)
        j = np.floor(v).astype(int)
        u -= i
        v -= j
        a, b, c, d = (
            self.h[j, i],
            self.h[j, i + 1],
            self.h[j + 1, i + 1],
            self.h[j + 1, i],
        )
        dx, dz = np.where(u >= v, b - a, c - d), np.where(u >= v, c - b, d - a)
        return a + dx * u + dz * v, dx / self.s, dz / self.s


def image(a):
    return Image.fromarray(np.uint8(np.clip(np.rint(a), 0, 255)))


def sheet(xx, zz, rng, scale):
    """Nonperiodic smooth paint fields, specified in metres."""
    s = max(scale * W / (X1 - X0), 0.5)
    n = gaussian_filter(rng.normal(0, 1, xx.shape).astype("f"), s, mode="reflect")
    return (n - n.mean()) / max(0.001, n.std())


def xy(x, z):
    return ((x - X0) * W / (X1 - X0), (z - Z0) * H / (Z1 - Z0))


def paint(src, folder):
    dest = Path(folder)
    dest.mkdir(exist_ok=True, parents=True)
    coast = Inlet(src)
    rng = np.random.default_rng(1907432)
    zz, xx = np.mgrid[0:H, 0:W]
    xx = X0 + (xx + 0.5) * (X1 - X0) / W
    zz = Z0 + (zz + 0.5) * (Z1 - Z0) / H
    ground, gx, gz = coast.floor(xx, zz)
    grade = gaussian_filter(np.hypot(gx, gz), 12)
    true_water = ground < -0.10
    wet_distance = distance_transform_edt(~true_water) * ((X1 - X0) / W)
    broad = sheet(xx, zz, rng, 2.65)
    vein = sheet(xx, zz, rng, 0.59)
    fine = sheet(xx, zz, rng, 0.16)
    grain = rng.normal(0, 1, (H, W))
    scale = (X1 - X0) / W
    # Actual drainage follows each original bank downhill. Moist hollows,
    # isolated rusty spots and grey seams do not respect an invented level.
    gullies = Image.new("L", (W, H))
    pen = ImageDraw.Draw(gullies)
    seeds = []
    for _ in range(2400):
        px, pz = rng.uniform(X0 + 4, X1 - 4), rng.uniform(Z0 + 3, Z1 - 5)
        h, dx, dz = coast.floor(np.array(px), np.array(pz))
        if not 5.2 < h < 9.7 or math.hypot(dx, dz) < 0.39:
            continue
        if any(math.hypot(px - x, pz - z) < rng.uniform(5.6, 8.0) for x, z in seeds):
            continue
        span, phase = rng.uniform(0.085, 0.27), rng.uniform(0, math.tau)
        points = []
        x, z = px, pz
        for k in range(65):
            g, dx, dz = coast.floor(np.array(x), np.array(z))
            if g < 0.08 or not X0 + 0.6 < x < X1 - 0.6 or not Z0 + 0.6 < z < Z1 - 0.6:
                break
            ln = math.hypot(dx, dz)
            if ln < 0.13:
                break
            turn = 0.21 * math.sin(phase + 0.24 * k) + 0.09 * math.sin(phase + 1.2 * k)
            points.append(xy(x, z))
            vx, vz = -dx / ln, -dz / ln
            x += 0.28 * (vx - vz * turn)
            z += 0.28 * (vz + vx * turn)
        if len(points) < 11:
            continue
        seeds.append((px, pz))
        for k in range(1, len(points)):
            fade = math.sin(math.pi * k / len(points)) ** 0.47
            pen.line(
                (points[k - 1], points[k]),
                fill=int(205 * fade),
                width=max(1, round((0.43 + fade) * span / scale)),
            )
        if len(seeds) >= 13:
            break
    rill = np.array(gullies.filter(ImageFilter.GaussianBlur(1.65))) / 255
    damp = (
        (1 - ease(0.15, 3.3 + 0.67 * broad + 0.3 * vein, ground)) * 0.72
        + 0.38 * rill
        + 0.08 * vein
    )
    damp = np.clip(damp, 0, 1)
    mott = np.clip(0.5 + 0.22 * broad + 0.18 * vein, 0, 1)
    clay = np.array([150, 128, 94])
    cold = np.array([122, 129, 119])
    rock = np.array([134, 138, 128])
    pigment = clay + (cold - clay) * mott[..., None]
    stone = ease(0.48, 1.18, grade) * ease(-0.43, 1.12, vein + fine * 0.21)
    pigment = pigment * (1 - stone[..., None]) + rock * stone[..., None]
    iron = ease(0.78, 1.7, broad * 0.58 + vein * 0.63) * (1 - 0.68 * damp)
    pigment += iron[..., None] * [15, -4, -13]
    from island_inlet_alluvium import alluvium

    lens, gravel, deposit_relief = alluvium(xx, zz, ground, gx, grade, fine, vein, rng)
    pigment = pigment * (1 - lens[..., None]) + gravel * lens[..., None]
    pigment += (4.2 * fine + 2.3 * grain)[..., None]
    pigment *= 1 - damp[..., None] * np.array([0.32, 0.24, 0.20])
    pigment -= rill[..., None] * [16, 11, 7]
    # A few subdued river-sorted, thumb-to-hand-size stones. Establish their
    # flat faces in the albedo and height field, not as new barriers to physics.
    color = image(pigment)
    brush = ImageDraw.Draw(color)
    relief = Image.new("L", (W, H), 128)
    stonebrush = ImageDraw.Draw(relief)
    sites = np.where((ground > 0.2) & (ground < 8.5) & (grade > 0.40))
    for i in range(2450):
        n = rng.integers(len(sites[0]))
        y, x = sites[0][n], sites[1][n]
        density = (
            0.10
            + 0.38 * max(0, float(mott[y, x] - 0.40))
            + 0.73 * damp[y, x]
            + 0.56 * lens[y, x]
        )
        if rng.random() > density:
            continue
        a = rng.uniform(0, math.tau)
        d = np.array((math.cos(a), math.sin(a)))
        v = np.array((-d[1], d[0]))
        ln = rng.uniform(0.04, 0.25) / scale
        w = ln * rng.uniform(0.34, 0.68)
        # An ellipse painted in plan alone stretches uphill on a steep bank.
        # Project both face-plane brush axes into the horizontal texture.
        slope = np.array([gx[y, x], gz[y, x]])
        grad = np.linalg.norm(slope)
        if grad > 0.01:
            downhill = slope / grad
            squash = 1 - 1 / math.sqrt(1 + grad * grad)
            d -= downhill * float(d @ downhill) * squash
            v -= downhill * float(v @ downhill) * squash
        c = np.array((x, y))
        phase = rng.uniform(-0.3, 0.3)
        poly = [
            tuple(
                c
                + d
                * (math.cos(k * math.tau / 7 + phase) * ln * rng.uniform(0.85, 1.06))
                + v * (math.sin(k * math.tau / 7 + phase) * w)
            )
            for k in range(7)
        ]
        gain = rng.uniform(0.90, 1.13)
        base = np.clip(
            (0.69 * pigment[y, x] + 0.31 * np.array([125, 130, 121]))
            * gain
            * (1 - 0.18 * damp[y, x]),
            0,
            255,
        )
        brush.polygon(poly, tuple(base.astype("u1")))
        brush.polygon(
            [*poly[:3], tuple(c)], tuple(np.uint8(np.clip(base * 1.10, 0, 255)))
        )
        stonebrush.polygon(poly, fill=int(rng.integers(147, 177)))
    rim = np.minimum.reduce((xx - X0, X1 - xx, zz - Z0, Z1 - zz))
    cross_cut = ease(0.30, 0.66, np.abs(gx) / (0.02 + np.hypot(gx, gz)))
    exposure = ease(0.32, 0.78, grade + 0.072 * vein + 0.052 * fine) * cross_cut
    mask = (
        exposure
        * ease(0.12, 0.47, ground)
        * (1 - ease(8.0 + 0.76 * broad, 10.2 + 0.60 * broad, ground))
        * (1 - ease(8.7, 15.1, wet_distance))
        * ease(1.6, 6.7, rim)
    )
    alpha = ease(
        0.19, 0.70, mask + (fine * 0.10 + grain * 0.026) * ease(0.06, 0.44, mask)
    )
    color.putalpha(image(255 * alpha))
    color.save(dest / "inlet-bed.png", optimize=True)
    h = (
        deposit_relief
        + 0.010 * fine
        + 0.004 * vein
        + 0.001 * grain
        - 0.034 * rill
        + (np.asarray(relief, dtype="f") - 128) * 0.00062
    )
    dy, dx = np.gradient(gaussian_filter(h, 0.53), scale)
    nn = np.stack((-dx, -dy, np.ones_like(dx)), axis=-1)
    nn /= np.linalg.norm(nn, axis=-1)[..., None]
    image((nn + 1) * 127.5).save(dest / "inlet-normal.png", optimize=True)
    oc = np.clip(0.93 - 0.26 * rill - 0.07 * damp, 0.52, 1)
    rough = np.clip(0.84 - 0.29 * damp + 0.025 * fine, 0.43, 0.99)
    image(255 * np.stack((oc, rough, np.zeros_like(oc)), axis=-1)).resize(
        (520, 460), Image.Resampling.BOX
    ).save(dest / "inlet-orm.png", optimize=True)
    return dict(
        region_xz=[X0, Z0, X1, Z1],
        runoff_starts=len(seeds),
        approach="Original triangulated cut bank, dabbed pigment and fitted drainage; no modified collision or water level.",
    )
