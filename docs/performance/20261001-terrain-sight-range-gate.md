# Terrain sight radius gate receipt — 2026-10-01

The bounded change checks the existing eight-unit sight radius before casting
an object's terrain visibility ray. Objects outside that radius already cannot
enter the observation result. Skipping their pure visibility calculation
preserves the exact perception result and all simulation work.

## Source and scope

Branch: `codex/cloud-live-preparation-20261001`. Baseline:
`3e8f3578dd7f2c967cce5ebd8fdf3dfee27ba618`, following the source examination
and earlier allocation fix. Implementation:
`163ec10b156ced439faca4aa9753501ae57da6af` (tree
`7dc94e52f95b7c7e38ad061186f86151947e1575`).
The only production edit is the predicate in
`crates/alife_world/src/headless.rs::physical_observation_snapshot_from_objects`.
The baseline predicate is unchanged on published main `e76e9e12`, CI follow-up
`ad99ac1c`, and integration candidate `f8ccfb13`.

No object is removed from the world or spatial index. Every accepted target
receives the same sight check, properties, contact test, confidence, sorting,
truncation, and downstream frame validation. Distant objects still participate
as occluders of accepted targets and terrain fan rays. The 16-ray terrain fan,
learning, physiology, persistence, population, and tick cadence remain intact.
The existing `<=` comparison preserves inclusion at exactly eight units and
exclusion of non-finite distance results. `object_in_sight` and its callees have
no mutation, tracking update, diagnostic counter, or fallible side effect.

The indexed perception query collects neighboring spatial cells before this
radius test. Those cells can contain objects beyond eight units, so this gate
also applies to the current live indexed path. GroundedObjectSlots does not cast
terrain sight rays; the CPU benefit is conditional on TerrainVision and nearby
out-of-radius objects. Subsequent coordinator extraction identifies the measured
Windows run as GroundedObjectSlotsV1, so this gate does not address that run.
Its dominant preparation path is investigated in the
[memory-validation receipt](20261001-memory-frame-validation.md).

This slice owns that predicate and its evidence. Other world changes, graphics,
training, launch operations, and the other CPU worker's files were not edited.
No GPU device, hardware test, training campaign, merge, or PR was run or created.

## CPU evidence

The [one-off probe](evidence/20261001-terrain-sight-probe.rs) calls the public
indexed perception preparation path for eight synthetic observers. Its flat
terrain contains 1,716 synthetic boxes: one near occluder and 1,715 distant
boxes. It also contains a visible near target, an occluded near target, a target
at exactly eight units from the first observer, and one at 8.0001 units.
Additional objects at 9–15 units lie within neighboring cells and within the
gaze cone, so the eager baseline actually casts their rays. Cases use 0, 32,
or 128 such objects and both grounded profiles.

These are synthetic world-preparation calls, not authoritative live ticks or
the island save. The box count matches the published island proxy count, but
geometry and object density do not reproduce the island. No neural phenotype,
GPU dispatch, learning step, or full simulation tick is executed by the probe.

Host and toolchain are the same EPYC/KVM Linux CPU and Rust 1.98.1 as the
[source examination](20261001-live-tick-path-examination.md). Both variants link
test-profile `alife_core` and `alife_world` rlibs at the existing opt-level 2,
with lean debug settings. The standalone probe uses `rustc -O`. Each process
warms 20 eight-row cohorts, then takes five samples of 20 cohorts. Timing
includes complete indexed draft preparation and destruction; final draft JSON
serialization and BLAKE3 comparison occur outside the timed section.

Three alternating process pairs provide 15 samples per variant and case. The
order is baseline/patched, patched/baseline, then baseline/patched. Final
results, sample ranges, executable hashes, and source identities are retained in
the [CPU evidence](evidence/20261001-terrain-sight-range-gate.json).

| Profile | Additional distant objects | Baseline median ms per eight rows | Patched median ms per eight rows |
| --- | ---: | ---: | ---: |
| GroundedObjectSlots | 0 | 0.512695 | 0.520635 |
| GroundedObjectSlots | 32 | 0.546673 | 0.538314 |
| GroundedObjectSlots | 128 | 0.545125 | 0.537739 |
| TerrainVision | 0 | 1.575748 | 1.463924 |
| TerrainVision | 32 | 3.477288 | 1.545382 |
| TerrainVision | 128 | 10.277293 | 1.603530 |

For TerrainVision with 32 and 128 additional distant objects, observed median
reductions are 55.6% and 84.4%, saving about 1.93 and 8.67 ms respectively in
this synthetic cohort. Those sample ranges do not overlap. Control-profile and
zero-additional-object ranges overlap; their median shifts are not a reliable
performance result. Zero additional objects still includes the explicit radius
boundary targets, some outside individual observers' radius.

Full serialized draft BLAKE3 hashes and lengths match for every before/after
case, including both sensor profiles and all densities. The evidence does not
establish Windows FPS/TPS, GPU execution cost, statistical certainty, or the
dominant cause of the published roughly 180 ms tick.

To reproduce each revision separately, retain its executable before rebuilding
the next revision. The artifact JSON avoids selecting a mismatched cached rlib:

```bash
source /workspace/cloud-cpu-validation/env.sh
cargo build -p alife_world --profile test --no-default-features \
  --message-format=json > /tmp/world-build.jsonl
python3 - <<'PY'
import json, os, pathlib, subprocess
artifacts = {}
for line in pathlib.Path('/tmp/world-build.jsonl').read_text().splitlines():
    record = json.loads(line)
    if record.get('reason') == 'compiler-artifact':
        libs = [p for p in record['filenames'] if p.endswith('.rlib')]
        if libs:
            artifacts[record['target']['name']] = libs[0]
args = ['rustc', '--edition=2021', '-O',
        'docs/performance/evidence/20261001-terrain-sight-probe.rs',
        '-L', 'dependency=' + os.environ['CARGO_TARGET_DIR'] + '/debug/deps',
        '-o', '/tmp/terrain-sight-probe']
for name in ['alife_core', 'alife_world', 'serde_json', 'blake3']:
    args += ['--extern', name + '=' + artifacts[name]]
subprocess.run(args, check=True)
PY
taskset -c 0 /tmp/terrain-sight-probe
```

## Validation and next evidence

The existing terrain visibility/occlusion, perception-candidate, and grounded
object-slot suites exercise the affected contracts. The before/after synthetic
draft comparison adds TerrainVision radius-boundary coverage to the CPU proof.
Independent Sol 6.1 review checks the predicate and measured evidence.

- Terrain visibility unit tests: 6 passed.
- `perception_candidates`: 20 passed.
- `grounded_object_slots`: 15 passed.
- `cargo clippy -p alife_world --lib --test perception_candidates --test grounded_object_slots --no-default-features -- -D warnings`: passed.
- `cargo fmt --check -p alife_world`, probe `rustfmt --check`, and
  `git diff --check`: passed.
- `bash scripts/docs_check.sh`: 77/77 assertions passed.
- Independent Sol 6.1 review (R2): source predicate, probe/executable identities,
  all raw summaries, and before/after draft identities verified; no blockers.

Rollback is reverting implementation commit `163ec10b`; the earlier allocation
change is independent.

The [source examination](20261001-live-tick-path-examination.md#smallest-next-measurement)
specifies the minimum existing-recorder gameplay measurement if the original
receipt cannot be recovered. Preserve identical graphics, starting save, and
run mode for any later end-to-end comparison. This source optimization does
not claim an improvement for the original GroundedObjectSlots run or
establish its out-of-radius target density.
