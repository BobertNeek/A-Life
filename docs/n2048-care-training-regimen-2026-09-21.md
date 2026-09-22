# N2048 care training regimen — 21 September 2026

Status: implementation design, not an executed training result. This is the
hardware-specific training subplan for the core-game-loop plan. It replaces the
earlier conversational training proposals. The v2.0 architecture remains the
only architecture authority.

## Outcome and hardware budget

Produce an opt-in N2048 founder that can obtain food, avoid known hazards,
recover, and acquire and retain useful preferences during ordinary life.
Communication, breeding, full founder promotion, and meta-training of plasticity
rules are later milestones. A care candidate does not satisfy all of
AOA-FOUND-003 merely by completing this regimen.

Observed hardware: Intel Core i7-3770K, 4 physical cores / 8 logical processors;
31.97 GiB system RAM; NVIDIA RTX 3050, 8 GiB VRAM. About 19.92 GiB RAM and
7.29 GiB VRAM were free during design. Availability is transient; these are not
training benchmarks. No training duration or speedup is proven.

| Resource | Starting configuration and limit |
| --- | --- |
| GPU | One authoritative GPU scheduler and one foundation optimizer; no competing trainer processes. |
| Experience | 8 independent worlds, pooled over 2 CPU world workers; benchmark 16, then 32 only while throughput improves. |
| Training batch | 2 sequences per physical GPU batch; accumulate 4 batches for an effective batch of 8 sequences. |
| VRAM | At most 6 GiB for the training process, retaining at least 2 GiB for desktop/driver headroom. |
| Host memory | At most 8 GiB for the training process, including a 4 GiB bounded replay store. |
| SLM | Disabled throughout the first campaign and acceptance. No concurrent local model inference. |
| Time | 8 elapsed hours after prerequisite repairs/checks; reserve the final 2 hours for acceptance. |

CPU workers advance worlds and prepare data; all neural inference, gradients,
and weight updates remain GPU-authoritative. Independent worlds have separate
bodies, object registries, memories, learning state, RNG streams, and eligibility.
Only offline foundation training combines their experience. Use globally unique
resident organism IDs and explicit world routing when batching across worlds.

Reuse class-bucketed GPU batching and the production organism transaction. If
the app currently owns one session per world, expose its prepare/dispatch/commit
boundary for a training coordinator; do not implement a second biology or action
loop. A failed world step must not apply another world's outcome or credit.

## Prerequisite: repair the trainer, not its reputation

Source inspection found the following gaps. Their fixes require execution proof
before the eight-hour campaign begins:

- The training forward shader omits production receptor gain/threshold effects,
  dendritic contributions, activity route masks, and lifetime/fast weight banks.
- Training action logits omit production memory and cognitive projections.
  Those decoder weights can be selected by stage masks without a corresponding
  gradient kernel, allowing decay without task learning.
- Training resets its neural state every 32 ticks. Its care curriculum uses
  synthetic lobe stimulation, motor-source clamps, and artificial features.
- Evaluation counts target-logit sign checks as episodes. Some language metrics
  are assigned rather than independently measured. These cannot promote a care
  candidate or validate generalization.
- Resume reloads foundation weights without restoring Adam moments, step count,
  sampler state, or the original development baselines.
- Every update runs a second full diagnostic forward pass and waits for a
  readback. Decoder kernels scan unrelated synapses. Nonfinite handling can hide
  a bad forward calculation or skip individual weight updates.

Repair the existing `alife_training` implementation. Share production WGSL math
where practical and reproduce all enabled production terms; fail explicitly on
unsupported terms. Add gradients for every weight group declared trainable.
Keep speech weights frozen. Do not change production dynamics to fit the trainer.

Make initial activation, neuron homeostasis, route/receptor context, effective
weight context, candidate inputs, episode boundaries, and padding explicit in
training sequences. Preserve nonlinear dendritic derivatives through their
source activations, even when dendritic parameters themselves remain frozen.

Differentiate neural computation within a window. Treat recorded world changes,
chemistry, retrieval decisions, and plasticity updates as fixed context. This is
conditional truncated backpropagation, not differentiation through an entire
life. Unsupported structural changes split replay at a compatible checkpoint;
never turn off normal deployed learning merely to simplify training.

Replace care target-logit regression with stable pairwise ranking:
`softplus(logit_competitor - logit_acceptable)`. Compare candidates that compete
for the same production selection/channel decision. Preserve jointly compatible
motor outputs and do not label all unchosen but acceptable actions as wrong.
Normalize over valid pairs and sequences, excluding burn-in and padding.

Add complete training-only checkpoints: genetic weights, Adam moments and step,
per-weight optimizer age for newly enabled masks, RNG/sampler state, curriculum
and dataset versions, development baselines, configuration, graph identity, and
source revision. Newly enabled parameters must not inherit an inappropriate
global Adam bias-correction age. Frozen weights and moments remain unchanged.
Reject a numerically invalid update as a whole; never retain partial writes.

Extend the training-only sequence/batch/receipt types and CLI for budgets,
batching, window length, resume, and output. Keep canonical foundation export
unchanged unless version validation requires a narrowly scoped extension. Add
an explicit N2048 asset option to existing New Game/import paths, preserving
saved identities and current defaults. Trainer state never enters game saves.

## Method: demonstrations, learner corrections, then long lives

Use recurrent imitation plus DAgger-style data collection for the first care
campaign. Start from the canonical N2048 foundation, not Nano512 weights.
Bootstrap with 32 complete grounded demonstrations, then collect and correct
the states the learner actually visits. The offline demonstrator may choose
training actions but has no presence in deployment or acceptance.

Do not add PPO, a value network, or a general reward designer to the first
campaign. The immediate bottlenecks are faithful training, grounded data, and
action competence. Pairwise imitation is not reinforcement learning and does
not provide arbitrary long-delay return credit. Ordinary biological reinforcement
and memory operate throughout learner lives. Their effectiveness is an acceptance
question, not an assumption. If delayed-credit behavior fails despite correct
execution and adequate examples, report it as the next algorithmic gap rather
than claiming longer windows solved it.

Use teachers to demonstrate generic skills, not hidden food identities or future
outcomes. Vary visual identity and consequences; teach sampling and revising
preferences from experienced outcomes. Teacher labels remain separate from the
sensory frame. No synthetic reward, distance-to-food reward, hidden survival
controller, forced consumption, or reserve refill enters the organism.

| Phase | Lives and lessons | Gradient window / burn-in |
| --- | --- | --- |
| Numerical bootstrap | Short real approach/contact/ingestion sequences. At most 10 minutes of pilot work at 32 ticks. | 32 / 0 only for diagnostics and birth-start sequences. |
| Basic care | 30–120 simulated seconds: approach, stop in reach, inspect/sample, ingest, and recover. | 256 / 128 ticks. |
| Integrated care | 5–10 simulated minutes: competing objects, obstacles, hazards, changing hunger/fatigue, repeated feeding and recovery. | 512 / 256 ticks. |
| Adaptation and retention | 20–30 simulated minutes: repeated object encounters, consequence reversal, natural sleep and revisiting. | 512 / 256; compare 1024 only on observed delayed-dependency failures. |

At 20 TPS, 256/512/1024 ticks provide 12.8/25.6/51.2 seconds of differentiable
history. A 30-minute life contains 36,000 ticks. Never confuse those lengths.
Death is a terminal event; a training time limit is truncation, not death.
Do not reset memories or plasticity at gradient-window boundaries.

Sample contiguous histories. For replay, restore the recorded compatible start
context and replay burn-in under current genetic weights without gradients.
This reduces stale-state error but is not exact reconstruction of a different
genetic brain's entire life. Record behavior-foundation versions and prioritize
recent learner trajectories. At least half of learner samples come from the
latest completed collection round; older learner data expires after 3 rounds.
Keep a separate fixed demonstration/earlier-skill reservoir.

Pin genetic foundation weights for each collected life; new versions apply to
new lives, not as midlife brain transplants. Collect parallel cohorts with a
common foundation version, then perform bounded training rounds. Collection and
training alternate on the one GPU; CPU preparation overlaps when useful. This
avoids distributed asynchronous-policy machinery and uncontrolled policy lag.

Initial replay mix: 50% demonstrations, 25% latest learner experience, 25% other
eligible learner experience. After basic care passes, use 25% earlier-skill
demonstrations, 50% current failures/corrections, and 25% mixed learner experience.
Within those pools retain complete approach-to-outcome and sleep/revisit segments;
never retain only successful Eat frames. Sample uniformly within each stratum.
On memory pressure evict oldest eligible learner episodes first; retain required
window context and never silently truncate a selected learning segment.

Keep replay compact: observations, candidate data, executed actions, outcomes,
versioned context snapshots and weight deltas. Do not store a dense full neural
tape or a full synapse-bank copy for every world tick. Allocate gradient tapes
only for the active batch. Preflight measured episode storage against the cap;
reduce resident worlds if the minimum complete replay set cannot fit.

## Eight-hour schedule and adaptation rules

Preparation/repair time is separate; campaign time includes collection, tuning,
evaluation, and interruptions. All GPU/Cargo work remains serialized with other
tasks. This task supervises the campaign and retains its best checkpoint.
The autoresearch-style comparisons below share this budget; they are not an
additional campaign. Spend at most 90 minutes of the first six hours on tuning
comparisons, including their builds, warmup, evaluation, and confirmation.

| Elapsed time | Work and stopping rule |
| --- | --- |
| 0:00–0:20 | Pilot: end-to-end grounded rollout, useful gradients, batching and resource measurements. Stop on correctness failure. |
| 0:20–2:00 | Basic care, demonstrations plus learner-state corrections. |
| 2:00–4:00 | Integrated care if the basic gate passes; otherwise diagnose and repair the current blocker. |
| 4:00–6:00 | Long-life adaptation if integrated care passes. Do not advance by elapsed time alone. |
| 6:00–8:00 | Final acceptance of the best qualified candidate, or bounded diagnosis/report if none qualifies. |

Start AdamW at learning rate `3e-4`, beta1 `0.9`, beta2 `0.999`, epsilon `1e-8`,
weight decay `1e-4`, global gradient clipping `1.0`. Accumulate gradients before
clipping and applying AdamW; use one update per effective batch. Train relevant
recurrent/action routes, then memory/cognitive routes as retention begins. Frozen
speech and unrelated weights remain bit-identical.

Evaluate a fixed development panel of 8 complete episodes every 30 minutes and
at phase transitions. Require 7/8 with no category wholly failing to advance
basic/integrated care. Preserve earlier care categories in later panels. These
small counts are engineering gates, not statistical guarantees.

Make one adjustment at a time, checkpoint first, and compare on the same
development panel. Confirm apparent improvement on one fresh 8-episode panel
before retaining an algorithmic adjustment; keep final acceptance seeds untouched.
Use the paired-control procedure below, not a comparison of a newly trained
candidate against its untrained starting checkpoint.

- Throughput: profile world work, packing, transfer, dispatch, gradients,
  synchronization, and evaluation. Increase worlds 8→16→32 only if aggregate
  completed-world-tick throughput improves by at least 10% within memory limits.
  Back off 8→4→2 if needed. More simulated ticks do not prove better learning.
- Batch size: test 2→4→8 physical sequences while retaining effective batch 8;
  stop at the first gain below 10% or a resource violation. Prefer fewer CPU
  workers; test a third only when CPU preparation demonstrably starves the GPU.
- Avoidable GPU cost: index decoder ranges, reuse packed buffers, omit frozen
  speech work, and measure post-update diagnostic loss every 32 updates rather
  than doing a second forward pass every update. Keep fault status checks.
- Repeated behavioral failure: dedicate half of new episodes to that category;
  retain earlier-skill replay. Inspect perception → selection → motor receipt →
  biology before assuming more optimization is needed.
- Regression/instability: restore the best complete checkpoint and halve the
  learning rate, at most twice. Nonfinite state requires diagnosis before retry.
- Delayed dependency: compare a 1024-tick window against 512 from the same
  checkpoint with matched episode data and equal wall-time budgets, at most
  15 minutes each. Retain it only for better behavior with no lost care skill.
- Stall: three development panels without improvement trigger one diagnosis
  and a corrective round of at most 30 minutes. If still stalled, stop and report.

Checkpoint latest and best states at least every 15 minutes at sealed boundaries.
Best means highest development task success, then fewer harmful contacts, then
lower time-to-success; training loss breaks no behavioral tie. Necessary code
repairs stop the worker, version the checkpoint relationship, and rerun only
affected checks. The eight-hour deadline is never silently extended.

## Autoresearch adaptation: bounded experiments with a protected evaluator

Reviewed Karpathy's [autoresearch instructions](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/program.md),
[overview](https://github.com/karpathy/autoresearch/blob/228791fb499afffb54b46200aca536f79142f117/README.md),
and training/evaluation code at commit
`228791fb499afffb54b46200aca536f79142f117` (26 March 2026).
It uses a small editable training surface, fixed-duration experiments, a protected
validation metric, a result ledger, and keep/discard decisions that favor useful
improvements and simplicity. Its GPT trainer, language-model metric, H100-oriented
defaults, and indefinite run instructions are not applicable defaults for A-Life.

Adopt the experiment discipline through this existing regimen and runner; do not
install another training framework or copy its transformer optimizer into WGSL.
The Codex supervisor proposes experiments. The creature's internal SLM remains
off and does not judge experiment success.

### Protected and editable surfaces

Freeze and hash the development evaluator, success definitions, physics/biology,
production action semantics, foundation capacity, and acceptance split at campaign
start. Tuning cannot weaken those to improve a score. An actual evaluator bug
ends that comparison: repair it, invalidate affected scores, and rebaseline both
arms. Never silently compare measurements from different evaluators.

Permitted tuning: learning rate/schedule, replay mix, updates per collected sample,
supported trainable masks, gradient/burn-in lengths, effective/physical batch
sizes, world concurrency, and equivalent training-kernel optimizations. Change
one factor per hypothesis. Do not change neuron count, remove biological costs,
disable deployed learning, shorten evaluation lives, inject oracle features, or
redefine consumption/recovery. Correctness repairs are prerequisites or explicit
interruptions, not score-improving experimental shortcuts.

### A comparison that cannot win merely by training longer

1. Record a concrete hypothesis tied to observed failure or profiling evidence.
   Snapshot the incumbent's full checkpoint, replay data, seeds, config, and
   evaluator version. Never start each experiment from a random new founder.
2. Create a control continuation and one variant from that identical checkpoint.
   Each gets 10 minutes of active work, counting world collection, data preparation,
   GPU work, synchronization, and recurring diagnostic overhead. Use 15 minutes
   per arm for the long-window/retention comparison. One-time build/warmup and
   evaluation are separately logged and still consume the global budget.
3. For optimizer/window experiments use the same frozen replay snapshot and
   sampling seeds. For collection/replay-policy experiments hold world-family
   distributions and RNG seeds fixed, allow policy-driven trajectories to differ,
   and count collection in both arms' time budget. Disclose which comparison was
   run. Alternate control/variant execution order between experiments.
4. Evaluate both resulting checkpoints on the same 8 complete development
   episodes with the teacher/SLM absent. Rank by task success, then harmful
   contacts, then capped time-to-success (timeouts receive the cap, never disappear
   from averages). Require no regression in any established care category.
5. A behavioral improvement is provisional until both arms are evaluated on the
   same fresh 8-episode confirmation panel. Retain only if the advantage survives
   without a category regression. On a tie, retain the incumbent method unless
   the variant delivers at least 10% higher valid throughput or a clear code
   simplification with unchanged numerical and behavioral results. Never select
   by training loss alone. Ambiguous differences remain inconclusive.
6. Keep the winning continuation's complete checkpoint. If the variant loses,
   continue from the trained control, not the older parent. Record keep, discard,
   crash, or inconclusive and the reason. No destructive reset of the shared tree;
   discard a variant in an isolated checkout or revert only its owned change.

For changes about long-life behavior, the comparison must also include a matched
development retention/reversal scenario from the current phase. Short care
scores cannot establish a memory improvement. If this cannot complete within
the remaining tuning budget, classify it unproven and continue the incumbent;
do not substitute a shorter test. Final acceptance remains a separate one-time
evaluation and never supplies training examples or tuning scores.

Run at most three paired hypotheses and stop earlier at the 90-minute tuning
cap, allowing for confirmation and the required final acceptance reserve. Reserve
enough time for both arms and evaluation before launching a comparison. A crash
consumes its actual time and permits one obvious repair retry within the same
cap. Do not grow a general parameter sweep around the three comparisons.

Choose hypotheses from the highest observed bottleneck: first batching/data
movement when utilization is poor; otherwise the missing behavioral category;
then temporal context/replay balance when retention is the demonstrated problem.
Microbenchmarks inside the initial pilot filter performance-only candidates;
they do not count as behavioral wins or justify extra tests.

Append one compact record per arm and decision under the run's ignored artifact
directory. Reuse the existing run receipts; a TSV/JSONL ledger suffices. Record
experiment/parent IDs, code and evaluator hashes, checkpoint/config/replay hashes,
seeds, hypothesis, duration breakdown, optimizer steps, valid world ticks, peak
RAM/VRAM, per-category outcomes, harmful contacts, time-to-success, and decision.
These receipts make unsuccessful experiments reusable knowledge without creating
another persistent simulation subsystem.

## Smallest meaningful verification and delivery

Extend existing focused checks rather than introducing a test framework:

1. Numerical training contract: real production/training forward agreement,
   sampled GPU finite-difference gradients for each enabled weight group,
   window continuity, frozen masks, and batched-world/state isolation. Probe
   gradients away from clipping/sign/nondifferentiable boundaries. Use mixed
   absolute/relative tolerances appropriate to f32; investigate discrepancies
   rather than widening tolerances to pass.
2. Resume/export contract: the next update and sample selection survive a full
   checkpoint roundtrip; failed updates cannot partially commit; candidate
   export/load preserves identity and rejects corrupt/incompatible assets.
3. One combined behavioral campaign, with no teacher/SLM attached:
   - 16 unseen episodes, four each for feeding, obstacle navigation, hazard
     avoidance, and recovery; require 14/16 overall and 3/4 in each category.
     Each is capped at two simulated minutes. Score actual completed outcomes.
   - Two counterbalanced consequence-reversal scenarios. Require the beneficial
     choice on at least 3/4 final probes in each, retention after natural sleep,
     and retained ability after a fresh-process save/reload. Match probe need
     states and positions so changed hunger or spatial preference cannot pass
     as learned object preference. If attribution is ambiguous, use one matched
     learning-disabled control; do not start a broad ablation campaign.
   - One 30-minute production session at normal 1x: first 20 minutes without
     food, then reachable food. Require survival and subsequent autonomous
     consumption with canonical biological benefit. The 20 minutes are an
     observation target, never a programmed grace period or starvation deadline.

Natural sleep must occur and finish in the retention observation; otherwise that
criterion remains unproven. Capture visible gameplay with passive video and
existing channel traces. Distinguish material energy from HUD vigor. No stress
benchmark runs concurrently with the normal-speed session.

Age-only death may be disabled with the existing flag. All other biology stays
active. Headless training may run faster than wall time while preserving the
ordinary timestep, motor costs, and learning/sleep rules. Do not change metabolism
to make training complete faster. Missing readiness means a retained experimental
checkpoint and a concrete blocker, not promotion.

Deliver the explicit candidate asset/digest, run configuration, compact behavioral
and performance receipts, video, and limitations. Keep large artifacts outside
Git. Do not switch the game's default founder until separately selected for that
purpose. Ordinary births inherit only the genetic foundation, never a training
creature's personal memories or acquired lifetime state.

## Why this design and what remains uncertain

- [DAgger](https://proceedings.mlr.press/v15/ross11a.html) motivates collecting
  corrections on learner-induced states rather than relying only on demonstrations.
- [R2D2](https://openreview.net/pdf/387fb2fcee8f74c53cf707a9856f40c458f33933.pdf)
  motivates sequence replay, burn-in, and explicit handling of recurrent-state
  staleness. Its large distributed benchmarks do not predict this machine's speed.
- [Creatures Genetics Kit manual](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1838430/manuals/Genetics_Kit_Manual.pdf?t=1640163808)
  distinguishes inherited instinct training from subsequent experience. Borrow
  the principle of useful priors plus revisable lifetime learning, not an extra
  instinct controller or unverified claims of implementation equivalence.

The selected method is the best-fit first campaign given the audited code and
machine, not a measured global optimum. Longer windows and more parallel lives
cannot by themselves establish long-delay credit or learning-to-learn. If the
care policy succeeds but adaptation fails, the next work targets the observed
memory/plasticity/credit failure; advanced RL or meta-training is then an explicit
follow-on design informed by those receipts, not an automatic overnight addition.

Requirement trace: AOA-FOUND-001 through AOA-FOUND-008; AOA-SLM-002/005/006;
AOA-TEACH-003/004/005/008. Offline supervision is permitted under section 27;
in-world teaching remains ordinary perception under section 29. This document
neither changes those boundaries nor claims the unexecuted gates have passed.
