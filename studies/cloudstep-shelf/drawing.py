"""Small single-core, bounded CPU drawing of the actual original placed meshes.
One set of visibility/lighting/camera data drives both sides. Not a native
screenshot, Blender render or performance measurement. See README for limits.
"""

import json, math, sys, shutil
from pathlib import Path
import numpy as np
from PIL import Image
from asset_scene import unit, traverse


def linear(a):
    return np.where(a <= 0.04045, a / 12.92, ((a + 0.055) / 1.055) ** 2.4)


def display(a):
    return np.uint8(
        np.clip(
            255
            * np.where(
                a <= 0.0031308, 12.92 * a, 1.055 * np.maximum(0, a) ** (1 / 2.4) - 0.055
            )
            + 0.5,
            0,
            255,
        )
    )


def sample(texture, uv, interpolate=True, srgb=False):
    im, wu, wv = texture
    h, w = im.shape[:2]
    xy = uv * np.array([w, h], "f") - 0.5
    base = np.floor(xy).astype("i")
    f = xy - base

    def get(a, b):
        x = base[..., 0] + a
        y = base[..., 1] + b
        x = x % w if wu == 10497 else np.clip(x, 0, w - 1)
        y = y % h if wv == 10497 else np.clip(y, 0, h - 1)
        v = im[y, x].astype("f") / 255
        if srgb:
            v[..., :3] = linear(v[..., :3])
        return v

    if not interpolate:
        return get(0, 0)
    fx = f[..., 0, None]
    fy = f[..., 1, None]
    return (1 - fy) * ((1 - fx) * get(0, 0) + fx * get(1, 0)) + fy * (
        (1 - fx) * get(0, 1) + fx * get(1, 1)
    )


class Board:
    near = 0.065

    def __init__(
        self,
        eye,
        aim,
        size,
        span,
        lens,
        shadow=False,
        day=None,
        folder=None,
        sun_views=(),
    ):
        self.eye = np.asarray(eye, "f")
        fw = unit(np.array(aim, "f") - self.eye)
        right = unit(np.cross(fw, [0, 1, 0]))
        up = np.cross(right, fw)
        self.ax = np.column_stack((right, up, fw)).astype("f")
        self.w, self.h = size
        self.span = span
        self.f = self.w / span if span else self.w * lens / 36
        self.shadow = shadow
        self.z = np.full((self.h, self.w), np.inf, "f")
        self.sun_views = sun_views
        self.tris = 0
        self.fragments = 0
        self.witnesses = {}
        self.shoulder_lod = 0
        if shadow:
            return
        self.sun = -unit(np.array(day["sun_ray_direction"], "f"))
        self.lamp = linear(np.array(day["sun_srgb"], "f"))
        self.fill = linear(np.array(day["sky_fill_srgb"], "f"))
        self.air = linear(np.array(day["air_srgb"], "f"))
        clear = display(linear(np.array(day["clear_srgb"], "f")))
        self.images = np.empty((2, self.h, self.w, 3), "B")
        self.images[:] = clear
        # Exact chart optical multipliers, sampled on original scene fragments.
        self.fields = []
        for name, box in [
            (
                "mountain-gain",
                json.loads((folder / "mountain-charts.json").read_text())[
                    "native_plan_xz"
                ],
            ),
            (
                "forest-shed-gain",
                json.loads((folder / "held-copse.json").read_text())["box_xz"],
            ),
        ]:
            self.fields.append(
                (
                    (
                        np.asarray(Image.open(folder / (name + ".png")).convert("RGB")),
                        33071,
                        33071,
                    ),
                    box,
                    128,
                )
            )

    def admits(self, p, r):
        x, y, z = (p - self.eye) @ self.ax
        if z + r <= self.near:
            return False
        if self.span:
            return abs(x) <= r + self.w / (2 * self.f) and abs(y) <= r + self.h / (
                2 * self.f
            )
        return abs(x) <= r + (z + r) * self.w / (2 * self.f) and abs(y) <= r + (
            z + r
        ) * self.h / (2 * self.f)

    def projection(self, c):
        s = self.f if self.span else self.f / np.maximum(c[:, 2], self.near)
        return np.stack(
            (self.w * 0.5 + c[:, 0] * s, self.h * 0.5 - c[:, 1] * s), axis=-1
        )

    def sunlit(self, world):
        vis = np.ones(len(world), "f")
        for board in self.sun_views:
            c = (world - board.eye) @ board.ax
            uv = board.projection(c) - 0.5
            ij = np.floor(uv).astype("i")
            edge = np.minimum.reduce(
                (ij[:, 0], ij[:, 1], board.w - 2 - ij[:, 0], board.h - 2 - ij[:, 1])
            )
            good = edge > 3
            if not good.any():
                continue
            xx, yy = ij[good].T
            delta = uv[good] - ij[good]
            fx, fy = delta.T
            z = c[good, 2]

            def look(i, j):
                return z <= board.z[yy + j, xx + i] + board.span / board.w * 0.68

            def bilinear(i, j):
                return (
                    (1 - fx) * (1 - fy) * look(i, j)
                    + fx * (1 - fy) * look(i + 1, j)
                    + (1 - fx) * fy * look(i, j + 1)
                    + fx * fy * look(i + 1, j + 1)
                )

            light = 0.48 * bilinear(0, 0) + 0.13 * (
                bilinear(1, 0) + bilinear(-1, 0) + bilinear(0, 1) + bilinear(0, -1)
            )
            wt = np.clip((edge[good] - 3) / 7, 0, 1)
            # The near map refines stem/branch penumbra, without erasing a
            # mountain shadow whose blocker lies outside that smaller map.
            vis[good] = np.minimum(vis[good], 1 - wt * (1 - light))
        return vis

    def send(self, model, normals, colour, uv, tri, texture, cut, tag):
        cam = (model - self.eye) @ self.ax
        tz = cam[tri, 2]
        normal = tz.min(1) > self.near
        self.tris += len(tri)
        pack = np.column_stack((cam, model, normals, colour, uv)).astype("f")
        self.draw(pack, tri[normal], texture, cut, tag)
        # Exact near-plane triangle clipping keeps the terrain touching the eye.
        slivers = []
        for ix in tri[(~normal) & (tz.max(1) > self.near)]:
            ring = []
            for i, j in zip(ix, np.roll(ix, -1)):
                a, b = pack[i], pack[j]
                aa, bb = a[2] > self.near, b[2] > self.near
                if aa:
                    ring.append(a)
                if aa != bb:
                    ring.append(a + (b - a) * ((self.near - a[2]) / (b[2] - a[2])))
            for l in range(1, len(ring) - 1):
                slivers.extend((ring[0], ring[l], ring[l + 1]))
        if slivers:
            p = np.asarray(slivers, "f")
            self.draw(p, np.arange(len(p)).reshape(-1, 3), texture, cut, tag)

    def draw(self, p, tri, texture, cut, tag):
        if not len(tri):
            return
        uv = self.projection(p[:, :3])
        q = uv[tri]
        delta1 = q[:, 1] - q[:, 0]
        delta2 = q[:, 2] - q[:, 0]
        det = delta1[:, 0] * delta2[:, 1] - delta1[:, 1] * delta2[:, 0]
        lo = np.maximum(0, np.floor(q.min(1))).astype("i")
        hi = np.minimum([self.w - 1, self.h - 1], np.ceil(q.max(1))).astype("i")
        keep = (abs(det) > 0.016) & (lo <= hi).all(1)
        ids = np.flatnonzero(keep)
        if not len(ids):
            return
        TX = 16
        # Bin only visible source triangles. In particular the many subpixel
        # guardian-forest leaf tips do not spawn a Python call per triangle.
        lo //= TX
        hi //= TX
        for j in range(lo[ids, 1].min(), hi[ids, 1].max() + 1):
            row = ids[(lo[ids, 1] <= j) & (hi[ids, 1] >= j)]
            if not len(row):
                continue
            for i in range(lo[row, 0].min(), hi[row, 0].max() + 1):
                group = row[(lo[row, 0] <= i) & (hi[row, 0] >= i)]
                if not len(group):
                    continue
                yy, xx = np.mgrid[
                    j * TX : min(self.h, j * TX + TX), i * TX : min(self.w, i * TX + TX)
                ]
                xx = xx.ravel()
                yy = yy.ravel()
                X = xx.astype("f") + 0.5
                Y = yy.astype("f") + 0.5
                for sl in range(0, len(group), 48):
                    ix = group[sl : sl + 48]
                    k = tri[ix]
                    d = det[ix, None]
                    x = X - q[ix, 0, 0, None]
                    y = Y - q[ix, 0, 1, None]
                    b = (x * delta2[ix, 1, None] - y * delta2[ix, 0, None]) / d
                    c = (delta1[ix, 0, None] * y - delta1[ix, 1, None] * x) / d
                    a = 1 - b - c
                    inside = (a >= 0) & (b >= 0) & (c >= 0)
                    if not inside.any():
                        continue
                    if self.span:
                        z = (
                            a * p[k[:, 0], 2, None]
                            + b * p[k[:, 1], 2, None]
                            + c * p[k[:, 2], 2, None]
                        )
                    else:
                        a /= p[k[:, 0], 2, None]
                        b /= p[k[:, 1], 2, None]
                        c /= p[k[:, 2], 2, None]
                        z = 1 / np.maximum(1e-10, a + b + c)
                        a *= z
                        b *= z
                        c *= z
                    good = inside & (z < self.z[yy, xx][None, :])
                    if not good.any():
                        continue
                    if cut:
                        ix0, px = np.nonzero(good)
                        k0 = k[ix0]
                        U = (
                            a[ix0, px, None] * p[k0[:, 0], 12:14]
                            + b[ix0, px, None] * p[k0[:, 1], 12:14]
                            + c[ix0, px, None] * p[k0[:, 2], 12:14]
                        )
                        keep_pixel = sample(texture, U)[:, 3] >= cut
                        good[ix0, px] = keep_pixel
                        if not good.any():
                            continue
                    z = np.where(good, z, np.inf)
                    win = z.argmin(0)
                    pix = np.flatnonzero(good.any(0))
                    winner = win[pix]
                    yi, xi = yy[pix], xx[pix]
                    self.z[yi, xi] = z[winner, pix]
                    self.fragments += len(pix)
                    if self.shadow:
                        continue
                    kk = k[winner]
                    a0 = a[winner, pix, None]
                    b0 = b[winner, pix, None]
                    c0 = c[winner, pix, None]
                    v = (
                        a0 * p[kk[:, 0], 3:]
                        + b0 * p[kk[:, 1], 3:]
                        + c0 * p[kk[:, 2], 3:]
                    )
                    world = v[:, :3]
                    for wp in tuple(self.witnesses):
                        if (wp[0] // TX, wp[1] // TX) == (i, j):
                            at = np.flatnonzero((xi == wp[0]) & (yi == wp[1]))
                            if len(at):
                                self.witnesses[wp] = (
                                    self.subject,
                                    world[at[0]].round(3).tolist(),
                                    tag,
                                )
                    N = unit(v[:, 3:6])
                    base = v[:, 6:9]
                    if texture is not None:
                        base *= sample(texture, v[:, 9:11], srgb=True)[:, :3]
                    if tag == "sky":
                        color = display(base)
                        self.images[:, yi, xi] = color
                        continue
                    l = np.maximum(0, N @ self.sun) * self.sunlit(world)
                    light = self.fill[None, :] * (
                        0.50 + 0.12 * np.maximum(0, N[:, 1, None])
                    ) + self.lamp[None, :] * (0.89 * l[:, None])
                    base *= light
                    haze = 0.15 * np.clip((self.z[yi, xi] - 28) / 215, 0, 1)[:, None]
                    if tag != "cone":
                        self.images[0, yi, xi] = display(
                            base * (1 - haze) + self.air * haze
                        )
                    if tag == "ground":
                        for chart, (a1, b1, c1, d1), neutral in self.fields:
                            u = (world[:, 0] - a1) / (b1 - a1)
                            v0 = (world[:, 2] - c1) / (d1 - c1)
                            mask = (u >= 0) & (u <= 1) & (v0 >= 0) & (v0 <= 1)
                            if mask.any():
                                base[mask] *= sample(
                                    chart, np.stack((u[mask], v0[mask]), 1)
                                )[:, :3] * (255 / neutral)
                    self.images[1, yi, xi] = display(
                        base * (1 - haze) + self.air * haze
                    )


def main(native, copse, folder, lens, water, seated=False, edition=None):
    guard = 4509715661
    reserved = 1024 * 1024
    if shutil.disk_usage(folder).free <= guard + reserved:
        raise SystemExit("Artist disk floor: defer image")
    poses = json.loads((folder / "views.json").read_text())
    p = poses[lens]
    day = json.loads((native / "coastal-daylight.json").read_text())
    sun = -unit(np.array(day["sun_ray_direction"], "f"))

    def frame(reseat,variant=None):
        lights = []
        for centre, span, n in [([60, 104, -364], 218, 416), ([81, 69, -410], 27, 320)]:
            centre = np.array(centre, "f")
            v = Board(centre + sun * 330, centre, (n, n), span, 0, True)
            v.shoulder_lod = 1 if p["span"] else 0
            traverse(v, native, copse, folder, shoulders=reseat,edition=variant)
            lights.append(v)
        board = Board(
            p["eye"],
            p["aim"],
            (672, 400),
            p["span"],
            p["lens"],
            day=day,
            folder=folder,
            sun_views=lights,
        )
        board.shoulder_lod = 1 if p["span"] else 0
        board.water=water
        if reseat:
            bounds = json.loads((folder / "shoulders.json").read_text())["talus"][
                "native_plan_xz"
            ]
            board.fields.append(
                (
                    (
                        np.asarray(
                            Image.open(folder / "shoulder-talus.png").convert("RGB")
                        ),
                        33071,
                        33071,
                    ),
                    bounds,
                    128,
                )
            )
            skin = json.loads((folder / "broken-granite.json").read_text())
            board.fields.append(
                (
                    (
                        np.asarray(
                            Image.open(folder / "broken-granite.png").convert("RGB")
                        ),
                        33071,
                        33071,
                    ),
                    skin["native_plan_xz"],
                    skin["neutral"],
                )
            )
        if variant:
            image_name,register_name={'joined':('joined-true-beds','joined-true-beds'),'pineglass':('pineglass-mineral','pineglass')}[variant]
            m=json.loads((folder/(register_name+'.json')).read_text())
            board.fields.append(((np.asarray(Image.open(folder/(image_name+'.png')).convert('RGB')),33071,33071),m['native_plan_xz'],m['neutral']))
        if lens == "summit-window":
            board.witnesses = dict.fromkeys(
                [
                    (337, 28),
                    (342, 60),
                    (343, 84),
                    (344, 112),
                    (349, 144),
                    (324, 200),
                    (234, 286),
                    (221, 337),
                ]
            )
        traverse(
            board,
            native,
            copse,
            folder,
            with_cones=not bool(p["span"]),
            shoulders=reseat,edition=variant,
        )
        return board

    b = frame(bool(edition))
    sz = (b.w, b.h)
    if seated:
        original = b.images[1 if edition else 0].copy()
        original_witnesses = b.witnesses
        del b
        b = frame(True,edition)
        b.images[0] = original
        if original_witnesses:
            print("Original witnesses", original_witnesses, flush=True)
    # Only the A/B pair, no title, invented skyline or chromatic grading.
    if b.witnesses:
        print("Source witnesses", b.witnesses, flush=True)
    gap = np.full((sz[1], 7, 3), [43, 46, 44], "B")
    joined = np.concatenate((b.images[0], gap, b.images[1]), axis=1)
    out = folder / (lens + ("-"+edition if edition else "") + "-matched.jpg")
    if shutil.disk_usage(folder).free <= guard + reserved:
        raise SystemExit("Artist floor: hold final picture")
    if lens == "sheltered-boles":
        # Smaller delivery budget for this supplementary comparison; both
        # sides retain their exact locked lens and identical encoding.
        import io

        for quality in (92, 89, 85, 80, 76, 70):
            compressed = io.BytesIO()
            Image.fromarray(joined).save(
                compressed, format="JPEG", quality=quality, subsampling=1
            )
            if compressed.tell() < 145000:
                out.write_bytes(compressed.getvalue())
                break
        else:
            raise ValueError("Do not overflow the small art-journal budget")
    else:
        Image.fromarray(joined).save(out, quality=94, subsampling=0)
    print(
        lens,
        "submitted triangles",
        b.tris,
        "changed final pixels",
        np.any(b.images[1] != b.images[0], axis=-1).sum(),
        flush=True,
    )


if __name__ == "__main__":
    main(
        Path(sys.argv[1]),
        Path(sys.argv[2]),
        Path(sys.argv[3]),
        sys.argv[4],
        Path(sys.argv[5]),
        any(k in sys.argv[5:] for k in ("--seated","--joined","--pineglass")),
        "joined" if "--joined" in sys.argv[5:] else "pineglass" if "--pineglass" in sys.argv[5:] else None,
    )
