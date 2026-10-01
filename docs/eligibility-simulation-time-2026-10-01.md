# Eligibility follows simulation time — 2026-10-01

Implementation evidence based on integration commit
`163457ca03141a4bfe5e80e22990f3d9eba4e3f3`, branch
`codex/eligibility-simulation-time-20261001`. This report does not amend v2.0.
Scope: AOA-TIME-001/002/003/004/005, AOA-LEARN-002/005 and AOA-BRAIN-007/009.

## Change and causal rationale

Existing eligibility multiplied by one receptor decay factor per decision;
inactive recurrent routes copied their traces unchanged. This retained credit
according to decision cadence and could retain dormant credit indefinitely.
Both recurrent and decoder traces now use `reference_decay ^ elapsed_ticks`.
The reference is one current 0.05s simulation tick
(`alife_world::headless::WORLD_TICKS_PER_SECOND = 20`), not a decision, render
frame, wall-clock interval or neural microstep. No universal cadence is added.

The existing .95 factor remains exact for a one-tick gap. Its half-life is about
0.676s: five ticks retain .773781, twenty retain .358486, twenty-six retain
.263520. Zero elapsed retains the trace; decay zero clears it for positive
elapsed; decay one preserves it. Unsigned 64-bit subtraction handles low-word
rollover. Large gaps use one analytic power, without catch-up loops. Inactive
routes decay prior credit without adding coactivation. All eligibility writes
canonicalize zero, preserving the checkpoint's finite, positive-zero contract.

Weight updates and captured replay eligibility now evaluate the staged trace at
the actual outcome tick. For a five-tick outcome delay, .5 staged eligibility
becomes about .386890 for both uses. The stored bank remains at decision origin:
the next decision decays from that origin, avoiding double attenuation. An
ordinary one-tick outcome therefore receives an additional .95 factor, an
intentional 5% reduction in its eligibility contribution; normalization is
unchanged. The horizon itself is unchanged. A longer useful care/navigation
horizon remains an experiment, not a claimed fix for the zero-meal diagnostics.

## Anchor and state preservation

No ABI fields, checkpoint schema, learned weights, optimizer state or phenotype
parameters change. The newest committed replay event's originating tick already
anchors the active eligibility bank: the successful plasticity finalization
writes that event and promotes its matching bank. Discard changes neither.
Sleep zeros the trace banks and journal. A fresh or reset empty journal admits
any current simulation tick because its active traces are zero.

The row prepass checks ring bounds/cursor and current origin at or after the
newest committed origin before subtraction. Runtime preflight rejects backward
frames against the resident's last activity tick. Checkpoint admission validates
chronological physical ring order, checkpoint/pending bounds and zero active
traces when no anchor exists. Inactive staged/discarded banks may retain values.
With a nonempty journal, a present last-commit key must match its newest sequence;
research migration may clear the key, and sleep may retain it with an empty ring.
Existing source-bound event and pending identities/digests are preserved.
Inconsistent snapshots fail explicitly; they are not silently reset or reanchored.

The ONE current actor checkpoint was unavailable here. No actual actor state was
rewritten. Desktop must retain its original exact checkpoint and use the normal
world/activity restore path. Global clock continuity after a direct neural-only
restore with an empty journal remains the scheduler's responsibility; this slice
guards the trace anchor and does not add a second resident clock.

Production ownership stayed within five reserved files: `closed_loop_abi.wgsl`,
`closed_loop_eligibility.wgsl`, `closed_loop_plasticity.wgsl`,
`closed_loop_runtime/tick.rs` and `closed_loop_checkpoint.rs`. Pipeline codec,
bucket layout, graphics, input digest, training curriculum and other workers'
production files were not edited.

## Evidence and desktop handoff

Cloud commands sourced `/workspace/cloud-cpu-validation/env.sh`.

- Checkpoint unit tests: 9 pass, including eight simulation-clock admission,
  chronology, pending/discard, physical wrap, large-tick reset, source-key and
  exact-snapshot cases. These validate CPU checkpoint contracts, not GPU restore.
- `closed_loop_wgsl`: 16 pass, including Naga validation and ABI reflection.
- `eligibility_simulation_time`: 2 CPU call-graph/Naga checks pass. Weight credit
  and replay capture both call the same production outcome-time helper.
- Feature-enabled `recurrent_eligibility_obeys_the_validated_activity_route_mask`:
  1 source-contract test passes; inactive work decays and adds no local activity.
- `cargo check -p alife_gpu_backend --features gpu-tests --test
  eligibility_simulation_time` passes. Its bounded 17-value hardware probe uses
  the exact production helpers, including zero/one decay, delays, low-word
  rollover and negative-trace zero/underflow. Hardware execution is **Blocked**
  in this cloud CPU environment; no numerical GPU or performance claim is made.
- Broad GPU-backend CPU unit suite: 52 pass, 2 failures. Both reproduce on the
  integration base: the fixed-arena 128MiB expectation and curated-founder cutover
  fixture compilation. Their production files belong to other integration work.
- Formatting, whitespace, core boundaries and docs checks pass. Independent R2
  review found the negative-zero edge; it was fixed. No remaining scoped source
  blocker was identified.

Desktop RTX checks, without training:

1. Run `cargo test -p alife_gpu_backend --features gpu-tests --test
   eligibility_simulation_time`; record its adapter and numerical results.
2. Use a normal committed transaction to seed nonzero recurrent/decoder traces.
   Check gaps of 0, 1, 5 and 20 simulation ticks, including disabled routes;
   compare a one-long-gap execution with equivalent shorter gaps without new
   coactivation. Include a negative trace decaying to canonical zero.
3. Seal an outcome five ticks after its origin. Verify weight credit and replay
   Q15 sample use the same attenuated trace, while the promoted bank retains its
   origin value. The next origin ten ticks after the first must use .95^10 once.
4. Exercise discard, full ring wrap, sleep then wake at a large world tick, and
   exact save/restore. Compare uninterrupted versus restored trace/bank state;
   reject a backward frame. Keep the actual actor's original checkpoint intact.

These are bounded production numerical/restore checks. Useful learned care and
navigation remain **Unknown** until tested through the ordinary gameplay path.
