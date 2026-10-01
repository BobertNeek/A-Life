# Cloud CI historical source validation — 2026-10-01

CI checkout now fetches complete history so the existing EI0 evidence validator
can read its locked historical producer's source. Rust implementation, tests,
report data, validation rules, Cargo features, and architecture are unchanged.

## Failure and source requirement

The coordinator reported Actions run `36859333876`, job `110359384378`, at
`9e804d325f938c7d3803b00ab915b4f8f31ffa95`: 1,154 tests passed before two EI0
integration tests failed; 14 were ignored. The App library passed all 209 tests.
Both failures were missing Git source, not neural-backend unavailability.

The validator reads nine exact source-contract paths with `git show` and checks
their bound digest and producing tree. Its historical producer is
`ab806c2c6e4910a29927cea4746f4007d75f3f15`, an ancestor of the tested branch.
`Cargo.lock` is the first path; its historical blob is
`f829a1803c19d0973a5a322df507ba0dd70f7151`. The committed historical report is
accepted only as the exact locked baseline, with existing causal and hardware
identity checks. This source audit does not execute or re-establish GPU evidence.

Checkout had no depth override. The [official checkout v4 input definition](https://raw.githubusercontent.com/actions/checkout/v4/action.yml)
sets the default depth to one and defines zero as complete branch/tag history.
The workflow now sets `fetch-depth: 0`; Development documents that prerequisite.

## Controlled history-only reproduction

Local reproduction used a separate `git clone --no-local --depth=1` of the
follow-up branch at the same `9e804d3` source. No shared object alternates were
used. Its HEAD and report bytes remained fixed while Git history changed.

The already compiled default-CPU integration executable
`ei0_exit_gate-77aa275780fba6f3` was run with `GIT_DIR` and `GIT_WORK_TREE`
pointing at that clone. This redirects the validator's Git reads without
rebuilding Cargo targets or changing the source/report fixture.

| History | Result |
| --- | --- |
| Depth one; historical source absent | Both reported tests failed with the exact `Cargo.lock` missing-at-producer error; each exited 101 |
| Same HEAD after `git fetch --unshallow` with branch/tag refspecs | Both tests passed; each exited zero |
| Complete existing EI0 integration executable after fetch | Three passed, zero failed, zero ignored, zero filtered |

The passing tests retain their rejection checks for refreshed-digest phenotype,
producer, hardware, mate, tick, actor, sequence, and self-consistent parent
rewrites. No assertion, fallback, historical reader, or test exception was added.

## Resources and remaining verification

The reproduction completed at `2026-10-01T12:38:17Z`, used no new Cargo build,
and left 16.34 GiB disk free. Cgroup memory was 7.91 GiB at completion against a
16 GiB limit; this includes cached pages from earlier work. The recorded cgroup
peak is cumulative and is not a peak for this reproduction.

Raw stage logs and `history-proof.json` are retained under
`/workspace/A-Life/target/artifacts/cloud-ci-history-9e804d3/`.
The changed workflow and docs pass the quick whitespace, static-boundary, and
documentation checks. Remote Actions verification of the published successor
remains coordinator-owned. No full workspace rebuild, physical GPU test,
desktop interaction, main merge, or training campaign was performed here.
