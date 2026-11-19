"""Source art operation; no game, collision output, or edits to native terrain."""

from pathlib import Path
import csv, json, sys
import numpy as np

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[1] / "scripts"))
sys.path.insert(0, str(here))
from strata import Rim, frame, BOX, HOLD, HASH
from fissures import build, grain
from heath import build as plants


def author(native, out):
    out.mkdir(exist_ok=True, parents=True)
    r = Rim(native / "island-v1.bin")
    r.chart(out)
    grain(out)
    stone = build(r, out)
    vegetation = plants(r, out)
    with (out / "outcrop-ledger.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["x", "z", "true_ground_y", "grade", "half_length_m"])
        for x, z, s in stone.pop("outcrop_centroids"):
            h, gx, gz = r.floor(x, z)
            w.writerow([x, z, round(h, 4), round(np.hypot(gx, gz), 4), s])
    props = json.loads((native / "props.json").read_text())
    cams = {
        t: frame(t, lambda x, z: float(r.floor(x, z)[0]))
        for t in ("upper-rim", "stone-eye")
    }
    marks = {str(i): props[i] for i in [1477, 1660, 1418, 1634, 1729, 643, 1675]}
    # A real downhill section from high west nose into the Sunspindle foredune.
    cross = []
    for x in np.arange(201.0, 341.0, 2.0):
        z = -143.0 + 0.29 * (x - 201.0) + 3.6 * np.sin((x - 201.0) * 0.052)
        h, gx, gz = r.floor(x, z)
        cross.append([x, z, h, np.hypot(gx, gz)])
    with (out / "slope-section.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["x", "z", "native_height_m", "native_grade"])
        w.writerows([[round(v, 5) for v in pt] for pt in cross])
    (out / "site.json").write_text(
        json.dumps(
            dict(
                name="Lanternback Rim",
                hold=HOLD,
                terrain_sha256=HASH,
                bounds=BOX,
                stations=cams,
                native_landmarks=marks,
                stone=stone,
                heath=vegetation,
                section_head_m=float(cross[0][2]),
                section_foot_m=float(cross[-1][2]),
                native_ground_changed=False,
                native_registration=False,
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    author(Path(sys.argv[1]), Path(sys.argv[2]))
