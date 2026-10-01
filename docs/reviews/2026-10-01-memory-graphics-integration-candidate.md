# Memory, persistence, and graphics candidate — 2026-10-01

`codex/cloud-repairs-graphics-candidate-20261001` now combines the reviewed
memory/upload changes with the previous repair, persistence, and island graphics
candidate. This is a tested correctness baseline for the playable game, not a
claim that 50 creatures achieve 20 FPS. Main integration, hardware gameplay, and
population-scaling work remain separately owned.

The narrow Rust 1.99 CI fix was published first at
`3912139597a3bb694b3c12b370a31cecc30c873a`, so its CI could run before the longer
combination. The reviewed [renderer/UI cleanup](2026-10-01-production-renderer-lints.md)
was published and remote-verified separately at
`387a206d9e732289a7e52ea53a7f46dc467f7972`, then incorporated by a fast-forward
of the candidate branch. Neither operation touched main.

## Reviewed memory source

All six commits from `cloud/memory-quality-20261001`, ending at
`33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e`, applied cleanly. Zero-context binary
stable patch IDs match the reviewed originals. The independent upload change was
absent from the candidate beforehand and is included exactly once.

| Reviewed commit | Candidate pick | Purpose |
| --- | --- | --- |
| `cba0740bb85c7c9cbc9c3ea923aa02ee4d8be3d9` | `31f774ea8b4d6b4bc71e371b2600990a86dcd486` | Shared recall, meaningful-event admission, individual exceptions, perceptual transfer and retention |
| `a21c21d7cce5da42abfeadc03ddcca1af64a81c3` | `148698963eac2d8a59550360715f49d79da57ac7` | Fresh upload self-comparison remains diagnostic in debug builds |
| `b6fa20fef7a94466ac2a51fc1c43248b33bfc75f` | `f3d11508db64548a991181e7b1f87612ad5c48ff` | Reviewed upload CPU evidence |
| `d0da0b58b78e17ff0487a96b3842e278c3397308` | `65509890933612660bb40a4134136ab54782ff39` | Memory test style cleanup |
| `c29b4812d01f118feecb33cd4529d2bd5cde728a` | `dbde6dc7eb43603ed4d0164098002ef77fe0b22f` | Reviewed recall/outcome CPU evidence |
| `33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e` | `4e973284d9b79d019b5f3693e7b3ab1a91612c5b` | Cross-creature and cross-call recall isolation regression |

Combined explicit GPU-profile compilation found one feature-only integration
defect in the reviewed upload test fixture: `SemanticStateVector` had been
removed from the GPU-only import group. The test-only correction is
`87c19312a34bb602198ddcda40c3c8690030138f`. It restores that import and changes
neither production behavior nor assertions. All code checks below bind this
revision; the subsequent report commit changes documentation only.

The diagnostics timing branch and the separately owned prepared-recall attention
salience performance slice are excluded. There are no current actor, founder
weight, save, training-output, or generated-build files in this integration.
The included performance JSON/probe files are the reviewed synthetic CPU evidence.

## Interaction review

The memory picks change no app or World source. World-owned carry/release and
terrain, persistence comparison policy, acquired neural banks, GPU decoder/layout,
and action authority remain as reviewed in the preceding candidate. Query reuse
is per creature and per immutable recall call; actual feature bits and organism
identity remain in the reuse boundaries. Perceptual-category lookup retains
organism/profile fences, and family lookup retains action-family identity.

Category indexes are derived state rebuilt on portable restore, rather than new
durable memory fields. The new ignored-event update kinds describe transient
update receipts, not serialized memory bank or save layout changes. Admission,
contradiction, fading/retention, bank compaction, and restore checks remain in the
Core owner. Retrieval supplies evidence for neural arbitration, not a host action
choice. The upload constructor retains validated input, binding, offset/encoding,
width, and finite-value checks; its strict public validator remains available.

Scope follows AOA-MEM-001/002/003/008, AOA-INV-003/006,
AOA-PERSIST-001/002/004, and AOA-PERF-009. This is scoped implementation evidence,
not full architecture certification or a live acquired-learning acceptance run.

## Combined validation

CPU test execution uses the preserved stable Rust 1.98.1. Strict compilation and
lint checks explicitly use official Rust 1.99.0, installed beside the preserved
toolchain to match the current CI compiler. The test source is the same in both.

| Check | Result |
| --- | --- |
| Core, `cargo +stable test --locked -p alife_core --all-targets -- --test-threads=1` | **465 passed, 0 failed**, one developer microbenchmark ignored; 57 result binaries |
| Focused memory coverage within that Core run | **33 passed**, including the new organism/call isolation regression |
| World library, `cargo +stable test --locked -p alife_world --lib -- --test-threads=1` | **67 passed**, including terrain and carry/save tests |
| App library, `cargo +stable test --locked -p alife_game_app --features gpu-runtime --lib -- --test-threads=1` | **211 passed**, including persistence comparisons, asset/launch/save and acquired-state fixtures |
| Backend `closed_loop_memory_context`, no default features, ordinary test profile | **14 passed** |
| Same backend target with `CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=false` | **14 passed**, including malformed foreign bindings, projection width/finiteness and tampered payload rejection |
| Core/World/backend/app all-target strict Clippy, `gpu-runtime` CPU workspace feature union, Rust 1.99 | **Pass**, `-D warnings` |
| Same packages, `production-voxel-frontend gpu-tests`, all-target strict Clippy, Rust 1.99 | **Pass**, `-D warnings`; full production and explicit hardware test compilation |
| Rust 1.99 format, quick static boundaries, whitespace and documentation checks | **Pass**, all 77 documentation assertions |

That is 771 passing test executions, including the repeated 14-test assertion
mode comparison. The assertion-disabled run is an ordinary test-profile check of
the constructor's release branch; it is not a release-performance measurement.
All other normal test runs keep their ordinary assertions. No full-workspace
build/link/test campaign was repeated. The prior headless benchmark smoke and
island geometry/canopy receipts remain separate evidence; they were not rerun
or used as new memory/FPS measurements here.

The explicit GPU profile compiles its test bodies without executing them. GPU
neural behavior, Windows rendering/gameplay, post-wake hardware persistence,
training, whole-game FPS and the 20 FPS/50-creature goal remain **Unrun/Unknown**.
No desktop, graphics window, GPU campaign, or new performance scout ran here.
The source owners' CPU timing reports are retained with their stated limits;
their gains are not summed into a game-frame speedup.

## Worker and publication evidence

The selected cloud worker recovered with the saved checkout and cache preserved.
A later tool-session refresh lost a command handle, but its Cargo process and
resource receipt writer continued. The existing process was monitored instead
of starting duplicate work. Lean env settings were sourced for every build;
incremental and dev/test debug output remain disabled. Build jobs were one for
profile compilation and one/two for the focused CPU checks; tests were serial.

The guards stop at less than 5 GiB free disk, more than 14 GiB total cgroup memory,
or 30 minutes per command; none fired. Observed combined-check peak was about
12.42 GiB cgroup memory including cache, with at least 12.32 GiB disk free.
Clean generated build pages were advised out of cache once; no source, saved
data, cache file, or build artifact was deleted.

Commands, logs, resource receipts, source/patch mappings and final remote
verification are local under
`/workspace/A-Life/target/artifacts/cloud-memory-graphics-candidate-20261001/`.
The narrow fix and separate renderer receipts remain under
`/workspace/A-Life/target/artifacts/cloud-repairs-graphics-candidate-e912bf0/`.
The final candidate is published only on its existing separate branch; original
reviewed branches and the frozen repair branch are preserved. No main merge or
PR is performed; the coordinator owns remote CI follow-up and subsequent review.
