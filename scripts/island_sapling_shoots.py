"""Young saltwood: newly forked leaders, alternate leaves, a little bronze flush.

Woody attachment positions are taken from the original game asset. Foliage is
fully modeled, not a shrunken adult, closed hull, textured card or new plant.
"""

import math, random
from mathutils import Vector

Z = Vector((0, 0, 1))


class Growth:
    def __init__(self):
        self.v = []
        self.c = []
        self.faces = []
        self.normals = {}

    def p(self, xyz, color, normal=None):
        i = len(self.v)
        self.v.append(Vector(xyz))
        self.c.append(tuple(color))
        if normal is not None:
            self.normals[i] = normal.copy()
        return i

    def tri(self, *v):
        self.faces.append(v)


def pigment(rgb, v=1):
    return [max(0, min(1, ((max(0, x * v) / 255 + 0.055) / 1.055) ** 2.4)) for x in rgb]


def twig(b, points, radii, rgb, n=4):
    rows = []
    for i, (p, r) in enumerate(zip(points, radii)):
        axis = (
            points[min(i + 1, len(points) - 1)] - points[max(i - 1, 0)]
        ).normalized()
        side = axis.cross(Vector((0.22, 0.45, 1))).normalized()
        other = axis.cross(side)
        row = []
        for j in range(n):
            t = math.tau * j / n
            d = math.cos(t) * side + math.sin(t) * other
            row.append(b.p(p + d * r, pigment(rgb, 0.88 + 0.13 * max(0, d.z)), d))
        if rows:
            old = rows[-1]
            for j in range(n):
                k = (j + 1) % n
                b.tri(old[j], old[k], row[k])
                b.tri(old[j], row[k], row[j])
        rows.append(row)


def leaf(b, base, direction, size, width, colour, lod, lilt):
    # Each waxy leaf has a narrow petiole, an asymmetric shoulder, distinct
    # midrib and a slender downturned tip. Front faces and soft normals point
    # toward daylight on both sides of the shoot.
    d = direction.normalized()
    cross = Z.cross(d).normalized()
    n = d.cross(cross).normalized()
    normal = (n * 0.49 + Z * 0.51).normalized()

    def make(p, c):
        return b.p(p, pigment(colour, c), normal)

    start = make(base, 0.79)
    point = make(base + d * size - Z * size * 0.17, 1.025)
    mid = base + d * size * (0.55 if lod == 0 else 0.50) + Z * size * 0.12
    left = make(mid - cross * width + Z * width * lilt, 0.92)
    right = make(mid + cross * width + Z * width * (0.28 - lilt), 1.035)
    if lod == 0:
        rib = make(mid + Z * width * 0.36, 1.13)
        b.tri(rib, start, left)
        b.tri(right, start, rib)
        b.tri(rib, left, point)
        b.tri(right, rib, point)
    else:
        b.tri(right, start, left)
        b.tri(right, left, point)


def crown(b, forks, lod):
    for j, kind, top in forks:
        rand = random.Random(419187 + j * 991)
        outward = Vector((top.x + 0.058, top.y - 0.021, 0)).normalized()
        side = Z.cross(outward)
        if kind == "leader":
            outward = Vector((0.72, -0.69, 0))
            side = Z.cross(outward)
        # Different crowns terminate on different silhouettes. Long ridges
        # are angled by the coast wind, not grouped around a spherical hull.
        lift = rand.uniform(0.18, 0.52) if kind != "leader" else 0.73
        run = rand.uniform(0.33, 0.59) if kind != "leader" else 0.30
        turn = rand.uniform(-0.27, 0.29)

        def at(t):
            return (
                top
                + outward * run * (0.36 * t + 0.64 * t * t)
                + side * turn * t * t
                + Vector((0.075, -0.045, 0)) * t
                + Z * (lift * (1.26 * t - 0.26 * t * t))
            )

        if lod == 2 and kind != "leader":
            # The original overview has only the trunk. Carry its woody path
            # to the leaf mass rather than hanging a distant green triangle in air.
            root = Vector((-0.075, 0.03, 1.26))
            join = top + outward * 0.035
            trio = [
                b.p(root + side * 0.014, pigment((102, 84, 64)), outward),
                b.p(root - side * 0.014, pigment((98, 81, 64)), outward),
                b.p(join, pigment((120, 98, 72)), outward),
            ]
            b.tri(*trio)
        if lod == 1:
            # Subpixel twigs need a connection, not a multi-ring tube.
            segment = [
                b.p(at(0) + side * 0.012, pigment((125, 103, 78)), Z),
                b.p(at(0) - side * 0.012, pigment((116, 96, 73)), Z),
                b.p(at(1), pigment((143, 120, 82)), Z),
            ]
            b.tri(*segment)
        if lod == 0:
            twig(
                b,
                [at(v) for v in (0, 0.53, 1)],
                [0.014, 0.0081, 0.0026],
                (128, 102, 73),
                4 if lod == 0 else 3,
            )
        # Small, descending lower sprays and a high leading fan. Distance
        # groups have the same directions and real openings between stems.
        for k in range(5):
            t = (0.18, 0.40, 0.55, 0.76, 0.96)[k]
            foot = at(t)
            sgn = -1 if k % 2 else 1
            turn2 = rand.uniform(0.53, 1.05) * sgn
            dist = rand.uniform(0.21, 0.42) * (1.15 if k < 3 else 0.89)
            d = (outward * 0.88 + side * turn2).normalized()
            lat = Z.cross(d)
            lift2 = rand.uniform(-0.08, 0.20) + (t * 0.14 - 0.08)

            def shoot(u):
                return (
                    foot
                    + d * dist * u
                    + Z * (lift2 * u - 0.16 * dist * u * u)
                    + side * turn2 * 0.031 * u * u
                )

            if lod == 2 and k in ((1, 4) if kind != "lateral" else (2,)):
                # Flatten the entire connected shoot into one moss-coloured
                # bough with a light forward tip and a darker bent underside.
                # Two directions overlap near the actual fork but stay open
                # on their outside contour. No bundle of floating triangles.
                rgb = (123, 148, 101) if j % 3 else (145, 157, 99)
                back = top - outward * 0.07 - Z * 0.053
                end = shoot(1) + d * 0.18 + Z * 0.007
                mid = back.lerp(end, 0.57) + Z * 0.085
                breadth = 0.151 + 0.031 * (j % 3)
                l = b.p(mid - lat * breadth - Z * 0.046, pigment(rgb, 0.84), Z)
                r = b.p(mid + lat * breadth, pigment(rgb, 0.88), Z)
                tip = b.p(end, pigment(rgb, 0.97), Z)
                start = b.p(back, pigment(rgb, 0.73), Z)
                ridge = b.p(mid + d * 0.027 + Z * 0.046, pigment(rgb, 0.99), Z)
                b.tri(r, start, ridge)
                b.tri(ridge, start, l)
                b.tri(ridge, l, tip)
                b.tri(r, ridge, tip)
            selected = k in (0, 2, 4)
            if lod == 0:
                twig(
                    b,
                    [foot, shoot(0.48), shoot(1)],
                    [0.0064, 0.0042, 0.0018],
                    (145, 103, 73),
                    3,
                )
            elif lod == 1 and selected:
                sl = [
                    b.p(foot + lat * 0.005, pigment((139, 113, 78)), Z),
                    b.p(foot - lat * 0.005, pigment((129, 102, 73)), Z),
                    b.p(shoot(1), pigment((153, 124, 84)), Z),
                ]
                b.tri(*sl)
            for l in range(9):
                u = 0.12 + 0.097 * l
                sf = -1 if l % 2 else 1
                lev = rand.uniform(-0.14, 0.37)
                length = rand.uniform(0.165, 0.247) * (1 - 0.21 * u)
                width = length * rand.uniform(0.27, 0.39)
                ang = rand.uniform(0.54, 1.12)
                direction = (
                    d * math.cos(ang) + lat * math.sin(ang) * sf + Z * (lev + 0.15 * u)
                ).normalized()
                # Older sheltered leaves are cool moss; selected young tips
                # are translucent spring olive or copper, never candy fruit.
                terminal = l >= 7 and (j * 3 + k) % 7 < 2
                fresh = t > 0.53 or l > 4
                col = (
                    (183, 144, 86)
                    if terminal
                    else ((154, 178, 101) if fresh else (117, 151, 99))
                )
                shift = rand.uniform(0.91, 1.07) * (1 - 0.04 * (1 - u))
                colour = [x * shift * (0.92 if lod == 1 else 1) for x in col]
                # Draw the same random sequence even when the far leaf is
                # omitted: all its surviving sisters retain their L0 direction.
                lilt = rand.uniform(-0.28, 0.35)
                if lod == 2 or (lod == 1 and (not selected or l not in (0, 3, 6, 8))):
                    continue
                leaf(
                    b,
                    shoot(u),
                    direction,
                    length * (1.52 if lod == 1 else 1),
                    width * (1.84 if lod == 1 else 1),
                    colour,
                    lod,
                    lilt,
                )
