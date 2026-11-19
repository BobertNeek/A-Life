"""Branch-borne alder sprays, grown from the eight original hardwood forks."""

import math
from random import Random
from mathutils import Vector

Z = Vector((0, 0, 1))


class Brush:
    def __init__(self):
        self.soft_normals = {}
        self.v = []
        self.uv = []
        self.f = []

    def add(self, p, uv):
        self.v.append(Vector(p))
        self.uv.append(uv)
        return len(self.v) - 1

    def tri(self, a, b, c):
        self.f.append((a, b, c))


def rod(b, pts, radii, sides):
    history = []
    length = 0
    for i, (p, r) in enumerate(zip(pts, radii)):
        p = Vector(p)
        t = (
            Vector(pts[min(i + 1, len(pts) - 1)]) - Vector(pts[max(i - 1, 0)])
        ).normalized()
        u = t.cross(Z)
        if u.length < 1e-4:
            u = Vector((1, 0, 0))
        u.normalize()
        v = t.cross(u).normalized()
        if i:
            length += (p - Vector(pts[i - 1])).length
        row = []
        for k in range(sides + 1):
            a = math.tau * k / sides
            row.append(
                b.add(
                    p + r * (math.cos(a) * u + math.sin(a) * v),
                    (
                        0.29 + 0.66 * k / sides,
                        0.517 + 0.060 * (0.5 + 0.45 * math.sin(length)),
                    ),
                )
            )
        history.append(row)
    for lo, hi in zip(history, history[1:]):
        for k in range(sides):
            b.tri(lo[k], lo[k + 1], hi[k])
            b.tri(lo[k + 1], hi[k + 1], hi[k])


def tile(b, root, forward, wide, roll, fall, chart, level, packet=False):
    axis = Vector(forward).normalized()
    side = axis.cross(Z)
    if side.length < 0.03:
        side = axis.cross(Vector((1, 0, 0)))
    side.normalize()
    up = axis.cross(side).normalized()
    cross = side * math.cos(roll) + up * math.sin(roll)
    # Bent, axial sprays grow from a precise point on the woody twig and
    # overlap neighbours obliquely; nothing rotates toward the camera.
    ts = (
        (0, 0.32, 0.66, 1)
        if level == 0 and packet
        else (0, 0.5, 1) if level == 0 or level == 1 and packet else (0, 1)
    )
    qs = (
        (-1, 0, 1)
        if level == 0 or (level == 1 and packet and chart % 3 == 0)
        else (-1, 1)
    )
    rows = []
    for t in ts:
        row = []
        for q in qs:
            p = (
                root
                + forward * t
                + Z * (fall * (0.64 * t * t - 0.25 * math.sin(math.pi * t)))
            )
            p += (
                cross * q * wide
                + Z * (0.11 * wide * q * q)
                + up * (0.13 * wide * q * q * math.sin(math.pi * t))
            )
            row.append(
                b.add(
                    p,
                    (
                        (chart + (q + 1) / 2) * 0.25,
                        (
                            (0.006 + 0.988 * t) * 0.5
                            if packet
                            else 0.5 + (96 + 416 * (0.006 + 0.988 * t)) / 1024
                        ),
                    ),
                )
            )
        rows.append(row)
    for lo, hi in zip(rows, rows[1:]):
        for k in range(len(qs) - 1):
            b.tri(lo[k], hi[k], hi[k + 1])
            b.tri(lo[k], hi[k + 1], lo[k + 1])


def crown(b, hub, center, level, index, live_fork):
    rng = Random(651973 + 991 * index)
    h = Vector(hub)
    out = h - Vector(center)
    out.z = 0
    if out.length < 0.15:
        out = Vector((math.cos(index * 2.39), math.sin(index * 2.39), 0))
    out.normalize()
    lateral = Vector((-out.y, out.x, 0))
    # The old pancakes were flat in almost every direction; alternating the
    # height, inclination and secondary tip placement of each fork makes
    # the middle crowns exchange volume without constructing a green roof.
    # Secondary, burdened side branches emerge from rings of the existing
    # source wood itself, not somewhere in thin air below a former crown.
    # Leave two read-throughs for the shoreline behind the open fork.
    if index not in (0, 4):
        q = Random(410231 + index * 303)
        hand = -1 if index % 2 == 0 else 1
        root = Vector(live_fork)
        elbow = (
            root
            + out * q.uniform(0.24, 0.52)
            + lateral * hand * q.uniform(0.37, 0.63)
            + Z * q.uniform(0.12, 0.38)
        )
        tip = (
            elbow
            + out * q.uniform(0.18, 0.50)
            + lateral * hand * q.uniform(0.35, 0.66)
            + Z * q.uniform(-0.14, 0.42)
        )
        rod(b, [root, elbow, tip], [0.060, 0.041, 0.009], 4 if level == 0 else 3)
        for j in range(4 if level == 0 else 2):
            a = elbow.lerp(tip, 0.25 * j)
            v = (
                out * q.uniform(0.44, 0.91)
                + lateral * (hand * q.uniform(0.35, 0.77))
                + Z * q.uniform(-0.52, 0.62)
            )
            tile(
                b,
                a,
                v,
                q.uniform(0.31, 0.48),
                q.uniform(-1.40, 1.39),
                q.uniform(-0.32, 0.22),
                (index + j) % 4,
                level,
                packet=True,
            )
    for arm in range(5):
        angle = (arm - 2) * 0.69 + rng.uniform(-0.23, 0.23)
        d = (out * math.cos(angle) + lateral * math.sin(angle)).normalized()
        c = Vector((-d.y, d.x, 0))
        ln = rng.uniform(0.74, 1.34)
        rise = rng.uniform(0.14, 0.66)
        if arm == 0:
            rise -= 0.42
        if index in (1, 2, 6) and arm == 3:
            rise += 0.29
        k = rng.uniform(-0.21, 0.19)
        bend = h + d * 0.34 * ln + c * k + Z * (rise * 0.6 + 0.15)
        tip = h + d * ln + c * k * 0.6 + Z * rise
        if level == 0:
            rod(b, [h, bend, tip], [0.042, 0.024, 0.006], 3)
        elif level == 1 and arm % 2 == 0:
            rod(b, [h, tip], [0.035, 0.006], 3)
        if arm in (0, 2, 4):
            knot(b, bend.lerp(tip, 0.27) + Z * 0.09, d, index * 5 + arm, level)
        # The body of a crown is several oblique and folded forked sprays.
        # Individual thinner shoots escape its margins and negative spaces.
        packet_rng = Random(731683 + index * 97 + arm * 1739)
        for j in range(2 if level == 0 else 1):
            a2 = 1 if j == 0 else -1
            o = (
                h.lerp(bend, 0.40 + 0.55 * j)
                - d * 0.27
                + Z * packet_rng.uniform(-0.07, 0.27)
            )
            axis = (
                d * packet_rng.uniform(0.95, 1.45)
                + c * (a2 * packet_rng.uniform(0.21, 0.66))
                + Z * packet_rng.uniform(-0.61, 0.67)
            )
            w = packet_rng.uniform(0.40, 0.64) * (1.22 if level == 1 else 1)
            tile(
                b,
                o,
                axis,
                w,
                packet_rng.uniform(-1.45, 1.43),
                packet_rng.uniform(-0.37, 0.19),
                (index + arm + 2 * j) % 4,
                level,
                packet=True,
            )
        # Each ordinary leaf is ~8–14 cm instead of a generous 0.8 m diamond.
        for cluster in range(6):
            t = 0.22 + 0.143 * cluster
            origin = (
                h.lerp(bend, min(1, t / 0.42))
                if t <= 0.42
                else bend.lerp(tip, (t - 0.42) / 0.58)
            )
            hand = 1 if cluster % 2 == 0 else -1
            yaw = hand * rng.uniform(0.30, 1.11)
            direct = (d * math.cos(yaw) + c * math.sin(yaw)).normalized()
            elev = rng.uniform(-0.73, 0.34) - (0.14 if cluster > 2 else 0)
            dist = rng.uniform(0.45, 0.77)
            fwd = direct * dist + Z * (elev * dist)
            width = rng.uniform(0.15, 0.25)
            roll = rng.uniform(-1.22, 1.18)
            fall = rng.uniform(-0.23, 0.08)
            ch = (index + arm + cluster) % 4
            if level == 0 and (
                cluster == 0 and (arm + index) % 2 == 0 or cluster == 4 and arm % 2 == 0
            ):
                continue
            if level == 1 and (cluster != 3 or arm % 2 != 0):
                continue
            if level == 2 and (arm not in (0, 2, 4) or cluster not in (1, 4)):
                continue
            tile(
                b,
                origin,
                fwd * (1.18 if level == 1 else 1.36 if level == 2 else 1),
                width * (1.36 if level == 1 else 1.90 if level == 2 else 1),
                roll,
                fall,
                ch,
                level,
            )


def distant(b, hub, index):
    """At a few hundred metres, give the mips an irregular solid mass, not needles.
    The eight capsule-shaped clusters follow each actual existing woody fork.
    """
    rng = Random(71941 + 197 * index)
    h = Vector(hub)
    rad = Vector((h.x, h.y, 0))
    if rad.length < 0.2:
        rad = Vector((1, 0, 0))
    rad.normalize()
    side = Vector((-rad.y, rad.x, 0))
    off = rad * rng.uniform(0.18, 0.39) + Z * rng.uniform(0.12, 0.37)
    c = h + off
    tw = rng.uniform(-0.48, 0.55)
    ids = []
    for k in range(5):
        a = (k + 0.14 * index) * math.tau / 5
        p = (
            c
            + rad * (1.13 * math.cos(a))
            + side * (0.82 * math.sin(a))
            + Z * (tw * math.cos(a) + 0.32 * math.sin(2 * a + 0.7 * index))
        )
        ids.append(b.add(p, ((0.5 + (index + k) % 6) * 16 / 1024, 0.545)))
    bot = b.add(c + side * 0.16 - Z * 0.48, ((0.5 + index % 2) * 16 / 1024, 0.545))
    top = b.add(c - rad * 0.28 + Z * 0.63, ((0.5 + 2 + index % 2) * 16 / 1024, 0.545))
    for k in range(5):
        b.tri(ids[k], ids[(k + 1) % 5], top)
        b.tri(ids[(k + 1) % 5], ids[k], bot)


def knot(b, at, facing, index, level):
    """Palm-sized sprays overlap around a shaded nonplanar heart of the bough.

    This little five-lobed core stays inboard of the loose terminal leaves:
    rotated, taller than its half-width, never an entire solid canopy cap.
    It uses the dense interior of the real cut-leaf print, not a flat chip.
    """
    rng = Random(842911 + index * 691)
    rad = Vector(facing)
    rad.z = rng.uniform(-0.3, 0.43)
    rad.normalize()
    side = rad.cross(Z).normalized()
    up = side.cross(rad).normalized()
    c = Vector(at)
    long = rng.uniform(0.36, 0.55)
    wide = rng.uniform(0.24, 0.41)
    high = rng.uniform(0.26, 0.43)
    opaque = level == 1 or index % 5 == 2
    if level == 1:
        long *= 1.80
        wide *= 1.86
        high *= 1.88
        c += rad * 0.15 + side * (0.16 * math.sin(index))
    n = 7 if level == 0 else 5
    shape = []
    chart = index % 4
    for k in range(n):
        a = (k + 0.15 * math.sin(index)) * math.tau / n
        shape.append(
            c
            + rad * (long * math.cos(a))
            + side * (wide * math.sin(a))
            + up * (0.29 * high * math.sin(2 * a + index))
        )
    bottom = c + side * wide * 0.12 - rad * long * 0.26 - up * high * 0.68
    top = c - side * wide * 0.21 + rad * long * 0.18 + up * high * 0.85

    def add(p, u, v):
        tex = (
            ((106 + 151 * u) / 1024, 1 - (426 + 79 * (1 - v)) / 1024)
            if opaque
            else ((chart + u) * 0.25, v * 0.5)
        )
        vid = b.add(p, tex)
        delta = p - c
        normal = (
            rad * (delta.dot(rad) / (long * long))
            + side * (delta.dot(side) / (wide * wide))
            + up * (delta.dot(up) / (high * high))
        )
        b.soft_normals[vid] = (normal.normalized() + 0.16 * Z).normalized()
        return vid

    # Overlap the moss-green leaf texture through this interior. It is neither
    # a silhouette disc nor a zero-thickness face-to-camera layer.
    for k in range(n):
        a = shape[k]
        z = shape[(k + 1) % n]
        u0 = 0.06 + 0.035 * (k % 2)
        u1 = 0.93 - 0.03 * (k % 3)
        b.tri(add(a, u0, 0.19), add(z, u1, 0.24), add(top, 0.52, 0.90))
        b.tri(add(z, u1, 0.83), add(a, u0, 0.79), add(bottom, 0.49, 0.13))
