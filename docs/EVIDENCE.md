# Evidence

## Principle

Evidence supports only the claim it causally measures. A green unit test supports the behavior it exercises. A preflight receipt proves preflight. A screenshot proves a rendered frame. None alone proves an end-to-end GPU organism loop.

## Reproducing a result

Record enough context to reproduce and interpret the result: the source and
configuration exercised, relevant inputs, command, outcome, and uncertainty.
For hardware measurements include the device, backend, workload, timings, and
memory use. For continuation or recovery, retain source package identities and
the state needed to detect corruption or missing acquired learning. Existing
codecs and logs can supply this; a new receipt schema or a documentation-authority
manifest is not required for every change.

Match the check to the claim. A save/load change needs restoration behavior;
a GPU change needs the relevant device path; a learning claim needs later
behavior rather than a counter or a successful optimizer update. Rendered and
player-control changes need observation through the actual application.

A result from an older source remains historical evidence for that source. Reuse
it when the relevant path and assumptions remain valid; rerun checks when a
change could affect the result. It does not automatically certify all of current
`HEAD`.

## Historical EI experiments

The retired EI0/EI1 report tools and their large self-test corpora are preserved
in Git history rather than the active repository. These reports describe their
original source; they are not promotion rules for current gameplay work:

- [EI0 lifecycle and GPU battery report](https://github.com/BobertNeek/A-Life/blob/168ff7867f7b761365a67cc5320187f0fc97a1ef/crates/alife_tools/reports/ei0_exit_gate_report.json):
  source `ab806c2c...`, eight founders, fourteen live organisms, three generations,
  128 post-restore ticks and tamper rejection. Two final genomes completed fifteen
  challenge worlds with 92 GPU dispatches each on RTX3050/Vulkan. It established a
  bounded lifecycle/archive/save result, not a complete autonomous game.
- [EI0 heuristic fixture](https://github.com/BobertNeek/A-Life/blob/168ff7867f7b761365a67cc5320187f0fc97a1ef/crates/alife_tools/reports/ei0_real_fixture_report.json):
  deterministic `HeuristicBaseline` tooling evidence, not learned GPU intelligence.
- [EI1 report](https://github.com/BobertNeek/A-Life/blob/168ff7867f7b761365a67cc5320187f0fc97a1ef/crates/alife_tools/reports/era1_promotion_report.json)
  and [causal sidecar](https://github.com/BobertNeek/A-Life/blob/168ff7867f7b761365a67cc5320187f0fc97a1ef/crates/alife_tools/reports/era1_trial_evidence.jsonl.zst):
  source `cb3c8dd...`, 2,640 descendant receipts and 52,800 GPU dispatches on
  RTX3050/Vulkan. Its verdict was `Blocked` by `NonPositiveIntact`,
  `ControlNotWorseForEveryCell`, and `ControlMarginBelowMinimum`; plateau was
  unknown. The count demonstrates measured work, not positive learning or a
  reason to impose the old campaign's rules on unrelated changes.

Historical reports may name documentation inputs from their original commits.
Those lists are reproducibility context, not authority over current work.

## Current care and training records

The [core game-loop plan](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/core-game-loop-plan-2026-09-20.md),
[training regimen](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/n2048-care-training-regimen-2026-09-21.md), and
[operator guide](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/n2048-training-operator.md) record bounded checks on their original source.
Teacher demonstrations, warm-up completion, and successful PPO handoffs do not
establish held-out learning, retention, or autonomous care. Raw `target/` paths in
those records refer to the originating machine and are not committed artifacts.

## Retained design and review context

- [October 1 CPU preparation measurements](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/performance/20261001-cpu-preparation.md):
  source-bound synthetic comparisons, retained samples and reproduction inputs.
- [October 1 cloud candidate validation](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/reviews/2026-10-01-cloud-validation.md):
  CPU/compiler results on the recorded source, earlier failures and unrun hardware checks.
- [Exact-population checkpoint decision](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/performance/exact-population-checkpoint-design.md):
  historical transaction and failure rationale.
- [Recovered review outcomes](https://github.com/BobertNeek/A-Life/blob/f49bc8d94b6f2733bb2fcaa59f10bd5267a079de/docs/reviews/2026-09-11-recovered-review-outcomes.md):
  closed campaign dispositions and two unresolved design questions; static review only.

## Current product checks

Use the current implementation and the behavior affected by the change to choose
validation. The old September gap list is historical, not a permanent set of
failed gates. Focus on the ordinary care and learning loop, truthful controls,
retained individual state, and performance for the population actually supported.
Separate a missing observation from a demonstrated defect and update status when
new observations resolve it.

## Honest failure states

- `Pass` means the named checks passed for the stated claim.
- `Blocked` means execution completed enough to measure a failing or missing prerequisite.
- `Unavailable` means required hardware or service could not execute.
- `Unknown` means the evidence does not measure the value.
- `Missed` means an expected observation was not produced.

Missing hardware is `Unavailable`, not fallback success.

Do not convert any of the last four states into a pass by imputation, relabelling, CPU substitution, or omission.
