"""Pack named cosmetic V1 scarp meshes. Keep all original fallbacks untouched.

DCC authoring, NOT a run of Qualia. Output only to a separate art directory.
The first three original mesh objects/scene/atlas and the original collision
asset stay intact; each new per-landmark group supplies one primitive per LOD.
"""

import argparse, copy, hashlib, json, tempfile
from pathlib import Path
from finished_island_glb import parse, save
from island_fault_scarp import sculpt


def pack(source, surface, placements, output, selection=None):
    source, surface, placements, output = map(
        Path, (source, surface, placements, output)
    )
    if output.resolve() == source.resolve():
        raise ValueError("Package to a separate art folder, never onto the source.")
    props = json.loads(placements.read_text())
    d, buf = parse(source)
    if any(m["name"].startswith("CliffSeat") for m in d["meshes"]):
        raise ValueError(
            "Use the frozen original Cliff source, not an already composed pack."
        )
    # Fallback is literally the old material/mesh data, not merely a
    # similarly named recreation. Reuse repeated attribute/index streams.
    cached = {}

    def key(raw, v):
        return (hashlib.sha256(raw).digest(), v.get("target"), v.get("byteStride"))

    for i, v in enumerate(d["bufferViews"]):
        off = v.get("byteOffset", 0)
        raw = buf[off : off + v["byteLength"]]
        cached[key(raw, v)] = i
    anchor_notes = {}
    with tempfile.TemporaryDirectory() as temp:
        tmp = Path(temp)
        for i, prop in enumerate(props):
            if prop["kind"] != "Cliff" or selection is not None and i not in selection:
                continue
            name = f"CliffSeat{i}"
            path = tmp / (name + ".glb")
            note = sculpt(source, path, surface=surface, landmark=prop, key=name)
            src, body = parse(path)
            if src["materials"] != d["materials"] or src["images"] != d["images"]:
                raise ValueError(
                    "Mismatched original native rock materials; stop authoring."
                )
            bv = {}
            for k, v in enumerate(src["bufferViews"]):
                off = v.get("byteOffset", 0)
                vraw = body[off : off + v["byteLength"]]
                token = key(vraw, v)
                if token not in cached:
                    buf.extend(b"\0" * (-len(buf) % 4))
                    start = len(buf)
                    buf.extend(vraw)
                    cached[token] = len(d["bufferViews"])
                    d["bufferViews"].append({**v, "buffer": 0, "byteOffset": start})
                bv[k] = cached[token]
            ac = {}
            for k, a in enumerate(src["accessors"]):
                ac[k] = len(d["accessors"])
                d["accessors"].append({**a, "bufferView": bv[a["bufferView"]]})
            mapping = {}
            for k, mesh in enumerate(src["meshes"]):
                m = copy.deepcopy(mesh)
                for p in m["primitives"]:
                    p["indices"] = ac[p["indices"]]
                    p["attributes"] = {n: ac[h] for n, h in p["attributes"].items()}
                mapping[k] = len(d["meshes"])
                d["meshes"].append(m)
            ns = []
            for node in src["nodes"]:
                ns.append(len(d["nodes"]))
                d["nodes"].append({**node, "mesh": mapping[node["mesh"]]})
            d["scenes"].append({"name": f"Original V1 landmark {i}", "nodes": ns})
            anchor_notes[str(i)] = {**note, "original_placement": prop}
    d["buffers"][0]["byteLength"] = len(buf)
    output.parent.mkdir(parents=True, exist_ok=True)
    save(d, buf, output)
    return {
        "original_asset": "ff545d9cbf4f71bbd428e87fa69482bbaa4b5f5d",
        "surface": "V1 original only",
        "fallback": "original three unmodified Cliff_L* meshes and original first scene",
        "anchors": anchor_notes,
        "pack_bytes": output.stat().st_size,
    }


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    for flag in ("source", "surface", "placements", "output"):
        ap.add_argument("--" + flag, type=Path, required=True)
    args = ap.parse_args()
    result = pack(args.source, args.surface, args.placements, args.output)
    args.output.with_suffix(".json").write_text(json.dumps(result, indent=2) + "\n")
