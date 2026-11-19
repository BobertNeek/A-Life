"""Sculpted point-bar finish, exclusively for this DCC study.

The pale deposits indicate the newly low, submerged-at-higher-tide surface.
The existing marine level and upstream cliff remain unpainted; this asset
is never emitted to the runtime registry. Texture is in original V1 chart
coordinates and travels with the shape.
"""

from pathlib import Path
import numpy as np
from form import Reach, ease

X0, X1, Z0, Z1 = 59.375, 109.375, 396.875, 446.875


def paint(surface, dest):
    from PIL import Image
    from scipy.ndimage import gaussian_filter

    r = Reach(surface)
    n = 648
    j, i = np.mgrid[0:n, 0:n]
    x = X0 + (i + 0.5) / n * (X1 - X0)
    z = Z0 + (j + 0.5) / n * (Z1 - Z0)
    q = x - (97.05 + 0.366 * (z - 419))
    before = r.height(x, z)
    _, fall = r.delta(x, z)
    after = before + fall
    gz, gx = np.gradient(after, (Z1 - Z0) / n, (X1 - X0) / n)
    rng = np.random.default_rng(416921)

    def field(s):
        a = gaussian_filter(rng.normal(size=x.shape), s)
        return a / np.std(a)

    broad = field(21)
    tiny = field(1.8)
    grain = rng.standard_normal(x.shape)
    # Tide-laid shell grit and damp silt on the newly opened inside run.
    bed = ease(-1.0, -0.22, after) * (
        1 - ease(0.72 + 0.17 * broad, 3.95 + 0.40 * broad, after)
    )
    corridor = (1 - ease(5, 20.5, np.abs(z - 425))) * (
        1 - ease(6.6, 13.7, np.abs(q + 15 - 0.14 * (z - 425)))
    )
    shelter = 1 - ease(0.35, 0.95, np.hypot(gx, gz))
    cover = bed * corridor * (0.45 + 0.55 * shelter)
    amount = np.clip(cover * (0.69 + 0.07 * broad + 0.06 * tiny), 0, 0.84)
    dry = ease(0.06, 1.25, after)
    silt = np.array([126.0, 135.0, 121.0])
    shell = np.array([167.0, 154.0, 127.0])
    wash = silt + dry[..., None] * (shell - silt)
    cut = 0.45 * np.sin(2.0 * (q + 0.006 * (z - 425) ** 2) + 0.50 * broad)
    pigment = wash + (3.1 * broad + 2.0 * tiny + 1.1 * grain + 2.2 * cut)[..., None]
    rgba = np.concatenate(
        [np.rint(pigment).clip(0, 255), np.rint(amount[..., None] * 255)], axis=2
    ).astype("u1")
    dest = Path(dest)
    dest.parent.mkdir(exist_ok=True, parents=True)
    Image.fromarray(rgba).save(dest, optimize=True)


def attach(r, texture):
    import bpy

    z, x = np.mgrid[Z0 : Z1 + r.s * 0.5 : r.s, X0 : X1 + r.s * 0.5 : r.s]
    positions = np.stack([x, -z, r.height(x, z) + 0.011], axis=-1).reshape(-1, 3)
    uv = np.stack([(x - X0) / (X1 - X0), 1 - (z - Z0) / (Z1 - Z0)], axis=-1).reshape(
        -1, 2
    )
    faces = []
    width = x.shape[1]
    for k in range(z.shape[0] - 1):
        for j in range(width - 1):
            a = k * width + j
            b = a + 1
            c = b + width
            d = a + width
            faces.extend(((a, c, b), (a, d, c)))
    mesh = bpy.data.meshes.new("Raised sand study • fitted original lattice")
    mesh.from_pydata(positions.tolist(), [], faces)
    mesh.update()
    ob = bpy.data.objects.new("Newly sheltered inner bar • study only", mesh)
    bpy.context.scene.collection.objects.link(ob)
    uvs = mesh.uv_layers.new(name="Depositional shore")
    for loop in mesh.loops:
        uvs.data[loop.index].uv = uv[loop.vertex_index]
    for f in mesh.polygons:
        f.use_smooth = True
    mat = bpy.data.materials.new("Wet silt to muted shell sand • study only")
    mat.use_nodes = True
    graph = mat.node_tree
    paint = graph.nodes.new("ShaderNodeTexImage")
    paint.image = bpy.data.images.load(str(Path(texture).resolve()))
    paint.image.pack()
    sh = graph.nodes.get("Principled BSDF")
    sh.inputs["Roughness"].default_value = 0.76
    graph.links.new(paint.outputs["Color"], sh.inputs["Base Color"])
    # Studio-only graded deposit. A binary cut edge turned the low shelf
    # into a polygonal beige sticker; blend the newly sheltered silt back
    # into the old gravel and moor instead.
    graph.links.new(paint.outputs["Alpha"], sh.inputs["Alpha"])
    mesh.materials.append(mat)
    ob.visible_shadow = False
    return ob
