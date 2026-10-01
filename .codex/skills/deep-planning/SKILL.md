---
name: deep-planning
description: Use for deep plan, /deep-plan, lateral planning, complex architecture, algorithm, system design, or research planning with interacting constraints or brittle conventional approaches. Build a source-grounded baseline, abstract the bottleneck, question it through an isolated external-domain expert, translate concrete mechanisms, compare alternatives, and produce a proportional execution plan. Do not use for routine coding, simple refactors, factual lookup, straightforward APIs, or tactical fixes.
---

# Deep Planning

Repository revision: 2026-10-01. Adapted from the user's May 22
`Complete_Deep_Planning_SKILL.md`; retain the original in Library.

Use independent domain questioning to expose overlooked mechanisms. Keep the
main agent responsible for the original problem, evidence, translation, and
decision. Return a literal, executable plan, never metaphorical advice.

## 1. Establish the baseline and evidence

Run when explicitly invoked or when the metadata's architectural criteria apply.
State the intended result and relevant assumptions. Read the applicable
instructions and enough actual source or evidence to identify the problem.
Respect the user's scope, existing plans, authorization, and safety boundaries.

Before lateral questioning, record the best ordinary plan:

- objective and observable success conditions;
- state ownership, inputs, outputs, constraints, and causal order;
- known bottlenecks, competing explanations, failure modes, and missing evidence;
- implementation effort, dependencies, validation, and rollback.

Separate **measured**, **source-supported**, **hypothetical**, and **Unknown**.
Bind measurements to source, configuration, hardware, workload, and units.
Do not add nested timers or incompatible runs, extrapolate synthetic speedups to
whole-system performance, or treat ordinal design scores as measurements.

For performance work, model the actual critical path before selecting a remedy.
Include serial work, scaling terms, waits, transfers, bandwidth, shared-resource
contention, and the target budget. Distinguish latency, throughput, presentation,
and useful work. State the proposed intervention, the cost it removes, what
replaces that cost, the expected new bottleneck, and an observation that would
falsify the benefit. Preserve the requested capability and cadence.

## 2. Abstract the unresolved structure

Strip source-domain nouns. Preserve state, transformations, feedback,
synchronization, resource limits, scaling, and terminal failures. Identify two
to four external fields with the same structural relationships. Select one
with a concrete standard mechanism, rather than a colorful analogy.

Translate the structure into a literal problem in that field. Preserve the
constraints, uncertainty, and success conditions; do not hint at another domain.
Keep the mapping table and baseline with the main agent.

## 3. Isolate the Domain Questioner

Spawn exactly one Domain Questioner with a fresh conversation and no inherited
turns, unless another domain is needed for a specific unresolved bottleneck.
Only this role requires blank project context. Do not fork the current chat.

Do not send the original prompt, original domain or terminology, repository
details, paths, names, prior attempts, baseline, mapping, or selection rationale.
Do not enumerate excluded source domains in its payload: that itself supplies
unnecessary source-domain cues. Send only the selected field, its literal
problem, and the payload below with the field substituted.

Disable tools and project access if the interface permits. Otherwise instruct
the questioner to use no tools and verify that no tools were used. External
reference is allowed only when the selected field specifically requires it;
then allow only that reference and document the exception. Never claim stronger
isolation than the interface provides.

```text
Role: Senior expert in [FIELD].
You receive only a literal problem in [FIELD]. Stay within that field; do not
infer or translate to another domain. Use no tools or external references.
Interrogate the design with sharp questions. Ask why established mechanisms
are absent. Prefer simpler standard mechanisms. If an objection is presented,
request its literal justification and attempt a local refinement before pivoting.
Return a concise design-review artifact:
1. Primary question.
2. Standard mechanism to try first and why it works.
3. Exact conditions under which it fails.
4. Local refinements.
5. Signs of excessive complexity.
6. A narrower field and exact bottleneck only if a pivot is necessary.
Do not provide private deliberation.
```

If fresh-context agents are unavailable, state that limitation. A main-agent
simulation is lower confidence and is never described as blank-context review.
Continue useful baseline analysis; do not let unavailable isolation block it.

## 4. Translate, challenge, and converge

For each returned mechanism record its structural function, literal equivalent,
concrete changes, benefit, new cost, failure conditions, and disposition. Accept
only implementable structures, algorithms, policies, experiments, or checks.
Reject decorative analogy.

Challenge each accepted mechanism against the actual source and constraints.
Ask whether it removes unnecessary work, increases capacity, or merely moves
cost. Check whether it depends on stale state, changes a capability, hides a
queue, or weakens an essential integrity or safety check.

Patch a promising mechanism in the same field when a concrete secondary issue
arises. Send only that domain-native issue to the same questioner. Continue
only while specificity, simplicity, or robustness improves. Stop after three
cascading patch levels, when complexity exceeds the baseline, when a hard
boundary appears, or when translation becomes unreliable. Pivot only the
unresolved slice. Do not force recursion or fill an arbitrary exploration quota.

Retain a broader alternative if the evidence has not distinguished competing
bottlenecks. Choose the smallest experiment that distinguishes them before
narrowing the implementation.

## 5. Compare standard, novel, and hybrid plans

Compare all three. Use measured budgets or bounded estimates where available.
State uncertainty, confidence, implementation cost, and reversibility. If useful,
score these dimensions once on a consistent 1–5 scale, with 5 favorable:

- correctness and failure isolation;
- effort, execution steps, moving parts, and failure points;
- bottleneck resolution and resource overhead;
- observability and testability;
- simplicity, maintenance burden, and reversibility.

Treat scores as design judgments. Do not total them into a pretend performance
prediction or add precision unsupported by evidence. Preserve the baseline when
it is better. Extract just the useful mechanism when a hybrid is simpler than
the full novel design.

## 6. Write the proportional execution plan

Honor the user's requested format and the repository's existing roadmap. Reuse
existing checks, receipts, and task tracking. Scale detail to the handoff risk;
do not manufacture a parallel contract, logging system, or validation framework.
Assume a competent implementer with the working tree and plan, but no chat history.

Always include, combining sections when concise:

- objective, assumptions, source/evidence boundary, and observable done criteria;
- baseline, selected literal mechanism, decision, and rejected alternatives;
- component boundaries, authoritative/transient state, causal data flow, and
  invariants relevant to the proposed change;
- ordered bounded milestones with affected files, dependencies, validation,
  expected outcome, and failure response;
- smallest first implementation or experiment with high information/value;
- validation mapping for the important success conditions and counterexamples;
- recovery: classify a failure, repair locally, rerun the relevant check, preserve
  atomicity, and roll back the change if assumptions fail;
- a place to record progress and material decisions, preferably existing tracking;
- final receipt: source/config, changes, checks/results, limitations, and next step.

Use observable failure or a meaningful regression before implementation when
appropriate. Do not mandate new tests for reversible documentation changes or
tests that simply mirror code. Retain crash, corruption, ownership, learning,
and other essential safety checks; qualify expensive exact diagnostic equality
separately from checks that protect production semantics.

For a long or high-risk autonomous handoff, expand milestones, failure modes,
validation, and receipt requirements enough to make decisions reviewable. Use
an execution contract or separate progress/decision tables only if they serve
that handoff; the headings themselves are not deliverables.

## 7. Remove bloat and verify the result

Remove all metaphor language from the executable plan. Include only literal
architecture, algorithms, state transitions, work packages, and evidence.
Give a cleaned trace only if requested, without private chain-of-thought.

Remove components that do not advance the objective, correctness, observability,
bounded work, recovery, maintainability, safety, or execution. Check that the
plan is clearer than the baseline and still works without this conversation.
If the lateral mechanism fails this check, return the baseline.

Do not change a skill solely because it names an older model. Evaluate the
workflow on real tasks, preserve useful methods and original source, and revise
only demonstrated friction. When this skill is revised, use skill-creator if
available and validate its frontmatter and a representative application.
