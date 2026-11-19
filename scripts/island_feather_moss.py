"""Ground-hugging feather moss. Irregular living fingers, not circular green mounds."""

import math
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter
from island_spruce_bark import maps, smooth


def colony(seed):
    rng = np.random.default_rng(seed)
    n = 256
    yy, xx = np.indices((n, n), dtype=float)
    coarse = gaussian_filter(rng.normal(size=(n, n)), 9.0)
    coarse /= coarse.std()
    detail = gaussian_filter(rng.normal(size=(n, n)), 1.7)
    detail /= detail.std()
    grain = rng.uniform(-1, 1, (n, n))
    # Branched streams of wetter soil agree in orientation, but fern allies
    # spread into crevices on either side, not in a stamped circular colony.
    trails = []

    def walk(pt, length, heading, phase, turn):
        out = [np.array(pt, dtype=float)]
        for k in range(length):
            angle = (
                heading
                + turn * math.sin(k * 0.21 + phase)
                + 0.12 * math.sin(k * 0.68 + phase)
            )
            out.append(out[-1] + 3.1 * np.array([math.cos(angle), math.sin(angle)]))
        return np.array(out)

    x0 = rng.uniform(21, 37)
    y0 = rng.uniform(94, 154)
    spine = walk(
        (x0, y0),
        int(rng.integers(52, 67)),
        rng.uniform(-0.26, 0.22),
        rng.uniform(0, 6),
        0.34,
    )
    trails.append((spine, 10.9))
    for j in range(7):
        p = spine[min(len(spine) - 2, 6 + j * 7)]
        tilt = (-1 if j % 2 == 0 else 1) * rng.uniform(0.36, 1.17)
        minor = walk(p, int(rng.integers(9, 26)), tilt, rng.uniform(0, 6), 0.30)
        trails.append((minor, rng.uniform(4.2, 9.1)))
    mass = np.zeros_like(xx)
    points = []
    for branch, width in trails:
        for k, xy in enumerate(branch):
            u = k / (len(branch) - 1)
            r = width * (0.11 + 0.89 * math.sin(math.pi * (0.11 + 0.87 * u)) ** 0.60)
            delta = ((xx - xy[0]) / (r * 1.16)) ** 2 + ((yy - xy[1]) / r) ** 2
            mass = np.maximum(mass, np.exp(-delta * 0.50))
            if k % 2 == 0:
                points.append((xy, r))
    # Dark soil is visible between the stems all the way through the colony.
    support = mass + 0.074 * coarse + 0.081 * detail - 0.12
    alpha = Image.fromarray((255 * smooth((support - 0.31) * 1.84)).astype("u1"))
    base = 0.59 * coarse + 0.41 * detail
    rgb = np.array((99, 110, 75)) + (5 * base + 3 * grain)[..., None]
    h = 0.29 + 0.075 * detail + 0.045 * grain
    col = Image.fromarray(np.clip(rgb, 0, 255).astype("u1"))
    height = Image.fromarray(np.clip(h * 230, 0, 255).astype("u1"))
    co = ImageDraw.Draw(col)
    re = ImageDraw.Draw(height)
    al = ImageDraw.Draw(alpha)
    for p, width in points:
        for _ in range(int(2.0 * width)):
            pt = p + rng.normal(0, width * 0.82, 2)
            # Narrow feather fans grow along the stem, not along texture rows.
            length = rng.uniform(2.2, 7.1)
            a = rng.uniform(0, math.tau)
            v = np.array([math.cos(a), math.sin(a)]) * length
            o = np.array([-v[1], v[0]]) * rng.uniform(0.20, 0.40)
            t = rng.uniform(-1, 1)
            g = rng.uniform(0.83, 1.1)
            tone = np.array((106 + 9 * t, 126 + 10 * t, 78 + 6 * t)) * g

            def line(a, b, c, h, width=1):
                pts = [tuple(a), tuple(b)]
                co.line(pts, fill=tuple(np.clip(c, 0, 255).astype("u1")), width=width)
                re.line(pts, fill=int(h), width=width)
                al.line(pts, fill=239, width=width)

            line(pt, pt + v, tone * 0.84, 92)
            for q in (0.18, 0.40, 0.60, 0.79):
                start = pt + q * v
                for side in (-1, 1):
                    end = (
                        start + v * 0.16 + side * o * (1.1 - q) * rng.uniform(0.6, 1.1)
                    )
                    line(start, end, tone * (0.91 + 0.12 * q), rng.integers(100, 135))
    # No solid rectangular texel at the edge can appear in a reduced mip.
    rim = np.minimum.reduce([xx, yy, n - 1 - xx, n - 1 - yy])
    alpha = Image.fromarray((np.asarray(alpha) * smooth((rim - 3) / 12)).astype("u1"))
    rgb, normal, orm = maps(
        np.asarray(col),
        np.asarray(height) / 160,
        224 + 6 * detail,
        240 + 6 * grain,
        0.69,
    )
    rgb.putalpha(alpha)
    return rgb, normal, orm


def field(p, r, x, z, angle, long, wide, variant, shard):
    from island_copse_mulch import terrain_stamp

    u0, v0 = variant * 256, 512

    gain = round(255 * r.uniform(0.63, 0.85))
    # A stable millimetre order only separates the occasional crossing. Both
    # footprints still trace the exact same terrain triangle, not parallel cards.
    lift = 0.0076 + 0.0009 * shard

    def sample(point, u, v):
        return p.lower(
            point,
            lift,
            ((u0 + 3 + 249 * u) / 1024, (v0 + 3 + 249 * v) / 768),
            (gain, round(0.99 * gain), round(0.97 * gain), 255),
        )

    terrain_stamp(p, x, z, angle, long, wide, sample)


def tuft(p, r, x, z, length, width, angle):
    # A few literal low creeping shoots thicken the densest fingers. No
    # polygon closes over a bare perimeter. Height tops out at 18 mm.
    phase = r.uniform(-0.5, 0.5)
    live = r.uniform(0.78, 1.02)
    greens = ((108, 133, 76), (112, 139, 83), (94, 126, 82), (121, 141, 76))
    from build_island_strandline import tint

    colour = greens[int(r.integers(len(greens)))]
    n = int(r.integers(3, 8))
    for k in range(n):
        drift = angle + r.normal(0, 0.59)
        d = np.array([math.cos(drift), math.sin(drift)])
        c = np.array([-d[1], d[0]])
        o = (
            np.array([x, z])
            + d * r.uniform(-0.41, 0.31) * length
            + c * r.uniform(-0.54, 0.54) * width
        )
        ln = min(0.14, length * r.uniform(0.20, 0.42))
        reach = math.sin(k * 1.91 + phase)

        def point(t):
            return o + d * (t * ln) + c * (0.18 * ln * math.sin(math.pi * t + reach))

        def v(pos, lift, brightness):
            return p.put(*pos, lift, tint(colour, brightness * live))

        base = v(point(0), 0.004, 0.74)
        for j, t in enumerate((0.29, 0.57, 0.81)):
            center = point(t)
            hi = 0.009 + 0.006 * math.sin(math.pi * t)
            stem = v(center, hi, 0.91 + 0.08 * t)
            w = min(0.021, ln * (0.16 - 0.085 * t))
            for side in (-1, 1):
                feather = v(
                    center + d * (0.18 * ln) + c * (w * 0.76 * side), hi + 0.001, 1.08
                )
                edge = v(center - d * (0.03 * ln) + c * (w * side), hi - 0.003, 0.89)
                p.fan(base, edge, stem)
                p.fan(edge, feather, stem)
            base = stem
