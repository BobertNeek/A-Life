# Combined plant authored-preview rotation correction, 2026-10-16

Status: PASS for the bounded local correction and checks below. This successor
is based on exact combined Git tree `d73077af17f96a73576abc64cc92ea2fa2947506`.
It was made in a fresh copy. No owner tree, remote, approval, asset, native source,
placement file, GPU, training run, installation or heavy build was changed.

## Correction and source lineage

Both authored-patch renderers copied meshes imported in `QUATERNION` mode and
then assigned `rotation_euler`, leaving authored yaw inactive. Each renderer now
sets `rotation_mode='XYZ'` immediately before assigning authored yaw. These are
the exact two one-line fixes from the separately reviewed preview correction.
The same placement blocks also orient their surrounding prop copies.

The two independent profiles and the original combined profile remain
byte-identical historical artifacts. Their original executable pins still
identify their historical renderers; those profiles are not rewritten to imply
that the correction was present in earlier evidence.

The active fixed successor is `scripts/fern-coastal-preview-rotation-profile.json`.
It retains all 140 art/world records and 48 manifest entries, preserves historical
profile hashes, and records explicit old/new SHA-256 transitions for only the
two renderer sources. `check_fern_coastal_composition.py` validates those exact
transitions before applying corrected source pins. Its 57 current/retained
source pins include the new focused probes. The existing CI job runs the same
74-test suite followed by four new stdlib guards, without adding dependencies.

- Coastal historical renderer: `995802c09efb276e0295aba76810abeecd5026fb1108a642f71a26f2f6d9d1ed`
- Coastal corrected renderer: `c4b5edb65856ce7a34d526958ff8f8c353e048ab6760160b5a2ac405f5e2b91a`
- Fern historical renderer: `fd28e3174657d56595b67c935e6e654034a73e7ea8e5c39ab3dd01e3bed6ad26`
- Fern corrected renderer: `bd533a774db83863e42bd65c8f0d635a1cbd8a717419034b24b0b62fb905d466`

The original combined source package was checked against all 1,361 tracked
Git blobs and was unchanged. In the corrected copy, 1,357 existing files remain
byte-identical; the four changed existing paths are the two renderers, combined
checker and existing CI workflow. New files hold the successor profile, probes
and this dated evidence. All historical review images and receipts are retained.

## Measured regression results

- Existing combined stdlib suite: PASS, 74 tests, no skips
- New stdlib guards: PASS, four tests
- Combined art/source/manifest audit: PASS, 140 art/world files, 48 entries
- Blender 4.3.2 CPU, no-render matrix regression: PASS, 24 corrected placements
  and 24 historical Euler-only negative controls
- Maximum corrected matrix-component error: `5.960464477539063e-8`
- Island shipping checks: PASS
- Boulder placement regressions: PASS, six tests
- Documentation checks: PASS, 77/77 assertions

The Blender probe imports each baseline/current Fern and CoastalShrub GLB from
this actual combined tree, including all three LODs. For each renderer it
extracts and executes the actual contiguous location/mode/Euler/scale statements
on imported mesh copies. It compares their world matrices against independently
constructed translation × authored-yaw × uniform-scale matrices. Removing only
the mode assignment reproduces inactive Euler yaw on every imported mesh.
Synthetic translation/ground height are used to isolate rotation; authored yaw
and scale come from the unchanged placement file. This is a focused mode
regression, not a full-scene placement or visual-composition audit.

The runtime and native SHA-256 values remain:

- Fern runtime: `00adc40d44b34bb99e81407efb3944ab53c1c48a2851c883df6ca784d4e57aa3`
- Fern native: `dac2ac84d94bcc15127e9bbaf42e683f2eb2d7dea99ee3d965bfbad2c2379cc5`
- CoastalShrub runtime: `7a2636250308b67ba4794914b8edc0b1738646233a249c78a5f42573944f2a18`
- CoastalShrub native: `65a83ed2922ad989fe9e4a67b1d6adcac7cc373bc72477e4ef584a4d64a6198d`

## Evidence and reproduction

- [Fresh combined audit](2026-10-16-combined-preview-rotation/art-check.json)
- [Exact matrix proofs](2026-10-16-combined-preview-rotation/rotation-matrix-receipt.json)
- [Preservation audit](2026-10-16-combined-preview-rotation/input-preservation.json)
- [Gate exit codes](2026-10-16-combined-preview-rotation/gate-exit-codes.json)

```sh
python -S scripts/check_fern_coastal_composition.py
python -S -m unittest discover -s scripts -p 'test_fern_coastal_composition.py' -v
python -S -m unittest discover -s scripts -p 'test_authored_preview_rotation.py' -v
blender -b -t 2 --factory-startup --python-exit-code 1 \
  --python scripts/check_authored_preview_rotation.py -- --output /tmp/rotation-receipt.json
```

## Evidence limits

No new images were rendered. The eight corrected individual-package views in
the separate correction bundle still certify only their individual input
packages. They are not combined-scene evidence. No combined visual-composition
acceptance, live-game rendering, LOD transition, GPU/FPS, whole-workspace build,
exact-head remote CI, publication or merge is established here. Historical
native/export-order receipts retain their original source scope; those exporters
were not rerun by this correction. The pending remote approval remains untouched.
