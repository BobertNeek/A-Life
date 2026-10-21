---
name: deep-planning
description: Use when the user asks for a deep or lateral plan, or when a difficult design problem benefits from comparing a conventional approach with mechanisms from another field. Skip routine fixes.
---

# Deep planning

Start with the actual problem, source, constraints, and a conventional solution.
Identify the important bottleneck or uncertainty before proposing a new design.
Separate measurements from estimates and hypotheses. For performance work,
trace the critical path and distinguish latency, throughput, presentation, and
useful work.

When another field may offer a better mechanism, translate the unresolved
structure into that field: state, transformations, feedback, synchronization,
resource limits, and failure conditions. Preserve those constraints rather than
looking for a decorative analogy.

An independent domain questioner can help expose assumptions. Give that reviewer
the translated problem and ask for standard mechanisms, conditions where they
fail, useful refinements, and signs of excessive complexity. Fresh context can
reduce anchoring; describe the actual isolation available rather than claiming
more. Keep source inspection and the final decision with the main implementer.
This technique is optional, not a prerequisite for ordinary planning.

Translate useful suggestions back into concrete code or system changes. Check
whether they remove work, add capability, move cost elsewhere, or weaken state
integrity. Compare with the conventional approach. Keep the simpler baseline
when the alternative does not improve the actual problem. Use a small experiment
when evidence is needed to choose between alternatives.

Write a plan proportional to the task: intended behavior, affected components,
important decisions, implementation order, relevant validation, recovery, and
remaining uncertainty. Use existing tracking when useful. A plan needs no fixed
artifact set, scoring table, review tier, receipt schema, or separate process log.

Preserve working behavior, acquired state, and recovery. Choose checks that can
reveal a relevant failure; do not add tests merely to mirror implementation or
require broad research studies for a small gameplay change. Report measured
results and limitations without turning estimates into performance claims.

This method was adapted from the user's May 2026 planning workflow and revised
for A-Life in October 2026. Earlier versions remain in Git history.
