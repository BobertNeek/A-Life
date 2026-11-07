# Life-free grip candidate qualification

This workflow checks the new opt-in grip interface and its compatibility contracts.
It does not reopen the closed eight-life reader study, run an autonomous assay,
execute an example's `main`, or make a GPU hardware claim. The old workflow and its
disabled native-study job remain unchanged.

The one serial job runs only for pull requests from the exact same-repository
`experiment/grip-aware-reader-20261107` branch. Checkout and an independent event,
HEAD and clean-tree check bind the work to the pull request's source SHA, not its
merge commit. The historical closed-study source commit must be an ancestor.

The fixed test inventory covers:

- New core profile, selector, wire, topology and operation-port contracts.
- Frozen N2048/N512 ABI, legacy candidate and perception-digest comparators.
- `memory_query` library fixtures, candidate memory queries and retrieval.
- World `headless::grip_research::tests::` legality, commands and joint fixtures.
- Fresh CPU founders, incompatible-interface restore rejection and full next-state
  checkpoint continuity, plus the prior memory-gain CPU fixture.
- GPU upload coordinates and WGSL validation without device execution.
- Only the `probe_grip_reader` example's `tests::` libtest harness: fresh inherited
  genes and detached neural score/gradient fixtures. Its `main` remains disabled.
- Changed-source rustfmt, static core boundaries, strict selected-target Clippy,
  this helper's source-only tests and the closed study's existing static guards.

Every fixture target is first compiled explicitly with Cargo `--no-run` and JSON
compiler messages. A separate `--list` invocation binds failure identifiers to
that target and filter. Execution requires a nonempty test inventory and a matching
libtest summary with at least one passing test and no failed tests; stale filters
cannot pass silently. Ignored tests are forbidden except the existing
`memory_cpu_recall_microbenchmark` in `alife_core/candidate_memory_retrieval`.
Its ignored identifier must appear in that harness's inventory and output, with
exact set/count agreement with the per-suite allowlist and summary. Missing,
duplicate or unexpected ignored tests fail qualification. The benchmark stays
ignored: no `--ignored` execution or ignore-attribute change is authorized.
The example's inventory must match the three reviewed fixture identifiers exactly.
All Cargo commands use `--locked --profile dev` and one cold target directory.

## Output and resource limits

Subprocess stdout/stderr, assertion diffs, checkpoints, world/neural payloads and
panic backtraces remain in private logs under `RUNNER_TEMP`. There are no artifact
uploads or downloads, runtime input arguments, cache restores, manual triggers,
matrix jobs, native-study dispatches or concurrent Cargo invocations.

Compiler output publishes at most 24 deduplicated records per command: code,
level, tracked Rust source file and valid line, and a fixed canonical message.
Compiler free text, snippets, suggestions, child diagnostics, build-script output,
dependency paths and unknown JSON are never printed. For a failed fixture, at most
12 bounded failed identifiers and 12 ignored identifiers from that harness's
inventory and the numeric summary are printed. Ignore reasons remain private.
No raw-log fallback exists, including infrastructure errors.

A cold build requires 12 GiB free. During each command a 250 ms monitor stops its
whole process group at a 3 GiB free-space watermark, leaving a 1 GiB cushion above
the 2 GiB floor. This is a monitored reserve, not a filesystem quota. Logs stop the
command at a monitored 64 MiB threshold. Space, size and deadlines are checked
again after process exit, before interpreting any output. The runner has a
24-minute total deadline inside a
26-minute workflow step and 30-minute job; no retries are automatic. Core dumps
are disabled. All temporary fixture files and the target are under `RUNNER_TEMP`.

Run only the source-safe helper tests locally:

```sh
python3 -B -m unittest discover -s scripts/grip_qualification -p 'test_*.py'
```

Do not invoke `qualify.py` on a local creature computer. It requires the verified
GitHub pull-request environment and does not accept custom commands or private
checkpoint inputs. Fixture success establishes contracts only, not learned skill,
generalization, autonomous performance, or new study evidence.
