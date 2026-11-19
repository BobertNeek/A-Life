"""Blender adapter for deformed meshes and rigid prop roots. Never exported to native."""

import numpy as np
from mathutils import Vector
from form import BOX, CONTROLS


def coord(ob):
    mesh = ob.data
    a = np.empty(len(mesh.vertices) * 3, np.float32)
    mesh.vertices.foreach_get("co", a)
    local = a.reshape(-1, 3)
    mat = np.array(ob.matrix_world)
    return local, local @ mat[:3, :3].T + mat[:3, 3], mat


def intersects(ob):
    world = np.array([list(ob.matrix_world @ Vector(q)) for q in ob.bound_box])
    x, z = world[:, 0], -world[:, 1]
    return (
        x.max() > BOX[0] and x.min() < BOX[1] and z.max() > BOX[2] and z.min() < BOX[3]
    )


def delta_local(model, world, mat, part=None):
    ux, uy = model.delta(world[:, 0], -world[:, 1], part)
    return np.column_stack((ux, np.zeros(len(ux)), uy)) @ np.linalg.inv(mat[:3, :3]).T


def deform(ob, model, controller=None):
    if not intersects(ob):
        return
    local, w, mat = coord(ob)
    delta = delta_local(model, w, mat)
    if not np.any(np.abs(delta) > 0.00001):
        return
    if ob.data.users > 1:
        ob.data = ob.data.copy()
    if controller is not None:
        ob.shape_key_add(name="V1 exact basis")
        for i, control in enumerate(CONTROLS):
            k = ob.shape_key_add(name=control)
            k.data.foreach_set(
                "co",
                np.ascontiguousarray(
                    local + delta_local(model, w, mat, i), "f"
                ).ravel(),
            )
            f = k.driver_add("value").driver
            v = f.variables.new()
            v.name = "strength"
            v.targets[0].id = controller
            v.targets[0].data_path = '["' + control + '"]'
            f.expression = "strength"
        return
    mesh = ob.data
    # Keep artist split normals (including stone edges, smooth neighbouring
    # terrain chunks and alpha-grass) and transform them with the local shape.
    n = np.empty(3 * len(mesh.loops), "f")
    ids = np.empty(len(mesh.loops), "i")
    mesh.corner_normals.foreach_get("vector", n)
    mesh.loops.foreach_get("vertex_index", ids)
    n = n.reshape(-1, 3) @ np.linalg.inv(mat[:3, :3])
    x = w[:, 0]
    z = -w[:, 1]
    ds = 0.35
    dx = [
        (a - b) / (2 * ds)
        for a, b in zip(model.delta(x + ds, z), model.delta(x - ds, z))
    ]
    dz = [
        (a - b) / (2 * ds)
        for a, b in zip(model.delta(x, z + ds), model.delta(x, z - ds))
    ]
    a = (n[:, 0] - dx[1][ids] * n[:, 2]) / np.maximum(0.08, 1 + dx[0][ids])
    b = n[:, 1] + dz[0][ids] * a + dz[1][ids] * n[:, 2]
    normals = np.column_stack((a, b, n[:, 2])) @ mat[:3, :3]
    normals /= np.maximum(1e-9, np.linalg.norm(normals, axis=1))[:, None]
    mesh.vertices.foreach_set("co", np.ascontiguousarray(local + delta, "f").ravel())
    mesh.update()
    mesh.normals_split_custom_set(normals.tolist())


def finish(model, terrain, rigid):
    for ob in terrain:
        deform(ob, model)
    for ob in rigid:
        x, z = ob.location.x, -ob.location.y
        if not BOX[0] < x < BOX[1] or not BOX[2] < z < BOX[3]:
            continue
        dx, dy = model.delta(x, z)
        ob.location.x += float(dx)
        ob.location.z += float(dy)
