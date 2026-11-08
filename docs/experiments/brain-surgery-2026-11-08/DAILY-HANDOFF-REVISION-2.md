# Daily brain-surgery handoff: revision 2, 8 November 2026

This is a chronological update to the [original handoff](DAILY-HANDOFF.md),
which remains byte-for-byte intact at commit
`71a916007e834b489f368899f389ecaac590dc38`. Its “v2 pending” wording describes
that earlier snapshot. V2 is now complete; its exact source/scalars and curated
technical account are in the [reader-v2 archive](reader-v2/README.md).
All earlier world-comparison and reader-v1 evidence stays unchanged.

The earlier historical-base availability limitation is now closed: the actual
Git commit `2da9dd323ad6f074556a851174a2edbc9e34e216`, tree
`e402bcb5d5fc034ac0244e78d49fa1a0da25ede3`, was recovered and checked.
Applying the archived v1 patch to its example, then the v2 patch to that result,
reconstructs every supplied source file exactly. This is read-only historical
source verification plus temporary patch application, not a native test rerun.
The earlier archive's availability note remains intact as historical context.

## What changed in the interpretation

Reader v2 ran 120 detached CPU replay rows: 12 unique contexts, two original-row
provenances and five gains {0, 1/16, 1/4, 1, 4}. No new physical dispatch,
world/host exposure, life or autonomous seed allocation occurred. Four selected
tests and scoped non-test example Clippy passed. This is not a new autonomous
behavioral comparison, learning result or GPU qualification.

Gains 1/16 and 1/4 preserve all baseline fixture choices; 1 and 4 reverse four
of twelve, including three constructed useful-handling losses. Thus overscaling
the prior is sufficient to explain the gain-4 fixture failures. **The earlier
interpretation that the route itself was necessarily goal-confounded was too
strong.** An unconditional memory prior can coexist with a stronger task score.
A new goal representation is not established as necessary. This correction
changes the interpretation, not the frozen v1 measurements or gain-4 rejection.

The verified input bounds are family value [-1,1] and confidence [0,1]. Eight
coefficients at gain 1/16 imply at most .5 inherited shift per candidate, but two
opposite operation shifts can change a pairwise margin by 1. Learned offsets
can invalidate these bounds, and unchanged argmax need not mean unchanged
sampling probabilities. No candidate is selected or promoted. The accepted
brain and profile-4 candidate status remain unchanged.

## Original research and the iteration it motivates

Keep [Sparks of Weight-Space Programming (MIT CSAIL / OASYS)](https://rikiyat.github.io/blog/weight-space-programming/)
in every daily handoff. Experiment 4 is targeted repair; Experiment 5 constructs
whole static game policies; Experiment 6 explicitly interleaves evaluation
feedback with successive weight edits. V1 → v2 is this kind of hypothesis
revision, not evidence that simply writing weights creates learning. Our
recurrent, embodied, plastic organism still needs evidence of useful behavior,
experience-dependent acquisition, retention and transfer.

## Biological-drive review now active

The requested next priority is to check that the creature's various biological
drives are wired correctly and that their weights make sense. The end-to-end
review and justified repairs are active; **there are no drive-review results in
this revision**. Do not mistake the memory-reader fixtures for drive coverage.

The review must inventory each drive and follow its actual path from body state
through sensed/encoded signals, chemistry and recurrent processing to action
scoring, physical consequences and feedback. For each path, examine sign,
normalization and weight scale, saturation, timing/latency and learning credit.
Check coupled/conflicting drives and observed behavior, rather than declaring a
coefficient sensible solely from its name or sign. Record concrete wiring or
scale defects separately from design hypotheses and intentional trade-offs;
repair only what the evidence supports, with focused regression checks.

## Concrete overnight and tomorrow continuation

1. Complete the biological-drive inventory and causal path inspection. Produce
   source-linked findings, measured scales/timings where available and focused
   tests for any suspected defect; implement and test justified repairs. Carry
   forward unresolved paths explicitly rather than calling the whole system wired.
2. Develop the passive shadow-scoring candidate on actual fresh states. Validate
   that observation leaves baseline actions, RNG, learning, recall and physiology
   unchanged. The proposal has no reserved seeds or executed lives. Look at all
   gains, actual margins, effective offsets, competing targets/operations and
   supported useful handling before proposing a physical comparison.
3. Continue the Experiment-6-style edit/evaluate/revise loop with concrete source
   and results, retaining failed hypotheses. Update the next handoff with drive
   findings and the shadow probe's real implementation/validation status, not
   another unqualified promise of improvement. Do not turn the .5 inherited
   bound or artificial unit margin into a promotion criterion.

Preserve the accepted baseline, acquired weights, recurrent/body/episodic state,
eligibility, optimizer moments and integer ages, save/checkpoint identities and
recovery copies. New interfaces/graphs/coefficients get new explicit identities;
existing lives are not silently rewritten. Keep CPU and GPU claims separate.
This revision archives completed evidence and current work; it performs no
native build, replay, exposure or publication.
