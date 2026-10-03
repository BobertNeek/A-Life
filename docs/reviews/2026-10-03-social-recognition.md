# Personal social-recognition endocrine bridge — 2026-10-03

Authorized follow-up to the SocialState part of finding 3 in the
[biology continuity audit](2026-10-02-biology-continuity-audit.md). Cassidy approved
recognition of liked individuals even when another action wins, with simple WWCD
semantics. Base: `18d32d8cd2a1af4485cb60d4925061f0c22ca4f4`; publication branch:
`phase3-social-recognition`. The integration owner retains merge authority.

## Creature-visible behavior

An organism can release its inherited SocialState endocrine response when it
currently notices an individual whose own retained social interactions have
positive net value. It can still choose Rest, Idle, food, or another action.
Previously SocialState had no production source. Positive physical contact
already had, and still has, a separate body-event response.

This reuses the creature's existing per-individual sealed consequence memory and
focal attention. It adds no relationship simulator, social labels, personality
schema, reward, or separate chemical stream. Six-founder construction, innate
competence, genotype and phenotype identity, GPU-authoritative action selection,
and acquired neural state remain unchanged. Relevant controlling requirements:
AOA-BIO-020, AOA-BIO-023, and AOA-MEM-007.

## Minimal mapping and boundaries

The existing exact-target recall searches at most 64 records in the organism,
sensor-profile, and tracked-individual namespace. Before category fallback or
ordinary target TopK truncation, the bridge filters that same matched shortlist
to Inspect, Approach, Contact, and Other only when the action is Hold. Successful
Avoid/escape, ingestion, and other Other actions cannot create this signal.
Stranger/category borrowing and static object affinity cannot create it either.

The social matches retain the existing four-record TopK. Similarity orders them;
recency, then record identity, breaks ties. The existing family aggregation
combines signed consequence values before taking the positive part. Confidence
uses the existing decision confidence, retention and similarity policy, then the
currently observed slot's confidence. Retention scales confidence rather than
changing signed value weights. Eviction and the bounded shortlist can remove
old evidence; this patch introduces no hard expiry or relationship ledger.

A candidate must bind the tracked object slot to a currently sensed SocialAgent
body entity. Finalized recall emits only for a current focal tracked identity.
Unseen, unattended, unknown, or net-negative individuals yield no signal. Among
multiple focal individuals, it chooses one maximum activity-times-confidence
signal and retains that individual's paired values; it does not sum a crowd.
Multiple candidate queries for the same individual also retain one strongest
paired signal, with activity then confidence breaking equal-strength ties.
This matters because PLAY has a distinct candidate feature. Reversing candidate
order cannot make the last candidate overwrite stronger recognition.

Evidence stays private and transient through the existing target cache, routed
recall and finalization. The production sealing function appends it to the
ordinary pre-motor neural-emission frame, using that frame's source tick and
neural dispatch generation. The inherited biochemical graph applies activity
and confidence through developmental expression and the existing SocialState
neuroemitter. The reference graph releases oxytocin; inherited graphs continue
to own the response.

Source: [exact social aggregation](../../crates/alife_core/src/memory/candidate_recall.rs#L189),
[current creature gate](../../crates/alife_core/src/memory.rs#L1041),
[focal individual selection](../../crates/alife_core/src/memory.rs#L335),
[production sealing](../../crates/alife_game_app/src/gpu_live_runtime.rs#L4785),
and [biochemical response](../../crates/alife_core/src/biochemical_graph.rs#L1424).

## Transactions, persistence, and cost

There is no new persisted field or pulse queue. Ordinary saves retain existing
personal memory, tracked identity, and resulting chemistry. Loading does not
replay interaction history into chemistry. A subsequent current encounter can
produce new recognition through ordinary preparation. Contact remains a
separate body event in the same authoritative biology transition.

The existing enclosing tick transaction restores world biology and resident
state on pre-commit failure; its sidecar transaction restores memory as well.
Retry derives the same frozen recognition from the restored pre-state. No new
rollback mechanism or independent commit point is introduced.

Source: [routed preparation](../../crates/alife_game_app/src/gpu_live_runtime/cpu_preparation.rs#L172),
[world/resident transaction](../../crates/alife_game_app/src/gpu_live_runtime.rs#L311),
and [sidecar rollback](../../crates/alife_game_app/src/gpu_live_runtime.rs#L7906).

The bridge adds a bounded filter, sort and four-record aggregate over the already
searched exact-target shortlist. It adds no similarity search or bucket read,
GPU readback, shader dispatch, neural bank, or GPU ABI change. Existing recall
search counters remain accurate. The private transient map is bounded by the
validated frame's object candidates.

## Independent review and verification

A separate SOL 6.1 reviewer found no blocking issue in design and implementation,
including signed history, identity gates, cache/finalization, paired maximum,
sealing order, persistence and enclosing rollback. CPU verification uses one
Cargo build job and one test thread. Final command outcomes are recorded in the
publication receipt.

Focused regressions cover unknown and negative individuals, positive avoidance
and ingestion exclusion, category borrowing and static affinity exclusion,
mixed signed histories, multiple individuals and observation confidence,
duplicate individual queries with reversed candidate order, unseen/unattended
friends, recognition with Idle winning, exact memory and
chemistry roundtrips, malformed-biology failure and retry, and the 64-record
search cap. A CPU production sealing test compares oxytocin with and without
recognition while Rest wins at age 120, after developmental expression activates.
A late failure after actual CPU sealing uses the real world/resident staged
transaction: it restores the entire canonical world signature and serialized
residents, and a retry produces exactly the uninterrupted biochemical post-state.

```sh
cargo test -p alife_core --test candidate_memory_retrieval --test candidate_memory_queries
cargo test -p alife_game_app --features foundation-training --lib seal_prepared_selection_
cargo test -p alife_game_app --features foundation-training --lib late_sealing_failure
```

Existing biochemical graph, development, nociception, prediction pulse, world
transaction and save/load suites provide adjacent regression coverage. Source
formatting, core/world all-target Clippy, the feature-enabled app build, boundary
checks and documentation checks are also part of the publication receipt.

All 233 selected CPU tests pass: 50 recall/query, 37 biochemistry, 143 world, and
three app sealing tests. The existing optional CPU recall microbenchmark remains
ignored. Core/world Clippy with all targets and warnings denied, formatting,
`git diff --check`, core boundaries, and all 77 documentation assertions pass.

Strict app Clippy with `foundation-training` reports 14 warnings on Rust 1.98.1.
The same command on the clean exact base `18d32d8c` produces the identical 14
messages and files: constant `chunks_exact`, manual range/multiple patterns,
existing training functions' argument counts, and the existing training
callback type. These are pre-existing feature lint limitations. No training
source or lint policy was changed. App Clippy with `gpu-runtime`, all targets,
and warnings denied passes, covering the shared production bridge separately.

## Training and remaining limits

Canonical foundation training uses this shared production sealing path, so later
integration changes its experience chemistry too. The
[cycle constructor](../../crates/alife_game_app/src/foundation_training_cycle.rs#L1070)
creates the same GPU live runtime that calls the modified sealing function.
The current PC N2048 short training experiment remains independent and untouched. This cloud publication
does not request an immediate trainer rebuild or restart.

GPU causal behavior, learned relationship quality over a full lifetime, a
GPU-injected late sealing failure, and acquired neural checkpoint restoration
with this recognition source remain Unknown. CPU source and transition evidence
does not establish improved social competence or an oxytocin effect size in
trained N2048 lives. Remote CI outcome must be confirmed by an account with
Actions read access if this executor's existing 403 restriction persists.
