# Cloud candidate validation — 2026-10-01

Correctness baseline: `a11dae6a5708bddf7d4255990f3bd4abaf12759a`.
This consolidates the completed cloud rounds. Main integration, GPU/Windows
gameplay, training and 20 FPS with 50 creatures remain separately owned.
The [CPU performance record](../performance/20261001-cpu-preparation.md) retains
measurement conditions and links to source-bound samples/probes; no game-speed
claim follows from the tests below.

## Finalization and serial comparison checkpoint

Source `d9724dfa03bb367809f35ae66900de55dd843f83` extends `ad49a325` with
reviewed finalization `4eed7acc` and evidence `7a8de010`, picked as `03003006`
and `d9724dfa`. The earlier fresh-recall guard is retained exactly once.
The five stale draft/owner/tick rejection cases now also exercise finalization.
Core debug/release each pass 45 tests; app attention/upload and 14 backend
memory-context checks pass: **105 passing executions, zero test failures**,
with the developer benchmark ignored in each Core profile. Strict Rust 1.99
workspace, production/GPU-test all-target and release Core Clippy pass.
Format, static boundaries, 77/77 documentation assertions and local links pass.

The [paired serial comparison](../performance/20261001-cpu-preparation.md#integrated-serial-guard-comparison)
measures both guards together against `dc3e916e`: all 864 batches/22,896 typed
instance checks and 24 input/result hashes match. At 50 owners/26 candidates/
64 records, helper wall is 63.954→49.095 ms and process CPU 63.939→49.091 ms;
this excludes live admission, topology observation, GPU and rendering work.
No performance percentages were stacked. The original report/probe/builder
are unchanged; original JSON formatting is 7,357→1,103 lines with no lost data.

The comparison link-search path was corrected without rebuilding libraries.
An attempted shared-target successor reused baseline rlibs and was rejected
before any timing; accepted successor artifacts use an isolated directory and
distinct hashes. No guard fired; memory stayed below 12.13 GiB and disk above
10.16 GiB. This checkpoint remains serial; later worker integration is separate.

## Fresh-recall successor

The successor extends green `dc3e916e` with reviewed source `e8d5633e` and
evidence `61edc70b`, picked without conflicts as `b92fca07` and `a807a6b8`.
The production diff is three added lines in `memory.rs`: the fresh private
result self-check becomes a debug diagnostic. Public validators stay strict;
live runtime, staged preparation and backend ordering remain serial and unchanged.

Checks bind source `a807a6b8010025ca760f23f86eee705653bebed8`. Core passes
45 tests in debug and 45 in true release, with the developer benchmark ignored
in each profile. App attention/upload equivalence, 14 backend memory-context
tests and tier-1/tier-10 headless smoke also pass: **106 passing executions,
zero test failures**. Strict Rust 1.99 workspace, production/GPU-test all-target
and release Core all-target Clippy pass with `-D warnings`. Format, static
boundaries, documentation assertions (77/77) and changed local links pass.
Jobs/tests were one/serial; memory peaked below 10.88 GiB, disk stayed above
11.44 GiB and no guard fired. Final edits change documentation/formatting only.

The [source-bound comparison](../performance/20261001-parallel-preparation.md),
probe and builder are unchanged; JSON is lossless formatting from 3,003 to
949 lines, retaining every field and ordered sample. Threaded code remains
evidence-only. Performance was not remeasured; prototype throughput does not
establish production parallelism, frame rate or responsiveness. Successor remote
CI and GPU/Windows gameplay remain separately owned.

## Topology successor

The successor extends `851c3a6ffcc498d6b2b3e9a43969cc57aa1308ec` with reviewed
topology `bf8f6b48` and evidence `b94c9f12`, picked without conflicts as
`035d76fa` and `0815e41c`. Source, report and probes match the originals;
the JSON is formatting-only compaction from 2,258 to 840 lines, preserving
every field and ordered sample. The predecessor's CI and fast gates are green.

Checks bind source `0815e41c8db2ed906cf6ee01898630e8aee51eb6`: debug and true
release each pass 12 map, eight sidecar and one private-plan diagnostic test.
App object-bound attention/upload equivalence and tier-1/tier-10 headless smoke
also pass: **44 passing executions, zero failures**. Strict Rust 1.99 workspace,
production/GPU-test all-target and release Core all-target Clippy pass with
`-D warnings`. Formatting, static boundaries, 77/77 documentation assertions and
changed local links pass. Jobs/tests were one/serial; observed memory peaked at
10.67 GiB, disk stayed above 11.60 GiB and no guard fired.

The final edit changes documentation/JSON formatting only. The separate
[topology CPU evidence](../performance/20261001-topology-preparation.md) is
preserved; its observation timing excludes the earlier attention chain.
Neither timing was repeated or combined into a gameplay estimate. Final-source
GPU/Windows gameplay, training and remote successor CI remain separately owned.

## Prepared-attention successor

The successor combines reviewed attention source
`471c55dfc2a29e760669e8bb6252865464e064fa` with the documentation cleanup.
All five attention source files and its three evidence files match that original
byte-for-byte; later topology work is excluded. Checks bind
`b242d2c315b1a46007597603d654db521888141d`, or its source-identical `771b9ef3`
before the final documentation-only payload trim.

Affected CPU checks pass: retrieval 35, queries 7, perception digests 3, app
attention/upload equivalence 1, backend memory 14 in each assertion mode and
headless benchmark smoke 1: **75 passing executions, zero failures, one ignored
developer benchmark**. Strict Rust 1.99 workspace CI Clippy and Core/World/backend/
app production/GPU-test all-target Clippy both pass with `-D warnings`.
Formatting, static boundaries, documentation assertions (77/77) and changed
local links pass. Peak observed cgroup memory was 10.27 GiB; disk stayed above
11.92 GiB. One build job and serial tests were used; no guard fired.

No performance benchmark was repeated for integration. The attention owner’s
separate [CPU evidence](../performance/20261001-attention-preparation.md) retains
its original limits. Current full-workspace/hardware execution remains separate
from these focused checks; the subsequent report edit changes documentation only.

## Base checks

Tests and four-package CPU/production Clippy bind code
`87c19312a34bb602198ddcda40c3c8690030138f`. The later Tools change
`b99ea6f29019014351006359879fb23d332d3917` removes only two redundant closure
borrows; its subsequent `a11dae6a` commit updates documentation only.

| Check | Result and command |
| --- | --- |
| Core | 465 passed, one ignored developer benchmark; `cargo +stable test --locked -p alife_core --all-targets -- --test-threads=1` (stable 1.98.1) |
| World | 67 passed; `cargo +stable test --locked -p alife_world --lib -- --test-threads=1` |
| App | 211 passed; `cargo +stable test --locked -p alife_game_app --features gpu-runtime --lib -- --test-threads=1` |
| Backend memory | 14 passed normally and 14 with `CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=false`; `cargo +stable test --locked -p alife_gpu_backend --test closed_loop_memory_context --no-default-features -- --test-threads=1` |
| CPU/production compilation | Rust 1.99 strict all-target Clippy passed for Core/World/backend/app with `alife_game_app/gpu-runtime`, then `alife_game_app/production-voxel-frontend alife_game_app/gpu-tests` |
| Complete CI Clippy profile | `cargo +1.99.0 clippy --locked --workspace --all-targets -- -D warnings` passed on clean `b99ea6f2` |
| Format/static/docs | `cargo +1.99.0 fmt --all -- --check`, `bash scripts/check.sh --quick`; pass, 77/77 documentation assertions |

There were 771 passing test executions including the repeated 14-test mode
comparison, zero failures and one ignored benchmark. The 33 focused memory tests
are included in Core's total. Assertion-disabled testing exercises the constructor
branch, not release timing. Explicit GPU bodies were compiled, not executed.
The two-line Tools lint change did not rerun these earlier tests.

## Earlier integration evidence

CPU policy source `450305408dc48dec6de12e6e80be6cf60ced95ce` passed the normal
unfiltered workspace suite: 1,569 passed, zero failed, 15 ignored, 196 targets.
Default registration excluded 125 audited hardware-only names while retaining
the five initially misclassified CPU-safe tests; explicit GPU registration
retained all hardware names without running them. The meaningful headless
`benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu` passed there and
again on graphics code `fa342c8a7fc98a873fc756d9b3bd4e6429bb9de1`.
The preceding filtered 644-pass round is superseded as CPU-suite evidence.

Graphics integration also passed asset geometry/LOD/collision checks and canopy
budgets. Separate renderer cleanup `387a206d9e732289a7e52ea53a7f46dc467f7972`
passed strict Rust 1.99 production all-target Clippy. Only function-scoped Bevy
injected-parameter allowances were used; no broad suppression or assertion
removal. Reviewed memory ends at `33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e`;
its six picks matched original stable patch IDs, with the upload change once.
Timing diagnostics are excluded; the prepared-attention successor above extends
this tested base.

The repair baseline fixes populated canonical saves, approved bundle admission,
full-distance founder demonstrations and the explicit scaled N512 founder
rebind. Its original asset remains archived byte-for-byte; the retained
[provenance receipt](../../assets/founders/scaled-choice-nociceptive-v1/rebind-receipt.json)
records preserved weights, zero optimizer/GPU steps and unpromoted status.

## Failures and limits retained

- CI at `9e804d32` failed two EI0 historical-source tests because depth-one Git
  checkout lacked producer `ab806c2c`. The same fixed source/report bytes failed
  with shallow history and passed after unshallowing; all three EI0 tests then
  passed. `2bad5e02` changes CI to `fetch-depth: 0`; no validator was relaxed.
- Initial explicit GPU compilation exceeded the 14 GiB memory guard twice due
  to clean file cache. Cache advice released pages without deleting files;
  compilation passed with one job. The original lean setup followed a 32 GiB
  disk-exhausting full-link attempt; no full workspace linking was repeated.
- Isolated App Clippy without `gpu-runtime` exposed existing fixture feature
  coupling. The CPU workspace feature union passed. Early allocation-only
  all-target Clippy failed unchanged GPU-only imports; later CI cleanup passed.
- Production Clippy initially found 46 library diagnostics, then seven test
  styles; renderer cleanup fixed them. Rust 1.99 additionally found the runtime
  closure borrow (`39121395`) and two Tools borrows (`b99ea6f2`), now fixed.
- Combined explicit GPU-profile compilation found a missing test-only
  `SemanticStateVector` import; `87c19312` restores it without changing assertions.
- The supplied Windows post-wake run at `a6ef9adc` exited 101 at first-response
  equality: restored input lacked zero-confidence hints, while the selected
  action was identical. Later learning assertions were unreached. The test-only repair
  retains acquired weights, world and substantive learning checks; exact receipt
  diagnostics require `ALIFE_PERSISTENCE_EXACT_DIAGNOSTICS=1`. Two CPU regressions
  pass, but final-source hardware readiness remains **Unrun**.

GPU causal behavior, current founder competence, hardware persistence, rendered
gameplay and population scaling remain **Unknown/Unrun**. Historical supplied
CI counts retain their source; this record does not invent a current full-suite
or hardware pass. Remote CI follow-up remains coordinator-owned.

## Environment and retained logs

Debian 13 cloud worker: four-CPU quota, 16 GiB cgroup memory, no swap, 32 GiB
filesystem. Preserved stable is Rust 1.98.1; official 1.99.0 plus rustfmt/Clippy
is installed alongside. Each build explicitly sources
`/workspace/cloud-cpu-validation/env_graphics.sh` (including `env.sh`); automatic
saved-startup application remains unverified. Incremental and dev/test debug
output are disabled, build jobs one/two and tests serial. Optional production
compile uses locally extracted official Debian libudev development files through
`setup-native-udev.sh`; no ALSA dependency was required for these features.

Guards are 5 GiB free disk, 14 GiB cgroup memory and 30 minutes/command.
Base combined checks peaked at 12.42 GiB including cache; their final full-workspace
Clippy took 125.059 seconds, peaked at 9.23 GiB and left at least 12.20 GiB free.
No final guard fired. A tool-session refresh lost a handle while Cargo continued;
the existing process was monitored instead of duplicating work.

Original detailed reports remain in Git at `a11dae6a`. Local command/resource,
failure and publication receipts remain under ignored `target/artifacts/`:
`cloud-cpu-ci-policy-513e468`, `cloud-ci-history-9e804d3`,
`cloud-post-wake-persistence-2bad5e0`, `cloud-repairs-graphics-candidate-e912bf0`,
and `cloud-memory-graphics-candidate-20261001`. These paths refer to this cloud
worker. Durable promotion/training artifacts and their validators are untouched.
