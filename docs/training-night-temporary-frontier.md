# Bounded night frontier: required protocol, not a deployed runner

Cassidy permits deletion and self-contained packing at the end of the night,
provided temporary recovery does not become indefinite history. This document
specifies the crash boundary needed to avoid packing every episode. The host
journal runtime below is **not implemented or qualified** by this change.

The implemented native seam is explicit final private-life capture on an ordinary
sealed directory continuation:

```
train_n2048_care --final-private-life-contract
train_n2048_care --resume-cycle PREVIOUS_DIRECTORY NEW_EPISODE 64 --seed N --lesson feeding --preserve-final-private-life
```

The query must report `kind=native-final-private-life-capture-v1`; the owner must
also pin the executable hash and source/build receipt. Caller-supplied source
metadata alone is not executable provenance. Older binaries do not support this
option. The option uses the existing exact runtime capture, durable snapshot
publication and runtime boundary rebind after collection and before offline PPO.
It does not import the predecessor individual's private state into the new cohort.
Founder weights and both full optimizer heads follow ordinary continuation.

Defaults stay unchanged: a current-file source captures final private state;
the old directory route does not. The opt-in requires an admitted retained source
and a lesson, and rejects objective, delayed-food, replay and actor-credit
overrides. Existing food-need and sampling controls retain their ordinary checks.

## Admission capability still required

The existing `inspect_archived_success --validate-checkpoint` admits and binds
only `cycle.json`, `trained.alife-foundation`, `actor-checkpoint.json` and
`value-checkpoint.json`, including native compiler and critic-continuity checks.
It does **not** admit the private `world.json`, declared assets or sleep journal.
That inspector alone cannot replace current-checkpoint full private admission.

This native slice adds `export_native_training_handoff --inspect-native-frontier
SOURCE --expected-policy POLICY`, which reuses the current migration's full
serial-continuity and typed private capture/validation path without encoding a
current checkpoint. It opens the authoritative durable manifest, resolves the
declared asset closure, checks sizes/digests/path isolation, loads the sleep
journal and validates the private world/lifetime state through existing loaders.
It also rejects a private save whose world tick differs from the receipt's final
collection boundary. The report supplies canonical compiler/actor/value/private
identities, exact decoded asset hashes and explicit legacy source dependencies;
it makes no synthetic seed-cursor or freshness assertion. Those canonical hashes
are not raw input-file hashes. The future host must bind the report to the entire
closed pointer, journal, source and asset bytes with pre/post content checks.
Final capture, native founder/optimizer admission and this private-closure
admission are all required before accepting a temporary frontier. The new CLI
source is not a claim that an available deployed binary supports it. Missing
capability fails closed; running `--import-current` each episode remains safe but
reinstates the packing cost this design seeks to avoid.

## One owner and explicit temporary authority

The store's stable OS writer lock remains held throughout a run. The accepted
current checkpoint is admitted first, then its seed allocation is reserved and
published atomically once. Keep that admitted self-contained anchor until final
packing succeeds. All allocated seeds are already excluded in the anchor.

Use `.run` with a new versioned mode such as `serial-portable-v1`, never reuse
the existing `serial` recovery interpretation. Journal metadata includes exact
anchor hash, pinned source/helpers/executables/prior configuration, allocation,
next seed, next lesson index, accepted portable frontier and at most one attempt.
The full seed union is stored once in a bounded immutable file, hash-bound to the
admitted anchor; the small journal stores the moving cursor. The metadata and
checkpoint byte caps continue to apply. Unknown modes/writers stop recovery.

While `.run` has a deferred frontier, the old anchor is not the newest accepted
brain. Launch, inspect, resume and the stable handoff must either resolve the
journal frontier explicitly or refuse; they must not silently train from or
report the anchor as the latest accepted state. The old runner already refuses
unknown run modes during reconciliation, but an inspection-only path needs this
explicit guard too.

## Dispatch and acceptance ordering

| Durable phase | Authority and crash recovery |
| --- | --- |
| Allocation reserved | Anchor excludes the whole owned range; journal cursor starts at its proven-unused first seed. A new allocation cannot overlap this range. |
| Attempt prepared | Atomically flush an attempt record with seed, lesson, unique output, immutable predecessor identity and incremented next-seed cursor **before launch**. Unknown launch outcome remains consumed; never retry that seed. |
| Worker launched | Record PID and stopped status using existing conservative process checks. Active or unknown workers prohibit cleanup or another dispatch. |
| Native seal present | Check exact row dose, lesson/need condition, source asset, policy increment and actual actor/value update deltas against the accepted predecessor. Admit the full native founder/optimizer and private closure. Missing/invalid seals retain recoverable files. |
| Frontier committed | Flush all admitted recovery payloads before the atomic journal acceptance record. Record the exact closed file identities and compact native reports. A restart re-admits those same bytes before accepting or dispatching. Only a matching committed frontier increments the lesson cursor, once. |
| Stopped unsealed attempt | First prove worker exit. Preserve/recover partial learning, or apply the owner's explicit abandonment rule. Its seed remains consumed/excluded, while the unfinished lesson remains due. No guessed completion. |

Journal writes alone are insufficient durability: the underlying actor/value,
founder, final private manifest/journal and declared assets must be flushed before
committing their acceptance. Native world/private closure validation is not a
substitute for flushing, nor is a host hash a substitute for native admission.

Keep stable unique episode paths. Do not rename a directory and assume internal
bindings still resolve. Retain the predecessor until a complete successor is
admitted and committed. A declared temporary-byte/file budget and disk reserve
bound the night; stop collection or compact at a reconciled boundary before
exceeding it. Avoid an ever-growing per-episode permanent ledger. Compact outcome
counters may persist, but unknown removed evidence remains unknown.

## Final packing and retirement

At collection cutoff, stop new dispatch. At hard end, stop only the owned GPU
tree. Reconcile a complete seal before explicit partial disposition. Device-free
finalization may then run under its existing bounded timeout; it starts no GPU
work and does not extend the training window.

Construct final seed metadata from the exact reserved union and the durable
attempt cursor: `pending_range=[next_seed,last]`, or null when exhausted.
Consumed and abandoned attempts remain excluded; preserve historical freshness
as originally admitted. Do not infer freshness or release unused allocations.

Use native `--import-current LATEST_ADMITTED_PORTABLE SEEDS_JSON NEW_CANDIDATE`
only after checking the complete accepted frontier and predecessor journal. Its
founder, both Adam heads/ages, compiler inputs, accepted seed and private bytes
must equal that frontier; only the aggregate seed metadata is newly assembled.
Fully admit the candidate, then use the existing flushed atomic Store publication
with an exact expected-anchor hash. Record candidate/before/after hashes before
publication so a crash after replacement is recognized as the same finalization,
not a second learning update.

Only after the published current is re-admitted and matches the final frontier
and seed disposition may `.run` and its accepted temporary dependencies be
removed. A failure leaves the anchor plus journal/payloads recoverable and blocks
the next training run. End with one self-contained accepted checkpoint and the
stable lock, never a detached archive moved elsewhere. This protocol does not
authorize the separately pending permanent legacy-quarantine purge.

## Qualification status and necessary checks

This slice adds device-free policy/early-admission regressions and CLI option
validation. Native compilation/tests are not claimed run by source formatting.
An explicitly ignored hardware-GPU regression creates only disposable worlds,
executes two directory continuations with the opt-in, checks saved private-world
tick equals the actual collection boundary, verifies policy and optimizer deltas,
then packs and re-admits after removing all directory sources. Its body is
compiled by ordinary foundation-training CI but not executed. It must be run by
the hardware owner on a qualified executable at a safe interval; ordinary CI
must not accidentally launch it alongside training.

Before any future journal deployment, test crashes before/after seed consumption,
unknown launch, unsealed stop, full seal, acceptance record, final import,
publication intent/replacement and dependency retirement. Assert no duplicate
seed/lesson acceptance, no stale-anchor fallback, preserved full state and exact
unused tail. Compare accepted episodes/hour and decision rows/hour with matched
row doses/lesson mix, plus phase timing and temporary peak bytes. State unmatched
world/runtime limits. Correctness tests alone do not establish a throughput fix.
