"""Prepare a truthful full-island artist stage for the high shelf (no native registration).
Never reuse the L0 selection centred on far-away eastern Gullglass; that would
present a coarse duplicate surface beneath these lens stations.
"""

import json, math, sys, importlib.util
from pathlib import Path

H = Path(__file__).resolve().parent
sys.path.insert(0, str(H.parents[1] / "scripts"))
from finished_island_glb import parse


def make(native, out):
    out.mkdir(parents=True, exist_ok=True)
    for n in ("habitats", "geology"):
        if not (out / n).exists():
            (out / n).symlink_to((native / n).resolve(), target_is_directory=True)
    spec = importlib.util.spec_from_file_location(
        "shared_source_subset", H.parent / "lanternback-rim/layout.py"
    )
    tool = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(tool)
    d, _ = parse(native / "habitat-ground.glb")
    available = {tuple(map(int, n["name"].split("_")[1:])) for n in d["nodes"]}

    # The true forest/steep middle wall and sighted 240 m peak are one held
    # contiguous high-detail region. A gentle or rough substrate uses fidelity
    # of the original artwork, never a substitute terrain surface.
    def high(i, j):
        return 4 <= i <= 6 and 1 <= j <= 3

    def pick(n):
        _, i, j, l = n.split("_")
        i, j = int(i), int(j)
        if high(i, j) and (i, j) in available:
            return False
        d = math.hypot(-450 + i * 100 - 72, -550 + j * 100 + 375)
        return l == ("L0" if high(i, j) else ("L1" if d < 230 else "L2"))

    tool.subset(native / "terrain-chunks.glb", out / "ground.glb", pick)
    tool.subset(
        native / "habitat-ground.glb",
        out / "near.glb",
        lambda n: high(*map(int, n.split("_")[1:])),
    )
    (out / "layout.json").write_text(
        json.dumps(
            dict(
                high_detail_native_cells=sorted(
                    (i, j) for i, j in available if high(i, j)
                ),
                centre_xz=[72, -375],
                fallback="original V1 L0 when a near habitat chunk does not exist; L1 within 230 m elsewhere, L2 beyond",
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    make(*(Path(p) for p in sys.argv[1:]))
