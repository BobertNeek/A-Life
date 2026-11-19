"""Pin and survey the *already authored native* copse floor before planting anything.
Original prop records do not list these roots/moss/ferns. Do not start from a
bare original height-field and then take credit for replacing a missing forest.
"""

from pathlib import Path
import sys, hashlib, json
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[1] / "scripts"))
from finished_island_glb import parse, values
from island_inlet_ground import Inlet

PIN = "7dc9cf345efbec44ec5cd1f0c24dadb7ed32a706"
BOX = (48.0, 127.0, -447.0, -369.0)


def survey(native, copse, out):
    raw = copse.read_bytes()
    sha = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if sha != PIN:
        raise ValueError(
            "Need the exact current native copse floor, not an older pre-fern study"
        )
    del raw
    d, data = parse(copse)
    b = Inlet(native / "island-v1.bin")
    a, c, e, f = BOX
    W, H = 395, 390
    occupied = Image.new("L", (W, H))
    pen = ImageDraw.Draw(occupied)
    ids = []
    total = 0
    near = 1e9
    body = 0
    under = 0
    for n in d["nodes"]:
        origin = np.array(n.get("translation", [0, 0, 0]))
        p = d["meshes"][n["mesh"]]["primitives"][0]
        att = p["attributes"]["POSITION"]
        meta = d["accessors"][att]
        lo = np.array(meta["min"]) + origin
        hi = np.array(meta["max"]) + origin
        if hi[0] < a or lo[0] > c or hi[2] < e or lo[2] > f:
            continue
        v = np.array(values(d, data, att)) + origin
        t = np.array(values(d, data, p["indices"]), int).reshape(-1, 3)
        centres = v[t].mean(1)
        relevant = (
            (centres[:, 0] >= a - 0.2)
            & (centres[:, 0] <= c + 0.2)
            & (centres[:, 2] >= e - 0.2)
            & (centres[:, 2] <= f + 0.2)
        )
        t = t[relevant]
        centres = centres[relevant]
        if not len(t):
            continue
        ids.append(n["name"])
        total += len(t)
        dist = np.hypot(v[:, 0] - 82.0, v[:, 2] + 408.0)
        near = min(near, float(dist.min()))
        body += int((dist < 8).sum())
        dy = centres[:, 1] - b.floor(centres[:, 0], centres[:, 2])[0]
        # Conservative flattened triangles of real root/floor foliage, not
        # an alpha-colour visibility claim and not a physics/navigation net.
        tt = t[(dy < 0.85) & (dy > -0.22)]
        under += len(tt)
        for corners in v[tt]:
            xy = [
                (float((x - a) / (c - a) * W), float((z - e) / (f - e) * H))
                for x, y, z in corners
            ]
            pen.polygon(xy, fill=255)
    occupied = occupied.filter(ImageFilter.MaxFilter(3))
    occupied.save(out / "held-copse-coverage.png", optimize=True)
    note = dict(
        copse_render_blob=PIN,
        source_chunk_names=ids,
        box_xz=BOX,
        mesh_triangles_in_window=total,
        actual_low_floor_triangles=under,
        existing_vertex_distance_from_summit_eye_xz_m=round(near, 3),
        source_vertices_within_8m_xy=body,
        conservative_cover_percentage=round(
            100 * float((np.asarray(occupied) > 0).mean()), 3
        ),
        placement_keepout_pad_m=0.2,
        not_navigation_or_alpha_scene=True,
    )
    (out / "held-copse.json").write_text(json.dumps(note, indent=2) + "\n")
    return note


if __name__ == "__main__":
    print(survey(*(Path(p) for p in sys.argv[1:])))
