"""Preserve the photographed parents, final plants and exact matching held frames."""

from pathlib import Path
import hashlib, json, sys, zipfile


def archive(held, new, out):
    meta = json.loads((new / "crown-receipt.json").read_text())
    meta.update(
        method="Blender 4.3.2 Cycles, 24 samples per full frame, held original same 840 × 500 cameras and coastal light",
        new_images={},
    )
    with zipfile.ZipFile(
        out / "camera-originals.zip", "w", compression=zipfile.ZIP_STORED
    ) as z:
        for tag, directory in (("before", held), ("after", new / "after")):
            for name in ("stone-eye", "pennon-eye"):
                p = directory / (name + ".png")
                data = p.read_bytes()
                path = tag + "/" + name + ".png"
                z.writestr(path, data)
                meta["new_images"][path] = hashlib.sha256(data).hexdigest()
    for n in ("Folded-Windcups.blend", "Folded-Windcups.jpg"):
        meta[n] = hashlib.sha256((out / n).read_bytes()).hexdigest()
    (out / "gallery.json").write_text(json.dumps(meta, indent=2) + "\n")


if __name__ == "__main__":
    archive(*(Path(s) for s in sys.argv[1:4]))
