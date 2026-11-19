"""Four hand-composed, ground-connected grass sprays for near-field swales.

Each ink silhouette consists of two to four interleaved rosettes. Individual
ribbons curl back at their tips, lose width and occasionally carry an oatlet.
The render mesh folds each finite spray; no square atlas border is visible.
"""

from pathlib import Path
import math
import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import distance_transform_edt


def draw(dest):
    rng = np.random.default_rng(2911873)
    plates = []
    for slot in range(4):
        scale = 2
        size = 256 * scale
        sheet = Image.new("RGBA", (size, size))
        brush = ImageDraw.Draw(sheet)

        def coords(p):
            return (
                round(size * (0.18 + 0.64 * float(p[0]))),
                round(size * (0.026 + 0.97 * float(p[1]))),
            )

        anchors = [(0.25, 0), (0.49, 0.008), (0.72, -0.014)]
        tone = [
            ((97, 118, 70), (164, 177, 106)),
            ((103, 126, 89), (173, 190, 133)),
            ((103, 119, 70), (180, 174, 103)),
            ((103, 123, 84), (160, 180, 128)),
        ][slot]
        for j in range(56):
            rx, ry = anchors[j % 3]
            root = np.array(
                [rx + rng.normal(0, 0.055), 1 + ry + rng.uniform(-0.014, 0.044)]
            )
            side = rng.choice((-1, 1))
            reach = rng.uniform(0.10, 0.39) * side
            # Some high leaders, mostly overlapping drooping leaves with a
            # common wind. Avoid fan-shaped paper triangles on the horizon.
            h = rng.uniform(0.28, 0.79) * (1.12 if j % 7 == 0 else 1)
            top = np.array([reach + 0.055, -h])
            bend = rng.uniform(0.16, 0.48)
            u = rng.uniform(0.006, 0.020) * (1.32 if j % 9 == 1 else 1)
            tint = rng.uniform(0.84, 1.12)
            low, high = [np.array(v) for v in tone]
            if j % 9 == 0:
                low = np.array([130, 126, 82])
                high = np.array([190, 174, 103])
            prev = None
            for t in np.linspace(0, 1, 17):
                c = root + np.array(
                    [
                        top[0] * (0.17 * t + 0.83 * t**1.42),
                        top[1] * (1.49 * t - (0.49 + bend) * t * t),
                    ]
                )
                tng = np.array(
                    [
                        top[0] * (0.17 + 1.17 * max(0.001, t) ** 0.42),
                        top[1] * (1.49 - 2 * (0.49 + bend) * t),
                    ]
                )
                n = np.array([-tng[1], tng[0]]) / max(0.001, np.linalg.norm(tng))
                w = u * (0.52 + 0.59 * math.sin(math.pi * t) ** 0.63) * (1 - t) ** 0.64
                a, b = c + n * w, c - n * w
                rgb = (low * (1 - t) + high * t) * tint
                if prev is not None:
                    brush.polygon(
                        [coords(prev[0]), coords(a), coords(b), coords(prev[2])],
                        fill=tuple(np.uint8(np.clip(rgb, 0, 255))) + (255,),
                    )
                    if u > 0.008:
                        brush.polygon(
                            [coords(prev[0]), coords(a), coords(c), coords(prev[1])],
                            fill=tuple(np.uint8(np.clip(rgb * 1.085, 0, 255))) + (255,),
                        )
                prev = (a, c, b)
        # Height variation and a few slightly split awns create an irregular
        # top profile; do not stamp a bloom or dry seedhead on every tuft.
        if slot == 2:
            for ix in (0, 1, 2):
                x = [0.27, 0.47, 0.69][ix]
                y = [0.19, 0.24, 0.31][ix]
                brush.line(
                    (coords((x - 0.025, 1)), coords((x + 0.04, y))),
                    fill=(126, 139, 86, 255),
                    width=2 * scale,
                )
                for k in range(4):
                    t = k / 4
                    cx = x + 0.04 + 0.020 * t
                    cy = y - 0.103 * t
                    a = -1 if k % 2 else 1
                    brush.polygon(
                        [
                            coords((cx, cy)),
                            coords((cx + 0.017 * a, cy - 0.016)),
                            coords((cx + 0.026 * a, cy - 0.046)),
                            coords((cx + 0.010 * a, cy - 0.039)),
                        ],
                        fill=(175, 160, 94, 255),
                    )
                    brush.line(
                        (
                            coords((cx + 0.021 * a, cy - 0.035)),
                            coords((cx + 0.036 * a, cy - 0.066)),
                        ),
                        fill=(184, 173, 106, 255),
                        width=scale,
                    )
        # Carry the nearest real pigment into transparent texels so filtered
        # mip edges do not develop a black halo against a bright meadow.
        sheet = sheet.resize((256, 256), Image.Resampling.LANCZOS)
        pix = np.array(sheet)
        mask = pix[..., 3] >= 128
        _, nearest = distance_transform_edt(~mask, return_indices=True)
        pix[~mask, :3] = pix[nearest[0], nearest[1], :3][~mask]
        plates.append(Image.fromarray(pix))
    atlas = Image.new("RGBA", (512, 512))
    for i, p in enumerate(plates):
        atlas.paste(p, ((i % 2) * 256, (i // 2) * 256))
    dest = Path(dest)
    dest.parent.mkdir(exist_ok=True, parents=True)
    atlas.save(dest, optimize=True)
