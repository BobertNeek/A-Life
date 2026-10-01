# Live perception preparation allocation receipt — 2026-10-01

The bounded change removes temporary offset storage and repeated packed-header
growth from the live CPU preparation path. It preserves exact upload validation,
all emitted payload words, and atomic overflow rejection. It does not establish
a live Windows FPS or TPS improvement.

Subsequent extraction of the existing Windows receipt identifies memory/cognitive
preparation as dominant. See the [memory-validation receipt](20261001-memory-frame-validation.md)
for that bounded follow-up; this allocation receipt retains its original scope.

## Source and ownership

Branch: `codex/cloud-live-preparation-20261001`, based on
`ad99ac1c31c53d8898703268446a2f785586fe4e` (tree
`55ee0779e37afaaeea70fbecdd8439056c879181`). Implementation commit:
`352ff546334b745bc773d213554a6c31e189b024` (tree
`0f66125bd6ada5ae0c3695ac8536f5f3fa132a8c`).

The source file `crates/alife_gpu_backend/src/closed_loop_buffers/perception.rs`
has the same baseline blob, `b551a61c18ab006f7fef49192eaa89e861c6d637`, on
main `e76e9e12`, CI follow-up `ad99ac1c`, and integration candidate `f8ccfb13`.
The existing shared training phenotype/snapshot repair remains intact. The
reviewed candidate branches do not repair this perception preparation seam.

Ownership is limited to that source file, its perception contract tests, and
this receipt with its CPU evidence. Graphics, training budgets, N512 credit
fixtures, sandbox save summaries, bundle/shader discovery, and operational
launch files were not changed. No merge, PR, local task, hardware test, or GPU
campaign was created or run.

## Reasoning and preserved contracts

`prepare_memory_context_upload` constructs one perception upload for its frame
binding. `GpuActiveBatchUpload::try_from_views` constructs another upload, rebases
it, and calls `validate_against`, which constructs and rebases an exact expected
upload. Every construction now reserves the complete packed-header capacity.
Every rebase validates all checked additions before changing any field, then
updates feature offsets in place. The temporary offset vector is unnecessary.

The rejection order stays: offset domain, candidate header offset, sensory
offset, then every candidate feature offset. The new regression checks complete
upload equality after each overflow category, including a later candidate that
overflows after the first candidate fits. Successful rebasing to `u32::MAX`,
exact validation, and repeated-rebase rejection remain covered.

No neural execution, learning, cognition, physiology, population, persistence,
tick cadence, or simulation work was reduced. The source change is consistent
with `AOA-GOAL-002`, and retains the authority, bounded-work, host-policy,
shared-step, and explicit-failure boundaries in `AOA-INV-003`, `005`, `006`,
`009`, and `010`. These CPU checks do not certify full organism capabilities.

## CPU measurement

The [one-off probe](evidence/20261001-live-perception-probe.rs) is evidence code;
it adds no runtime instrumentation or profiling framework. It repeats the
construction/binding, construction/rebase, and exact validation operations for
eight synthetic upload rows using one N512 slot. It does not create eight live
residents, drive an authoritative tick, or construct a GPU device.

Host: AMD EPYC 9V74 under KVM, Linux 6.18.44, five exposed CPUs. Executables were
pinned to CPU 0. Rust: `1.98.1 (48a229cea 2026-09-01)`. Workspace test rlibs used
the existing `opt-level = 2` overrides for `alife_core` and `alife_gpu_backend`,
with debug symbols disabled by `/workspace/cloud-cpu-validation/env.sh`. The
probe was linked with `rustc --edition=2021 -O`.

Three alternating baseline/patched process pairs each warmed 200 cohorts, then
collected nine samples of 1,000 cohorts. Each variant has 27 samples per candidate
count. Allocation/reallocation calls were counted separately over 100 cohorts;
counter increments were disabled during timing. The allocator still checks its
enabled flag, identically in both binaries.

| Candidates per row | Allocation calls per cohort, before → after | Reallocation calls, before → after | Median CPU µs, before → after | Observed median reduction |
| --- | --- | --- | --- | --- |
| 2 | 160 → 144 | 384 → 360 | 74.328 → 73.029 | 1.7% |
| 8 | 160 → 144 | 472 → 384 | 123.221 → 117.673 | 4.5% |
| 32 | 160 → 144 | 576 → 408 | 297.459 → 276.814 | 6.9% |

All before/after packed-header and frame-payload BLAKE3 digests match for every
case. [Raw samples and source identities](evidence/20261001-live-perception-preparation.json)
also retain timing ranges and summed bytes requested through allocation and
reallocation. Those bytes are allocation traffic, not peak or retained memory.
The sample ranges overlap; these are observed microbenchmark medians, not a
statistical guarantee or a live performance budget pass.

To reproduce, build the baseline and patched revisions in isolated worktrees
using the lean environment, and link a copy of this same probe against each
revision's test artifacts before moving to the next revision:

```bash
source /workspace/cloud-cpu-validation/env.sh
cargo test -p alife_gpu_backend --test closed_loop_buffer_contracts
```

The probe may be copied into either worktree's `docs/performance/evidence`
directory; its relative fixture import must resolve to that revision. These
commands require a target directory containing only that feature configuration
and deliberately reject ambiguous rlibs:

```bash
python3 - <<'PY'
import glob, os, subprocess
deps = os.environ['CARGO_TARGET_DIR'] + '/debug/deps'
args = ['rustc', '--edition=2021', '-O',
        'docs/performance/evidence/20261001-live-perception-probe.rs',
        '-L', 'dependency=' + deps, '-o', '/tmp/perception-probe']
for name in ['alife_core', 'alife_gpu_backend', 'blake3']:
    libs = glob.glob(f'{deps}/lib{name}-*.rlib')
    assert len(libs) == 1, libs
    args += ['--extern', f'{name}={libs[0]}']
subprocess.run(args, check=True)
PY
taskset -c 0 /tmp/perception-probe
```

Probe SHA-256:
`3086657367566380f8c8912d566309c7775c1d3bbbbf9e9e0897a5400b67a585`.
Baseline executable SHA-256:
`35b49ab2f136ae6a4779182c89a69ed517e8cdb09d05259d39b8cd242bcacc14`.
Patched executable SHA-256:
`7069548dfad5541df0327fb7794d5ddca044cbbd99360467b1c362895cd780f4`.

## Validation and remaining evidence

- `cargo test -p alife_gpu_backend --test closed_loop_buffer_contracts`: 29 passed.
- `cargo test -p alife_gpu_backend --no-default-features`: 159 CPU tests passed.
- `cargo clippy -p alife_gpu_backend --lib --test closed_loop_buffer_contracts --features training-rollout -- -D warnings`: passed.
- `cargo fmt --check -p alife_gpu_backend`: passed.
- `bash scripts/docs_check.sh`: 77/77 assertions passed (Linux invokes the
  script behind the Windows PowerShell wrapper).
- `git diff --check`: passed.
- Independent Sol 6.1 review (R2): no blocking source or evidence findings.

The broader `cargo clippy -p alife_gpu_backend --all-targets --features
training-rollout -- -D warnings` invocation fails on pre-existing GPU-test-only
imports in unchanged `tests/closed_loop_activity.rs`: `Tick`,
`BRAIN_ATP_BASAL_DEBIT_Q16`, `BRAIN_ATP_Q16_MAX`, and `GpuActivityRestoreInput`
are unused when `gpu-tests` is disabled. The focused strict check covers the
changed library and test target without expanding this patch into fixture work.

Published main [island verification](https://github.com/BobertNeek/A-Life/blob/e76e9e123e7dac767a360e9703f365cb1b07d7f4/crates/alife_game_app/assets/landscape/island/README.md)
reports 330 frames in 60.18 seconds with eight creatures on an RTX 3050,
95.7% simulation tick wall time, and 6.82 ms renderer/present/uninstrumented
residual. Its raw JSON receipt was absent in this cloud checkout. Tick wall time
includes GPU completion waits, so it does not establish CPU preparation as the
dominant cause. This patch saves only about 1.3–20.6 µs per synthetic cohort here;
it cannot be presented as resolving the roughly 180 ms Windows frame time.

Fresh source-bound RTX end-to-end proof remains unrun. Keep the already batched
GPU readback and world rollback atomicity. Broader duplicate serialization and
mapped-word copy opportunities need their own validation-preserving design and
evidence. Rollback for this slice is reverting implementation commit `352ff546`.
