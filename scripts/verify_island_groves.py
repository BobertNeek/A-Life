"""Compare finished canopy variants to the original mesh, placements and world proxies."""

import argparse
import json
from pathlib import Path
import struct
import numpy as np
from finished_island_glb import parse, values
from build_island_grove_forms import D, SURFACE, GUARD


def verify(pack, tree, props, surface):
    d, b = parse(pack)
    original, bb = parse(tree)
    source = {m["name"]: m["primitives"][0] for m in original["meshes"]}
    assert d["materials"] == original["materials"] and len(d["meshes"]) == 24
    frozen = total = 0
    heights = {}
    for m in d["meshes"]:
        form, suffix = m["name"].split("_")
        pf = int(form[8:])
        assert 1 <= pf <= 8
        a = source["Conifer_" + suffix]
        c = m["primitives"][0]
        ia = np.array(values(original, bb, a["indices"])).ravel()
        ib = np.array(values(d, b, c["indices"])).ravel()
        assert np.array_equal(ia, ib) and len(m["primitives"]) == 1
        pos = np.array(values(original, bb, a["attributes"]["POSITION"]))
        pp = np.array(values(d, b, c["attributes"]["POSITION"]))
        low = pos[:, 1] <= GUARD
        assert np.array_equal(pos[low], pp[low]) and pp[~low, 1].min() > GUARD
        for k in ("NORMAL", "COLOR_0"):
            before = np.array(values(original, bb, a["attributes"][k]))
            after = np.array(values(d, b, c["attributes"][k]))
            assert np.array_equal(before[low], after[low]), (form, suffix, k)

        def areas(p):
            v = p[ia.reshape(-1, 3)]
            return np.linalg.norm(
                np.cross(v[:, 1] - v[:, 0], v[:, 2] - v[:, 0]), axis=1
            )

        old, new = areas(pos), areas(pp)
        assert (new[old > 0.0001] > old[old > 0.0001] * 0.03).all()
        assert (
            (7.1 < pp[:, 1].max() < 8.3) if pf <= 4 else (9.4 < pp[:, 1].max() < 10.9)
        )
        heights.setdefault(form, []).append(round(pp[:, 1].max(), 3))
        frozen += int(low.sum())
        total += len(ia) // 3
    for sampled_heights in heights.values():
        assert max(sampled_heights) - min(sampled_heights) < 0.06
    placements = json.loads(props.read_text())
    mapping = json.loads(pack.with_suffix(".json").read_text())
    raw = surface.read_bytes()
    w, h, _, _, _, n = struct.unpack_from("<IIfffI", raw, 8)
    boxes = np.frombuffer(raw, "<f4", n * 6, 32 + 4 * w * h).reshape(-1, 6)
    i = 0
    pine = 0
    for idx, prop in enumerate(placements):
        kind = prop["kind"]
        if kind not in (
            "Broadleaf",
            "Sapling",
            "Conifer",
            "Cliff",
        ) and not kind.startswith("Boulder"):
            continue
        box = boxes[i]
        i += 1
        if kind != "Conifer":
            continue
        pine += 1
        x, y, z = prop["position"]
        s = prop["scale"]
        assert np.allclose(
            box,
            [x - 0.175 * s, z - 0.175 * s, x + 0.175 * s, z + 0.175 * s, y, y + 3 * s],
            atol=0.00011,
            rtol=0,
        )
        if str(idx) in mapping:
            assert (
                mapping[str(idx)]["xz"] == [x, z]
                and 1 <= mapping[str(idx)]["form"] <= 8
            )
    assert i == n and pine == 421 and len(mapping) == 286
    return dict(
        total_existing_pines=pine,
        original_leaders=pine - len(mapping),
        varied_existing_crowns=len(mapping),
        identical_root_zone_vertices_across_variants=frozen,
        pooled_triangles_for_24_variants=total,
        added_draws_or_triangles_per_tree=0,
        protected_original_proxy_height_m=3.0,
        unchanged_through_m=GUARD,
        height_by_form_lods_m=heights,
    )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("pack", type=Path)
    cli.add_argument("--conifer", type=Path, default=D / "Conifer.glb")
    cli.add_argument("--placements", type=Path, default=D / "props.json")
    cli.add_argument("--surface", type=Path, default=SURFACE)
    a = cli.parse_args()
    print(json.dumps(verify(a.pack, a.conifer, a.placements, a.surface), indent=2))
