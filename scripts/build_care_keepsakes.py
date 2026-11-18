"""Build the sanctuary's keepsakes and nursery groundcover. Run in Blender."""
from math import cos, pi, sin, tau
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/alife_game_app/assets/care"
OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.context.preferences.filepaths.save_version = 0

# Keep each mesh normalized around its contact centre. The runtime supplies
# physical size, canonical base color, roughness, placement and ownership.
def surface(name, rings, segments, point, colour, *, closed=False):
    vertices = []
    colours = []
    for j in range(rings):
        v = j / (rings if closed else rings - 1)
        for i in range(segments):
            u = tau * i / segments
            vertices.append(point(u, v))
            colours.append((*colour(u, v), 1.0))
    faces = []
    for j in range(rings if closed else rings - 1):
        for i in range(segments):
            a, b = j * segments + i, j * segments + (i + 1) % segments
            faces.append((a, b, (b + segments) % len(vertices), (a + segments) % len(vertices)))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    for face in mesh.polygons:
        face.use_smooth = True
    layer = mesh.color_attributes.new(name="Pigment", type="FLOAT_COLOR", domain="POINT")
    for item, color in zip(layer.data, colours):
        item.color = color
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    return obj


def ellipsoid(name, xyz, radius, color):
    def point(u, v):
        # Sample the poles just off-axis so exporter never emits zero-area faces.
        a = pi * (0.0001 + v * 0.9998)
        return (xyz[0] + radius[0] * sin(a) * cos(u),
                xyz[1] + radius[1] * sin(a) * sin(u), xyz[2] - radius[2] * cos(a))
    return surface(name, 17, 32, point, lambda _u, _v: color)


def torus(name, major, minor, xyz, tilt, color):
    def point(u, v):
        a = tau * v
        r = major + minor * cos(a)
        y, z = r * sin(u), minor * sin(a)
        return (xyz[0] + r * cos(u), xyz[1] + y * cos(tilt) - z * sin(tilt),
                xyz[2] + y * sin(tilt) + z * cos(tilt))
    return surface(name, 10, 48, point, lambda _u, _v: color, closed=True)


def orchard_fruit():
    def flesh(u, v):
        a = pi * (0.0001 + v * 0.9998)
        r = sin(a) ** 0.85 * (0.80 + 0.115 * cos(a) + 0.045 * cos(8 * u))
        return (r * cos(u), r * sin(u), -0.07 - 0.72 * cos(a))
    parts = [surface("Eight-groove fruit", 23, 48, flesh,
                     lambda u, v: (0.83 + .16 * sin(pi * v), .66 + .25 * v, .60 + .27 * v))]
    # Seven curled sepals nest into the fruit rather than float over it.
    for index in range(7):
        angle = tau * index / 7
        def leaf(u, v):
            d = .075 + .47 * v
            width = (.003 + .12 * sin(pi * v)) * cos(u)
            z = .62 + .23 * sin(pi * v) - .11 * v + .025 * sin(u) * sin(pi * v)
            return (d * cos(angle) - width * sin(angle), d * sin(angle) + width * cos(angle), z)
        parts.append(surface("Folded calyx", 13, 12, leaf, lambda u, v: (.33, .55 + .15*v, .30)))
    parts.append(ellipsoid("Stem", (0, 0, .76), (.062, .075, .18), (.39, .47, .25)))
    return parts


def orbital_rattle():
    return [
        torus("Porcelain cage", .77, .102, (0, 0, 0), 1.22, (.92, .96, 1.0)),
        torus("Counterweight ring", .64, .082, (0, 0, 0), -.55, (.58, .85, .87)),
        torus("Spindle ring", .45, .047, (0, 0, 0), .25, (1.0, .75, .44)),
        ellipsoid("Suspended bell", (0, 0, 0), (.28, .28, .32), (.90, .77, .55)),
        *[ellipsoid("Axle cap", (x, 0, 0), (.11, .13, .13), (.90, .72, .46)) for x in (-.75,.75)],
    ]


def chime_table():
    parts = [ellipsoid("Stoneware resonator", (0,0,-.22), (.89,.89,.47), (.69,.78,.84)),
             torus("Woven rim", .80, .072, (0,0,-.10), 0, (.94,.75,.52))]
    for i in range(3):
        a = tau * i / 3
        x,y = .41*cos(a), .41*sin(a)
        parts.append(ellipsoid("Tuned pad", (x,y,.16), (.34,.34,.17),
                               ((.74,.96,.88),(.93,.76,.65),(.82,.80,.98))[i]))
        parts.append(torus("Pad inlay", .14,.019, (x,y,.31), 0, (1.,.96,.83)))
    return parts


def export_piece(name, parts):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    obj = bpy.context.object
    obj.name = obj.data.name = name
    bpy.ops.export_scene.gltf(filepath=str(OUT / (name + ".glb")), use_selection=True,
                              export_format="GLB", export_vertex_color="ACTIVE", export_materials="NONE",
                              export_all_vertex_colors=True, export_normals=True, export_texcoords=False)
    return obj


def silverbell():
    """Silhouetted folded leaves with nodding translucent-looking bell corollas."""
    parts = []
    for i in range(9):
        a = i * 2.39996
        length = 0.33 + .15 * (1 + sin(i * 1.37))
        def blade(u, v):
            d = length * v ** 1.45
            w = (.002 + .066 * sin(pi*v)) * cos(u)
            h = .54*sin(v*1.85) + .016*cos(u*2)*sin(pi*v)
            return d*cos(a)-w*sin(a), d*sin(a)+w*cos(a), h*(.7+.25*cos(i))
        parts.append(surface("Pleated sage blade",9,8,blade,
            lambda u,v: (.16+.12*v,.34+.22*v,.24+.15*v)))
    for i in range(5):
        a = i*2.4
        h = .63 + .16*cos(i*2)
        def stem(u,v):
            r = .016*(1-.5*v)
            d = .32*v**1.6
            return d*cos(a)+r*cos(u),d*sin(a)+r*sin(u), h*sin(v*1.88)
        parts.append(surface("Curved reed",8,8,stem,lambda u,v:(.27,.44,.34)))
        def corolla(u,v):
            r = .012 + (.075+.019*cos(5*u))*v**.75
            return (.32*cos(a)+r*cos(u),.32*sin(a)+r*sin(u),
                    h*sin(1.88)-.11*v+.019*cos(5*u)*v**2)
        parts.append(surface("Nodding five-lobed bell",7,20,corolla,
            lambda u,v:(.61+.30*v,.49+.32*v,.66+.20*v)))
    return parts


def ceramic_waystone():
    parts=[]
    for i in range(4):
        angle = i*2.4
        d = .27 if i else 0
        r = (.32,.23,.17,.11)[i]
        x,y = d*cos(angle),d*sin(angle)
        def pebble(u,v):
            a=pi*(.0001+v*.9998)
            return x+r*sin(a)*cos(u), y+r*.77*sin(a)*sin(u), .025-r*.22*cos(a)
        parts.append(surface("River-worn ceramic",9,18,pebble,lambda u,v:(.53+.10*v,.58+.11*v,.55+.10*v)))
    def seam(u,v):
        a=.3+1.60*pi*v
        r=.175+.009*cos(u)
        return r*cos(a), .77*r*sin(a),.088+.005*sin(u)
    parts.append(surface("Inset brass arc",24,6,seam,lambda u,v:(.72,.48,.24)))
    return parts


pieces = [export_piece(name, build()) for name, build in [
    ("orchard-fruit", orchard_fruit), ("orbital-rattle", orbital_rattle),
    ("chime-table", chime_table), ("silverbell", silverbell), ("ceramic-waystone", ceramic_waystone),
]]
for i,piece in enumerate(pieces):
    piece.location.x = (i-2)*2.3
bpy.ops.object.select_all(action="DESELECT")
bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "care-keepsakes.blend"), compress=True)
