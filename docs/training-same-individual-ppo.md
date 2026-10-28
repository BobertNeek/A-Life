# Native same-individual PPO boundary

`train_n2048_care --resident-continuous STORE REQUEST_JSON` restores the single
living individual and world embedded in an admitted Current. It does not create
a new scenario, age hunger, place food, replace the saved sensor profile or
resume the old fresh-cohort curriculum. The allocation's seed is a **session
nonce**. The original world seed and individual identity remain unchanged.

The request has schema 1, `seed_range`, `total_trained_decisions` (1..10,000,000),
`decisions_per_update` (1..36,000), `snapshot_every_updates`,
`sampling_temperature`, `collection_cutoff_unix_seconds`,
`finalization_deadline_unix_seconds`, and `max_temporary_bytes`. The host admits
ownership, resources, the actual executable/source and fresh dated deadlines.
The Native producer owns the store lease and durable session seed WAL. Recovery
and exact Current publication remain separate owner operations.

The live runtime and both optimizer states remain resident. A frozen-policy
window collects ordinary body, cognition, recurrent/lifetime learning and
biology. Its last real waking action supplies the bootstrap; it advances the
world and is excluded from the PPO loss. Sleeping/non-capture ticks remain real
causal ticks. At a deadline, the existing last record is the bootstrap: no
fictitious next state or extra post-cutoff action is created. A window with fewer
than two records has no PPO update. Raw replay uses the existing buffered cohort
format, one seal/sync per window, then is removed after the update. Raw replay,
feature/RAM, adapter binding and aggregate temporary-write limits still apply.

The actor PPO and sampled residual critic use the existing recurrent replay,
measured physiology reward, elapsed-time discount and full Adam histories. The
exported global policy is installed only at a sealed live boundary. That install
updates compiled base weights and identity without writing private mutable
words. It retains structural additions, replay/eligibility/lifetime banks,
recurrent activations, private memory, body/genome/ecology, action sequence and
exploration stream. The GPU learning anti-replay cursor is explicitly rebound
without resetting its committed sequence. Completed sleep-cycle tombstones
remain; old-policy cached success cannot authorize the new policy.

An active Pending/Prepared/Submitted/Completed sleep transaction defers offline
policy installation. The collector then leaves heads/Adam unchanged and saves
the actual private sleep state, reporting the captured window as untrained. It
does not force a wake or reset consolidation. A previously tagged saved sleeping
life restores its matching policy directly without an unnecessary install.

Within each PPO window, the residual critic keeps one private GPU head with its
weights, Adam moments and integer age, three pipelines, and reusable feature/row
scratch across accepted actor updates. Batch rotation still starts at row zero
for each window, uses at most 2048 loss rows, and normalizes the partial final
batch by its actual row count. Each batch uploads its actual captured features;
equal shapes never authorize reusing old feature contents. Before/after metrics
and the per-step GPU finite-success readback remain unchanged.

Replay admission may reuse successful exact compiler/private-content bindings.
Keys retain raw float bits, including signed zero; failed admission is never
cached. Decoded records have a process-local 512-entry / 256 MiB conservative
retention-charge ceiling, separate from the measured host-memory guard. Every
optional retained-record copy also needs fresh measured allocation admission;
failure skips the cache without failing already-admitted uncached work. Every
external replay load still verifies actual stored lengths and BLAKE3 bytes
before consulting that cache. Least-recent records are evicted to retain ordinary
burn-in overlap when its charged size fits; oversized records remain uncached.
These charges are estimates, not a physical-memory bound. At most eight PPO spans,
within a separate 512 MiB conservative owned-clone charge, are retained as
immutable session-local windows for repeated passes. Every cloned hit repeats
measured host-memory admission;
optional retention is skipped if the extra copy cannot fit. Changing a disk file
afterward cannot change these already-admitted RAM snapshots, but a later
external load must reject the changed file. The PPO-window snapshot cache is
dropped at that policy-window's training boundary; process-wide caches remain
content-bound.

The existing CPU value-head checkpoint handoff remains at the logical PPO-window
boundary: one head read from PPO before the fit, one upload into the private
residual context, one completed-fit readback, then restoration for subsequent
PPO replay. There is no head checkpoint or pipeline reconstruction between
residual updates. A failed fit drops its private GPU state without publishing a
partial checkpoint. Keeping this boundary avoids aliasing the different PPO and
residual scratch layouts and preserves the existing exact checkpoint/recovery
path. Pipelines and scratch are recreated for the next window; per-update metric
readbacks remain. This is an allocation/transfer optimization, without a measured
hardware speed or learning-quality claim.

Legacy fresh-cohort Current paired pre-PPO private state with post-PPO global
heads. On its first same-individual session, the producer explicitly adopts
those accepted global heads, preserving private mutable state; it does not
invent a historical private policy version. A missing legacy sampling payload
initializes a declared new stream. Present saved sampling seed, counter and
temperature always win over a new request. Gameplay preserves that payload
without activating training sampling.

Periodic and final snapshots pair the installed global actor/value/full Adam
with this exact **post-install** live private closure and explicit retained
session row/update counters. Provisional updates since the latest snapshot can
be lost on interruption. A natural death is terminal, not truncation: the
producer retains the last valid snapshot and reports the terminal tick and
unretained suffix. The pending terminal window is currently discarded before
PPO; this path does not yet train a zero-bootstrap terminal target. Retaining
that learning needs an explicit terminal-head handoff, separate from the last
exact living-private snapshot. The observed terminal disposition remains inside
Current after temporary cleanup and blocks continuation even from a copied file.
It never resurrects an earlier living snapshot or silently relabels a fresh
creature as the same individual.

`continuous-started-v1`, `continuous-update-v1`, untrained-window/terminal events
and the finished summary distinguish actual captures, loss rows, bootstrap,
epochs, policy versions and actor/value optimizer ages. The startup/finish
hardware receipt identifies the selected native adapter/backend. A host NVML
adapter observation alone does not establish that selection.

`causal_collection` measures actual unique committed world ticks at the
advertised `world_ticks_per_second` (20), divided by measured collection wall
time. It excludes setup/first prior prime, PPO, installation and snapshots;
intervening replay and later prior work are included. Its capture counters are
foundation training-state readbacks, not all GPU traffic or GPU time. Whole
retained-session throughput additionally includes admission, restore, PPO,
save, recovery/publication and cleanup. Compare equal decision exposure with
explicit update/epoch counts: 512 rows in one update and eight 64-row updates
are different learning workloads. Neither source compilation nor row counts
establish hours of simulation per minute, better self-care, or useful learning
gain. Those require a separately admitted hardware measurement.
