# Three bounded experiment blueprints — 2026-10-01

The most useful new distinction is **retaining a harmful experience versus
trusting a prediction about current conditions**. Re-reading a record provides
no new independent evidence. Age can increase uncertainty about whether an
object remains harmful; neither elapsed time nor eviction establishes safety.
Test that distinction within the current individual memory system before
adding storage policy. The other selected refinements make preparation
dependencies explicit and screen teaching dependence before a long campaign.

These are conditional hybrid plans, not implemented features or measured
improvements. The coordinator ran three fresh-context, tool-free domain
questioners, established the baselines before questioning, performed translation
and challenge loops, and supplied the selected mechanisms and scorecards.
This document adds source inspection and executable experiment criteria; it
does not claim another questioner round or reproduce the raw dialogue.

## Objective, scope, and assumptions

Advance stable, useful individual creatures toward 20 FPS with 50 individuals
on i7-3770K / 32 GB / RTX 3050 8 GB, improve memory without losing valuable
exceptions, and establish autonomous learning/transfer/retention evidence.
Preserve current cognition cadence, GPU neural authority, grounded teacher
channels, crash/ownership/learning checks, and exact continuity.

This publication is planning/source inspection only. It launches no runtime
benchmark, training, campaign, desktop control, or GPU workload. Campaign
approval remains **Blocked**; an experiment blueprint does not resolve it.
Missing actor checkpoints, hardware windows, or an exact integrated source
also block dependent execution. Cloud CPU fixtures cannot establish game FPS
or autonomous creature competence.

The [controlling architecture](architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md)
and [requirement registry](architecture/requirement_registry.csv) govern the
work: AOA-AUTH-002/005, TIME-001/003, MEM-001/002/005, LEARN-002,
FOUND-006/007, TEACH-004/005, PERF-003/005, PERSIST-002/004 and FAIL-001/002.
The [earlier 50-creature plan](performance/20261001-fifty-creature-optimization-plan.md)
and [revised skill](../.codex/skills/deep-planning/SKILL.md) remain unchanged.

## Source boundaries and existing mechanisms

Known published heads inspected on 2026-10-01:

- Memory: `cloud/memory-quality-20261001` at
  `33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e` (**M** below).
- Founder design: `codex/cloud-founder-integration-20261001` at
  `808ef1128744b3ba1758d3c7b8ad9f69e0b5ef4b` (**F** below).
- Planning: `cloud/deep-planning-20261001`, prior publication
  `a6d76d9014aab93167b2eeabcf66c7e5ffebc201`, based on M.

F is a separate branch and does not contain M's full memory-quality changes;
it is not the combined candidate. Pin the actual combined commit/tree,
phenotype/asset identities and executable before execution. Do not substitute
a similarly named integration branch or attribute one branch's tests to another.

| Integration point | Already present | New question/experiment |
| --- | --- | --- |
| M [staged preparation](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L580) | Baseline/routed recall and two complete finalizations; batched GPU dispatch | Single finalization was already planned; explicit dependency-bound preparation proposals refine its later concurrency step |
| M [world draft extraction](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_world/src/headless.rs#L2671) | A per-tick spatial batch index and mutable organism-specific tracking | Which preparation stages are pure, or have safely deferred independent effects? |
| M [recall/admission](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_core/src/memory.rs#L894) | Within-call exact feature reuse, meaningful-event admission, compatible merging, bounded capacity/eviction and owner fences | Matched 256-record stress cases before introducing any diversity allocation |
| M [confidence aggregation](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_core/src/memory/candidate_recall.rs#L433) and [retention](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_core/src/memory/candidate_recall.rs#L567) | Recall confidence uses retention factor; strong consequences retain full strength/priority; reads do not mutate records | Separate historical-event evidence, current prediction uncertainty and storage priority conceptually; test whether their existing coupling causes a concrete defect |
| M [corroboration](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_core/src/memory.rs#L1155) and [reversal](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_core/src/memory/candidate_recall.rs#L624) | A new compatible measured outcome can refresh retention; current-outcome selection supports reversal; corroboration does not amplify belief | Recall-only repetition, genuinely recovered targets, changed context and rare outliers under equal encounters |
| F [teacher-free collection/resume](https://github.com/BobertNeek/A-Life/blob/808ef1128744b3ba1758d3c7b8ad9f69e0b5ef4b/crates/alife_game_app/src/foundation_training_cycle.rs#L553) | Existing teacher-absent lesson layout collection, optimizer/value resume and export checks | Matched assistance/layout screens; the collection API is a training-cycle path, not a read-only evaluation command |
| F [teacher interface](https://github.com/BobertNeek/A-Life/blob/808ef1128744b3ba1758d3c7b8ad9f69e0b5ef4b/crates/alife_school/src/teacher.rs#L1) | Embodied perceptual events and separate teacher authority | Measure dependence on ordinary support without teacher-private leakage |
| M [retention/restore diagnostic](https://github.com/BobertNeek/A-Life/blob/33982d3988f9dd0ca319d7bf2d84996bd5ee2f7e/crates/alife_game_app/src/gpu_live_runtime/choice_retention_tests.rs#L253) | A focused acquisition/restore/sleep case with complete checkpoints | Broader useful-outcome/transfer screens; this ignored fixture is not a validated founder evaluation battery |

## Baselines and comparison judgments

Preserve the actual ordinary baselines: scaling profiles/removes known duplicate
work then uses bounded parallelism; memory retains the current 256-record
merge/recall/retention system and tests; learning uses causal teachers, short
PPO/export/resume pilots and held-out evaluation. None is assumed broken merely
to favor a lateral mechanism. The baseline's 256-record budget is the planned
experiment budget, not a claim that every production configuration uses it.

All scores below are the coordinator's **engineering prioritization judgments**,
1–5 with 5 favorable, including lower effort/overhead/maintenance. They are not
measured success probabilities, throughput predictions or weighted totals.
Timing benefit, behavioral benefit, calibrated confidence, worker scaling and
robustness remain **Unknown**. Choose the ordinary plan if a refinement adds cost
without clearing the measured defect.

| Required baseline metric | Scaling | Memory | Learning |
| --- | ---: | ---: | ---: |
| Correctness likelihood | 4 | 4 | 3 |
| Simplicity | 4 | 4 | 4 |
| Low implementation effort | 4 | 4 | 4 |
| Bottleneck resolution | 2 | 3 | 2 |
| Observability | 4 | 4 | 3 |
| Testability | 4 | 4 | 4 |
| Maintainability | 4 | 4 | 4 |
| Failure isolation | 4 | 4 | 4 |
| Low resource overhead | 4 | 4 | 3 |
| Reversibility | 5 | 5 | 5 |

Scaling: standard vs aggressive concurrency/GPU relocation vs staged hybrid.

| Required comparison metric | Standard | Novel | Hybrid |
| --- | ---: | ---: | ---: |
| Fewer total execution steps | 4 | 2 | 3 |
| Low implementation effort | 4 | 2 | 3 |
| Fewer moving parts | 4 | 2 | 3 |
| Fewer failure points | 4 | 2 | 3 |
| Bottleneck resolution | 2 | 4 | 4 |
| Observability | 4 | 3 | 4 |
| Testability | 4 | 2 | 4 |
| Reversibility | 5 | 3 | 4 |
| Low maintenance burden | 4 | 2 | 3 |
| Low resource overhead | 4 | 3 | 4 |
| Simplicity | 4 | 2 | 3 |

Memory: standard vs separate protected-evidence subsystem vs small experiment-first hybrid.

| Required comparison metric | Standard | Novel | Hybrid |
| --- | ---: | ---: | ---: |
| Fewer total execution steps | 4 | 2 | 4 |
| Low implementation effort | 4 | 2 | 4 |
| Fewer moving parts | 4 | 2 | 4 |
| Fewer failure points | 4 | 2 | 4 |
| Bottleneck resolution | 3 | 3 | 4 |
| Observability | 4 | 4 | 5 |
| Testability | 4 | 3 | 5 |
| Reversibility | 5 | 3 | 5 |
| Low maintenance burden | 4 | 2 | 4 |
| Low resource overhead | 4 | 2 | 4 |
| Simplicity | 4 | 2 | 4 |

Learning: standard vs large factorial campaign vs three-screen hybrid.

| Required comparison metric | Standard | Novel | Hybrid |
| --- | ---: | ---: | ---: |
| Fewer total execution steps | 4 | 1 | 4 |
| Low implementation effort | 4 | 1 | 4 |
| Fewer moving parts | 4 | 1 | 4 |
| Fewer failure points | 3 | 2 | 4 |
| Bottleneck resolution | 2 | 4 | 4 |
| Observability | 3 | 5 | 5 |
| Testability | 4 | 4 | 5 |
| Reversibility | 5 | 2 | 5 |
| Low maintenance burden | 4 | 2 | 4 |
| Low resource overhead | 3 | 1 | 4 |
| Simplicity | 4 | 2 | 4 |

## Scaling experiment: make dependencies explicit before concurrency

First remove the unused baseline finalized frame as described in the earlier
plan; retain its attention-required evidence and final routed recall. Profile
that change alone. Next identify immutable per-tick inputs and return per-creature
preparation proposals. Keep authoritative world conflicts resolved in stable
organism/action order. Do not parallelize the entire current mutable loop.

Bind each reusable artifact to its **actual dependencies**: organism/profile,
causal sequence and world tick, memory-bank generation, phenotype/graph epoch,
canonical body/biochemistry view, tracking identity, topology/predictor version,
and the relevant candidate/state features. This is a dependency list, not a new
serialized schema or a requirement to hash all state every tick. Validate input
construction once; at proposal commit, check the changed dependency versions
and mandatory ownership/finite/bounds guards instead of rebuilding a full plan
for exact equality. Untrusted public inputs still require their normal checks.

Defer hysteresis/tracking/prior side effects or prove they operate on independent
owners. Collect proposals deterministically; only then route/finalize the
production inputs at the chosen existing boundary. Keep the finalization count
at one. Reuse only unchanged channel results/artifacts: routing can change
candidate indexes, novelty, query features and digests, so do not reuse whole
baseline recall or finalized decision keys across it.

Future paired measurements: 8/16/32/50 actual independent awake residents,
fresh/populated personal memory, spread/crowded placement and shared-object
conflicts; same source/config/seed, p50/p95 plus tails, CPU/GPU/wait/transfer
cost and work per organism. Compare serial then 1/2/3 preparation workers only
after duplicate-work removal. Pass requires sustained current 20 Hz progressed
ticks, every due awake neural row, no growing lag and target frame performance.
No dummy residents, skipped neural work or scheduling slowdown qualifies.

Rendering may consume the last committed read-only snapshots for responsiveness;
record their age, FPS, cognitive TPS, due/completed rows and catch-up debt
separately. Existing batching and spatial indexing are retained. Reject a
concurrency step if copying, locking, stale input, conflict order or rendering
contention outweighs its preparation savings.

## Memory experiment: historical evidence, current prediction, retention

Use the current bank as control at **256 total records** and the same retrieval
caps and event opportunities for every arm. Record what was observed, when,
by which organism/action/context, and whether an outcome is genuinely new.
Conceptually separate confidence in the historical harm event, confidence that
the object is harmful now, and priority for retaining its detail. The current
record's confidence originates from decision confidence; it is not already a
calibrated historical-event probability. First diagnose its observable behavior,
rather than relabelling that scalar as a solved new mechanism.

Run equal-encounter streams with repetitive benign events, rare harmful
individual exceptions, fresh similar objects, changed body/context and objects
that genuinely recover under a measured world intervention. Include read-only
repetition and time-only aging with **no new physical outcome**, as well as
independent compatible and contradictory outcomes. Benign approach is not
evidence that ingestion is safe. Match harmful/safe encounter opportunities;
avoidance must not manufacture a favorable rate by suppressing exposure.

| Metric | Observable criterion |
| --- | --- |
| False generalization | Count safe predictions/choices for known harmful individuals separately from unseen similar targets, per eligible encounter; absent detail is uncertainty, not safety |
| Harm retention | Track whether rare harm remains retrievable after benign pressure/aging; report losses and harm exposures rather than only record presence |
| Reversal delay | Count new relevant measured safe outcomes before current prediction/behavior changes for a genuinely recovered target; distinguish permanent fear from justified caution |
| Evidence independence | Recall-only repetition must not change persisted confidence/value/retention; time-only aging or eviction must not add positive safety evidence |
| Budget/isolation | At most 256 records per owner, unchanged search bounds, CPU/work cost and unrelated-owner/category controls; stable save/load behavior |

Start with existing `candidate_memory_retrieval` cases for boring events,
individual poison/category boundaries, changed context, corroboration, fading,
saturated reversal and unrelated harm. These already address part of the
hypothesis. Extend a fixture only for the missing matched-budget counterexample
when execution is authorized. If existing eviction demonstrably loses valuable
outliers, test a small bounded diversity/exception allocation **inside the same
bank**, against the same 256 total budget and CPU envelope. Derive its size from
that result; do not assume 192/64 quotas, fixed revisit fractions, indefinite
protected warnings or a second memory subsystem. If there is no defect, retain
the ordinary implementation.

## Learning experiment: three small dependence/transfer screens

Use disposable evaluation clones from one sealed common checkpoint and a pinned
before/after learned-state pair. Preserve the original actor, optimizer/value
checkpoints, priors, personal memories and provenance. An exported weight-only
foundation is not an exact living-organism checkpoint. Use supported restore
and evaluated-state construction APIs; never edit JSON to fabricate body/tick/
identity agreement. If the intended matched-state comparison cannot be built
without changing locked bindings, mark it Blocked and narrow the design.

For each before/after variant, match starting physiology, body needs, development,
innate prior/phenotype, random seed, opportunities, elapsed simulation time since
training, and observation length. Document the teacher/support treatment, SLM
setting and any rescue action. Evaluate at the same elapsed time using separate
clones; do not run screens sequentially on one learner and confound order with
additional learning. Keep ordinary online learning policy fixed across arms;
report within-evaluation change separately from the starting learned-state effect.

| Screen | Conditions | Falsification signal |
| --- | --- | --- |
| 1. Familiar with usual support | Familiar arrangement and the ordinary bounded assistance; same support for both learned-state variants | No useful autonomous improvement even here, or favorable totals consist of assisted actions/rescues |
| 2. Familiar with minimum assistance | Same arrangement, remove avoidable teacher prompting/arrangement assistance; define any essential support before running | Gain disappears without prompts/support; narrowed suspicion is dependence, not automatically failed learning everywhere |
| 3. Changed arrangement with minimum assistance | Same minimum assistance, altered layout/target instance while preserving reachability and needs | Retained familiar performance fails to transfer; narrowed suspicion is representation/transfer rather than adding curriculum stages |

Measure actual independent consumption/care/navigation or the declared useful
task outcome, harmful outcomes, assistance/rescue/death counts and opportunity
denominators. On **all three screens**, apply equal delayed observation and the
same rest/fresh-process restore comparison; report delayed harm and retention.
Preserve causal stimulus→GPU decision→world outcome→learning receipts. Optimizer
finiteness, taught-action counts and training steps do not establish competence.

One matched pair can falsify an obvious dependence but cannot prove robustness.
Retain individual outcomes and exceptions; do not manufacture a go decision by
averaging failures. Mixed results mean hold or a cautiously bounded follow-up
targeting the identified confound. Only convincing screens justify proposing a
larger campaign, still subject to the unresolved approval. Do not invoke
`run_foundation_training_cycle_with_lesson` as if it were a harmless evaluator:
the existing path includes collection and training/export.

## Milestones, validation, and recovery

1. **Bind evidence before execution.** Pin the actual integrated tree and
   checkpoint, record missing inputs/approval, choose one explicit outcome and
   smallest counterexample per area. No source/runtime change in this planning
   task. Done: state/config comparisons are reviewable; no missing input is
   silently replaced.
2. **Scaling first seam.** Future owner changes only the prepared attention/
   single-finalization seam, uses existing memory/query and causal attention
   tests, then paired population receipts. Expected: identical semantic
   boundaries with less redundant preparation; if false, retain the old path.
   Concurrency follows only after dependency/side-effect inspection and timing.
3. **Memory screen before policy.** Future owner reuses/extends bounded fixtures
   and records the table's matched-opportunity outcomes. Expected: identify
   whether historical/current confidence coupling or eviction produces a defect.
   No defect means no new allocation or subsystem.
4. **Learning screens before campaign.** After explicit authorization and valid
   state inputs, evaluate the three disposable screens and delayed/rest/restore
   observations. Expected: a literal pass/failure/confound per condition, with
   a smallest next step. Failure does not authorize more training.

Future mechanism checks may reuse
`cargo test -p alife_core --test candidate_memory_retrieval --test candidate_memory_queries -- --test-threads=1`.
Use relevant existing topology/ownership/atomicity tests when those surfaces
actually change. GPU attention, persistence, behavior and target timing tests
require their authorized environment and exact features. No CPU neural shadow
or synthetic test substitutes for that proof.

Recover locally: distinguish missing-input, semantic, integrity and performance
failures; retain prior source/checkpoints; repair the smallest relevant defect;
rerun its focused check; abort partial commits and reject stale/foreign state.
Preserve pending eligibility resolution, sealed save boundaries and archive
before admission/retirement. Do not weaken guards or expand trials to hide a
failure. Device loss remains fail-stop.

## Decision/progress log and final receipt

| Area | Decision | Current progress / next evidence |
| --- | --- | --- |
| Scaling | Staged hybrid; ordinary known duplicate-work removal remains first | Source inspected; implementation/timing Unknown; dependency audit precedes concurrency |
| Memory | Experiment-first hybrid inside one bank | Existing protections identified; matched 256-record counterexamples unrun |
| Learning | Three-screen hybrid with delayed checks across all screens | Existing APIs identified; common actor state/approval and behavioral evidence remain Blocked/Unknown |
| Rejected | Broad GPU relocation, global cross-creature recall cache, frequent full-plan/state integrity hashing, permanent fear quotas, large trial framework | No measured justification; ordinary plans remain fallback |

Append only material source-bound results/deviations to this log and the existing
[roadmap](ROADMAP.md); do not add trial-management infrastructure. A final
execution receipt must identify source/tree/branches, checkpoint/assets,
config/seeds/assistance/body matching, actual work and opportunities, per-screen
outcomes and confounds, CPU/GPU/record budgets, validation and raw artifacts,
crash/save/owner checks, rejected hypotheses, limitations and the smallest next
action. At publication, this is a blueprint only; no proposed benefit is proven.

Publication checks: documentation assertions pass 77/77, local links resolve,
all supplied scorecard arrays match exactly, and whitespace checks pass.
Independent Sol 6.1 read-only source review found no P0/P1 findings. Earlier
planning/source files and the revised skill are unchanged; only this document
is added.
