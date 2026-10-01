# Memory frame validation CPU receipt — 2026-10-01

The bounded change removes repeated whole-frame validation inside already
validated memory loops. It preserves each outer frame guard, candidate query,
episodic key, digest, retrieval receipt, and cognitive-context check. Synthetic
CPU measurements show a substantial reduction in these isolated preparation
operations; Windows gameplay FPS/TPS improvement remains Unknown.

## Existing Windows evidence and source relevance

The coordinator supplied extracted fields from the existing release receipt at
`D:/A life/target/artifacts/island-hand-implementation/island-hand-performance.json`.
That run used implementation `c050013d5cefe4760b019e46cb0ebb37d774f300`, RTX 3050,
eight Nano512 brains, and GroundedObjectSlotsV1 (profile 2). It completed 331
ticks in 60.18 seconds, with 330 paired GPU timestamp samples and eight rows
per batch. This is supplied existing Windows evidence, not a new cloud or
hardware measurement. The raw file and exact argv/environment remain unavailable
in cloud; starting save, candidate counts, and memory/topology occupancy are Unknown.

Runtime wall time averaged 173.76 ms per tick. Preparation averaged 111.85 ms
(64.4%): topology/concept 46.56 ms, episodic retrieval 40.42 ms, GPU upload
preparation 19.35 ms, and grounded perception 2.40 ms. Selection preparation
was 20.07 ms; sealed commit was 14.42 ms inclusive. Inference and learning
transactions were 5.57 and 2.32 ms; covered GPU inference and plasticity were
2.43 and 0.45 ms. Rollback copies cost 0.54 ms. Ordinary full snapshots,
checkpoint captures, and sleep captures were zero. These are nested or separately
scoped counters and must not all be summed as disjoint runtime stages.

Current `gpu_live_runtime/staged_tick.rs` performs baseline recall, cognitive
context construction, finalization, and finalized validation inside the episodic
timer. The topology/concept timer includes routed recall, cognitive projection,
finalization, and validation. GPU upload preparation calls
`GpuMemoryContextUpload::try_from_finalized`, whose construction and exact-upload
validation each call `FinalizedMemoryRecall::validate_for_frame`. The changed
loops therefore occur in all three measured preparation regions. Their precise
share of the Windows totals is not isolated by the existing receipt.

The earlier terrain radius gate does not address this GroundedObjectSlots run.
The small perception-allocation and receptor probes also do not explain its
dominant preparation time. See the [source examination](20261001-live-tick-path-examination.md)
for the earlier hypotheses and existing-recorder readout.

## Source, ownership, and preserved behavior

Branch: `codex/cloud-live-preparation-20261001`. Baseline:
`a9fefbc4a1f5a135afec26384408449813f241a8`, after the independent perception
allocation and terrain predicate changes. Both affected production files have
identical baseline blobs on measured `c050013d`, published main `e76e9e12`,
CI follow-up `ad99ac1c`, integration candidate `f8ccfb13`, and this baseline.
The previously repaired training snapshot sharing does not remove this seam.

Implementation: `911cd99f175381b041743828a2eb20b5c00e7ade` (tree
`25af0e147e852516ea9cec79c95de4254615522f`). The evidence records the exact
production blobs linked by the patched probe.

The production diff adds two crate-private helpers and changes three loop call
sites in `alife_core/src/memory.rs` and `memory_query.rs` (21 added lines, three
removed). Public singular query encoding and frame validation retain their full
input checks and error ordering. Each batch caller validates its immutable
frame before invoking the helper. Query encoding still checks candidate fields,
profile, index, slot, and target; finalized recall still validates every episodic
key and compares exact queries and authority digests. No public contract, cache,
validation state, new dependency, or runtime instrumentation is introduced.

For C candidates, `MemoryBank::recall_frame` previously validated the entire
draft 2 + 2C times: at entry, in each query encoder, and again in the returned
prepared recall and its query loop. It now performs the same two outer draft
checks. Prepared and finalized recall validation each remove C duplicate
whole-frame scans while retaining their own outer guard. Whole-frame validation
already examines all candidates, so the old nested scans grow quadratically
with candidate count. Per-candidate authentication and memory lookup work remain.

Baseline and routed recall remain distinct. Attention, topology, prediction,
learning, physiology, persistence, population, tick cadence, rollback, GPU
selection, and GPU neural authority are unchanged. This preserves the relevant
`AOA-INV-003`, `006`, `009`, `010`, `AOA-AUTH-005`, `AOA-TIME-003`, and
`AOA-MEM-002`/`003` boundaries while addressing `AOA-GOAL-002` latency.
The CPU proof does not certify causal full-organism capabilities.

Ownership is limited to these two core files, a retrieval regression test, and
dated performance evidence. Graphics, launch operations, save summaries,
training budgets, experimental N512 fixtures, and bundle/shader discovery were
not edited. No hardware test, GPU campaign, merge, PR, or local task was run or
created.

## CPU equivalence and measurements

The [one-off probe](evidence/20261001-memory-validation-probe.rs) uses existing
retrieval-test fixture helpers to build valid synthetic GroundedObjectSlots
drafts with 2, 8, or 26 candidates and banks containing 0 or 64 learned records.
Two tracked targets and four candidate families repeat across the synthetic
roster. Records are admitted through existing sealed-patch test fixtures before
timing; they do not represent GPU execution or the original save. Cognitive
memory expectancies are attached from actual fixture recall. The probe does not
execute app topology, predictor, GPU upload conversion, or a live tick.

Each timed cohort repeats eight calls on one immutable draft/bank, rather than
eight live residents. Recall includes bank digest and all recall validation.
Finalization includes prepared/draft cloning, destruction, and all finalization
checks. Finalized validation measures the public whole-recall validator.
Serialization for equivalence occurs outside timing and covers the complete
final frame, recall context, all candidate keys, receipt, cognitive context,
and memory bank. BLAKE3 digests and serialized lengths match for every case
across all baseline and patched processes.

Host: AMD EPYC 9V74/KVM Linux 6.18.44, five exposed CPUs, pinned to CPU 0.
Rust: `1.98.1 (48a229cea 2026-09-01)`. Both binaries link the existing opt-level
2 test-profile `alife_core` with lean debug settings; the probe uses `rustc -O`.
Each operation warms 20 eight-call cohorts, then records five samples of 20
cohorts. Three alternating process pairs (baseline/patched, patched/baseline,
baseline/patched) yield 15 samples per variant, operation, and case.

Median CPU milliseconds per eight calls:

| Candidates | Learned records | Recall before → after | Finalize before → after | Finalized validation before → after |
| ---: | ---: | ---: | ---: | ---: |
| 2 | 0 | 0.206908 → 0.113679 | 0.325870 → 0.219615 | 0.138891 → 0.087576 |
| 2 | 64 | 0.782150 → 0.693864 | 0.325766 → 0.218666 | 0.147114 → 0.091326 |
| 8 | 0 | 1.025269 → 0.374063 | 1.506167 → 0.647304 | 0.785551 → 0.291139 |
| 8 | 64 | 1.579148 → 0.926718 | 1.463738 → 0.654816 | 0.797586 → 0.284558 |
| 26 | 0 | 5.826487 → 1.077948 | 8.282325 → 1.987451 | 4.836587 → 0.875066 |
| 26 | 64 | 6.473649 → 1.683021 | 8.330038 → 1.921215 | 4.703639 → 0.849823 |

At 26 candidates and 64 records the observed reductions are 74.0%, 76.9%, and
81.9% respectively. All phase/case sample ranges are disjoint except the
two-candidate, 64-record recall case. These isolated observed medians are not a
statistical guarantee or a projected Windows tick improvement. The production
candidate roster and resident memory occupancy are still Unknown.

[Raw evidence](evidence/20261001-memory-frame-validation.json) retains all
samples, summaries, source blobs, executable/probe hashes, and supplied Windows
fields. The probe includes a test file whose new regression differs between
revisions, but `#[test]` functions are excluded from standalone compilation;
hashes of every executed fixture helper are identical and retained separately.

To reproduce each source revision separately, copy this probe to the same
relative path and retain its executable before rebuilding the next revision.
Select matching cached dependencies through Cargo artifact JSON:

```bash
source /workspace/cloud-cpu-validation/env.sh
cargo build -p alife_world --profile test --no-default-features \
  --message-format=json > /tmp/memory-build.jsonl
python3 - <<'PY'
import json, os, pathlib, subprocess
artifacts = {}
for line in pathlib.Path('/tmp/memory-build.jsonl').read_text().splitlines():
    record = json.loads(line)
    if record.get('reason') == 'compiler-artifact':
        libs = [p for p in record['filenames'] if p.endswith('.rlib')]
        if libs:
            artifacts[record['target']['name']] = libs[0]
args = ['rustc', '--edition=2021', '-O',
        'docs/performance/evidence/20261001-memory-validation-probe.rs',
        '-L', 'dependency=' + os.environ['CARGO_TARGET_DIR'] + '/debug/deps',
        '-o', '/tmp/memory-validation-probe']
for name in ['alife_core', 'serde_json', 'blake3']:
    args += ['--extern', name + '=' + artifacts[name]]
subprocess.run(args, check=True)
PY
taskset -c 0 /tmp/memory-validation-probe
```

The world package build only supplies the dependency artifacts; the probe
executes core memory operations. No GPU device is constructed.

## Validation and remaining proof

- `candidate_memory_queries`: 7 passed.
- `candidate_memory_retrieval`: 20 passed, including the new populated-recall
  equivalence and changed-context rejection regression.
- `perception_digest_golden`: 3 passed.
- GPU backend `closed_loop_memory_context` without default features: 11 CPU
  tests passed; device tests gated by `gpu-tests` were not enabled.
- Scoped core Clippy with `-D warnings`: passed for the library and three
  affected test targets.
- `cargo fmt --check -p alife_core`, probe `rustfmt --check`, and
  `git diff --check`: passed.
- `bash scripts/docs_check.sh`: 77/77 assertions passed.
- `bash scripts/check_core_boundaries.sh --static`: passed.
- Independent Sol 6.1 review (R2): source, commit/tree/blobs, Cargo build records,
  probe/fixture/executable identities, all raw summaries, and complete serialized
  output proofs verified; no blockers or remaining nonblocking findings.

Rollback is reverting implementation `911cd99f`;
the earlier allocation and terrain changes are independent. The remaining
integration proof belongs to the desktop owner: use the same save, graphics,
build mode, run mode, and diagnostic environment with the existing performance
recorder. Preserve preparation stage totals, candidate counts, memory occupancy,
GPU timestamp coverage, and capture counts. No new profiling framework or
training launch is required to compare these gameplay receipts.
