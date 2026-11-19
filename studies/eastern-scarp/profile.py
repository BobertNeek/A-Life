"""Locate the true eastern cut and contextualize its already-authored native sea."""

from pathlib import Path
import csv, hashlib, json, sys
import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
from island_inlet_ground import Inlet

HASH = "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"


def profile(native, sea, out):
    out.mkdir(parents=True, exist_ok=True)
    src = native / "island-v1.bin"
    if hashlib.sha256(src.read_bytes()).hexdigest() != HASH:
        raise ValueError("Unrecognised V1 source")
    sea_ids = {}
    for f, old in [
        ("shore.png", "4bd37bff0fe575cbed69089fea51cc20456ef3f8"),
        ("ripple.png", "ad8380546003865c90bbbf674bcaf87687f79800"),
    ]:
        b = (sea / f).read_bytes()
        k = hashlib.sha1(b"blob " + str(len(b)).encode() + b"\0" + b).hexdigest()
        if k != old:
            raise ValueError("Not the held native coast texture: " + f)
        sea_ids[f] = k
    site = Inlet(src)
    rows = []
    for z in np.arange(-164.0, -66.99, 1.0):
        x = np.arange(310.0, 448.0, 0.25)
        v = site.floor(x, np.full_like(x, z))[0]
        cuts = np.nonzero((v[:-1] >= 0) & (v[1:] < 0))[0]
        if len(cuts) != 1:
            continue
        i = cuts[0]
        shore = x[i] + 0.25 * v[i] / (v[i] - v[i + 1])
        samples = [
            float(site.floor(shore + offset, z)[0])
            for offset in (-25, -15, -10, -6, -3, 0, 5, 10, 25)
        ]
        rows.append([z, shore, *samples])
    with (out / "sections.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            (
                "native_z",
                "zero_water_x",
                "y_minus25",
                "y_minus15",
                "y_minus10",
                "y_minus6",
                "y_minus3",
                "y_sea",
                "y_plus5",
                "y_plus10",
                "y_plus25",
            )
        )
        w.writerows([[round(float(x), 5) for x in r] for r in rows])
    props = json.loads((native / "props.json").read_text())
    landmarks = {
        str(i): q
        for i, q in enumerate(props)
        if -164 < q["position"][2] < -66
        and 350 < q["position"][0] < 422
        and q["kind"]
        in (
            "Cliff",
            "Scree",
            "BoulderA",
            "BoulderB",
            "BoulderC",
            "Broadleaf",
            "Conifer",
        )
    }
    (out / "shore.json").write_text(
        json.dumps(
            dict(
                name="Eastern wind cut · source scout",
                native_hash=HASH,
                control="Stage the real V1 sea on both sides, never the previous approximate flat artist reference.",
                native_water_git_blobs=sea_ids,
                source_material=json.loads((sea / "water.json").read_text()),
                survey_rows=len(rows),
                max_rise_from_sea_at_10_m=max(r[4] for r in rows),
                landmarks=landmarks,
                priority="Real east face / high-angle rock, aspect-corrected geological material rather than duplicate broad roof geometry.",
                not_native=True,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    profile(*(Path(q) for q in sys.argv[1:4]))
