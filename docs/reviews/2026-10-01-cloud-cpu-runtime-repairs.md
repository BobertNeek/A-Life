# Cloud CPU runtime repair validation — 2026-10-01

The four confirmed CPU runtime gaps are repaired on
`codex/cloud-ci-followup-20261001`. The separately reviewed perception allocation
change is included as two exact patch-ID cherry-picks. This validates the affected
CPU paths and development checks; it does not certify live GPU control, rendered
behavior, founder competence, or RTX performance.

## Source boundaries

- Repair base: `ad99ac1c31c53d8898703268446a2f785586fe4e`.
- Four-gap code boundary: `e6252f8ed9f454963576bde6e54088ba066696b1`.
- Combined code/evidence boundary: `b8280594c27569bd619491f17c5c3a9f17b34dd3`.
- Frozen integration candidate: `f8ccfb13d1c267ac1e83c29504d09f204bc36ffc`;
  its worktree remains clean and unchanged.

The four repair commits are `e802b13`, `fa57010`, `c218aea`, and `e6252f8`.
Perception source/evidence commits `352ff546` and `e993fcfc` are retained as
`6e7d7d1` and `b828059`, with exact stable patch IDs. This report is a subsequent
documentation-only checkpoint; source execution below binds to `b828059`.
No main merge, desktop interaction, N2048 actor edit, or campaign launch occurred.

## Causes and resulting behavior

| Gap | Cause | Result |
| --- | --- | --- |
| Populated sandbox saves | Export supplied no creature summaries while retaining organism and habitat ownership; full saves exceeded the inline transport bound. | The session retains source cognition, GPU metadata and asset references, updates biology projections from owned records, and uses file serialization. Missing source cognition fails explicitly. |
| App bundle admission | Blanket small-file and 768 KiB summary caps rejected current population saves and approved art. Directory discovery counted the offline training shader. | Canonical P34 file validation owns save admission. Config/WGSL and manifest bounds remain enforced before decoding. Approved art retains its canonical 12 MiB individual/32 MiB total limits; discovery validates all 14 production shaders. The obsolete 768 KiB policy constant is removed. |
| Founder demonstrations | A fixed 32-command window ended before a 4 m approach reached contact at 0.1 m per interval. | The budget uses initial visible distance and actual first-step displacement, reserves inspection/ingestion, and enforces the existing 2048-step replay limit. Every causal receipt is retained; failed or zero progress rejects. |
| Experimental N512 fixture | The archived decoder/provenance bytes failed the current strict fixed-graph constructor. The explicit launch option is still reachable. | The original asset and README are archived byte-for-byte. A reproducible offline tool preserves all 1799 weight words and invariant ABI bytes while rebinding the current manifest. It is bootstrap and unpromoted, with current behavioral evidence Unknown. |

Persistence and error handling were reviewed against AOA-PERSIST-001/002/004;
ordinary world consequences and sequence retention against AOA-WORLD-003 and
AOA-TIME-001/003. N512 evidence remains limited under AOA-FOUND-007 and
AOA-OBS-005. These are scoped contract checks, not full architecture compliance.

The [N512 provenance receipt](../../assets/founders/scaled-choice-nociceptive-v1/rebind-receipt.json)
records zero optimizer steps and zero GPU execution. Current canonical digest:
`6874680625c5b9d87e97f3ba7eceb7dc2f07b38a2f1ebb3f82f902683611ed52`.
The original still fails strict current admission; no historical runtime
compatibility or relaxed decoder validation was added.

The [perception evidence](../performance/20261001-live-perception-preparation.md)
reports only about 1–21 microseconds saved per synthetic eight-row cohort with
overlapping timing ranges. Packed payload digests match. This is CPU preparation
evidence on its original source boundary and does not resolve the measured
Windows frame time or establish an RTX improvement.

## Checks

The subsequent [CPU CI policy validation](2026-10-01-cloud-cpu-ci-policy-and-consolidation.md)
corrects the earlier filter classification: five of the 130 names were CPU-safe
optional-diagnostic/file tests. The 125 remaining names require hardware.
The historical run totals below remain unchanged; the successor retains those
five tests and uses the normal unfiltered workspace suite.

The four-gap boundary passed 469 CPU tests across 73 targets: zero failures,
14 ignored, and 32 test names filtered. Strict workspace Clippy passed.
An independent read-only review found no actionable source or evidence blockers.

At the combined boundary:

- **644 CPU tests passed across 97 targets; zero failed; 14 ignored; 130 test names filtered** using the existing audited explicit test-name list.
- All five original positive failures passed; their useful assertions remain.
- Full populated save authority, acquired/GPU metadata references, larger file
  roundtrips, inline transport rejection, missing/oversized metadata, shader
  inventory, complete demonstrations, and canonical New Game restoration passed.
- All 29 perception buffer contracts passed, including whole-upload equality
  after overflow and valid rebasing to the representable boundary.
- The meaningful headless benchmark smoke at tiers 1 and 10 passed.
- Format and strict whole-workspace/all-target Clippy passed.
- Static and dependency boundary checks, boundary self-test, and all 77
  documentation assertions passed.

Affected-package compilation and execution use `alife_game_app`,
`alife_training`, `alife_tools`, and `alife_gpu_backend`, with app `gpu-runtime`
for production type coverage. Real GPU test bodies are excluded from execution.
No full workspace build/test repeat was performed in this repair round.

## Environment and resources

Debian 13, official Rust/Cargo 1.98.1 stable, rustfmt and Clippy; cc, pkg-config,
and Python available. The environment has a four-CPU quota, 16 GiB memory limit,
no swap, and a 32 GiB filesystem. Every Cargo command sources the lean CPU setup:

```bash
source /workspace/cloud-cpu-validation/env.sh
# CARGO_INCREMENTAL=0
# CARGO_PROFILE_DEV_DEBUG=0
# CARGO_PROFILE_TEST_DEBUG=0
# CARGO_BUILD_JOBS=2
# RUST_TEST_THREADS=1
```

Combined compilation took 351.229 seconds and CPU execution 218.097 seconds.
Peak sampled cgroup memory was 12.59 GiB, including file cache; minimum free disk
was 19.25 GiB. No resource stop or generated-output cleanup was needed.

Cloud receipts and exact commands remain under
`/workspace/A-Life/target/artifacts/cloud-cpu-combined-followup-ad99ac1/`, with
`REPORT.md`, `terminal-summary.json`, `results.json`, `test-summary.json`, source
metadata, byte-preservation provenance and per-check resource journals.
The preceding repair-only round is preserved under
`/workspace/A-Life/target/artifacts/cloud-cpu-runtime-gaps-ad99ac1/`.

Do not reuse compiled test executables across worktrees whose checkout mtimes
may predate them. Registry downloads can be shared; use a per-worktree target
when changing source boundaries. The prior stale core-budget binary was
independently rebuilt and passed in an isolated target; this round remained in
one source worktree, and changed dependencies were rebuilt before execution.

## Remaining evidence

GitHub Actions status is unverified here: its read-only API request returned
Forbidden. Credentials were not changed and no permission workaround attempted;
the parent will inspect CI for the exact published SHA through its connected
read tool.

GPU and graphics execution, fresh current-fixture N512 behavioral evaluation,
Windows/RTX end-to-end performance, and the live training campaign remain unrun
here. The campaign remains approval-blocked and is owned by the parent.
