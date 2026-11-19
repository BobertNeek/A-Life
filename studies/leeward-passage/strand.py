"""Wind-exposed root of existing bluff alder 559, with a close sediment chart.

The beach is still the ordinary V1 triangle bed. Roots lie embedded in it;
buried intervals are deliberate. The central dry footway never gets a new
vertical obstacle. The outer wet sand textures are born at the real water.
"""

import math
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter
from ground import ramp

BOUNDS = (119.0, 149.0, 439.0, 470.0)
# First point replaced with the actual original #559 stem. Spokes are
# topographic bends around the bluff, never mirrored across the trail.
CROWN = [
    [
        (133.0, 444.8),
        (132.38, 445.36),
        (131.61, 446.08),
        (130.68, 446.33),
        (130.01, 447.17),
        (129.58, 447.73),
        (128.76, 448.13),
        (128.39, 448.89),
        (127.83, 449.31),
    ],
    [
        (133.0, 444.8),
        (132.87, 445.69),
        (132.69, 446.55),
        (132.09, 447.12),
        (131.99, 448.01),
        (131.48, 448.68),
        (131.30, 449.15),
    ],
    [
        (133.0, 444.8),
        (133.52, 445.28),
        (133.82, 446.05),
        (134.43, 446.42),
        (134.37, 447.18),
        (134.94, 447.66),
    ],
    [
        (130.69, 446.33),
        (130.52, 447.27),
        (130.88, 447.91),
        (130.53, 448.42),
        (130.74, 449.08),
        (130.22, 449.67),
    ],
    [
        (129.73, 447.56),
        (129.45, 447.35),
        (128.78, 447.55),
        (128.21, 447.38),
        (127.65, 447.56),
    ],
    [
        (129.01, 448.01),
        (129.20, 448.42),
        (128.88, 448.98),
        (129.14, 449.43),
        (128.95, 449.82),
    ],
    [(132.05, 447.17), (131.83, 447.76), (131.12, 448.14), (130.98, 448.46)],
]


def lines(anchor):
    from scipy.interpolate import PchipInterpolator

    out = []
    for i, knots in enumerate(CROWN):
        p = np.array(knots, float)
        if i < 3:
            p[0] = anchor
        dist = np.r_[0, np.cumsum(np.linalg.norm(np.diff(p, axis=0), axis=1))]
        m = np.linspace(0, dist[-1], max(8, round(dist[-1] * 13)))
        out.append(
            (np.column_stack([PchipInterpolator(dist, p[:, j])(m) for j in (0, 1)]), m)
        )
    return out


def build(reach, placements, folder):
    from flora import Mesh, save_tiles
    import json
    from scipy.spatial import cKDTree

    alder = json.loads(placements.read_text())[559]
    if alder["kind"] != "Broadleaf":
        raise ValueError("The saved bluff alder is missing.")
    anchor = np.array(alder["position"])[[0, 2]]
    paths = lines(anchor)
    R = np.random.default_rng(940110)
    g = Mesh()

    def face(a, b, c):
        A, B, C = (g.xyz[i] for i in (a, b, c))
        if (B[2] - A[2]) * (C[0] - A[0]) - (B[0] - A[0]) * (C[2] - A[2]) < 0:
            b, c = c, b
        g.face(a, b, c)

    def point(xz, h, col):
        return g.pt(*[xz[0], h, xz[1]], col)

    bark = np.array([131.0, 125.0, 113.0])
    for idx, (p, m) in enumerate(paths):
        norm = np.gradient(p, axis=0)
        norm /= np.maximum(0.0001, np.linalg.norm(norm, axis=1)[:, None])
        norm = norm[:, ::-1] * [1, -1]
        prev = None
        phase = R.uniform(0, math.tau)
        trunk = (0.168, 0.136, 0.121, 0.070, 0.055, 0.045, 0.055)[idx]
        for j, (q, dist) in enumerate(zip(p, m)):
            t = dist / m[-1]
            fade = (1 - t) ** 0.70
            w = (
                trunk
                * (0.015 + fade)
                * (0.85 + 0.15 * np.sin(2.9 * dist + phase) + 0.06 * np.sin(7.3 * dist))
            )
            # Lower some sections back into overburden. The remaining visible
            # connected root curves are flattened, barked and asymmetric.
            buried = (
                0.014
                + 0.061 * np.maximum(0, np.sin(dist * 1.8 + phase)) ** 1.6 * t
                + 0.018 * np.maximum(0, np.sin(dist * 3.1 + phase))
            )
            # On the washed lower forks, two cut tongues of sand cross the
            # wooden core. Break the wire-like uninterrupted bow, and expose
            # irregular flakes at the shoulders rather than round tubing.
            buried += (
                0.086
                * (np.maximum(0, np.sin(2.7 * dist + phase + 0.32)) ** 3)
                * (0.30 + 0.70 * t)
            )
            buried += 0.018 * np.maximum(0, np.sin(7.1 * dist + idx)) ** 5
            lift = min(0.069, w * (0.57 + 0.13 * math.sin(dist * 1.4))) - buried
            curr = []
            for k in range(9):
                aa = -math.pi * 0.5 + k * math.pi / 8
                xz = q + norm[j] * w * math.sin(aa) * (1 + 0.11 * np.sin(2 * dist + aa))
                h = (
                    reach.floor(*xz)[0]
                    + lift
                    * np.maximum(0.0, 1.0 - abs(math.sin(aa)) ** 3)
                    * (0.91 + 0.09 * np.sin(11 * aa + 1.4 * dist))
                    - 0.011 * abs(math.sin(aa)) ** 5
                )
                c = (
                    bark
                    * np.array([1.02, 0.99, 0.96])
                    * (
                        0.76
                        + 0.15 * math.cos(aa)
                        + 0.145 * math.sin(aa * 6 + 0.55 * math.sin(dist * 1.9))
                        + 0.06 * np.cos(2.6 * dist)
                    )
                )
                curr.append(point(xz, h, c))
            if prev:
                for k in range(8):
                    face(prev[k], curr[k], curr[k + 1])
                    face(prev[k], curr[k + 1], prev[k + 1])
            prev = curr
    # Small fractured plates of local siltstone, not new boulders. Bury their
    # crooked margins and preserve the original rise where they span a cell.
    beds = [
        (128.6, 448.85, 0.90, 0.34),
        (129.63, 448.31, 0.72, 0.29),
        (127.17, 451.13, 0.97, 0.25),
        (128.9, 452.61, 0.82, 0.23),
        (131.54, 456.27, 1.04, 0.38),
        (134.65, 461.69, 1.33, 0.47),
    ]
    nstones = 0
    for jj, (x, z, L, W) in enumerate(beds):
        u = np.array([math.cos(0.46 + jj * 0.15), math.sin(0.46 + jj * 0.15)])
        v = np.array([-u[1], u[0]])
        for k in range(int(R.integers(7, 18))):
            q = (
                np.array([x, z])
                + u * np.clip(R.normal(0, 0.63), -1.3, 1.3) * L
                + v * np.clip(R.normal(0, 0.44), -1, 1) * W
            )
            l = R.uniform(0.053, 0.152)
            w = l * R.uniform(0.25, 0.58)
            ang = R.normal(0.61, 0.22)
            tangent = np.array([math.cos(ang), math.sin(ang)])
            right = np.array([-tangent[1], tangent[0]])
            col = np.array(
                R.choice(
                    [(153, 149, 133), (173, 161, 139), (148, 145, 129), (175, 168, 147)]
                ),
                float,
            ) * R.uniform(0.91, 1.07)
            old = []
            up = []
            for j in range(6):
                aa = math.tau * j / 6 + R.uniform(-0.17, 0.17)
                dq = tangent * l * math.cos(aa) + right * w * math.sin(aa)
                e = q + dq
                old.append(point(e, reach.floor(*e)[0] - 0.009, col * 0.84))
                top = q + dq * 0.52 + tangent * 0.14 * l
                up.append(
                    point(
                        top,
                        reach.floor(*top)[0]
                        + 0.006
                        + l * (0.06 + 0.025 * math.cos(aa)),
                        col * R.uniform(0.96, 1.05),
                    )
                )
            for j in range(6):
                k2 = (j + 1) % 6
                face(old[j], old[k2], up[k2])
                face(old[j], up[k2], up[j])
            for j in range(1, 5):
                face(up[0], up[j], up[j + 1])
            nstones += 1
    triangles = save_tiles({(7, "root_and_silt"): g}, folder / "bluff-foot.glb")
    x0, x1, z0, z1 = BOUNDS
    w, h = 1056, 1092
    z, x = np.mgrid[:h, :w]
    x = x0 + (x + 0.5) * (x1 - x0) / w
    z = z0 + (z + 0.5) * (z1 - z0) / h
    floor, dx, dz = reach.floor(x, z)

    def noise(n):
        a = gaussian_filter(R.normal(0, 1, x.shape).astype("f"), n)
        return a / max(0.001, np.std(a))

    broad = noise(33)
    marble = noise(6)
    grain = noise(0.57)
    fine = noise(0)
    rootdist = cKDTree(np.vstack([q for q, _ in paths])).query(
        np.stack((x, z), axis=-1)
    )[0]
    snag = np.exp(-0.5 * (rootdist / (0.27 + 0.066 * np.sin(0.73 * x + 0.8 * z))) ** 2)
    origin = np.array([132.4, 449.2])
    lee = np.exp(
        -0.5
        * (
            ((x - origin[0] + (z - origin[1]) * 0.11) / 5.7) ** 2
            + ((z - origin[1]) / 7.0) ** 2
        )
    )
    deflation = np.zeros_like(x)
    for xx, zz, ll, ww in beds:
        kx = x - xx
        kz = z - zz
        a = 0.70
        ax = kx * math.cos(a) + kz * math.sin(a)
        az = -kx * math.sin(a) + kz * math.cos(a)
        deflation = np.maximum(
            deflation,
            np.exp(
                -0.5
                * ((ax / (ll * 1.38)) ** 2 + ((az + 0.1 * marble) / (ww * 1.45)) ** 2)
            ),
        )
    wet = (1 - ramp(0.35, 3.8, floor)) * ramp(-0.03, 0.43, floor)
    edge = (
        ramp(x0, x0 + 3, x)
        * (1 - ramp(x1 - 3, x1, x))
        * ramp(z0, z0 + 3, z)
        * (1 - ramp(z1 - 2, z1, z))
    )
    # Gentle swash ripples follow actual changing height, with intersections
    # broken by wind and capillary patches. No parallel stripes on the bluff.
    ripple = (
        np.maximum(
            0, np.cos((floor * 0.85 + 0.05 * broad + 0.017 * marble) * math.tau / 0.20)
        )
        ** 4
    )
    dry_ripple = (
        0.5 + 0.5 * np.cos(0.9 * x + 7.6 * z + 1.18 * broad + 0.39 * marble)
    ) ** 8
    tex = np.zeros((*x.shape, 3), "f") + np.array([178, 160, 129])

    def mix(c, a):
        nonlocal tex
        a = np.clip(a, 0, 1)[..., None]
        tex = tex * (1 - a) + np.asarray(c) * a

    mix([165, 157, 128], 0.63 * lee)
    mix([144, 127, 100], 0.79 * snag)
    mix([137, 143, 132], 0.65 * deflation)
    mix([161, 157, 139], 0.76 * wet)
    light = np.clip(marble * 0.45 + grain * 0.51 - 0.20, 0, 1.4)
    tex += (
        np.clip(broad, -2.7, 2.7) * 3.6
        + marble * 2.8
        + grain * (3.6 + 4.8 * deflation)
        + fine * 1.6
        + light * 8 * deflation
        + wet * ripple * 10
        + dry_ripple * 3.4
    )[..., None] * [1.06, 1, 0.81]
    alpha = (
        edge
        * ramp(0.03, 0.32, floor)
        * np.clip(0.21 * lee + 0.62 * snag + 0.70 * deflation + 0.65 * wet, 0, 0.93)
    )
    Image.fromarray(np.uint8(np.clip(np.dstack((tex, alpha * 255)), 0, 255))).save(
        folder / "bluff-grain.png"
    )
    return dict(
        near_only_triangles=triangles,
        existing_alder=559,
        actual_stem=alder["position"],
        exposed_root_fingers=len(paths),
        siltstone_fragments=nstones,
        bounds=BOUNDS,
        scope="Ground relief only; terminal close field, no collision or object placement.",
    )
