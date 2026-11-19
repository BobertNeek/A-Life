"""Read the real surface between the selected pennon colony and its first native spruce.

The separations and sightlines are placement guidance for decorative authoring,
not a walkability or physics qualification.
"""

import csv, hashlib, json, math, struct, sys
from pathlib import Path
import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
from island_inlet_ground import Inlet

BOX = (225.0, 260.0, -195.0, -144.0)
DIGEST = "8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38"
EYE = (245.0, 48.26626788330077, -140.6)


class Bed(Inlet):
    def __init__(self, native):
        src = native / "island-v1.bin"
        s = src.read_bytes()
        if hashlib.sha256(s).hexdigest() != DIGEST:
            raise ValueError("This artist layout belongs to the original V1.")
        super().__init__(src)
        w, d, _, _, _, n = struct.unpack_from("<IIfffI", s, 8)
        boxes = np.frombuffer(s, "<f4", n * 6, 32 + w * d * 4).reshape(n, 6)
        x0, x1, z0, z1 = BOX
        self.solid = boxes[
            (boxes[:, 0] < x1 + 3)
            & (boxes[:, 2] > x0 - 3)
            & (boxes[:, 1] < z1 + 3)
            & (boxes[:, 3] > z0 - 3)
        ]
        props = json.loads((native / "props.json").read_text())
        self.original = {
            i: q
            for i, q in enumerate(props)
            if x0 - 10 < q["position"][0] < x1 + 10
            and z0 - 10 < q["position"][2] < z1 + 10
        }

    def keep_out(self, x, z, pad):
        b = self.solid
        return bool(
            np.any(
                (b[:, 0] - pad < x)
                & (x < b[:, 2] + pad)
                & (b[:, 1] - pad < z)
                & (z < b[:, 3] + pad)
            )
        )

    def visible(self, x, z, above=0.30, eye=EYE):
        x, z = np.broadcast_arrays(x, z)
        target = self.floor(x, z)[0] + above
        clearance = np.full_like(x, 999.0, dtype=float)
        for s in np.linspace(0.025, 0.98, 50):
            xx = eye[0] + (x - eye[0]) * s
            zz = eye[2] + (z - eye[2]) * s
            clearance = np.minimum(
                clearance, (eye[1] + (target - eye[1]) * s) - self.floor(xx, zz)[0]
            )
        return clearance


def survey(native, out):
    out.mkdir(exist_ok=True, parents=True)
    bed = Bed(native)
    x0, x1, z0, z1 = BOX
    xx, zz = np.meshgrid(np.arange(x0 + 0.5, x1, 1), np.arange(z0 + 0.5, z1, 1))
    h, dx, dz = bed.floor(xx, zz)
    v = bed.visible(xx, zz)
    vis0 = bed.visible(xx, zz, 0.015)
    eye = np.array(EYE)
    near = sorted(
        (
            (math.hypot(q["position"][0] - eye[0], q["position"][2] - eye[2]), i, q)
            for i, q in bed.original.items()
            if q["kind"] == "Conifer" and q["position"][2] < eye[2] - 2
        ),
        key=lambda t: t[0],
    )
    with (out / "sampled-ground.csv").open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(
            (
                "x",
                "native_y",
                "z",
                "slope_degrees",
                "clearance_to_ground_m",
                "clearance_to_30cm_crown_m",
                "solid_envelope_at_40cm",
            )
        )
        for x, y, z, g, v0, v1 in zip(
            xx.flat,
            h.flat,
            zz.flat,
            np.degrees(np.arctan(np.hypot(dx, dz))).flat,
            vis0.flat,
            v.flat,
        ):
            w.writerow(
                [
                    round(x, 2),
                    round(y, 4),
                    round(z, 2),
                    round(g, 2),
                    round(v0, 3),
                    round(v1, 3),
                    bed.keep_out(x, z, 0.4),
                ]
            )
    # Never infer a convenient stand from map density: these are the closest
    # already-positioned original trees in this actual forward sector.
    selected = {
        str(i): dict(horizontal_m=round(dist, 3), **q) for dist, i, q in near[:4]
    }
    (out / "sightline.json").write_text(
        json.dumps(
            dict(
                windward="Lanternback to existing spruce #643",
                bounds=BOX,
                held_eye=EYE,
                distinct_roots=selected,
                native_hash=DIGEST,
                original_landform=True,
                sampled_area_m2=xx.size,
                visible_30cm_cells=int((v > 0).sum()),
                visible_surface_cells=int((vis0 > 0).sum()),
                solid_envelopes=int(len(bed.solid)),
            ),
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    survey(*(Path(q) for q in sys.argv[1:3]))
