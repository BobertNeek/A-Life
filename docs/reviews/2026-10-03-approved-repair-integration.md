# Approved repair integration — 2026-10-03

Base: main `d2e4a157ec8b14fc21445d55d560ab36c4551d49`.
Four-tip source candidate: `093bbc3cdb4d52be9109b26523b02fd2ed1492e5`,
tree `20e351db92640238077b2596a7daed503b1f1a85`.
This report is a subsequent documentation-only commit. Cassidy authorized
integration, normal nonforce main publication, and CI verification at 04:12 UTC
(`Sentinel_c66523b6e8cc8191822abb779e902eaf`).

## Preserved repairs

Each approved tip is an ancestor of the candidate through a separate `--no-ff`
merge. There were no conflicts or implementation edits. Every source and report
blob changed by each tip matches the combined candidate exactly, including the
taste ancestor shared by the current-age branch.

| Repair | Approved tip | Full / fast branch CI |
| --- | --- | --- |
| Taste continuity | `c8c78d28bedcb5910f9f1a1ee30a2601dc6ab180` | [37070399825](https://github.com/BobertNeek/A-Life/actions/runs/37070399825) / [37070399828](https://github.com/BobertNeek/A-Life/actions/runs/37070399828) |
| Current-age sensitive periods | `3ee6b0351b4c0b2949c93d5e61dd430a69b0078c` | [37077458580](https://github.com/BobertNeek/A-Life/actions/runs/37077458580) / [37077458686](https://github.com/BobertNeek/A-Life/actions/runs/37077458686) |
| Topology lifecycle | `994fceac4c6777b4390dc17ed1e4a7354b76930a` | [37093796718](https://github.com/BobertNeek/A-Life/actions/runs/37093796718) / [37093796627](https://github.com/BobertNeek/A-Life/actions/runs/37093796627) |
| Player hand interaction | `37f2d2b94a5c9370e799415478e906514474322e` | [37095339399](https://github.com/BobertNeek/A-Life/actions/runs/37095339399) / [37095339366](https://github.com/BobertNeek/A-Life/actions/runs/37095339366) |

All eight branch runs were verified successful at their exact tip. Their results
do not certify the combined candidate. Publication requires full and fast CI on
the final candidate and again on main, preserving any intervening main work.

Requirement tracing remains with the individual source-bound reports:
[taste](2026-10-02-biology-continuity-audit.md),
[current age](2026-10-02-sensitive-period-prototype.md),
[topology](../performance/20261003-topology-lifecycle.md), and
[hand](2026-10-03-player-hand-repair.md). Relevant contracts include
`AOA-SENSE-001`, `AOA-DEV-002/003/006`, `AOA-PERSIST-001/002/004`,
`AOA-CON-001/003/005`, and `AOA-PERF-003/005/006`.

## Combined CPU verification

The final four-tip source candidate passed:

| Focused check | Result |
| --- | --- |
| Production frontend library build, current-age audit filter | 3 passed |
| Production frontend hand helper filter | 3 passed |
| World player-hand legality, movement, persistence and release | 3 passed |
| Taste continuity integration | 9 passed |
| Save roundtrip and organism registry persistence | 24 + 12 passed |
| Core topology reference equivalence, rollback and provenance | 5 passed; manual timing probe intentionally ignored |
| Backend waking/sleep development metadata unit tests | 2 passed |
| Population 1/10 headless benchmark smoke | 1 passed |
| Formatting, core boundaries, whitespace | Passed |
| Documentation assertions | 77/77 passed |

The preceding three-tip candidate `d27e6ffca697615c1fe5ba78138d51d3b56ea0fe`
also passed three developmental transport/WGSL tests. Their backend source is
unchanged in the final candidate. These tests cover N512/N1024/N2048 rate
transport, restore identity, and validated waking/sleep shader entrypoint call
graphs; they do not dispatch GPU learning. Final taste/save/registry tests were
rerun after adding the hand merge.

One independent R2 review using SOL6.1 approved source candidate `093bbc3` with
no blocking or nonblocking findings. It verified preserved ancestry/blobs,
mutable age metadata versus immutable phenotype/learned banks, post-seal memory
then topology observation on learning rejection, fresh topology diagnostics,
and independent world ownership of taste and player holds.

Receipts, exact commands, source SHA-256s, resource observations, and logs are
under `target/artifacts/cloud-repair-integration-20261003/` in the cloud
workspace. `focused-integration-review.json` records the reviewed tree;
`approved-tip-ci.json` records the eight exact branch runs. Candidate/main CI
completion receipts are captured there after publication.

## Resource bounds and incomplete qualification

Cloud: Rust stable 1.98.1 with rustfmt and Clippy, four CPU quota cores,
16 GiB memory, no swap, and a 32 GiB filesystem. Commands manually source
`/workspace/cloud-cpu-validation/env_graphics.sh`; no startup settings were
persisted. All Cargo commands use one job, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_PROFILE_TEST_DEBUG=0`.

The first frontend compile was stopped at the unchanged 14 GiB cgroup guard.
Its retained file cache accounted for about 12 GiB after the compiler stopped.
Advising the cache of generated target files with `POSIX_FADV_DONTNEED` reduced
usage without deleting artifacts or modifying source. The retry passed within
the same limits. Successful final-candidate checks peaked at approximately
10.03 GiB cgroup usage and retained at least 12.78 GiB free disk. No full local
workspace build/test, GPU dispatch, native renderer session, desktop interaction,
or training was performed.

The approved hand appearance patch has been handed to the PC owner for a
Blender 5.2 CPU background export. No qualified production visual tip was
supplied for this candidate. The paired source/export/specification/manifest,
normalized handedness, baked poses, exported contacts, and native five-pose
review remain required. Preview approval alone does not qualify those artifacts.

The forthcoming PredictionResidual repair must provide a next-update once-only
pulse with exact once behavior across save/resume; its owner's preliminary six
CPU tests are not merged or certified by this candidate. Fixed instinct weights
are intended to lock after training while other weights remain plastic. Social
chemistry is intended to reflect relationship, care and meaningful interaction,
with contact potentially contributing. Those mask/social semantics are being
traced separately and are not repaired here. GPU weight effects, current-age
device-bank continuity, GPU performance, final hand appearance and overall
training readiness remain unqualified by these CPU checks.
