# Fresh recovery-timescale V1: prospective body calibration

## Scope and frozen design

Base: accepted main `68beb760f61fddd9f838221f75b006748845d821`, tree
`e8b75aebef97d6f85b2cafb0ca72f170e4ea3008`. This is a source/body fixture study,
not a sampled life, neural-policy study, default-recipe change or deployment claim.
No saved life, private bank, trained weights or checkpoint is an input.

Prospective targets, declared before implementation:

- A fully expressed awake founder starting with zero fatigue reaches the existing
  M0 fatigue reference 0.35 in 200–400 biology ticks.
- The same founder reaches the existing M1 pressure reference 0.60 in 900–1300
  awake ticks, allowing 3–5 sleep/rebuild cycles before unchanged age death.
- Fatigue can be relieved by real stationary awake rest while pressure continues
  accumulating. Only real body sleep supplies the existing pressure-clearance
  event. Several fatigue/rest intervals fit within one pressure interval.
- Existing development, aging, material and energetic costs, biological value
  weights, motor references, body sleep and cognitive sleep semantics stay intact.

Candidate: change only two inherited Awake emitter gains, fatigue to 0.0012 and
sleep pressure to 0.0006 concentration units per biology tick at full expression.
Retain baseline 0.2, retention 0.99995, expression floor 0, receptor projection,
and all sleep/awake-rest emitter gains. This is a separately named fresh-genotype
constructor/helper, applied to both homologs before the organism is expressed.
It is not invoked by ordinary constructors or loading. No new schema/brain ABI.

These are engineering/gameplay targets, not measured mammalian physiology or a
claim to have replicated Creatures 3. The rates are round prospective choices
within the declared intervals, not fits to sampled actions. Fatigue remains an
awake-duration timer: actual work-based fatigue needs a separate mechanism and
study. Idle is not silently charged an invented exertion cost.

## Source and units audit

- `alife_core/src/biochemistry.rs`: early-mammal cadence is hormone 1, metabolism
  6, development 60, reproduction 120 ticks; catch-up bounded at 64 steps.
  `maximum_lifespan_ticks = round(1824 * (2 + lifespan_scale))`. Biological age
  is not multiplied by metabolic turnover. This founder has lifespan_scale
  0.475 and age death 4514. Its inherited locus bounds [0,1] span 3648–5472
  age ticks. Generic ContinuousLocus validation accepts other authored finite
  bounds, so this is not a universal legal-genotype lifespan limit.
- `alife_core/src/evolutionary_genetics.rs`: maturation duration is
  round(2400-1200*maturation_rate), here 1824. Puberty is round(1824*0.61)=1113.
  Biochemical gate 0.11 gives continuous full-expression age 200.64, sampled
  at the next development boundary 240. Birth expression and regulatory initial
  concentrations are zero. Genetic variation need not have this clock; within
  this founder family's inherited [0,1] bounds, maturation spans 1200–2400 and
  the activation gate spans [0,1]. Other explicitly authored locus bounds are
  not universally excluded by genotype validation.
- `alife_world/src/new_game.rs`: canonical founder turnover is inherited near
  1/400. The waking-recovery graph removes energy-deficit fatigue and direct
  SleepRecovery ATP injection. Reserve metabolism remains real.
- `alife_core/src/biochemical_graph.rs`: canonical fatigue species 2 and pressure
  species 18 both have baseline .2, retention .99995, Awake gain .00007,
  cadence 1, analogue noninverted response, threshold/floor 0. No reactions or
  neural emitters alter these two species in this founder. Awake source equals
  !body.sleeping. Decay occurs first, followed by emitters. At unit cadence,
  x[t+1] = r*x[t] + e[t]*(b*(1-r)+g), before clipping and recovery.
  Their brain-facing receptors have threshold 0/gain 1/nominal 0. Fatigue also
  reduces locomotor capacity through the inherited negative receptor.
- SleepRecovery subtracts .70 fatigue and .75 pressure per full-expression unit
  dose. AwakeRestRecoveryV1 subtracts .70 fatigue only. World dose and realized
  execution must be checked; selecting a name is not evidence of recovery.
- Chromosome traits modify regulatory retention by r^d, hormone production by
  a power-of-two factor, and fatigue receptor threshold by its threshold-locus
  offset from .69. Thus these exact numbers are one canonical founder, not a
  universal legal-genotype guarantee. Arbitrary legal graph genes permit wider
  ranges still. Unsupported fresh calibration shapes fail explicitly in the
  helper, not through global mutation rejection.

With actual f32 parameters promoted to f64, r=.9999499917030334,
b=.20000000298023224, g=.00007000000186963007. Granting full expression at birth
is an optimistic uninterrupted-awake bound:

x(t) <= (b + g/(1-r)) * (1-r^t).

It gives x(4514)<=.323276168, first F=.35 at 4938, first P=.60 at 9401.
Therefore neither reference can be reached naturally in this founder's default
life, even before allowing for maturation or recovery. Increasing lifespan or
disabling age death would mask rather than repair that mismatch.

The candidate predicts zero-start full-expression F=.35 at 292 and P=.60 at 1009.
The unchanged retention half-life is 13860.297 ticks. Keeping retention isolates
production-rate calibration: changing retention alone cannot quickly build a
zero-start concentration above baseline when the source is this small. Lowering
motor thresholds could expose tiny concentrations but would also change the
frozen neural comparison and would leave F/P production timescales identical.

Memory is separate: candidate recall low-value confidence uses
1/(1+age/16384), with a .05 floor, rather than exponential deletion; strong
consequences >=.5 retain full strength. At 4514, an unused weak record retains
about .784 confidence before capacity eviction. Episodic storage also has finite
capacity, so no universal tick-based retention guarantee follows. Sleep replay
is bounded by its request/phenotype and not triggered by changing these rates.
No memory/replay parameter is changed here.

## Fixed verification plan

Fixed world seed `0x0343_4D31_4658_0001`, the same domain-valid M1 fixture seed.
Body-only ordinary Nano512 fixtures use the same canonical physiology; a
Standard2048 construction comparison verifies relevant parameters, without
neural execution. Fixed horizon 4514 age ticks. No seed search, horizon tuning,
forced neural selection, age override or concentration patch.

1. Verify ordinary/awake-rest constructors remain unchanged, and candidate graph
   differs from awake-rest control only at the two gain values. Log genotype,
   asset, clock, source and expressed graph identities.
2. Run baseline and candidate uninterrupted natural waking buildup through age
   death; record first .35/.60 crossings and full-expression/development points.
3. From naturally accumulated states, apply matched real awake-rest, real Rest,
   and ordinary NO_POSTURE world commands. Label these actions scripted, not
   selected. Assert dose, body sleep, no pressure relief from awake rest, no
   nutrition/reward grants, honest energy/ATP, and exactly one biology advance.
4. Equal scripted schedule in both arms: through age 4400, awake-rest at 400,700,
   1450,1750,2450,2750,3450,3750 and body sleep at 1200,2200,3200,4200. This schedule
   is fixed prospectively from the target ranges, not adapted to observed actions.
   Require natural re-buildup and pressure>=.55 before each scripted sleep in
   the calibrated arm; expect baseline stays below references. Actions remain
   scripted tests of machinery, not autonomous recovery behavior.
5. Verify developmental boundaries 0/59/60/179/180/239/240 and age death 4513/4514;
   save/restore exactness before/after a cycle; no implicit state reinitialization;
   inheritance/recombination and legal mutation compatibility; unsupported
   calibration variants rejected only by the explicit construction helper.

Keep first failures in evidence. No remote write before final source/result
freeze and parent coordination. Build only after a native lane is released.

Method reference: https://rikiyat.github.io/blog/weight-space-programming/
Experiments 4 (targeted), 5 (whole), 6 (iterative) motivate explicit interventions and
iterative tests. They are not evidence for these biological parameters.

## Prospective second schedule (before its implementation/execution)

The original 1000-spaced script is retained exactly. Its >=.55 condition does
not establish four .60 crossings: the analytic zero-start crossing needs 1009
fully expressed awake ticks. Add a separate fixed schedule 1200/2300/3400/4500,
with awake rest 400/700/1450/1750/2550/2850/3650/3950. Keep the same seed, gains,
normal 4514 age-death horizon, baseline arm and scripts. Log actual upward .60
crossings independently and require one before each scripted sleep. This is a
new declared verification schedule, not replacement evidence or gain fitting.

Numerical qualification: the closed form above is an ideal-real recurrence with
f32 parameter values, not an exact f32-arithmetic upper bound. A source-equivalent
scalar check gives .323293716 at 4514 with full expression, about 1.75e-5 above
that closed form. A conservative four-f32-epsilon per-step rounding allowance
adds 4514*4*epsilon≈.002153; the resulting bound is still below .35. Tests retain
the measured f32 graph result and source precision rather than claiming exact
real arithmetic. The integer 4938/9401 crossings are ideal optimistic estimates.
