# Memory outcome quality and shared recall CPU receipt — 2026-10-01

Repeated-candidate recall now reuses identical queries within a frame. On the
bounded CPU fixture below, median recall cost falls from **1.734 to 0.501 ms per
frame (71.1%)**, with similarity evaluations falling from **4,096 to 128**.
Distinct-candidate cost is approximately unchanged. Memory now admits meaningful
outcomes, retains individual contradictions, transfers conservative perceptual
evidence to unfamiliar objects, and fades unused weak evidence.

The focused suite passes **32 tests**, and the full core suite passes **464
tests**. Independent Sol 6.1 R2 review approves the memory implementation with
no remaining P0/P1 findings. Live GPU behavior and Windows/RTX FPS improvement
are **Unknown**; no training or GPU execution ran.

## Source and ownership

Repository: `BobertNeek/A-Life`; branch: `cloud/memory-quality-20261001`.
Baseline: `2bad5e02b6bbf125a10822d3c7effd7debc96d8f`.
Memory implementation: `cba0740bb85c7c9cbc9c3ea923aa02ee4d8be3d9`.
Combined source, including the independent backend optimization and test-only
Clippy cleanup: `d0da0b58b78e17ff0487a96b3842e278c3397308`, tree
`6e631ecb7c342d9a6f4f1884735c5c0158da1a13`.

The memory slice owns `memory.rs`, `memory/candidate_index.rs`,
`memory/candidate_recall.rs`, `memory/sidecar.rs`,
`tests/candidate_memory_retrieval.rs` in `crates/alife_core`, and the affected
paragraphs in `docs/ARCHITECTURE.md`. It changes no neural decoder, GPU layout,
training assets, or neural action authority.

The separately owned backend slice is cherry-picked as `a21c21d7` and
`b6fa20fe`; its original implementation is
`ffeea2528d0475541ff1078a3510762538d71c5e`. Its source and measurements are
documented in the [fresh upload validation receipt](20261001-memory-upload-validation.md).
Its reported release-constructor gain is a separate measurement and is not
added to the recall gain here.

## Bounded implementation and counterexamples

Within a frame, target recall is reused only when the actual state and target
feature bits match; family recall also requires the actual action feature bits
and family to match. Quantized index equality alone cannot authorize reuse.
Candidate-local sealed identities remain distinct. Single-use candidate groups
skip cache allocation. Receipts count actual work once and identify the earlier
candidate supplying reused evidence; validation checks that provenance.

Admission considers measured valence, pain, meaningful drive/ATP changes,
frustration with disappointment, prediction error, consumption, blocking,
physical manipulation, and a first novel inspection. Ordinary locomotion with
the runtime's small energy/ATP costs and prediction error is ignored. Ignored
events still advance the sealed-sequence replay guard. Hundreds of such events
cannot fill the bank, merge into an existing belief, or refresh its retention.

Similar episodes merge only when their measured consequences agree. Contradictory
episodes remain separate. For equally relevant individual action-family evidence,
the latest measured consequence is the current expectation; same-tick ties use
the sealed causal sequence. A harmless approach cannot erase painful ingestion
evidence in target recall. Compaction prefers the current representative when
compression must discard one of two conflicting episodes. A direct reversal in
a full bank can replace its own older contradiction without displacing another
individual's strong danger evidence.

An unfamiliar tracked object can borrow evidence from perceptually similar
objects using observable colour, material, shape, chemistry, and temperature
cues. Affordance alone is insufficient. Fallback requires multiple cue groups,
per-group agreement and an empty individual namespace. Both channels retain
organism/profile fences; family recall also retains its action-family fence.
Borrowed confidence is discounted; it does not
create personal familiarity. A known poisoned individual keeps its exception,
including when changed body/context features prevent an exact recall match.
Different fruit chemistry or shape does not inherit the fixture's reward.

Weak evidence loses recall confidence and eviction priority with age, reaching
half confidence after 16,384 simulation ticks and retaining a 0.05 floor.
Measured strong danger/reward retains full confidence and protected capacity
priority. Corroborated retrieval can increase retention only after a compatible
new measured outcome; recall alone never changes confidence, consequence values,
or persisted state. A lower-value new event cannot crowd out a bank of strong
outcomes.

All retrieval channels retain the existing 64-evaluation ceiling and top-four
match bound. The category indexes are derived, nonserialized state, rebuilt on
restore; no acquired-memory fields or compatibility framework are added.
Compaction's comparison with at most 64 newer records per exact identity is an
explicit approximation for unusually dense context histories.

## CPU comparison

The [raw receipt](evidence/20261001-memory-quality-and-recall.json) includes
every retained batch sample, executable and source hashes, work counts, commands,
test results, and review disposition. The ignored developer test
`memory_cpu_recall_microbenchmark` calls the real `MemoryBank::recall_frame` API
on a 256-record synthetic bank. Baseline production source is unchanged; only
the identical benchmark test is appended to its isolated worktree.

Host: AMD EPYC 9V74/KVM, five exposed CPUs. Rust:
`1.98.1 (48a229cea 2026-09-01)`. Both variants use the unoptimized Cargo test
profile, debug assertions enabled, debug information and incremental compilation
disabled. Build concurrency is two; tests are serial. Benchmark processes are
pinned to CPU 0. This is CPU recall timing, not release game-frame timing.

Three alternating process pairs each record five batches of 100 calls per
workload. The table uses the median of all 15 batch times divided by 100;
returned-value destruction is included. The baseline is built in a separate
Cargo output directory, and both captured executables must report their expected
evaluation counts. An earlier shared-output retry that reused the patched
binary was rejected and contributes no samples to this comparison.

| Workload | Before ms/frame | After ms/frame | Change | Evaluations before → after |
| --- | ---: | ---: | ---: | ---: |
| 32 identical object/family queries | 1.73372 | 0.50073 | 71.1% lower | 4,096 → 128 |
| 16 distinct object queries | 0.54455 | 0.55095 | 1.2% higher | 512 → 512 |

The distinct-query control supports approximately neutral cost when there is
nothing to reuse. These two bounded synthetic cases establish neither
population scaling nor accelerator, rendering, persistence, or training cost.

## Correctness and review

`cargo test -p alife_core --all-targets -- --test-threads=1` passes 464 tests
across 57 test binaries, with one developer benchmark intentionally ignored.
The combined branch's focused memory suite passes 32 tests with that same
benchmark ignored. Both ordinary and assertion-disabled test-profile backend
memory-context runs pass 14 CPU tests. The latter exercises the production
constructor's `cfg(debug_assertions = false)` branch without claiming a release
performance measurement. The default feature set excludes GPU-dependent tests.

The memory tests cover positive/negative outcomes, poisoned individual and
changed-context exceptions, differing perceptual categories, boring repetition,
bounded retrieval/capacity, reversal and compaction, same-tick sequence order,
confidence fading and corroborated retention, exact save/load recall, portable
asset tampering rejection, and each compaction crash-recovery phase. Existing
organism/profile isolation, finite-value, sealed-decision, and replay guards pass.

Rustfmt, core Clippy with warnings denied, core boundary checks, `git diff
--check`, and all 77 documentation assertions pass. Independent R2 review is
read-only; its identified blockers were corrected before approval.

## Architecture evidence scope

This is dated implementation evidence, not a change to the controlling v2.0
architecture or a certification of its complete gameplay capabilities.

| Requirements | Evidence here | Remaining evidence |
| --- | --- | --- |
| AOA-MEM-001, AOA-MEM-008 | Bounded delayed recall, temporal confidence, compatible merging, contradiction and retention tests | Live behavioral retention/generalization remains Unknown |
| AOA-MEM-002, AOA-MEM-003, AOA-INV-006 | Candidate-local evidence and sealed identities preserved; neural authority unchanged | GPU causal action effect remains Unknown |
| AOA-INV-001, AOA-CTX-003 | Generalization uses measured perceptual cues and outcomes, never identity digest geometry | Full concept capability remains Unknown |
| AOA-INV-005, AOA-PERF-001, AOA-PERF-003, AOA-PERF-009 | Fixed capacity/channel bounds, unique-work receipts, CPU control comparison | AOA-PERF-005 population/FPS evidence remains Unknown |
| AOA-PERSIST-001, AOA-PERSIST-002, AOA-PERSIST-004 | Current memory state roundtrip, derived-index rebuild, tamper rejection, crash-phase tests | Whole live organism behavioral equivalence remains Unknown |
| AOA-EVAL-002, AOA-EVAL-004, AOA-EVAL-005, AOA-EVAL-008 | Mechanism-level reversal, recall, novel-instance and unrelated-danger counterexamples | End-to-end neural gameplay gates remain Unknown |
