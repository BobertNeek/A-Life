# Fern + CoastalShrub composition rehearsal, 2026-10-16

Status: PASS for the bounded local source/asset composition below. No remote
write, branch publication, merge, readiness change, deployment or installation.
This is a disposable local rehearsal, not exact-head CI or game acceptance.

## Inputs and retained ownership

- Fern: approved PR #72 head `b45b532227569406f3dd2b3ec9404544d60ae5cf`.
  The local input snapshot exactly reproduces published Git tree
  `bf21f5ac227dc2da5485b1a8142f3c92d091065e`.
- CoastalShrub: independently approved exact folded-leaf prototype bytes,
  integrated on common main `73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`.
- Both native sources, both exact prior-runtime snapshots, the older native
  sources, both independent profiles/checkers, and both original review/evidence
  sets are preserved. Original review screenshots retain their original branch
  contexts; no new combined screenshot was produced.
- Fern native SHA-256:
  `dac2ac84d94bcc15127e9bbaf42e683f2eb2d7dea99ee3d965bfbad2c2379cc5`.
  Fern runtime SHA-256:
  `00adc40d44b34bb99e81407efb3944ab53c1c48a2851c883df6ca784d4e57aa3`.
- CoastalShrub native SHA-256:
  `65a83ed2922ad989fe9e4a67b1d6adcac7cc373bc72477e4ef584a4d64a6198d`.
  CoastalShrub runtime SHA-256:
  `7a2636250308b67ba4794914b8edc0b1738646233a249c78a5f42573944f2a18`.

The original worker trees were not edited. The final input-retention audit
checks all 1,334 Fern tracked files against the exact input tree and all 21
CoastalShrub package paths against its integration receipt.

## Reconciliation

`check_fern_coastal_composition.py` is a new fixed successor, rather than a
weakened version of either independent profile. Its fixed inventory contains
140 art/world files, the exact union of four new native/baseline files and both
approved runtime changes. It validates byte lengths, SHA-256, executable modes,
path completeness and symlink rejection. All other art/world data and placements
remain byte-retained; this includes terrain, collision proxies, all 170 shrub
placements, Broadleaf, Conifer, BerryBush and creature art.

The specification retains both new source/recipe registrations and their
original dimensions. The production manifest retains all 48 unique entries,
with exactly the Fern and CoastalShrub digest/size/generator deltas from the
common predecessor. The other 46 registration records are unchanged.

The source inventory pins 54 retained/current files, including both independent
profiles, both checker implementations and their imported geometry helpers.
The existing CI job ID `broadleaf-canopy` and name `Broadleaf canopy CPU regression`
remain unchanged. Its bounded Python steps advance to the combined successor and
regression loader. No duplicate frontend build or new dependency is introduced.

The union loader runs every retained Fern and CoastalShrub test except their
four independent full-tree positive cases, which are replaced by combined
positives. All retained Broadleaf negative controls continue to run once.
Both independent full-tree checkers intentionally reject the later combined
snapshot; their profiles and implementations remain unchanged.

## Results

- Combined standard-library art/source/manifest audit: PASS, 140 files, 48 entries
- Combined standard-library regressions: PASS, 74 tests, no skips
- Island shipping terrain/placement/physics contracts: PASS
- Boulder placement regressions: PASS, 6 tests
- Documentation assertions: PASS, 77/77
- Git whitespace/diff check: PASS
- Fern native topology: PASS, seven rooted connected fronds per LOD
- CoastalShrub native audit: PASS, retained stems, all 108 blade attachments,
  unchanged L2, unchanged native/runtime BerryBush and exact source reload/export

Fern L0/L1/L2 remain 700/294/154 triangles and 952/441/231 exported vertices.
CoastalShrub remains 672/204/218 triangles and 600/276/163 exported vertices.
Both retain exact original per-LOD bounds and one-primitive bindings.
Broadleaf remains 4,462/916/86 triangles. The two current GLBs total 93,720 bytes.

## Verified export order

The installed Blender 4.3.2 was used in background mode, two CPU threads, with
no rendering or GPU operation. The bounded rehearsal first ran the historical
Fern and two-species shrub exporters. Both overwrite the corresponding newer
runtime as expected. It then reapplied connected Fern and new CoastalShrub,
refreshed registrations, and reran the complete combined audit. Both exact
approved runtime hashes and every native/art/world byte were restored.

Reversing the two current exporters produces the same final bytes and passes
the same 140-file audit. Thus their mutual order commutes; each must still run
after its historical exporter. Normal reload/export does not repeat authoring
or save native sources. The shared README records this precedence explicitly.

## Reproduction

```sh
python -S scripts/check_fern_coastal_composition.py
python -S -m unittest discover -s scripts -p 'test_fern_coastal_composition.py' -v
python -S scripts/check_island_art.py
python -S -m unittest discover -s scripts -p 'test_island_boulder_placement.py' -v
bash scripts/docs_check.sh
```

The optional export-order command temporarily replaces the two runtime GLBs,
so run it only in a disposable checkout with installed Blender 4.3.2:

```sh
python -S scripts/check_fern_coastal_export_order.py
```

The new [art receipt](2026-10-16-fern-coastal-composition/art-check.json),
[export-order receipt](2026-10-16-fern-coastal-composition/export-order.json),
[Fern native check](2026-10-16-fern-coastal-composition/fern-native-check.json),
[CoastalShrub native check](2026-10-16-fern-coastal-composition/coastal-native-check.json),
[input retention](2026-10-16-fern-coastal-composition/input-retention.json) and
[gate exit codes](2026-10-16-fern-coastal-composition/gate-exit-codes.json)
record the measured results.

## Limits and next boundary

No live frontend, game capture, combined visual composition review, live LOD
transition review, physical GPU/FPS, whole-workspace build, PC activity or training
was run. The current runtime still culls the Fern and CoastalShrub L2 selections;
those contracts were preserved rather than changed. Lower geometry/file counts
are not performance evidence. Existing source-bound render and glTF validation
receipts remain valid for the exact byte-retained individual assets.

The binary integration patch is based on the exact Fern PR #72 tree. It includes
the approved shrub package and shared metadata/check reconciliation. Applying it
to a different base requires a fresh reconciliation. Exact-head remote CI and
publication are outside this local-only rehearsal; no merge is authorized.
