"""The DCC review reads the same triangle split and 100m chunks as the game.

Keep immutable grid dimensions separate from the current camera's lens/span.
There is no organism in the review, so occupied-chunk overrides don't apply.
"""

from dataclasses import dataclass
from pathlib import Path
import math
import struct


@dataclass(frozen=True, slots=True)
class Grid:
    width: int
    depth: int
    x0: float
    z0: float
    step: float
    heights: tuple

    @classmethod
    def read(cls, path):
        raw = Path(path).read_bytes()
        assert raw[:8] == b"HLAND001"
        w, h, x, z, s, _ = struct.unpack_from("<IIfffI", raw, 8)
        return cls(w, h, x, z, s, struct.unpack_from(f"<{w*h}f", raw, 32))

    def height(self, x, z):
        u, v = (x - self.x0) / self.step, (z - self.z0) / self.step
        i, j = math.floor(u), math.floor(v)
        assert 0 <= i < self.width - 1 and 0 <= j < self.depth - 1
        u, v = u - i, v - j
        a = j * self.width + i
        p, q, r, s = (
            self.heights[a],
            self.heights[a + 1],
            self.heights[a + self.width + 1],
            self.heights[a + self.width],
        )
        return (
            p + (q - p) * u + (r - q) * v if u >= v else p + (r - s) * u + (s - p) * v
        )

    def level(self, chunk, focus, span):
        i, j = chunk
        cx = self.x0 + min(32 * i + 16, self.width - 1) * self.step
        cz = self.z0 + min(32 * j + 16, self.depth - 1) * self.step
        dist = math.hypot(cx - focus[0], cz - focus[1])
        return 0 if span < 100 and dist < 150 else 1 if span < 500 and dist < 350 else 2


def check():
    from build_island_strandline import Beach, SURFACE
    import json

    surface = Beach(SURFACE)
    g = Grid.read(SURFACE)
    assert (g.depth, g.width) == surface.h.shape
    props = json.loads(
        (
            SURFACE.parents[2] / "alife_game_app/assets/landscape/island/props.json"
        ).read_text()
    )
    for q in props:
        x, _, z = q["position"]
        assert abs(g.height(x, z) - surface.surface(x, z)[0]) < 1e-8
    # East side must not get spuriously pinned to the image-width index.
    for f, span in (
        ((-161.7, -38.25), 93.6),
        ((247, -184), 144),
        ((254.68, -177.68), 8),
    ):
        for j in range(12):
            for i in range(10):
                d = math.hypot((-450 + 100 * i) - f[0], (-550 + 100 * j) - f[1])
                expect = 0 if span < 100 and d < 150 else 1 if d < 350 else 2
                assert g.level((i, j), f, span) == expect
    return len(props)


if __name__ == "__main__":
    print(
        f"Same ground as physical triangles at {check()} original prop positions; all 360 camera/chunk checks passed."
    )
