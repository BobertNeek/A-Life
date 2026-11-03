# Conservative foundation consolidation

Keep immutable copies of both parents, their full actor/value Adam checkpoints,
all private life archives and seed histories. Run comparisons and candidate
training in separately owned output directories. Policy numbers and training
meal counts do not establish which parent is better. Never combine optimizer
moments or private memories by averaging weights.

## Inputs and native tools

Obtain the other owner's supported, stopped/sealed checkpoint copy and expected
hashes. Do not read a live mutable store or guess a checkpoint. Request its native
validation receipt, source/build identities, consumed/reserved/assessment history,
skill-domain evidence and actual common-ancestor chain. Verify copied bytes before
and after use. Do not normalize the accepted originals to make validation pass.

PR #145's `merge_founder_inference` validates original actor/asset pairs and the
compiled graph, actual trainable masks, frozen coordinates and curriculum/evaluation
versions. Only completed-stage counts are normalized in disposable inference views.
The ancestry JSON is pinned but not authenticated by that tool: authenticate both
actual chains to the supplied common parent separately. An old checkpoint found in
one archive is not proof that it is the common ancestor of the other branch.

After that audit, make the request with `parent`, `pc`, `cpu` and `ancestry_audit`.
Each source supplies exactly one `phenotype` or `compiler_inputs`, plus
`foundation` and `actor_checkpoint`. For a native CPU source, derive the separate
compiler-input JSON from its already validated `training-handoff.json`; do not
substitute the preceding collection-policy inputs.

```text
cargo build --release --locked --offline -p alife_training --features receipt-lineage --bin merge_founder_inference
merge_founder_inference REQUEST.json ABSENT_REVERT_OUTPUT --conflict-policy ancestor-revert
merge_founder_inference REQUEST.json ABSENT_MIDPOINT_OUTPUT --conflict-policy midpoint
merge_founder_inference REQUEST.json ABSENT_SOUP_OUTPUT --conflict-policy uniform-soup
merge_founder_inference REQUEST.json ABSENT_TIES_OUTPUT --conflict-policy ties20
```

All outputs are unpromoted, inference-only and non-resumable. Critic, Adam and
private life state are not merged. Numerical success is not skill retention.

The two new experimental modes retain the same graph/mask/frozen-coordinate and
original-actor admission guards. Uniform soup averages the two parameter values
equally on the existing sparse graph; it does not implement a pruning/retraining
pipeline. The TIES-style experiment computes each branch's deltas from the supplied
verified ancestor, retains the largest ceil(trainable-count/5) absolute deltas in
each branch (deterministic index tie-breaking), elects the sign of their sum, then
averages only retained nonzero deltas with that sign at scale1. Exact sign ties
retain the ancestor. These fixed parameters and the special tie rule are recorded
in the receipt; this is a conservative experiment, not a qualified reproduction of
published benchmark results. Unlike the older revert mode, an opposing delta is
not automatically erased when one retained direction dominates.

The method references are [model soups](https://arxiv.org/abs/2203.05482),
[sparse model soups](https://arxiv.org/abs/2306.16788),
[TIES-Merging](https://arxiv.org/abs/2306.01708) and
[Distral](https://papers.neurips.cc/paper_files/paper/2017/hash/0abdc563a06105aee3c6136871c9f4d1-Abstract.html).
Their reported results do not establish retention for recurrent embodied A-Life
policies. Test that empirically with both original domains and changed conditions.

`train_cpu_care_round evaluate --spec ASSESSMENT.json` evaluates an explicitly
selected native CPU source with its full frozen offline actor/value states. It
removes teacher/prior input and retains ordinary waking plasticity, world biology
and private recovery. It requires exact source-bound seed history. It does not
accept a bare merged founder asset: do not fabricate a CPU handoff or zero Adam
to force that admission.

`evaluate_cpu_founder` provides a separate founder-only native CPU path:

```text
cargo build --release --locked --offline -p alife_game_app --features cpu-training --bin evaluate_cpu_founder
evaluate_cpu_founder --spec REQUEST.json
evaluate_cpu_founder --verify OUTPUT
evaluate_cpu_founder --compare CONTROL_OUTPUT CANDIDATE_OUTPUT
```

The request selects `foundation`, `expected_foundation_sha256`, `seed_anchor`,
`seed_history`, `output`, `seed`, `decisions`, `temperature`, `lesson`, optional
`food_need_condition` and explicit `known_exclusions`. The seed anchor is an
unchanged admitted native CPU source, used only to authenticate the declared
exclusions and fix fresh-body construction. It is not the candidate's source,
critic or optimizer. Union both parents' known exclusions before held-out use.

The sealed `CpuFounderPolicy` interface gives the same canonical CPU host either
the existing full trainer or `CpuInferenceFounder`, which contains only the exact
asset, compiled phenotype and sparse graph. No candidate actor checkpoint, critic,
Adam moments or optimizer age is initialized. The same shared care scene/outcome
code and host perform perception, action sampling, legality, physiology, recurrence,
memory, waking plasticity and sleep. All lives start fresh; no personal memories
are combined. Outputs cannot be admitted as an offline training source.

The evaluator binds the existing fresh-founder world comparison projection before
runtime admission, prepared physiology, sampling/dose and the actual first neural
inputs. `--compare` verifies both seals and reports `matched`; callers must check
that field. Later private states are outcomes. Keep the exact pinned binary for
verification. Every completed life retains a native private checkpoint and is cold
restored and recaptured exactly during verification; an existing output is never
silently restarted. Failed/incomplete prefixes are preserved without qualification.
Receipt schema2 binds raw result/comparison files by SHA256, avoiding JSON float
reparse as a raw-evidence canonicalization scheme.

The qualifications include full-trainer/founder-only action and whole-private-state
parity, nonzero waking eligibility, sleep consolidation and subsequent waking,
cold restore and further steps, failed-state preservation and source integrity.
These establish CPU execution scope, not GPU parity or cross-domain skill retention.
The supported CPU care lessons have a bounded decision dose; absent domains and
long-horizon survival/continued-learning retention still need separate evidence.

For initial two-parent inference comparisons the PC owner can supply an immutable
minimal model export: canonical founder asset, original actor checkpoint,
phenotype/compiler inputs, expected hashes, native validation/build information,
actual ancestry and exclusions, and domain evidence. The owner's critic, complete
optimizers and private lives must remain sealed and intact; private life payloads
are not needed to compare fresh founder policies. A resumable winner handoff later
requires its full supported native training state, under its own authorization.

The existing GPU founder-only alternative is:

```text
ALIFE_SLM_PRIOR=off train_n2048_care --evaluate ABSENT_OUTPUT FOUNDER_ASSET LESSON DECISIONS WORLD_SEED FOUNDER_SEED --evaluation-sampling-temperature 1 --silent-request
```

It uses the GPU runtime and requires a qualified device/backend. The silent care
option covers held/reachable food and Feeding. `ALIFE_SLM_PRIOR=off` uses the
supported runtime configuration; verify recorded absence of prior delivery, since silence alone
does not disable a semantic prior. Fresh Feeding supports the explicit
`--food-need-condition ordinary_healthy_aging_v1`. Other domains need their own
supported, matched controls. Preserve every evaluation life and seal.

## Comparison order and forgetting checks

First compare both untouched parents on the same runtime, world/body construction,
sampling, decision budget and physiology. Cover each parent's demonstrated domains,
shared skills, and changed conditions. Cloud care alone does not cover PC language,
navigation, social or longer-horizon skills. Determine the PC's actual domains from
its evidence. Include healthy hunger, held/reachable food, distant/relocated food,
ordinary navigation, survival and safety where supported. Record outcomes and time
to outcome rather than only a scalar score.

Predeclare development cases separately from held-out cases before tuning. Exclude
the union of both parents' known training, reservation, interrupted and assessment
seeds. Preserve any unknown historical freshness; a new seed or changed scene is
not proof of globally unseen experience. Keep enough distinct seeds for the claimed
precision; repeated conditions on one seed do not add independent samples.

For fresh GPU care probes, use `compare_foundation_probes.py` with explicit original
asset and executable hashes and `--assessment-scope fresh_n2048_foundation_checkpoint_v1`.
It verifies native initial world/sensory/physiology controls despite stage-count
differences. Missing native identity fails closed. Its supported scope excludes
RelocatedFood and does not certify arbitrary restored private-life equivalence.
Do not present unmatched results from different backends as a causal comparison.

Evaluate both merge modes against both untouched parents, reporting every domain
and failure. Reject observed retention losses rather than canceling them with gains
elsewhere. `scripts/consolidation_retention.py` provides read-only analysis of
normalized, independently verified observations, strict paired losses, physiological
safety, predeclared coverage and a multiplicity-adjusted zero-loss bound. Its flags
are supplied by an upstream verifier; this script does not authenticate native
evidence or authorize adoption. Its tests are synthetic analysis tests.

Even a passing bounded suite leaves untested domains and long-horizon continued
learning. Validate canonical save/load, physiology, source/frozen parameters and
optimizer semantics separately. No adoption occurs automatically.

## Distillation and fallback

If blending loses either parent's strengths, prefer a disposable student initialized
from the better-supported parent's full native actor/value state. Keep both teacher
parents frozen. Existing recurrent imitation APIs (`cpu_imitation_objective`, PPO
auxiliary targets, `train_recurrent_imitation`) support command targets; they are
not a turnkey two-teacher consolidation CLI or a qualified soft-logit KL path.
Collect ordinary lawful behavior on both domains and preserve command/candidate
identity. Student labels belong in loss targets, not sensory input or world actions.
Balance rehearsal from both domains and limit updates/learning rate. Train the
student's own state; do not average teacher critics or Adam moments. Stop on
development regression, then test untouched holdouts and continued-learning
retention. This needs a reviewed replay/target adapter before a real training claim.

If no merged student preserves skills, report the parents' domain tradeoffs. Use
the stronger supported parent as the shared founder only when the evidence supports
that choice; no blanket winner follows from a policy count or one aggregate score.
Preserve the other parent and its unique skills for later rehearsal/distillation.
Transfer a winner through its native full-state export/import path. Its private
lives remain separate; adopting shared weights does not merge personal memories.

## Regular sharing

At each completed, sealed training batch, pin the new immutable checkpoint and
run the fixed development/retention suite. When two branches have advanced, audit
ancestry and evaluate disposable candidates again. Never extend a completed plan,
share a mutable checkpoint between writers or automatically install a regressing
candidate. Retain known seed exclusions in every subsequent history.

For ongoing sharing, prefer one authoritative optimizer trajectory and supported
collector/central-learner batches rather than repeated unqualified weight blending.
`train_cpu_care_round` implements a native CPU cooperative collection/central-update
boundary under one frozen source; it does not establish mixed CPU/GPU collector
compatibility. Qualify that boundary before using heterogeneous collectors. Live
private organisms keep their own recovery state. Persistent scheduling and owner
handoff must point at durable authorized artifacts, not an ephemeral worker path.
