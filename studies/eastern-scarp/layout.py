"""Small true-LOD selection centred on the eastern cliff, not the distant western hill."""

from pathlib import Path
import sys, math

H = Path(__file__).resolve().parent
sys.path[:0] = [str(H.parent / "lanternback-rim"), str(H.parents[1] / "scripts")]
from finished_island_glb import parse
from importlib.util import module_from_spec, spec_from_file_location

s = spec_from_file_location("native_subset", H.parent / "lanternback-rim/layout.py")
m = module_from_spec(s)
s.loader.exec_module(m)


def stage(native, out):
    out.mkdir(exist_ok=True, parents=True)
    for n in ("geology", "habitats"):
        dst = out / n
        if not dst.exists():
            dst.symlink_to((native / n).resolve(), target_is_directory=True)
    hd, _ = parse(native / "habitat-ground.glb")
    hab = {tuple(map(int, n["name"].split("_")[1:])) for n in hd["nodes"]}

    def near(i, j):
        return i in (8, 9) and j in (3, 4, 5)

    def chosen(n):
        _, i, j, l = n.split("_")
        i, j = int(i), int(j)
        if near(i, j) and (i, j) in hab:
            return False
        dist = math.hypot(-450 + i * 100 - 396, -550 + j * 100 + 119)
        return l == ("L0" if near(i, j) else "L1" if dist < 200 else "L2")

    m.subset(native / "terrain-chunks.glb", out / "ground.glb", chosen)
    m.subset(
        native / "habitat-ground.glb",
        out / "near.glb",
        lambda n: near(*map(int, n.split("_")[1:])),
    )


if __name__ == "__main__":
    stage(*(Path(x) for x in sys.argv[1:3]))
