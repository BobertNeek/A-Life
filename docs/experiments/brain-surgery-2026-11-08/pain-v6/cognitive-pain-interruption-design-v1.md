# Proposed cognitive interruption boundary

Status: source-grounded design, not implemented or qualified. Body-only v5 physical withdrawal/recovery interruption is separate. Its pending native CPU fixture must first confirm the actual sleeping-resident path.

## Existing contract
core/src/sleep.rs requires Awake to have active_cycle_id0 and no consolidation. Leaving sleep for Awake requires Committed, increments cycles_completed and advances last_consolidated_cycle_id. Calling that transition for pain would falsely complete work. runtime/src/sleep_scheduler.rs tracks emitted intent, per-cycle work and cadence; these must remain intact. world/src/organism.rs::seal_sleep_phase requires monotone cycle identity. CPU host preserves replay until its synchronous consolidation commits; GPU can hold Pending, Prepared, Submitted, Completed or Committed transactions and durable journal authority.

## Smallest honest phase representation
Use explicit interruption phase tags that encode the suspended phase (Entering, ForcedRecovery, Consolidating or Waking), rather than setting Awake or overloading an unrelated field. Retain the exact active cycle, entered-sleep tick, consolidation request/job/staged output, counters, replay, optimizer and cadence. Appending unit enum variants avoids appending a field to an old nested sequence; introduce an explicit new-version acceptance rule for interrupted states while ordinary legacy states retain their old wire. This still needs golden format tests, not an assumption of compatibility.

Interruption does not increment cycles_completed or last_consolidated_cycle_id. Resume the same cycle and suspended phase. Restart that phase's minimum dwell clock at the real resume tick rather than counting aroused time as sleep; this is conservative and needs no invented completed work or extra duration field. A fresh nociceptive trigger is explicit; chronic injury alone is not an interruption source.

## Neural writer ownership is the key boundary
- ConsolidationNone: no frozen optimizer transaction exists, so an aroused policy can run if its ordinary ATP budget permits. Preserve old replay and append new genuine decisions. Resume later consolidates the actual current state once.
- Committed: the existing commit is real, so waking activity may proceed without relabeling it as a newly completed sleep cycle. Resume/completion counts that cycle once.
- Pending/Prepared/Submitted/Completed: preserve the frozen inputs, job and staged output. Do not dispatch new policy/plasticity against weights whose pending output could overwrite it. The body reflex remains immediate; retain an interruption request and drain/commit the actual pending transaction through its existing guarded driver. Only then permit neural activity. This is a truthful delayed cognitive interruption, not instant wake by dropping work or suppressing learning.

## CPU seam
After the body-only response and one world biology advance, apply the interruption event at the new authoritative boundary, seal the phase in the world record, then seal resident authority. No experience row, credit or sequence increment is created by that event. On the next tick, interrupted-but-safe phases may be motor-eligible subject to ordinary ATP admission. Resume after fresh physical contact has ceased, never based on accumulated pain alone. No forced constant activity: Rest remains a legal policy choice.

## GPU seam
Update sleep_scheduler eligibility/driver guards for interrupted phases, preserving its serialized cadence and emitted-intent markers. Add an actual post-body interruption journal edge in gpu_live_runtime/staged_tick.rs before durable authority publication, with rollback and monotone cycle checks. Preserve Pending/Submitted ownership; do not synthesize Completed/Committed. Extend exact sleep-journal/restore tests, not just controller unit tests.

## Required qualification before normal enablement
For every suspended phase and each consolidation state: interrupt, repeated fresh contact, blocked withdrawal, quiet release, save/restore at interruption, resume, then next cycle. Assert exact replay/optimizer and pending request/job/output identity, no duplicate work/credit, unchanged completed counters until actual completion, and strictly monotone cycle IDs. Test low ATP remains physically reactive without fabricated neural work. For GPU, cover real asynchronous completion during interruption and failure/restore on both sides of publication. Ordinary no-pain histories must remain exact controls.

## Implementation plan after V6 freeze

Use appended explicit phase enum variants for interrupted EnteringSleep, ForcedRecoverySleep, Consolidating and Waking, not a new field in the old nested Serde sequence. Gate their admission with a local new SleepState schema version while preserving old configuration version, existing phase discriminants, old state bytes and old decode behavior. Old schema must reject new phases. Interrupted state retains its entire consolidation payload, active cycle, original entered tick and last trigger; phase_started_tick can conservatively restart phase dwell on resume. Ordinary advance must not finish an interrupted cycle.

Expose state queries distinguishing physical sleep, neural write permission and pending drain. Only None or genuinely Committed permit neural policy while interrupted. Pending/Prepared/Submitted/Completed remain drain-only; real driver events progress the exact retained transaction. No new intent, bounded replay work, replay clearing or optimizer writes are admitted during the suspended drain. Resume maps to the same original phase; it does not increment completion counters. Normal real committed Waking-to-Awake is the only completion edge.

CPU host currently uses phase equality with Awake at admission, ATP lookahead, scheduling, body recovery and policy gating. Those sites need explicit permission queries, not blindly replacing every non-Awake branch. In particular force_sleep currently clears consolidation for some transitions; an interrupted pending transaction must not enter that path. The CPU synchronous consolidation block must run only normal Consolidating/None, never interrupt a frozen transaction. New voluntary replay after an already committed interrupt remains for a later cycle, not cleared by resuming the old committed cycle.

GPU scheduler progress_driver currently only drives Consolidating. Extend its drain eligibility specifically for interrupted Consolidating with a frozen pending stage, while never generating a new intent in the interrupted state. Preserve scheduler cadence and last_emitted_intent_cycle on interrupt/resume, and thread the phase through journal/recovery validation without falsely marking output committed.
