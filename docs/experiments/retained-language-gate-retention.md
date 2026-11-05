# Four-update witnessed speech-gate correction

The fixed correction produced no observed held-out behavioral benefit. On two
fresh seeds, Untouched, ZeroGradient4, PositiveOnly4 and Balanced4 had identical
action choices, motor bundles, ownership/consumption events, utterances, energy
changes and scored safety trajectories. This includes their paired pre/post-sleep
assessments. Real gradient and parameter changes did not establish improved
factual speech or language competence.

[Measured receipts](retained-language-gate-retention-results.json) contain every
arm, per-seed speech/physical audits, actual update records, cold checks,
experimental-policy provenance, preserved failures and artifact hashes. Complete
optimizer checkpoints, private states, traces and measured executables remain
local. No accepted brain or default asset was replaced or promoted.

## Fixed training and controls

All arms start with MixedGrounded4 from V24 at global optimizer step 436, retaining
its full weights, moments, coordinate ages and optimizer configuration. A new
experimental mask admits exactly 64 SpeechPayload synapses: the 32 inputs to each
presence/stop output. Every other weight, moment and integer age is frozen.
Changing the mask resets no numeric state. Untouched uses the same mask but
takes no optimizer step.

| Actual arm | Native Adam updates | Final global step | Gate-weight change L2 |
|---|---:|---:|---:|
| Untouched | 0 | 436 | 0 |
| ZeroGradient4 optimizer control | 4 | 440 | 0.012367 |
| PositiveOnly4 | 4 | 440 | 0.063959 |
| Balanced4 | 4 | 440 | 0.073369 |

The zero-gradient control measures inherited Adam momentum and weight decay.
PositiveOnly4 and Balanced4 receive the exact same 0.5 positive-report component
per update. Balanced4 adds 0.5 witnessed-absence mass. There are no extra shadow
Adam steps, changed learning rates, optimizer resets or subsequent training.
All 12 saved native update vectors/states remain unchanged after evaluation and
recovery. Their gradients and native Adam arithmetic were verified read-only,
including weights, moments and coordinate ages.

The positive corpus consists of three actually emitted, timely declarations
from seed 10309219: EatFood at step 4, GetFood and GetToy at step 1. Nine collection
decisions reproduce V24's complete native row hashes and actual emissions.
Every positive refers to a real earlier ownership or food-consumption event.
The two absence examples reuse the actual premature EatFood/GetFood declarations
at step 1 of seed 16309347 from V25. Neither object was held, and no prior or current
physical event supported those declarations. Other missing targets remain
unlabeled; the valid GetToy declaration in that same world is not a negative.

Native positive speech loss normalizes 18 payload outputs; silence normalizes
two gates. The gate-only objective therefore multiplies positive native gradients
by 9 before masking, so each gate class has the same two-output normalization.
Analytic direct-gate derivatives match native BPTT before any update. The initial
balanced gate-gradient L2 is 16.490188. This fixed loss definition and the positive
component are shared across controls; gradient norms are not artificially balanced.

## Independent matched assessment

Predeclared seeds 19310729 and 19508423 are absent from 56 earlier seed receipts and
exclude all teaching/diagnostic seeds. They use familiar fixtures 2/4 with new
object identity padding 1833/1874. There are two independent seed units, not four
after counting sleep phases. Each arm receives the same saved world/private/RNG
starting states, apart from its explicitly recorded policy intervention.

Each seed has EatFood, GetFood, GetToy and silent episodes before/after induced
native RecoveryProtocol sleep: 24 decisions per episode, Player cues at 0/2/4/6,
then quiet. All 192 scheduled deliveries across arms reach both expected native
heard-token slots. No teacher, prior, forced action or new SLM enters evaluation.
Native biology, somatic learning, memory and sleep consolidation remain active.

Every row below is identical in all four arms, separately in each sleep phase:

| Outcome per phase | Result |
|---|---:|
| Correct latest-activity reports within 3 ticks | 4 |
| Premature taught-activity declarations | 1 |
| Total actual utterances / scored decisions | 8 / 192 |
| Utterances in silent episodes | 2 |
| EatFood trials nourished within 8 ticks | 2 / 2 |
| GetFood trials consuming food within 24 ticks | 1 / 2 |
| Physical noun pairs within 8 ticks | 1 / 2 |
| Physical verb pairs, 8-tick and strict 24-tick criteria | 1 / 2 |
| Joint physical/latest-report noun pairs | 0 / 2 |
| Joint physical/latest-report verb pairs | 1 / 2 |

All scored ticks survive with health 1, injury/pain 0; minimum energy is 0.787883997.
First spoken noun and verb phrase distinctions are 2/2 each, already present in
Untouched. They coexist with the premature report and do not establish new learning.

The primary timing criterion reports the latest actual activity within three
ticks, matching the training protocol. A secondary criterion permits a recent
matching completion despite an intervening activity: this counts 5 requested
reports in every arm/phase. GetFood after Get-then-Eat is a recent historical
report, not the clean premature example. The remaining two utterances use
untaught/status vocabulary; they are reported separately rather than labeled false.
There are no incomplete taught-word declarations in this assessment.

## Explicit policy intervention and exact recovery

The first evaluation correctly stopped at the living-policy guard after
Untouched's 384 decisions. The updated actors and all four updates per trained
arm had already been saved. Subsequent checks exposed the separate requirement
in native advance/credit that ordinary founder weights equal compiled genetic
weights. Two failed repair attempts are preserved; neither produced a trained-arm
evaluation row or another optimizer update.

The final path is an explicitly tagged offline simulator branch. It requires the
same experimental V3 phenotype and asset, the exact 64-coordinate target mask,
unchanged optimizer configuration and every non-gate weight/moment/age. Its policy
admission validates the compiled genetic anchor and actual branch weight digest,
and permits no non-gate base-parameter difference. Source/branch policy, actor,
snapshot and optimizer provenance are retained. The source host is immutable.

Only the policy-binding digest and explicit admission metadata differ at the
fork. World, body, neural/private history, memory, topology and RNG compare exactly.
The native forward and credit formulas use the actual branch parameters. Normal
advance/credit and admission retain their genetic-weight checks; non-gate changes
are rejected. Ordinary checkpoints omit the optional label, preserving their old
serialization. This is a counterfactual policy intervention, not exact continuation
of the prior policy, a biological genotype rewrite or accepted-life migration.

The completed Untouched hot trace is reused without another rollout. All full
hot/cold traces, global optimizer states and next-gradient hashes match. Trained
arms additionally have exact hot/cold final-private hashes. Untouched's old hot
final-private summaries were not written before the first failure; its cold full
trace matches the original and all 16 cold final-private states load exactly in
the parent process. No extra Adam step is used to test recovery.

There are 1536 unique hot scored decisions plus 1536 cold repetitions, not extra
independent samples. Original training/Untouched runtime is 29.024 seconds; the
successful evaluation-only resume takes 170.139 wall seconds. Failed repair runs
take 5.240 and 4.996 seconds. Build/repair time is separate. All 315 protected prior
files, including original actors and failure artifacts, remain byte-identical.
The explicit test, strict CPU library/test Clippy and formatting pass.

Training source is `ec05c212e04418d1a818e38afae9f4dafc650ad7`; successful evaluation
source is `fcdd38c744fb0fc816fa98f4a9ae799625b23030`, executable SHA256
`b63523552dde99d79cb2745a8b9b6e2f45420dbae7450cc53801885948daa99e`.
The source and all saved actors stay experimental. These two fresh seeds,
familiar layouts/words and induced sleep without an awake-delay control cannot
establish broader comprehension or natural sleep retention. No Luna 401 or denied
upload is retried.

The opt-in ignored test is
`foundation_training::retained_language::curriculum::factorial::balanced::witnessed_gate_retention::compare_four_gate_updates`.
It requires `ALIFE_GATE_SOURCE` (V24), `ALIFE_GATE_WITNESSES` (V25) and a new
`ALIFE_GATE_OUT`. Use test profile debug=0, incremental disabled and
`profile.test.package.alife_training.opt-level=2`. A partial run must resume from
saved actors with `ALIFE_GATE_RESUME_EVAL=1`, without repeating training. Tagged
traces preserve failed outputs. All raw artifacts in this comparison are retained.

#268 head `4350b88d` has three terminal-success workflows. Its Jules comment
[reports IN_PROGRESS](https://github.com/BobertNeek/A-Life/pull/268#issuecomment-6649203375),
so workflow success supplies no completed substantive review or approval.
