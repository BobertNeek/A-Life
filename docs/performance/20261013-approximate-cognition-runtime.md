# Approximate live cognition — 2026-10-13

Authorized scope: reduce ordinary-play preparation cost aggressively, accepting
lossy recall and changed decisions. The learned brain, organism identity,
episodic store and save assets remain intact. This is an implementation policy,
not a revision of the controlling v2.0 architecture.

Implementation branch: `codex/approximate-cognition-runtime-20261013`.
Initial checkout: `d2e4a157`; upstream integration and final evidence are recorded
below when verification finishes.

## Runtime policy

| Work | Ordinary play | Foundation training |
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
allocations. A bounded shortlist uses the index's retained order instead of
sorting the complete neighbor union. The app constructs the GPU frame binding
directly instead of encoding and discarding a duplicate perception payload;
the backend still uploads the actual current perception.

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

Initial candidate-memory suite: 37 passed, one benchmark ignored. It includes
runtime cache binding, real-checksum export/restore and the existing strict
learning, isolation, contradiction and compaction cases. Final integrated
checks and same-source CPU measurements follow after upstream integration.
