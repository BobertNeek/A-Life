# Cloud candidate validation — 2026-10-01

Correctness baseline: `a11dae6a5708bddf7d4255990f3bd4abaf12759a`.
This consolidates the completed cloud rounds. Main integration, GPU/Windows
gameplay, training and 20 FPS with 50 creatures remain separately owned.
The [CPU performance record](../performance/20261001-cpu-preparation.md) retains
measurement conditions and links to source-bound samples/probes; no game-speed
claim follows from the tests below.

## Combined readiness and preparation — 2026-10-02

Combined source `706eec8fb39ff89b59601b5e993ad962b278ba8f` extends readiness
checkpoint `de9c307c0de7e4cad82ab03fb5da1322091502b6`. Reviewed performance
source `4b48c4c9` and evidence `9ac88fc2` are picked once as `51ef0d8` and
`706eec8`; their common `5abafe19` prerequisites are already present.
The two branches have no overlapping files. Each source patch and the original
performance report, builder and raw receipt retain their original bytes.
Integration review is R0; the independent R2 source reviews remain applicable.

On this combined source, 15 preparation/copy/fallback/rollback/re-admission
tests, six founder/handoff tests, one existing attention test, seven world
state/embodiment tests and the tier-1/tier-10 headless smoke pass:
**30 passing executions, zero failures**. Two CPU timing tests are deliberately
ignored. Strict Rust 1.99 world/app all-target production Clippy, formatting,
full Core boundaries and 77/77 documentation assertions pass. Training-feature
compilation and admission tests pass; the pre-existing strict training-feature
Clippy warnings reported below remain outside the repair scope.

The [performance report](../performance/20261002-preparation-copy.md) measures
five fewer allocation/reallocation calls and 4,568 fewer requested bytes for
the C26/two-slot fixture row. Whole-preparation timing remains inconclusive.
No new speed, 50-creature FPS, GPU or desktop claim follows from integration.
Source hashes, clean-composition checks and guarded command/resource receipts
are retained under ignored
`target/artifacts/cloud-readiness-preparation-combined-20261002/`.

## Creature sensing and training handoff — 2026-10-02

Source `e24c5b38cdc7877128c6eedb7c3a6891c08e9bff` extends green combined
`5abafe1961efaf099cd07287bd7ca09e8ea754cd`. Terrain depth rays now apply the
organism's existing Vision gain once, matching the other calibrated modalities.
The ordinary and indexed perception paths share this operation. It changes
bounded sensory measurement, without choosing an action or adding CPU cognition.

Training cycle admission now binds actor and value optimizer counters to the
published handoff and requires the recorded successor decision. A changed value
counter with retained Adam moments changes WGSL bias correction and the next
coefficient; an actor counter at `u32::MAX` cannot prepare the next replay.
Actor bias correction still uses per-weight update ages. These checks run once
when admitting a previous training cycle, not per organism tick, and do not
gate ordinary creature saves or foundation loading. Matching handoffs retain
all weights, moments, ages, masks and value parameters.

Both defects were reproduced with failing CPU regressions before repair.
Seven world state/embodiment tests and six training founder/handoff tests pass.
The valid training fixture uses a compiled current N2048 founder, its correct
seed, real recurrent mask and a 2048-feature value head; checkpoint JSON
roundtrips preserve learned data and enter the normal value restore decoder.
A test-only scalar diagnostic of the existing value shader demonstrates the
counter's numerical consequence. It is not CPU neural training or executed GPU
resume evidence. Two independent blank-context Sol6.1 reviews accepted the final
bounded patches. The review corrected the terrain reference calibration and
training fixture founder seed before final validation.

The tier-1/tier-10 headless benchmark smoke passes: **14 final passing test
executions, zero failures**. Strict Rust 1.99 world/app all-target production
Clippy, formatting, full Core boundaries and 77/77 documentation assertions
pass. Across this round the observed cgroup memory stays below 11.8 GiB and
disk free above 15.4 GiB; no resource guard fires. Full workspace linking/testing
is not repeated in this 32 GiB environment.

Strict Rust 1.99 all-target `foundation-training` Clippy fails on 14 existing
warnings in unchanged `foundation_training.rs` and `gpu_live_runtime.rs`.
No warning suppression or unrelated repair is included. Actual training-feature
compilation and the six selected CPU tests pass. Raw command logs, source hashes,
patch snapshots and resource/review receipts are retained under ignored
`target/artifacts/cloud-creature-readiness-20261002/`.

The environment has stable Rust 1.98.1 and Rust 1.99 with rustfmt/Clippy, four
CPU quota cores and 16 GiB memory. Fresh shells do not load Rust paths or lean
Cargo settings automatically: source
`/workspace/cloud-cpu-validation/env_graphics.sh`, then set `CARGO_BUILD_JOBS=1`.
That recipe disables incremental/dev/test debug output, selects the shared
CPU target, uses serial tests and supplies the local official native dependency
path. Every Cargo command here used it with disk/memory guards.

GPU behavior, GPU training/resume and desktop work remain Unrun. Founder archive
reconstruction and per-organism gustatory evidence remain separate follow-ups;
neither is declared fixed. Performance-worker and CI-comment remediation source
are outside this patch.

## Terrain hint successor — 2026-10-02

Combined source `df8520a2f62102184dfd7a8cf072ae5aff154fc5` extends green
`f73564ba` with reviewed Terrain source `5a8000a8` and evidence `acbf586`,
picked cleanly as `4ba552ef` and `df8520a2`. Existing finalization and production
runner prerequisites remain present once. All crates, assets, manifests and
workflows match reviewed `acbf586` byte-for-byte. The original
[Terrain report](../performance/20261001-terrain-hint-workers.md), raw receipt
and updated worker-report note are unchanged. Integration review is R0;
the unchanged source retains its documented independent R2 clearance.

Debug and true release each pass 13 preparation/hint/failure/re-admission tests,
one existing attention test and two persistence-response regressions. The
tier-1/tier-10 headless smoke also passes: **33 passing executions, zero failures**.
The dated timing benchmark is ignored in each profile; no new timing claim is
made. Fresh debug/release executables, compiler profiles, source hashes and all
command/resource receipts are retained under ignored
`target/artifacts/cloud-terrain-workers-combined-candidate-20261002/`.

Strict Rust 1.99 workspace, production/GPU-test and release app all-target
Clippy pass with `-D warnings`. Normal app and `foundation-training` library
compilation pass; no training executes. Formatting, full Core dependency/source
boundaries, 77/77 documentation assertions and local links pass. One build job,
serial tests and disabled incremental/dev/test debug output were used. No guard
fired; observed memory stays below 11.27 GiB and disk above 15.69 GiB. Core and
backend source remain identical to the prior validated candidate; their earlier
focused test receipts retain that source and are not counted as new executions.

Terrain hints now use the authorized per-owner serial capture boundary before
bounded CPU computation. Late replies wait for later preparation; outputs match
for identical captures, while old interleaved wall-clock reply schedules can
produce different captures. This documented tradeoff is the only intended
semantic change. Two participants remain bounded to one child plus the caller;
small cohorts, one CPU and spawn failure still compute serially. Prior/provider/
language identity bytes, strict re-admission, learning, persistence and graphics
are preserved. Host rollback retains its existing limits for prior request/cache
effects; this is not exact provider-state rollback or live GPU recovery proof.
No source conflicts or local verification blockers remain. Real provider/cache,
GPU/training/PC execution and 50-owner trained Terrain gameplay remain Unrun;
the report retains the missing founder/save/config/manifest hardware inputs.

## Production two-worker successor

Combined source `c9d324a51b01a48d533fe875c0fe9110a7224144` extends serial
checkpoint `0a7fa449` with reviewed worker source `f03d2dc3` and evidence
`e540e820`, picked cleanly as `b24417ca` and `c9d324a5`. Finalization is already
present as `03003006`; prerequisite `1c53224a` was not duplicated. All crates,
assets, manifests and workflow bytes match reviewed `f03d2dc3`. The original
[worker report](../performance/20261001-two-worker-production.md) and its
314-line raw receipt match `e540e820` exactly. The separate paired serial
comparison below retains its original sources, inputs and scope.

Worker/failure tests, metadata re-admission and existing app attention each
pass in debug and true release: 11 executions per profile. Core query/retrieval,
three-factor learning and perception-digest suites pass 54 checks; backend
buffer/memory contracts pass 43. The explicit ignored release runner smoke
also passes: **120 passing executions, zero failures**, with three intentionally
ignored executions in the normal selections. The actual CPU runner completes
72 timed batches and 1,908 complete typed owner comparisons, including exact
upload metadata. A separate initial pass checks once-only row visits. No device
is created. Its new raw
samples are retained separately; the original timing table is unchanged.

Strict Rust 1.99 workspace, production/GPU-test all-target and release app
all-target Clippy pass with `-D warnings`. Normal app and `foundation-training`
library compilation pass; no training runs. Format, full Core dependency/source
boundaries, 77/77 documentation assertions and local links pass. Debug workspace
package output was cleaned to prevent stale shared-target reuse; release uses
the isolated target, with a fresh executable/profile/source-hash receipt.
One build job and serial tests were used. No guard fired; observed memory stays
below 11 GiB and disk above 10.16 GiB, including the pre-cleanup measurement.

The eligible suffix uses one child plus the caller; fewer than eight dispatchable
rows, one CPU, active terrain-prior polling and thread-creation failure retain
their documented serial paths. Ordered publication, full metadata re-admission,
panic joining and controlled restoration remain in the reviewed source. Tests
do not establish live GPU recovery, nonempty resident restoration, provider
lifecycle or whole admitted tick performance. Main integration and successor
remote CI remain coordinator-owned; GPU/Windows gameplay and FPS remain Unknown.

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

## 2026-10-02 main reconciliation candidate

`codex/cloud-main-readiness-preparation-candidate-20261002` merges main
`9f5df3ed2fe9cd743fcf666d9909e63a68cc4950` into combined readiness/copy checkpoint
`13585fcef6e3bc9a99420824bb16808db1defe28`, preserving both histories. The latter
passed complete workspace CI run `37038307449` and fast run `37038307436`.
This candidate does not merge or push main.

Main run `37037644535` failed two retired N512 identity snapshots and Clippy's
constant `chunks_exact(4)` diagnostic. The combined checkpoint already contains
the reviewed current-state receptor/innate-priority assertions and equivalent
`as_chunks` cleanup; no founder bytes or substantive validator are changed to
satisfy those failures.

The integration retains main's explicit external-actor cohort seal: registered
biology and external actors are disjoint and together exactly cover Agent
objects. Direct restore validates creature summaries before constructing the
world. The combined habitat-membership guard also remains. Complete historical
registries without the optional seal restore, and durable comparison normalizes
only an owned clone. Their serialized save anchors remain unchanged through
journal publication. Older ambiguous mixed creature/teacher saves without actor
provenance still require explicit migration (AOA-PERSIST-001/002/003/004).

Both histories preserve family bias and every innate drive/cue field in research
growth. The combined four-file constructor/encoder implementation remains intact
because it additionally authenticates source compiler inputs and reconstructs
the exact remapped sensory plan. Main's measured innate-contribution and
mismatched-input regressions are added alongside the combined source/inputs
immutability, numeric-bit, dynamics, replay and address assertions. N4096 remains
research-only and unpromoted (AOA-ID-004, AOA-GROW-008).

Terrain calibration, optimizer-age/policy-version handoff, and preparation-copy
source/test files are byte-identical to `13585fc`. The newer island tree/water
pack is retained coherently with its builder and manifest. Independent read-only
inspection matched all 16 island/hand asset digests and sizes; retained overview
tree counts are 88/172/56. This is source/asset consistency, not rendered or GPU
performance evidence.

Two separate Sol6.1 read-only reviewers provided R2 review. Review caught and
closed two automatic-merge fixture mismatches: the hazard relocation conflicted
with main's two pain-free transit intervals, and a positive social affinity
conflicted with main's negative-affinity assertion. The corrected fixtures retain
the stronger contact/transit and positive/negative affiliation coverage. Neither
reviewer found an open substantive blocker.

At publication, 23 focused Core, 91 World and 10 durable-checkpoint tests passed,
plus formatting, static boundaries and all 77 documentation assertions. Further
App CPU/readiness/copy, checkpoint compatibility, replay codec, founder admission,
headless benchmark, strict affected-crate/production-feature Clippy, and final
exact-commit remote CI are recorded in the cloud worker's
`target/artifacts/cloud-main-readiness-preparation-candidate-20261002/` receipts;
pending publication-time checks are not claimed as passes here. Local commands
disable incremental/dev/test debug output, use one build job and serial tests,
and stop below 5 GiB free disk or above 14 GiB cgroup memory. No full workspace
test/link is repeated locally.

The rebound founder retains all 1,799 historical weight bits, archived original
bytes and explicit rebind provenance. It remains bootstrap, unpromoted and
behaviorally Unknown; canonical admission does not establish competence. No GPU
dispatch, training, desktop interaction or real GPU performance claim is made.

The cloud environment remains ready after explicitly sourcing
`/workspace/cloud-cpu-validation/env_graphics.sh`. A fresh shell has neither Rust
on PATH nor the required Rust/Cargo lean-build settings. Boot execution of the
saved setup draft remains unverified. Exact settings and the validated official
dependency/setup recipe are retained in `validated-environment-recipe.json` in
the same receipt directory for the coordinator to save in this environment.
