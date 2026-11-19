# Shellback Reach: the placed coast

This is a portable, unbound visual layer for the real 219 m V1 route. It is not a second terrain, a replacement save, or a game release. The [full artist version](../gallery) remains available independently. The new [same-site photographs](assets/Compared.jpg) compare **that finished artist version** with the lighter bank, not the empty island.

## What to open

- **`placed-palette.glb`**: one instanced glTF object bank for the complete 1,578-site patch. One vertex-colour material, 12 actual source specimens (seven plant families), three modelled levels. Select **one** named scene: creature-close, intermediate, or guardian. glTF does not automatically switch these three scenes. The twelve instance groups in each scene use typed TRANSLATION, ROTATION and SCALE accessors under `EXT_mesh_gpu_instancing`. A loader must support that extension for the result to be a planted landscape.
- **`specimens.glb`**: the same 36 meshes as ordinary glTF 2.0, with no required extensions, for native importers lacking the instancing extension. The 12 specimens in each scene have **artificial display-shelf offsets**; take their local mesh, *not* their shelf transforms. Load geometry once per needed mesh, then use the placed bank's original per-root transforms. `roots.csv` gives readable plant family, original within-family stool, old tile and matching specimen, X/ground Y/Z, render-seat offset, yaw (radians), scale, and fit. The CSV rounds to six decimals; the float accessors are the source of truth.
- **`fitted-flakes.glb`**: the actual individual shapes inside seven pre-existing combined tiles, not copies of a generic pebble. These are the original 22,920 shell-and-clast triangles. **`bluff-foot.glb`** is the original 6,288-triangle alder-root and siltstone field. A label or repeated decoration does not replace material contact at creature height. Both paint maps are unchanged, and blend onto the original world-mapped V1 surface. `route.json` and `bluff-foot.json` carry their X/Z chart bounds. In this repo the original root mesh, both maps and both metadata files live in [the preserved gallery](../gallery).
- **`stations/`** in the portable ZIP: the real, preselected plant batches photographed from each frozen station. Use the named station's **`planted.glb` instead of** the full bank, plus that station's **`fitted-flakes.glb`** if present. Add **`bluff-foot.glb`** only at the alder and strand. These are comparison presets, not four landscapes to load together. The bank alone does not include the native island, its old landmark trees, camera, sky or ocean.

## Soils, shapes and costs

The original location of each tuft and scrub stool was recorded by the full-quality painter **before** tile assembly. A small real donor from the same family is fitted at that root. Fertile oats keep their hanging heads; sterile oats stay sterile. Width and yaw are constrained inside 16 measured horizontal support limits of the original plant; this is not an exact copy of every leaf. Median directional coverage is 79%. A few outlying fits are narrower still. Ground pins from the actual original V1 triangles place the new base at or just below the surface; the maximum extra vertical seating is 1.53 cm. No collision or ground mesh changes.

The canonical bank holds **4,442 unique triangles across all 36 meshes**, not 4,442 triangles for the whole rendered coast. Selecting an entire fixed level instead of mixing by camera yields 207,789 / 103,367 / 27,000 expanded plant triangles respectively. The corresponding expanded high artist plants were 274,596. The plant bank is 303,192 bytes. All three visual geometry files together are 1,060,600 bytes instead of 11,348,720 (90.7% smaller); excludes unchanged paint, optional fallback shelf, receipts and alternate presets.

| Locked view | Full artist triangles | Mixed/fixed selection | Full groups → selected groups |
| :-- | --: | --: | --: |
| Guardian | 274,596 | 27,000 | 16 → 12 |
| Meadow, 1.04 m | 297,516 | 78,473 | 23 → 31 |
| Alder turn, 1.04 m | 303,804 | 74,588 | 24 → 31 |
| Strand, 1.04 m | 303,804 | 77,755 | 24 → 34 |

These counts compare only the added visual layer visible to its own range rules, before frustum or occlusion culling. They include the retained local shingle/root when applicable. A group is one single-primitive source mesh on the left, or one single-primitive instance batch / unchanged local mesh on the right, not a measured renderer draw. The three eye positions use **more groups** despite substantially fewer triangles. No claim about frame time, overdraw, actual GPU instancing, animated wind, LOD popping or delivered game appearance follows from these offline Blender photographs.

The eye study uses horizontal range 17 m (detailed) and 52 m (intermediate). Beyond that it keeps projected diameter above an approximate 1.1 px; grass stops at 105 m, scrub at 148 m. Guardian uses the independent folded-shoot and blade silhouettes. The 17/52 m splits here are hard selection boundaries for camera-locked study only. Do not ship hard switching without reviewing motion and the cost of near mixed batches.

## Reopen or reproduce the artist's bank

Only the original V1 surface with SHA-256 `8d0e9cf6343ffe8ea5e82bcb7ad3e4ef77eef7207042cf0274de5d91f2457c38` is the intended mount. Coordinates are native right-handed metres: Y is up; use actual X, Z and terrain height. Keep the originally placed bodies, physical map, save identity and V1 default unmodified. No literal V1 heights or colliders are embedded in this plant asset.

From the root of the same A-Life revision, with its existing art Python environment (NumPy, SciPy, Pillow), the writer below produces the bank, all four selections, separate fitted flakes, roots and the counts:

```sh
study=studies/leeward-passage
PYTHONPATH="scripts:$study" python "$study/palette/export_lods.py" \
  crates/alife_world/assets/island-v1.bin "$study/gallery" /tmp/shellback-bank
PYTHONPATH=scripts python "$study/palette/specimens.py" \
  /tmp/shellback-bank/placed-palette.glb /tmp/shellback-bank/specimens.glb
```

The full-size pictures were shot on the artist's existing four locked cameras in Blender 4.3.2 Cycles. The guardian's planted groups and gaps still lead toward the real strand; the distance version has slightly darker/sharper little clumps than the high model. The meadow's warm seedheads still make a usable landmark. At the two low shore stations the branching root, shallow stone lenses and the open strip of sand remain in place. The largest visible concession is in the blue-grey leaves nearest the viewer: several narrower/more upright reed silhouettes replace some of the high model's broader, curling blades. See the honest same-site comparison rather than interpreting the geometry counts as equal fidelity.

**Next-week decision:** inspect a moving approach from the nursery fringe through both actual 17/52 m boundaries and down to the original strand before considering a native bind. Check the closest glaucous reed clumps, slope contact, and whether the extra near batches are worth their reduced triangle count; for a visibly thin specimen, retain a genuine local hero clump rather than widening into the clear path. If an importer does not implement the instancing extension, take the standard shelf and reconstruct batches from the stored transforms. No native setting, game, test, build or user PC was involved in this export.
