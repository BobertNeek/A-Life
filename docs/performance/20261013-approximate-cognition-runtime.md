# Approximate live cognition — 2026-10-13

Authorized scope: reduce ordinary-play preparation cost aggressively, accepting
lossy recall and changed decisions. The learned brain, organism identity,
episodic store and save assets remain intact. This is an implementation policy,
not a revision of the controlling v2.0 architecture.

Implementation branch: `codex/approximate-cognition-runtime-20261013`.
Initial checkout: `d2e4a157`; integrated main: `6f295921` (consolidated main).
Final measured code: `e94e61a1`, including the full-refresh deadline,
untracked-entity isolation and confidence-weighted attention fixes.

## Runtime policy

| Work | Ordinary play | Training / audited diagnostics |
| --- | --- | --- |
| Current physical perception, GPU selection and GPU learning | Every neural tick | Every neural tick |
| Attention memory cue | Reused owner-bound cue buffer | Full preliminary recall |
| Focused episodic retrieval | Every four ticks; immediate on significant change or new target/action | Every tick |
| Search budget per target/family channel | 16 retained records | 64 retained records |
| Concept/gap summary | Every ten ticks or significant change | Every tick |
| Topology observation | Every ten ticks or significant outcome | Every sealed patch |
| Memory observation | Every sealed patch; lightweight owner/sequence checks | Every sealed patch; full audit |
| Bank binding on ordinary ticks | Owner, generation and retained count | Canonical checksum |
| Full bank validation and checksum | Save/load and compaction boundaries | Also in tick audits |

At 20 neural ticks per second, four ticks is 5 Hz and ten ticks is 2 Hz.
Those are tick intervals, not additional wall-clock timers. Pain above 0.01,
absolute valence above 0.15, an absolute drive delta above 0.03, contradiction
or teacher feedback triggers an immediate refresh. Newly visible tracked
objects and a drive change above 0.15 also refresh preparation. Missing or
evicted cached sources and new target/action identities force real retrieval.
Partial cache misses do not reset the four-tick full-refresh deadline. A stream
of changing candidates therefore cannot keep retained expectancies stale
indefinitely. Partial updates still publish current attention memory cues.

Active foundation training and receipt-heavy causal/soak diagnostics select
the audited path explicitly. Ordinary play uses approximation even when the
binary includes training support. Those diagnostic protocols do not measure
the approximate production policy's performance or behavioral competence.

Untracked object candidates retain their current world entity identity in the
transient cache key, so same-action objects cannot reuse each other's
expectancies.

Cached expectancies deliberately omit exact current features. Current
perception, frame binding, candidate index, query and episodic decision key
are reconstructed every tick, so the selected action's sealed experience
describes the current observation. Caches never cross owner/profile boundaries
and are discarded at restore. A restored creature can make a different next
decision because its transient summary is refreshed; that is the authorized
approximation, not a loss of lifetime learning.

Memory retention, merging, contradictions and existing capacity limits remain
unchanged. Runtime receipts explicitly identify approximate operation and omit
the detailed per-candidate audit vector. A generation binding is never treated
as a portable checksum. Save export and compaction materialize the real digest.
GPU neural execution remains authoritative; no CPU neural fallback is added.

Shared cue/summary buffers and fixed drive storage avoid repeated scratch
allocations. Bounded concept/gap selection avoids full temporary lists and
sorts; the episodic shortlist uses the index's retained order instead of sorting
the complete neighbor union. The app constructs the GPU frame binding
directly instead of encoding and discarding a duplicate perception payload;
the backend still uploads the actual current perception.
Cached attention cues use the same confidence, source support and signal
weighting as audited recall. Weak extreme outcomes cannot gain maximum
salience merely because their raw value is large.

## Requirements and limits

AOA-TIME-002 permits different cadences. AOA-MEM-001/002/003 and AOA-INV-011
remain relevant: episodic evidence retains source IDs and current decision
binding and reaches neural arbitration. AOA-PERF-008 motivates moving bulky
audit provenance away from ordinary execution. AOA-INV-012 motivates removing
cost without demonstrated benefit. AOA-BRAIN-014 still requires behavioral
tolerances to be assessed; this change has no new quantized weight format or
brain-layout change.

The tradeoff is explicit: an important episode outside the 16-record shortlist
can be missed, quiet concept/topology information can lag, and cached values
can briefly reflect older drive or geometry conditions. Population behavior,
rendered gameplay, long-run competence and the 50-creature frame-rate target
remain **Unknown** until measured. Focused tests cannot establish those claims.

## Verification

Candidate-memory suite: 47 passed, one benchmark ignored. The CPU preparation
suite passed 16 tests with three benchmarks ignored, including five recall
refreshes and two summary refreshes over 20 quiet ticks, fresh query keys,
unchanged retained records and an off-cadence forced refresh. A separate
candidate-churn regression passed: cache updates on every tick retain full
refreshes at ticks 100, 104, 108, 112 and 116. Concept/gap selection matches the
audited bounded output. Initial core boundaries, 77 documentation assertions
and GPU-feature Clippy passed before consolidation. Final focused runtime
library Clippy (`--no-deps --features gpu-runtime --lib -- -D warnings`)
passed on consolidated main. The broader training-feature lint command remains
blocked by pre-existing trainer warnings and the Windows-only unused `mut` in
`alife_semantic/src/luna_prior.rs`; those unrelated files were not changed.

The final weighted-cue regression passes: supported moderate evidence outranks
weak extreme evidence. The refresh-cadence, fresh-key, bounded-summary and
candidate-churn regressions pass with the final app executable.

Real GPU validation on an NVIDIA GeForce RTX 3050 passed the canonical
four-creature portable-checkpoint test. It performed live neural ticks,
retained old and new saves and restored both on the GPU: 52 current manifest
roots, 40 obsolete roots excluded from the new manifest and 12 shared roots
retained. This is checkpoint/integration evidence, not a behavioral assay.
The legacy `v11_authoritative_admission_real_runtime` fixture fails before
neural admission because it creates no organism registry record; it was not
used as the final gate or repaired in this performance change.

The [final CPU receipt](evidence/20261013-approximate-cognition-runtime.json)
is source-bound to `e94e61a1`, an optimized test build with debug assertions.
Nine samples alternate mode order. Two workers prepare 26 candidates and 64
retained records per creature. The cycle column includes both scheduled recall
and summary refreshes over 20 sequential ticks, rather than only warm hits.

| Creatures | Audited preparation | Runtime cold | Runtime cached | Runtime cycle average |
| --- | ---: | ---: | ---: | ---: |
| 8 | 12.66 ms | 2.51 ms | 2.36 ms | 2.40 ms |
| 50 | 80.29 ms | 16.94 ms | 14.62 ms | 15.06 ms |

This is about 81% less CPU preparation time in quiet fixtures. Some samples
overlap local core compilation; every raw sample is retained. The timer
excludes draft construction and publication, world stepping, sealed memory
observation, GPU execution and rendering. It cannot establish game FPS/TPS.
Changed scenes can force more cold refreshes. These measurements include all
three cache fixes and consolidated main. Earlier measurements remain in Git
history and do not replace this final-source receipt.
