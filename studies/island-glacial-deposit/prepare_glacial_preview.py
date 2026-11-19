"""Prepare an OFFLINE erratic-only game-asset overlay in a NEW external folder.

Does not edit a checkout, apply code, build, run tests, launch a game, use a
player profile, or install the optional till / cliff proposal. Used from the
preserved study directory on an isolated checkout when native review resumes.
"""

from pathlib import Path
import argparse
import copy
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent
ISLAND = Path("crates/alife_game_app/assets/landscape/island")
MANIFEST = Path(
    "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
)
ORIGINAL = {
    "BoulderA.glb": "464be6dd28139b5ce68a9b4ed76c630038877f43",
    "BoulderB.glb": "2aec950c9790d3a71ee0c670e9e76f244fc3ffc2",
    "BoulderC.glb": "b1e43ad7ef5e6381da61fdb8a7102c770973b8ca",
}


def git_blob(path):
    data = path.read_bytes()
    return hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()


def digest(data):
    out = 0xCBF29CE484222325
    for b in data:
        out = ((out ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv1a64:{out:016x}"


def prepare(destination):
    destination = Path(destination).resolve()
    if destination.exists() or destination.is_relative_to(ROOT):
        raise ValueError("Choose an unused output path outside this source checkout.")
    if any(git_blob(ROOT / ISLAND / name) != sha for name, sha in ORIGINAL.items()):
        raise ValueError(
            "Different native rock source; do not replace without reconciling it first."
        )
    manifest = json.loads((ROOT / MANIFEST).read_text())
    rows = {row["local_path"]: row for row in manifest["entries"]}
    shipped = [Path(n) for n in ORIGINAL] + [
        Path("erratic") / n
        for n in ("granite.png", "granite-normal.png", "granite-orm.png")
    ]
    payload = {n: (SOURCE / "models" / n).read_bytes() for n in shipped}
    patch = (SOURCE / "erratic-native-material-family.patch").read_bytes()
    stamp = {
        "tool": "Qualia glacial erratic sculpt and oriented weather rind",
        "config_path": "scripts/studio_erratic_rock.py",
        "date": "2026-11-19",
        "seed": "V1 selection 01d86f19, original native A/B/C and deterministic object-domain mineral field",
    }
    for name, data in payload.items():
        key = (ISLAND / name).as_posix()
        if name.name in ORIGINAL:
            row = rows[key]
        else:
            if key in rows:
                raise ValueError(
                    "Preview path already registered in this checkout: " + key
                )
            row = copy.deepcopy(rows[(ISLAND / "minerals/fracture.png").as_posix()])
            row.update(asset_id="approved-island-glacial-" + name.stem, local_path=key)
            manifest["entries"].append(row)
        row.update(
            digest=digest(data),
            size_bytes=len(data),
            generator=stamp,
            source="generated:glacial-erratic-rub-faces-and-directional-rind",
            replacement_policy="regenerate-from-versioned-original-A-B-C-and-update-the-scoped-manifest",
        )
    destination.mkdir(parents=True)
    for name, data in payload.items():
        p = destination / ISLAND / name
        p.parent.mkdir(exist_ok=True, parents=True)
        p.write_bytes(data)
    out_manifest = destination / MANIFEST
    out_manifest.parent.mkdir(exist_ok=True, parents=True)
    out_manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    (destination / "erratic-native-material-family.patch").write_bytes(patch)
    (destination / "READ-ME-FIRST.txt").write_text(
        "Three erratic models, three near material maps, a scoped manifest and an UNAPPLIED material-family patch. No near till binding. Move these files only into a clean separate test checkout and apply the patch there, next week. Preserve the original three minerals/ maps for Cliff/Scree and the stock overview.\n"
    )
    print("Prepared external overlay only:", destination)


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("new_external_directory", type=Path)
    prepare(cli.parse_args().new_external_directory)
