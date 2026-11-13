# Detached positional decision readout and output weights

This extends the existing default-off `experimental-decision-context` CPU feature.
It does not enable a collector, recurrent replay, action selector, host/checkpoint
format, motor-channel layout, GPU execution or a game default. Existing R/S and
transport APIs remain unchanged. No acquired brain, optimizer history or save is
migrated. This represents one imperfect remembered location, not a learned world
map or a food-desirability model.

## Numerical representation and routing

`PositionHead2018` reads `[relative world X/32, Z/32, occupancy, confidence]`.
With `v=4*[x,z]`, `u=v/sqrt(1+dot(v,v))`, the hidden feature is
`tanh(occupancy*confidence*(gT*dot(u,translation)+gA*dot(u,angular)))`.
Its two inner gains and Adam history remain separate from the new two output
amplitudes in `PositionOutputHead2029`. For explicit Move/Translation candidates
use amplitude0 times the hidden feature; Look/Angular uses amplitude1; Other
routes neither amplitude and preserves the inherited zero's sign. The two
amplitudes are freely signed finite weights, not another global clipping cap.
Both layers have independent optimizer ages/moments and transactional updates.
Unit amplitudes recover the original hidden readout. Zero amplitudes preserve
base-score bits and retain live output gradients when the hidden feature is
nonzero. There is no recommendation for nonunit values.

The candidate kind is metadata, not inferred from a nonzero effect vector.
HoldGaze or a clamped Look can have zero effect and still be Angular.
`PositionOutputFrame2029::bind` and `validate_candidates` check it against the
same actual candidate and original24 physical features. Structural `validate`
alone cannot authenticate a candidate that is not among its arguments. The
inherited current/reference/source tags are kept intact; this detached layer
does not certify that a source was physically observed or processed. In
particular, a reference tick behind the current tick denotes a stale belief.
No host-side target coordinates, hidden object IDs or visibility/Ready gate
is added.

Effects are supplied by the caller. The experimental source used nominal
body-axis deltas for body turns (not full gaze with head yaw), gaze deltas for
head looks, and pre-Locomotion pose for angular effects. Locomotion can change
pose before an orientation command executes. These limitations are not silently
fixed by output amplification, and the library does not predict future pose.

## Gradients, validity and numerical state

The local derivatives include both output and both inner parameters, all four
source coordinates, translation/angular effects and base score. Effect adjoints
are ambient derivatives; only active-kind coordinates can vary under the typed
layout. Forward scores are checked against exact reconstruction before the
low-level inner backward calculation. The output forward tape is immutable and
its backward API rejects changes to either current weight vector. Optimizer
updates and typed cold roundtrips preserve unequal histories. Existing
experimental head identity is retained for the standalone inner model; the
output model has its own identity and cannot load old head bytes as itself.
This is not a full brain or host checkpoint.

Saturated or zero-scaled derivative paths skip unused overflowing products.
A stable projection fallback handles an overflowing unnormalized dot product
when the normalized derivative remains finite, preserving ordinary arithmetic
order otherwise. Genuinely nonfinite active outputs or derivatives fail rather
than clipping weights. No freeze of normal signed mutations is introduced.

## Work and evidence limits

The shared-source batch inner kernel reports4 source reads,9 scalar operations,
one square root, then7 reads,10 arithmetic operations and2 bound/merge units per
candidate. The output wrapper adds explicit kind read/dispatch and final merge
per candidate, plus one amplitude read/multiply for each Translation/Angular
candidate even when its amplitude is zero. Calling the single-candidate function
repeatedly prepares the source repeatedly; only the batch API shares that work.
These are logical forward-kernel units, not measured FLOPs, backward costs or a
biological debit. Validation, provenance/own-effect construction, serialization,
hashing and allocation are excluded. A future host must meter its complete path
before admission.

Frozen prototype evidence: unit amplitudes matched39,015 saved components/final
scores (171 preflight plus38,844 closed-body components), while zero outputs
preserved all38,844 body base scores. Native tests cover derivatives, signed zero,
independent Adam/cold state, explicit zero-effect kinds, malformed tapes and
finite extremes. This is numerical compatibility, not navigation improvement.
The separate same-state amplitude screen found that large gains can steal the
global representative from Ingest. Existing motor-channel reachability and
postmeal residual influence remain relevant. Stopping after eating is not by
itself harmful fixation. Amplification supplies neither learned desirability nor
a safe or effective behavioral recipe.

When the outer chain product overflows but local derivatives can still yield
finite results (including exact zeros), the new wrapper evaluates the analytic
chain in f64 before any intermediate adjoint is rounded to f32. Tiny local
adjoints must not underflow before amplification. Ordinary finite-chain ordering
is unchanged. The inner score's artificial base is fixed at literal zero, so
its exposed `inner.before_score` adjoint is zero; the actual base-score adjoint
is the separate `base` field. True nonfinite active derivatives still fail.
