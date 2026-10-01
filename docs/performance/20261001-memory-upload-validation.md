# Fresh memory upload self-validation CPU receipt — 2026-10-01

Release memory upload construction now validates its inputs and builds the
encoded upload once. Its immediate exact self-comparison remains enabled in
debug builds. Synthetic CPU comparisons show 47–53% lower constructor cost;
live Windows/RTX frame-time improvement is **Unknown**.

## Source and measured seam

Branch: `codex/cloud-live-upload-validation-20261001`, based on consolidated
`2bad5e02b6bbf125a10822d3c7effd7debc96d8f`. Implementation:
`ffeea2528d0475541ff1078a3510762538d71c5e`, tree
`6e8973ef90700a3f8f87b849ffc77771d689503b`.

The supplied Windows receipts identified CPU preparation as the dominant live
cost. The earlier [memory frame-validation optimization](20261001-memory-frame-validation.md)
is already present in this baseline. This slice measures an additional backend
seam rather than repeating that repair or snapshot sharing work.

`GpuClosedLoopBackend::prepare_memory_context_upload` validates the backend,
resident handle, frame, organism ownership, and profile before constructing a
perception binding and calling `GpuMemoryContextUpload::try_from_finalized`.
The constructor previously performed:

1. Full `FinalizedMemoryRecall::validate_for_frame`.
2. `build_local` with binding, row, projection, and encoding checks.
3. `validate_against`, which repeats full recall validation, rebuilds the same
   upload from the same immutable inputs, and compares it to the fresh upload.

The three-line production diff gates step 3 with `cfg(debug_assertions)`.
Both builds keep steps 1 and 2. `validate_against` remains public and strict;
batch admission, `validate_for_frame_and_slot`, offset-domain checks, rebasing,
GPU/WGSL validation, and neural receptor binding are unchanged. Fresh inputs
are ordinary owned values behind immutable references; the builder uses no
clock, randomness, I/O, or mutation between the two constructions.

The retained checks reject foreign frame/recall identity, organism/profile/tick
mismatches, malformed candidate evidence, invalid counts and offsets, incorrect
slot/generation/digests, projection bounds, nonfinite encoded projection values,
and unequal episodic context encoding. The constructor still rejects errors
before returning an upload. No validation state, cache, new dependency, public
contract, GPU layout, or instrumentation is added. Removing the duplicate build
also removes its temporary row/context reconstruction; allocation counts were
not separately measured.

## CPU fixture, measurements, and output proof

The [one-off probe](evidence/20261001-memory-upload-probe.rs) creates valid
synthetic GroundedObjectSlots drafts with 2, 8, or 26 candidates and banks
containing 0 or 64 records admitted through existing sealed-event test helpers.
Two tracked targets and four action families repeat. Cognitive memory
expectancies come from the actual fixture recall. Projection-present cases add
bounded, valid synthetic predictor inputs with thirteen successor lanes.

The existing checked-in `scaled-choice-nociceptive-v1` asset is compiled through
its current explicit Nano512 cognitive extension for this CPU fixture. Its
slot has decoder stride 54, so projection-present cases exercise actual encoded
cognitive rows rather than the 36-lane zero-row path. This changes no live brain,
foundation file, admission policy, or experimental fixture.

Each cohort repeats eight calls on one immutable frame, recall, and slot. It
represents eight constructor operations, **not eight live creatures**. Timing
includes construction, allocations, validation, and destruction. No GPU device,
app tick, topology operation, filesystem persistence, or render work runs.
Public upload validation and finalized-recall validation are measured separately
as unchanged controls; these timings overlap semantically and must not be summed
as independent stages.

Host: AMD EPYC 9V74/KVM, Linux 6.18.44, five exposed CPUs; processes are pinned
to CPU 0. Rust is `1.98.1 (48a229cea 2026-09-01)`. Both library variants use the
true release profile, opt-level 3 and debug assertions disabled. The standalone
probe also uses opt-level 3 with debug assertions disabled and asserts that
configuration. Matching rlibs are selected from Cargo compiler-artifact JSON.
The evidence records their profiles, features, hashes, and executable hashes;
core and BLAKE3 rlibs are byte-identical between variants. Library hashes are
captured at link time; Cargo later reuses the shared output paths.

Each operation warms twenty eight-call cohorts, then records five samples of
twenty cohorts. Three alternating process pairs (baseline/patched,
patched/baseline, baseline/patched) yield fifteen samples per variant,
operation, and case. A sample is elapsed nanoseconds divided by twenty.

Median CPU milliseconds per eight constructor calls:

| Candidates | Records | Cognitive projection | Before | After | Reduction |
| ---: | ---: | --- | ---: | ---: | ---: |
| 2 | 0 | Absent | 0.217492 | 0.102450 | 52.9% |
| 2 | 0 | Present | 0.212438 | 0.102475 | 51.8% |
| 2 | 64 | Absent | 0.210721 | 0.098710 | 53.2% |
| 2 | 64 | Present | 0.196593 | 0.103533 | 47.3% |
| 8 | 0 | Absent | 0.670498 | 0.325685 | 51.4% |
| 8 | 0 | Present | 0.673095 | 0.348242 | 48.3% |
| 8 | 64 | Absent | 0.661032 | 0.332466 | 49.7% |
| 8 | 64 | Present | 0.651653 | 0.323918 | 50.3% |
| 26 | 0 | Absent | 2.037552 | 1.009346 | 50.5% |
| 26 | 0 | Present | 2.043615 | 1.057594 | 48.2% |
| 26 | 64 | Absent | 2.034211 | 1.065864 | 47.6% |
| 26 | 64 | Present | 1.980206 | 1.018319 | 48.6% |

For eight candidates and 64 records with projection, constructor median drops
from 0.651653 to 0.323918 ms per eight calls (50.3%). At 26 candidates with the
same fixture settings it drops from 1.980206 to 1.018319 ms (48.6%). Across all
twelve cases, explicit-validator and recall-validator median changes stay
within approximately ±6.1%; no speed improvement is claimed for those controls.

The [raw receipt](evidence/20261001-memory-upload-validation.json) retains every
timing sample and summary. Every header, memory-row, and cognitive-row u32 word
was compared directly across all six processes. All twelve cases match exactly.
One complete word vector is retained per case, along with matching payload
lengths and BLAKE3 hashes for every process. Full Debug representations of the
frame, finalized recall, memory bank, and resulting upload—including host
digests and offset domain—also have matching byte lengths and BLAKE3 hashes.
Full host Debug text is not retained, so its hash comparison cannot be
independently recomputed from the committed JSON alone. The complete GPU word
vectors are retained. Proof construction runs outside timed windows. No neural receptor effects are
bound in this fixture; their unchanged dedicated validation is outside this
comparison. This is a preparation equivalence check, not evidence of learned
behavior or a causal full-organism capability.

## Safety coverage and independent review

Three new CPU tests use a populated sealed-event bank and the actual 54-lane
extension fixture. They check valid episodic/projection values, reject seven
foreign binding identities and a foreign frame, reject modified headers, row
indices, missing rows, and NaN payloads through the public validator, and prove
that constructor projection width/finiteness checks remain active in release.
The latter cases explicitly pass initial recall validation before receiving
`MalformedUpload` or `NonFinitePayload` from encoding. Existing assertions are
unchanged; two existing sealed-event helpers are made available to CPU tests.

Checks, after sourcing `/workspace/cloud-cpu-validation/env.sh`:

- Original and patched source: `cargo test -p alife_gpu_backend --release --test
  closed_loop_memory_context --no-default-features`: all 14 CPU tests pass.
- Patched debug/test profile: the same command without `--release`: 14 pass.
- `cargo clippy -p alife_gpu_backend --lib --test closed_loop_memory_context
  --no-default-features -- -D warnings`: pass.
- `cargo fmt --all -- --check`, standalone probe rustfmt check, `git diff
  --check`, and `bash scripts/docs_check.sh`: pass.
- R2 separate-worker source, test, and evidence review: no blockers. The reviewer
  recomputed every summary, checked artifact identities/profiles, and compared
  every retained payload word across the six raw probe outputs.

No `gpu-tests` feature, hardware test, GPU campaign, desktop control, main merge,
or PR is used. The Windows PowerShell launcher is not available in this Linux
host; its actual Bash documentation checker is run directly. Full renderer
host compilation is outside this backend slice and remains unavailable here
without the system SDK dependencies documented in the timing handoff.

## Ownership, integration, and limits

Owned production/test files are only `alife_gpu_backend/src/closed_loop_memory.rs`
and `tests/closed_loop_memory_context.rs`, plus this report and its two evidence
files. Core memory admission, outcome-aware merging, generalization, retention,
shared retrieval, and their APIs are owned by the separate memory worker.
Graphics, persistence summaries, training budgets, experiment fixtures, app
bundle discovery, frozen CI source, and launch operations are unchanged.

The source preserves cognition, physiology, persistence, population, learning,
simulation work, and tick frequency. It preserves the relevant `AOA-INV-003`,
`006`, `009`, `010`, `AOA-AUTH-005`, `AOA-TIME-003`, and `AOA-MEM-002`/`003`
boundaries while addressing `AOA-GOAL-002` latency. These are scoped source
observations, not a complete architecture certification.

This branch is committed locally for coordinator integration. The separate
optional timing branch is published at
`11b814685a5d4a75b842d3d2062f26f5a0ef9aaf`. Combine this backend commit with the
memory worker's completed branch, then rerun the release/debug safety tests and
relevant memory tests against the combined source. Windows upload-preparation
and frame-time improvement remain **Unknown** until measured through the
existing recorder by the desktop owner. No FPS/TPS extrapolation is made.

Rollback is to revert the implementation commit, restoring the unconditional
fresh self-comparison. There is no save format, GPU ABI, or migration change.
