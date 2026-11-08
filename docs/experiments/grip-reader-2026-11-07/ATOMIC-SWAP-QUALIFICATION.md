# Grip candidate on the atomic-swap world

## Result and source

The 2026-11-08 local fixed-fixture qualification passed: **160 functional
fixtures, zero failures, and 18 strict Clippy targets**. The one pre-existing
CPU recall microbenchmark remained intentionally ignored. The source-only
Python suites passed 39 and 38 tests; static core boundaries and rustfmt for
38 changed Rust files also passed.

The tested local merge commit is
`4b99eaf3693674b6d42735b53496b290a32f38ff`, tree
`af9120f4cb345d1528c55d9a76151cfbcd667526`. Its parents are PR #301 head
`20bcb45104e5bec688b01abefa96cdf95bdcee60` and merged main
`ebc2b0ff8969d87751e62d34cf8d26906dbbda4b`. The latter contains PR #302's
world source `30efe5ee3bb5a5cdf270288eb603b227d6c5cdfd`. Changes made after
this qualification are documentation only; the JSON binds the tested crates,
scripts and lockfile independently of those later documentation changes.

The published code anchor
[`fec6065d2d973f59450c42c356eb52ed73ce7aec`](https://github.com/BobertNeek/A-Life/commit/fec6065d2d973f59450c42c356eb52ed73ce7aec)
was fetched and verified to have the **same complete Git tree** and the same two
parents as the tested local merge. Its commit identity differs because it was
created remotely. This mapping does not rewrite the original tested identity or
claim a second execution. The final publication adds only these documentation
changes as a child of that code anchor.

[The bounded result](qualification-atomic-swap-local.json) records stage status,
per-target fixture counts and source identities. The earlier
[156-fixture hosted result](qualification-260ca86b.json) remains byte-for-byte
unchanged and still applies only to its old-world source `260ca86b`.

## Reconstructed compatibility coverage

The four added tests are a new reconstruction, **not recovered bytes of the
lost overlay**. The input v2 patch has SHA-256
`6d5bc37545e18da9cc63e3e0ec430a5d003ae8aaefb1d7629ebde655768dd53f`.
Three formatting-only hunks normalized that patch within the complete Rust
file before review and execution. The final file hash and formatter-derivative
hash are recorded in the JSON. The overlay changes no production prefix or
existing test body; it also corrects one obsolete test comment.

The 22 selected world grip tests comprise the original 18 and four additions:

- Close on B while deliberately holding A places A and acquires B, with exact
  ordered Placed/Acquired receipts. Repeating Close on B does not release it.
- Other ownership, player holding, or unavailable supported placement causes
  Close on B to fail without changing the previous constraint or object state.
- Registered Close and Release keep distinct command/target receipts; stale
  and newly observed repeated Release cannot acquire the target.
- An intentionally corrupt disposable two-hold fixture still permits explicit
  Release to detach the selected constraint without disturbing the other.
  This is an escape-boundary test, not admission of corrupt saves or perception.

PR #302 is a separate world factor: targeted pickups now replace a previous
held object atomically when placement succeeds, including supported placement
near the actor's feet. Failure preserves the original hold. Explicit Close
therefore never toggles its selected target into Release, but it can place a
different held object before acquiring the selected target. Release still
detaches its selected constraint in place and never acquires on retry. The
candidate's neural graph, identity and inherited genes were not changed here.

## Execution and limits

This was direct local Cargo execution of the 18 fixed suites declared by
`scripts/grip_qualification/qualify.py`, using `--locked --profile dev`, explicit
compile and inventory stages, and exact expected-ignore and example-test
inventories. The GitHub event/HEAD gate was neither invoked nor impersonated.
Rust 1.99.1 used one build job, no incremental compilation or debug symbols,
one test thread and a 32 MiB Rust minimum stack. Vulkan device discovery was
disabled. Build, fixture and Clippy commands ran serially in a fresh target.

The functional count is 130 core, 22 world grip, two CPU continuation, three
GPU upload/Naga and three detached reader/gradient fixtures. This is selected
contract coverage, not a full workspace test pass. The 25 PR #302 hand fixtures
are not included in these 160 selected grip-qualification fixtures.

No autonomous study, example `main`, retained/private creature input, or GPU
device execution ran. Fresh CPU continuation fixtures and detached neural tests
are unit evidence only. No learned release, sustained feeding, generalization,
GPU parity, promotion or causal advantage over historical worlds is claimed.
No closed study was reactivated. Raw logs, runtime state, checkpoints and
build products are not part of this source publication.
