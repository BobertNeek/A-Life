"""Extract a portable macro crop and editable curve from the same native inputs."""

from pathlib import Path
import argparse
import csv
from PIL import Image
from reach import Reach


def prepare(surface, pigment, output):
    output.mkdir(parents=True, exist_ok=True)
    r = Reach(surface)
    with (output / "stations.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(("metres", "native_x", "native_y", "native_z"))
        for m, (x, z) in zip(r.m, r.p):
            y = float(r.floor(x, z)[0])
            w.writerow((f"{m:.5f}", f"{x:.6f}", f"{y:.6f}", f"{z:.6f}"))
    im = Image.open(pigment).convert("RGB")
    # Same UVs as Island_*_L1. Cropped material for the editing slab,
    # not a reprojection or substitute for the original native texture.
    x0, x1, z0, z1 = 43.75, 165.625, 250.0, 478.125
    shape = im.size
    uv = (
        (x0 + 500) / 1000 * shape[0],
        (z0 + 600) / 1200 * shape[1],
        (x1 + 500) / 1000 * shape[0],
        (z1 + 600) / 1200 * shape[1],
    )
    im.transform(
        (round(uv[2] - uv[0]), round(uv[3] - uv[1])),
        Image.Transform.EXTENT,
        uv,
        resample=Image.Resampling.BICUBIC,
    ).save(output / "editor-pigment.png")


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    for n in ("surface", "pigment", "output"):
        cli.add_argument(n, type=Path)
    args = cli.parse_args()
    prepare(args.surface, args.pigment, args.output)
