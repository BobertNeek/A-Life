# Prospective motor-coordination design

This source-only handoff records the motor-control-lobe proposal, the reasons for it, and the qualifications needed before trying it in A-Life. Start with [DESIGN.md](DESIGN.md).

## What is established

- In the closed 640-interval newborn CPU assay, 43 intervals selected Rest; 32 of those also selected object-directed Approach. All 32 share sampling counter 3 across matched conditions and are not independent trials. In 26 of them, no-locomotion already had the higher logit. The eleven stationary rests had no remaining object-directed locomotion competitor.
- The previous B penalty can initially excite Approach because its source-minus-reference neural contrast changes sign. Matched signal/reference circuitry is proposed to cancel common biochemical offsets; a negative readout coefficient alone is not proof of inhibition.
- Six original scalar/structural tests passed. After the mediation refinement, a subsequent verification ran `python -B -m unittest -v test_circuit_proposal.py`: **all nine tests passed**, including three mediation-construction tests. Both original logs are preserved. No tests were rerun for publication.
- The [developmental-context addendum](https://github.com/BobertNeek/A-Life/blob/599ffadd09dd28912c846e675ca73e3d7e05bf06/docs/experiments/ordinary-physiology-b-v1/DEVELOPMENTAL-CONTEXT-ADDENDUM.md) establishes that all 640 fatigue changes were decay only at zero biochemical expression. Canonical waking fatigue and Rest clearance were gated together. Fatigue was seeded at 0.1 or 0.65 rather than the ordinary newborn value zero. This limits the assay's physiology interpretation; it does not demonstrate an ordinary-world fatigue defect.

## Proposed variants

- **Direct supervision:** matched drive/reference and recurrent arbitration pairs add candidate-logit terms. It is parallel supervision, not full motor mediation.
- **M0:** controller outputs pass through 20 fresh motor-execution pairs before the decoder. It adds 96 capacity slots, 70 branches with 184 inputs, and 50 readout rows. Existing action-proposal pathways remain separate.
- **M1:** M0 plus individual transfer of all 128 old action-proposal neurons through fresh signed motor-execution pairs. It adds 352 capacity slots, 256 recurrent copy rows, and 8,754 decoder rows. Transfer distortion, latency, native cost and continued learning remain unqualified. Innate, memory, cognitive and speech paths are not all centralized by this proposal.

All variants require a new identity, typed reference encoder, capacity/admission and pricing. They are not a class 5 weight patch or admitted runtime. Paired parameters need explicit versioned fixed protection or symmetric plasticity; zero alpha or zero genetic weights do not freeze ordinary learning. Pain's separately admitted physical reflex remains outside voluntary recovery competition.

## Files and provenance

- `DESIGN.md`: publication-derived report, with original/published hashes in the manifest. Changes are public-facing wording, current test chronology, and the linked developmental interpretation.
- `circuit_proposal.py`, `test_circuit_proposal.py`, and all three circuit/mediation manifests: exact source bytes from the design package.
- `algebra-tests.log`: exact historical six-test result. `owner-algebra-tests.log`: exact later nine-test result; the filename is retained for provenance.
- `rest-rows-analysis.json`: exact 43-row scalar extract from the fresh closed assay. It contains case/arm identity, primitive IDs, probabilities and scalar outcomes, not learned state.
- `publication-manifest.json`: file identities, report derivation and scope. `SHA256SUMS`: SHA-256 closure for every published file except itself; the Git tree identifies the checksum file too.

The original raw 640-interval diagnostic, executables, bytecode, learned weights/banks, checkpoints and credentials are excluded. No native proposal implementation/build/replay, world/life execution, new study, pull request, merge or default promotion is part of this publication. Passing scalar/construction tests is not learned or behavioral success.

The inspiration remains [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), experiments 4, 5 and 6: iterative hypotheses, explicit controls and inspected circuit surgery, not a claim that those controllers are equivalent to A-Life's biochemical organism.
