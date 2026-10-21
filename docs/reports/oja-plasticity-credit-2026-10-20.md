# Causally gated production Oja stabilization — 2026-10-20

## Measured finding and final rule

The original waking rule normalized effective strength without local outcome
credit. The first PR correction gated that term by receptor credit magnitude,
but still changed a synapse with **exactly zero credited eligibility**.
The follow-up confirms that this reaches masked recurrent routes and unselected
decoder families, including nonzero acquired fast/lifetime state.

For genetic+lifetime .75, fast .2, alpha .25, post .5 and normalization .001,
100,000 zero-eligibility outcomes gave these executed WGSL results:

| Receptor event | Projected m | Original E | Receptor-only PR E | Final E |
|---|---:|---:|---:|---:|
| Novelty 1 | .051282052 | .25 | .58062446 | .8 |
| Frustration 1 | -.12820512 | .25 | .35900068 | .8 |
| Pain 1 | -.25641024 | .25 | .25 | .8 |

The final rule follows the same per-synapse signed credit in both terms:

~~~text
effective = genetic + lifetime + alpha*fast
q = projected_third_factor*credited_eligibility
delta_fast = learning_rate*alpha*q
           - normalization_rate*abs(q)*post*post*effective
next_fast = clamp(fast + delta_fast, fast_min, fast_max)
~~~

The implementation computes q first, as an f32 product, in
fast_plasticity_delta in closed_loop_plasticity.wgsl. Rates, alpha placement,
the effective-weight target, clamp, finite guards, bank transactions,
eligibility capture/decay, sleep and replay remain unchanged.

Zero m or zero credited eligibility gives an exact zero delta for valid finite
inputs. Existing bounds remain enforced: an already out-of-bounds fast value
can still be clamped. No trained history is reset or inferred backwards.
Commits 2da160fe and dde4fa4f retain historical evidence for the earlier
receptor-only law, not the final rule.

## Why this is a causal assignment defect here

Classical Oja can intentionally change an inactive incoming weight:
delta w = eta*(post*pre - post^2*w). That competition acts across inputs to
an active postsynaptic output. Zero pre alone is not a universal mathematical
reason to prohibit all homeostatic normalization.

The architecture also permits a bounded normalization/homeostatic term separate
from eligibility in its reference equation. It does not mandate that every
future normalization mechanism use q. This finding is specific to the current
sealed-outcome evaluator:

* Eligibility kernels distinguish current route participation, selected decoder
  family/input, and persisted causal traces.
* Plasticity visits all synapses and samples a neural target coordinate. It has
  no selected-family or activity-mask gate of its own.
* Receptor-wide novelty, frustration or pain could therefore counteract
  expressed genetic/lifetime strength on a decoder family with no local credit,
  merely because its associated neural coordinate remained active.
* This is not evidence of deliberate incoming-weight competition restricted to
  the active candidate output. It is an outcome-driven regularizer reaching
  connections excluded by that outcome's local assignment.

For the project's causal three-factor update, this unwanted reach is a credit
assignment leak. The final rule makes this event-driven stabilization follow
per-synapse evidence. Existing neuron homeostasis and separately scheduled sleep
remain. A future independent homeostat would need explicit semantics/evidence;
this change does not outlaw that design.

## Credit conditions, rates, and retained traces

Credit lanes are prediction residual, pain, homeostatic improvement,
frustration, novelty, social consequence, biochemical appetitive and aversive.
Projection divides the signed receptor dot product by receptor L1 norm, clamps
to [-1,1], and returns zero for an all-zero receptor.

General default weights are [.2,-1,1,-.5,.2,0,.5,-.5], L1 3.9.
Signed-choice action/cognitive/memory profiles omit prediction residual,
[0,-1,1,-.5,.2,0,.5,-.5], L1 3.7. Improvement/pain 1 project to
+/- .25641024 or +/- .27027026 respectively. Opposing lanes can cancel.
Biochemical appetitive/aversive lanes are gated by measured consequences in
OutcomeCreditPacket; baseline hormone tone alone is not reward.

Default compiler inventory (seed 42, age tick 0, maturation .35,
GroundedObjectSlotsV1), printed by the tests:

| Class/row | Count | alpha | eta | lambda |
|---|---:|---:|---:|---:|
| N512 decoder | 984 | .25 | .01 | .001 |
| N512 memory decoder | 96 | 1 | .01 | .001 |
| N512 enabled recurrent | 123 | .125 | .0005 | about .00005 |
| N512 fixed recurrent | 431 | .25 | 0 | 0 |
| N1024 decoder / memory | 984 / 96 | .25 / 1 | .01 | .001 |
| N1024 enabled / fixed recurrent | 431 / 1551 | .125 / .25 | .0005 / 0 | about .00005 / 0 |
| N2048 masked slow | 2048 | .125 | .00125 | .000125 |
| N2048 masked fast | 1024 | .125 | .005 | .0005 |
| N2048 unmasked slow | 11008 | .25 | .0025 | .00025 |
| N2048 unmasked fast | 10496 | .25 | .01 | .001 |
| N2048 decoder / memory | 4096 / 4096 | .25 / 1 | .01 | .001 |

Procedural recurrent receptor scale includes .1 times the projection mask.
The default TemporalPredictive-to-ActionPlanning mask is .5. N2048 uses section
slow/fast scales and masks. Decoder policies have inherited head/family scales.
Current critical periods multiply recurrent rates by 1+max(matching bias),
capped independently at inherited learning/replay/normalization ceilings.
Fixed zero rates stay zero; decoders bypass this transient multiplier.
Existing compiler-oracle transport tests cover these interactions.

Activity masks gate fresh coactivation, not old traces:

~~~text
enabled route: staged = clamp(reference_decay^elapsed*previous + pre*post,-1,1)
masked route:  staged = reference_decay^elapsed*previous
at outcome:    e = staged*reference_decay^(outcome_tick-origin_tick)
~~~

Default reference decay is .95 per .05-second simulation tick. An unselected
decoder adds no fresh family-specific eligibility but retains/decays old
eligibility. The final rule uses eligibility after outcome decay, rather than
a current-mask gate. Masked routes with persisted credit can still learn;
expired, canceled or absent traces cannot be revived into normalization by m.

The isolated complete-kernel test uses actual compiled arena metadata/CSR
indices, executes the unchanged recurrent/decoder eligibility kernels, then
calls the actual plasticity evaluator. It checks 70 combinations of no event,
novelty, pain, frustration and mixed credit against active/masked zero/tiny/
signed eligibility, unselected decoders, persisted traces, 20-tick delayed
trace reversals and 10,000-tick expired traces. Actual sampled post is asserted
to equal .5. Device-read rates include recurrent alpha .125, eta .0005,
lambda .000050000002 and decoder alpha .25, eta .01, lambda .001.

Unselected decoder novelty with zero eligibility changes fast .2 to .19998918
under the receptor-only control and leaves .2 in the final evaluator. Pain
changes .2 to .19994594 in that control and leaves .2 now. A masked recurrent
trace .4 stages to .38 and credits .32580248 after three outcome ticks; both
signs still update appropriately. Nonzero lifetime .25 stays unchanged.
These are isolated WGSL results, not production-backend hardware acceptance.

## Five-rule comparison and chosen tradeoff

Let K = eta*alpha/(lambda*post^2), with nonzero applicable factors.

| Rule | Normalization factor | Unclipped equilibrium |
|---|---|---|
| Original | 1 | K*m*e |
| First PR | abs(m) | sign(m)*K*e |
| Exact-nonzero eligibility gate | abs(m) if e!=0, else 0 | same as first PR when e!=0 |
| Common clock | abs(m*e), learning drive also multiplied by abs(e) | same as first PR when e!=0 |
| **Final** | **abs(m*e), unchanged linear learning drive** | **sign(m*e)*K** |

The exact-nonzero gate fixes literal zero but is discontinuous: arbitrarily
small nonzero traces authorize almost full receptor-only contraction.
The device comparison checks that it equals the first PR whenever e!=0.

The common-clock alternative is continuous and preserves the earlier
eligibility-dependent equilibrium, but changes the drive from m*e to
m*e*abs(e). A trace delayed by d ticks already includes decay^d; this drive
includes decay^(2*d). It weakens delayed/sparse learning a second time and can
push an otherwise measurable update below f32 coefficient precision.

The final rule interprets |q| as the amount of local evidence controlling speed.
Consistent weak evidence may accumulate toward a strong association; an older
or sparser trace does not itself instruct almost full-speed erasure of an
established inherited response. This is a game-learning choice, not a claim
that classical Oja's equilibrium is universally wrong.

The cost is explicit: equilibrium no longer scales with |e| or |m|. Persistent
biased tiny q can eventually reach a fast bound in real arithmetic. More
default constant-input cases therefore end at a clamp. This is bounded but
changes calibration and could overlearn persistent biased noise; no gameplay
benefit is claimed. Preserving the earlier equilibrium instead requires the
discontinuity, extra eligibility attenuation, or a differently designed
normalization policy. Causal continuity and linear delayed credit are preferred
for this unreleased runtime. Founder/rate calibration remains separate work.

Fast-only normalization is not used. It could be made stable with bounds but
changes the target by a genetic/lifetime-dependent offset and, if ungated,
still forgets fast state without local evidence. Signed q is not used on
normalization: that makes negative credit expansive before clipping.

## Tiny sustained noise and time to a bound

Executed f32 comparison: post=.5, genetic+lifetime=.75, initial fast=.2,
novelty m=.051282052, one million outcomes. These are controlled exposures,
not an observed founder activity distribution.

| Coefficient / initial E | e | Exact-nonzero gate E | Final E |
|---|---:|---:|---:|
| Ordinary decoder / .8 | 0 | .8 | .8 |
| Ordinary decoder / .8 | 1e-7 | .25 | .8 |
| Ordinary decoder / .8 | 1e-6 | .25 | .8 |
| Ordinary decoder / .8 | 1e-4 | .25 | .8037253 |
| Alpha-1 memory / .95 | 1e-7 | .00232857 | .95 |
| Alpha-1 memory / .95 | 1e-6 | .00236458 | .95 |
| Alpha-1 memory / .95 | 1e-4 | .00632453 | .9947035 |
| Procedural slow / .775 | 1e-6 | .71551454 | .775 |

The same exposure is tested at post .1 and 1. Zero m/e retention is exact.
Tiny positive credit can be below the ULP of a stored nonzero fast coefficient:
this measured f32 stagnation is not an epsilon threshold added to the rule.

For constant nonzero q, h=alpha*lambda*abs(q)*post^2 and
E_n=E_star+(E_0-E_star)*(1-h)^n before clipping. If E_star is above endpoint U,
the real-valued time to U is

~~~text
n = ceil(log((E_star-U)/(E_star-E_0)) / log1p(-h))
~~~

At e=1e-6, post=.5 and the table's novelty projection:

| Default coefficients | E_star | Upper | Real-valued outcomes to upper |
|---|---:|---:|---:|
| Ordinary decoder | 10 | 1.25 | about 15,646,732,796 |
| Alpha-1 memory | 40 | 2.75 | about 3,680,892,705 |
| Procedural slow | 5 | 1 | about 682,966,696,648 |

For ordinary decoder e=1e-7 / 1e-4 these become about 156.47 billion /
156.47 million outcomes. The exact-nonzero gate instead reaches lower endpoint
.25 in about 362,904 / 363,763 outcomes (362,912 at e=1e-6), despite positive
eligibility and novelty. Its magnitude does not tend to zero with the trace.
The huge final-rule counts are real-valued predictions, not claims f32 will
actually reach that bound: the measured tiny updates stagnate in this initial
state. This distinction matters for both learning capacity and noise.

## Fixed points, reversal, and stability

For constant q, before clipping:

~~~text
E_next = (1-h)*E + d
h = alpha*lambda*abs(q)*post^2
d = eta*alpha^2*q
E_star = sign(q)*eta*alpha/(lambda*post^2)
~~~

For validated alpha, rates and |q|<=1 with default bounded tanh activity,
0<=h<=1. The affine map is a nonnegative contraction for h>0. Clipping is
nonexpansive and selects an attainable endpoint when E_star lies outside the
fast interval. Negative outcome/eligibility changes the drive, not the
contraction sign. Zero post/normalization removes contraction; fast clipping
still bounds learning. Custom unbounded identity/ReLU activity can exceed
this regime; existing clamping and finite guards remain. No global convergence
theorem for the coupled recurrent organism is claimed.

Default eta=.01, lambda=.001, alpha=.25, post=.5 and e=.01 have E_star=+/-10:
with genetic+lifetime=0, sustained credit ends at attainable +/- .5.
The first PR instead approached +/- .1. This is an intentional difference.

Interior points are also tested: eta=.0001, lambda=.001, alpha=1, post=1,
e=+/- .4, m=+/-1 approach E_star=+/- .1. Executed results include +.10000931
and -.09999068 after 100k updates. Both signs of m/e, maximum contraction,
fixed rates, alpha 0/.125/.25/1, zero activity and both fast bounds are covered.

A controlled two-phase reversal starts at E=.05, reaches +.5 with m=1,e=.4,
then uses e=-.4*.95^20 for 10,000 further updates. Final E is -.40009335;
the inverse-sign experiment ends at +.40009335. Common-clock controls remain
at +.3342165 / -.3342165 over that second segment. Separate complete-kernel
cases verify that opposite coactivation reverses a persisted trace and the
actual outcome delay preserves its new sign.

CPU mirror vectors: eta=.01, lambda=.001, alpha=.25, e=.4, post=.5,
genetic+lifetime=.75, fast=.2 give fast deltas
0, +.0000092, -.0000108, +.00092, -.00108
for m=0,+.01,-.01,+1,-1. Device output includes raw deltas before clipping,
distinguishing sub-ULP drives from coefficient stagnation.

## Life, sleep, and preserved history

Updates run per committed outcome, not per recurrent microstep/wall frame.
Credited eligibility uses outcome chronology; there is no second elapsed-time
learning-rate multiplier. Final normalization now shares per-synapse temporal
exposure rather than outliving its trace.

Sleep replay still adds bounded replay_rate*alpha*replay_credit. Consolidation
still promotes staging_rate*replayed_fast into lifetime (clamped), and multiplies
fast by 1-fast_decay_rate. Defaults are .5, 4 and .5. Raw-fast promotion does
not generally conserve effective strength when alpha!=1. Earlier cancellation
can become actual lifetime change at sleep. This correction prevents new
uncredited cancellation; it does not reverse acquired history.

Alpha zero still suppresses waking fast expression/learning drive rather than
normalization itself. Eligible nonzero q can alter latent fast state and later
raw sleep promotion under that existing semantic. Born-fixed zero-rate rows
remain delta-disabled. This separate alpha/sleep behavior is disclosed, not
silently repaired by the causal-eligibility change.

The correction preserves accepted backups, learned payloads, Adam history, and
checkpoints. It changes future credit-gated updates without rewriting acquired
history or requiring a state migration.

## Validation and evidence limits

The focused suite executes 299 vector cases against five WGSL rules, including
million-step tiny-noise exposures. It validates full production WGSL, uses
closed-form oracles, asserts exact zero-local-credit retention, and prints
actual compiler/profile values. It also executes 70 complete eligibility/
evaluator kernel cases from compiled arenas. All nine focused tests passed on
SwiftShader Device (Subzero), CPU/Vulkan. This is isolated shader evidence;
production software-adapter rejection is unchanged.

One-step oracle tolerance is 2e-7; ordinary long recurrences use 8e-4.
The million-step noise group uses .02 to expose accumulated f32 rounding and
stagnation against real arithmetic; observed maximum deviation is about
.00652859 (ordinary-case maximum .00017703). Exact zero m/e retention, finite
bounds and exact-gate discontinuity
are asserted separately. Category maxima are printed.

Focused checks at the report revision: closed_loop_wgsl (16 passed), developmental_plasticity
(3 passed), strict backend all-target Clippy, formatting and the hardware
regression build. Physical production sealed-outcome execution remains Blocked
by NeuralBackendUnavailable on the report's cloud host. Production admission
was not bypassed.

Historical full workspace validation at the preceding law passed compilation,
1,941 tests across 216 targets (23 ignored), strict workspace Clippy, and core
boundaries. That broad run is not final-q-law evidence and was
not duplicated for this follow-up. New validation targets changed WGSL and
actual eligibility consumers. The earlier symbol-heavy build exhausted disk;
only session-created compiler artifacts were removed before the saved compact
cloud settings completed its remaining checks.

~~~sh
cargo test --locked -p alife_gpu_backend --test closed_loop_wgsl --test developmental_plasticity
VK_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json \
  cargo test --locked -p alife_gpu_backend --test oja_credit_stability -- --include-ignored --nocapture
cargo clippy --locked -p alife_gpu_backend --all-targets -- -D warnings
cargo test --locked -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity --no-run
~~~

Baseline: fetched main bda3b65be5fa6f01865b850640adf2e8d01f4990. This report
corrects the earlier classification of zero-eligibility normalization.
Gameplay improvement and empirical post-activity distributions on an accepted
trained founder remain Unknown; this local defect does not require a perfect
founder and does not establish global learning failure. The correction subsequently
merged through [PR #112](https://github.com/BobertNeek/A-Life/pull/112); this dated
report retains the measured results and limitations of its original source.
