# Detached Avoid truth table V2: passed

All 24 fixed fresh contexts completed: three source arms × hunger/fear/pain {0,.9}³, 48 source iterations each, 1,152 attempted and completed. Nine signed-cue decoder readouts per iteration. No biological simulation steps, learning or old state. No claim about action choice, toxin recognition or future behavior.

## Main result

The same unchanged source can favor positive chemical cues for Avoid cold and negative cues after warming. With hunger=fear=pain=0, positive-cue (+.85) Avoid is +1.992369 at iteration 1 but −6.675733 at iteration 48. The matched reference changes these to +0.000000954 and +0.000031114 respectively. The output-cut sham restores +1.992369 and −6.675754. Zero-cue Avoid is zero and opposite-cue scores are opposite, as asserted.

At iteration 48, hunger=0:

- Baseline low fear/pain: fear motor .0962632, pain motor .0962668, reference −.2964252, contrast .7853803.
- Candidate low fear/pain: fear motor .0962644, pain motor .0962681, reference .0962681, contrast −.00000366569.
- Baseline fear=pain=.9: fear motor .2031446, pain motor .2031463, reference −.2964252, contrast .9991413.
- Candidate fear=pain=.9: fear motor .2031454, pain motor .2031469, reference .0962685, contrast .2137554.

Candidate high fear only produces positive-cue Avoid −.908454; high pain only −.908436; both high −1.816921. Negative chemical cue produces corresponding positive Avoid. The need response is preserved while the tonic pedestal is removed: maximum baseline-versus-candidate difference in high-minus-low fear or pain contrast is 1.252e−6 across the recorded iterations and corners. The maximum Avoid contrast variation from hunger is 8.345e−7. The sham differs from baseline by at most 3.279e−6 in Avoid contrast.

The six-row repair also affects Ingest and Contact, by design. At iteration48 with zero fear/pain, positive-cue Ingest is baseline 3.584999 at zero hunger and 4.614964 at hunger .9; candidate 0 and 1.029956. Contact's zero-curiosity positive-cue pedestal changes from .667558 to −.00001831. These are removal of authored operating-point biases, not demonstrated gains in biological care. Existing prior ecological results already warned positive-cue consumption can become slower.

## Interpretation and limits

A blanket Avoid coefficient sign flip would be wrong: cold and warm baseline signs differ, while the matched reference preserves measured distress sensitivity. The correction is a native matched dynamical reference, not an urgency clamp or positive literal baseline. Small residuals reflect native network dynamics; do not call all references exactly zero.

Signed chemical[0] is a cue, not an intrinsic hazard label. This matrix cannot show whether a harmless negative chemical should be avoided, or whether high fear should flee a positive food target. It also leaves all unspecified raw lanes zero with one synthetic fixed receptor context. Wider receptor conditions and actual embodied tradeoffs remain unqualified here.

## Retained failure and resources

V1 compiled but failed during trainer construction, before any rows were durably emitted. It is retained unchanged and not counted as success. Its exact exposure cannot be recovered from its arm-unlabeled log: control flow permits 0, 384 or 768 completed source iterations. V2 confirms all baseline/repaired admissions and fixes the sham by compiling its own fresh phenotype, without weakening validation or using learned offsets. V2 logs each arm/context attempt and completion with fsync.

V2 compile: 1.734 s, peak process-group RSS 546,205,696 bytes. V2 assay: 34.467 s, peak 116,461,568 bytes. Allocation at assay completion 19,927,040 bytes including V1. Approximately 9.52 MiB growth in V2, within its new 32 MiB cap. No floor/timeout guard triggered. Native window explicitly released after completion.

Evidence: v2-rows.jsonl (durable actual rows); RESULTS-V2.json (scalar reductions and artifact hashes); V2-FREEZE.json; LINKAGE.json; compile-v2/assay-v2 logs and result records. Summary generation is read-only arithmetic on these completed source rows, not another native dose.
