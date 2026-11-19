# Northern massif: separate physical island study

This is **not** a new default game terrain or a save migration. The study builds
a second physical map and its render asset from the same 321 × 385 samples. The
shipped terrain, its old saves, brain fixtures, and the original drafts' materials
remain in place. No simulation backend or user save is required to view it.

```sh
python scripts/prototype_island_ridge.py /tmp/qualia-ridge-v2
python scripts/verify_island_ridge.py /tmp/qualia-ridge-v2
cargo test --locked -p alife_world --test island_ridge_v2_prototype -- --ignored
```

The first command requires NumPy, SciPy and Pillow; it is opt-in, writes to the
given directory and refuses to overwrite crate assets. It produces
`terrain-chunks.glb`, its five editable PNGs, `props-ridge-v2.json`, and
`island-ridge-v2.bin`. The binary in this directory is the identical frozen
compatibility fixture consumed by the Rust test (SHA-256:
`0f4a908856766fce5287c080a1edde3dc54446d1247474e217060ae660b93d0d`).
Existing `island-v1.bin` stays pinned at
`8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38`.

[Marked mountain A/B](physical-ridge.jpg) and [habitat at 88 cm](physical-habitat.jpg):
matched CPU Blender studies using actual generated meshes, not native Bevy or
target-GPU evidence. The western summit is a tilted shoulder, the eastern crest
stays pointed, and the river cut remains. Only samples over 105 m change. The
separate near material is part of the base terrain draft, not an effect of
reshaping the mountains.

[Local geometry receipt](validation.json). The graph checks full triangle grade
against 0.85, includes only triangles above the wading limit, and discards a
whole cell when a solid obstacle (expanded for a 0.2 m body) overlaps it. This
is a conservative **corridor comparison**, not an exhaustive pathfinder or an
organism training result. All 37,795 triangles in the nursery's original
component remain identical in reachability. High mountain traversability is
not unchanged: 1,262 grade-legal faces become too steep and 1,250 formerly
illegal faces pass. All five individually reviewed landmark anchors stay put.
All 3,716 props keep their x/z, yaw, scale and ground-relative anchor; all
1,716 solid proxies retain footprint and vertical extent. The largest intentional
height change is 27.31 m; the peak remains 240 m.

The native test constructs both maps in memory, restores each using A-Life's
real full-world save/restore path, checks ray/surface agreement, and rejects a
V2 payload paired with the old binding. Only the dedicated branch workflow
requests it; the test is ignored in an ordinary build. There is no toggle in
the production game and no checked-in duplicate material pack.
