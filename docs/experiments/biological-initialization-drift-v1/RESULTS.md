# Fresh chemistry is not lived chemistry

**The passive drift is reproduced exactly by the inherited core equations. It
is not movement cost and does not show a 20x time conversion, reversed sign, or
duplicate advance. However, it is not merely a harmless first-tick blip: the
legacy reference breed sustains strong hunger/fatigue/ATP pressure at ordinary
body reserves.** No production biology, weights, ABI or acquired state changed.

One fixed opt-in test passed, with exactly **870 core advance calls**, 59,160
biochemical work units, **zero world intervals and zero neural emissions**.
The six earlier prescribed Approach intervals remain closed. This audit never
ran their test or used their marker. Fresh public-genome copies are calculation
controls, not a new world cohort. No retained life/checkpoint was read or written.

## Exact source and reproduction

- Base: `550f79ad1f765132c634e7a272da56b521b57ec0`.
- Frozen test/plan execution source: `bc0bd4a0cebfd8301fee0b9827fb8add729b14c1`.
- The source-only final patch removes one identity `f64::from(f64)` after strict
  Clippy reported it. That lint-only change has no numerical or execution effect.
  It was compiled/linted, not used to repeat the finite physiology run.
- Public seed: 921412; founder identity `(10,1,7,N512)`. No lineage recipe added.
- Full measurements: [outcomes.jsonl](outcomes.jsonl), 46 JSON records extracted
  from the preserved [run.log](run.log). Initial and final concentrations are
  reported only as the eleven public drive readings, not a serialized organism.
- Earlier physical evidence: `grounded-approach-need-v1-evidence.tar.gz`, SHA-256
  `500f3934f1a0b28ca0322952a716f7f02a4012693a6aa4f22d35f981c74c622e`.
  That experiment's RESULTS/FROZEN_PLAN, rather than its marker or test, were read.
- Accepted thermal source tree `449decc0e70fc694e498ae769c8429e23bdb858a` (local
  commit `c6c295588ebc2535aaa2e9eb25704926e92f28f3`, corresponding to accepted
  main `dd6501f801d95a0a86789d762f15c6930ded27fd`) has identical graph, genetics
  and chemistry files. Its biochemistry changes add candidate thermal wrappers
  and a body thermal branch; ordinary entry points preserve the legacy arithmetic.
  This is a source comparison, not an accepted-tree execution claim.

## Why the first interval moves so much

Initialization creates body reserve `0.7925000191` and chemical baselines. It does
not integrate preceding history or solve a body-conditioned equilibrium.
At age600: maturation `.3289473653`, expression1, puberty=false (threshold1113).
The eleven fresh readings are `.25,.2,.1,0,.15,.45,.75,0,.1,0,0`.

On tick600 to601, neural-support and circulatory organ upkeep lower reserve to
`.7920667529`. The 6-tick metabolic and 60-tick developmental boundaries are not
crossed; the fast chemical graph still advances once. All baseline-to-baseline
decay terms are initially zero. Persistent `EnergyDeficit = 1 - post_event_energy`
is `.2079332471`, and the three inherited emitters produce:

| Lane | Coefficient per tick | Passive raw change | Movement raw change | Movement minus passive |
|---|---:|---:|---:|---:|
| Hunger | +.18 | +.0374279916 | +.0387479961 | +.00132000446 |
| Fatigue | +.12 | +.0249519944 | +.0258319974 | +.00088000298 |
| ATP availability | -.12 | -.0249519944 | -.0258319974 | -.00088000298 |

Other eight lanes are unchanged in this ordinary-baseline comparison. Setting
reserve to1 for a graph-only step eliminates the entire drift exactly. That is
an intervention control, not a proposal to initialize all bodies fully fed.

The signed movement body event is -.04. Organ shares sum to1.1; reserve is their
six-organ mean, so the additional reserve change is `-.04 * 1.1 / 6`, measured
`-.00733339787` (turnover1). Multiplying its magnitude by .18/.12 reproduces the
additional drive changes. The prior physical receipt's .08 cost, this -.04 event,
organ reserve and ATP are different engine quantities. Treating them as equal
energy units would be a new assumption, not a conservation test.

No scalar reward, drift subtraction, normalization or synthetic neural provenance
was created. Raw changes remain raw. Between-control contrasts diagnose causes;
they are not alternative observations to store as a creature's experienced relief.

## Cold transient versus sustained setpoint

For fixed reserve E and expression1, these three chemical lanes follow
`c_next = clamp(b + .96 * (c - b) + g * (1-E), 0, 1)`.
The unclipped equilibrium is `b + g*(1-E)/.04`, not the bare baseline b.
The decay half-life is about16.98 ticks, or .849 seconds at the current20Hz.
There is no real-animal time calibration here.

At fixed reserve `.7925000191`, the limiting hunger/fatigue/ATP readings are
approximately **1 (clipped from1.18375), .8225, .1275**. The 128-step graph trace
reaches **1, .81915164, .13084921** with the body completely unchanged. The
separate body-integrating juvenile trace ends at reserve `.75826329`, with
**1, .90728128, .04271875**. The common drift decays; the resulting pressure does
not return to the fresh baseline.

A separate600-step newborn core-only development trace ends with exactly the
same developmental readiness as fresh age600, but reserve `.63872868` and
**hunger1, fatigue1, ATP0**. This includes no feeding/care, external surroundings,
neural work, action selection or world life. It is not evidence that an actual
creature necessarily reaches this state. It demonstrates that assigning age600
is not equivalent to advancing core physiology600 times.

Coefficient judgment: signs are coherent for the legacy energy-deficit circuit;
the relative gains are large beside .04 decay. Hunger saturates in steady state
below roughly83.3% reserve, fatigue below73.3%, ATP below75%. These are very strong
legacy couplings, not proof of appropriate calibration for a longer-lived breed.
The existing opt-in `with_waking_recovery` recipe explicitly removes the
energy-deficit fatigue emitter and weakens the ATP emitter to-.02. It is not
present in this fixture and was not activated or assessed by this run. Do not
silently attribute that recipe's expected behavior to this legacy founder.

## All eleven drive maps and event signs

Every active drive receptor in this exact founder is analogue, nominal0,
threshold0, gain1. Baselines below are full-expression values. Each corresponding
chemical has retention.96; all are regulatory except ATP, which is typed Material.
The raw receptor/source/emitter records are in outcomes.jsonl.

| ABI index / lane | Baseline | Inherited non-decay inputs | Audit interpretation |
|---|---:|---|---|
|0 Hunger|.25|energy deficit +.18; nutrition -.30|Nutrition .1 gives raw +.00330377 yet contrast -.03412423: useful relief can coexist with a positive raw interval change.|
|1 Fatigue|.20|energy deficit +.12; sleep -.70|Sleep .1 contrast -.07000001; legacy fatigue is tied to reserve, not independent time awake.|
|2 Fear|.10|damage +.35|Damage .1 contrast +.03500000; no dedicated threat-perception emitter in this reference graph.|
|3 Pain|0|damage +.80; sleep -.45|Damage .1 contrast +.08000001; zero-floor clipping hides sleep relief when pain starts0. Persistent injury distress is a separate opt-in recipe.|
|4 Loneliness|.15|social contact -.60|Contact .0125 contrast -.00749999; no-contact at baseline is0. The prior zero-pressure fork instead relaxed toward baseline by+.006.|
|5 Curiosity|.45|none in this founder|Play, novelty and investigation pulses each produce0 change/contrast. Their circuits require an inherited opt-in recipe; a mapped channel alone does not prove a functioning exploratory drive.|
|6 ATP availability|.75|energy deficit -.12; sleep +.30; nutrient conversion|Nutrition .1 contrast +.04274952; sleep .1 contrast +.03000003. Availability has the opposite desirability sign to deficit drives.|
|7 Temperature stress|0|persistent body thermal load +.30|Thermal event .1 first becomes body load .0495 after tolerance; drive contrast+.01485. This is the ordinary core path, not external thermal-world qualification.|
|8 Reproductive|.10|mating opportunity +.85|Opportunity .1 contrast+.085; juvenile reproduction still not ready. This is arousal, not a reward for maximizing it.|
|9 Extension0|0|no receptor or species assigned|Intentionally unused here. Opt-in play recipe assigns boredom; do not interpret zero as broken routing.|
|10 Extension1|0|no receptor or species assigned|Intentionally unused here.|

The ten pulse controls verify the named ordinary mappings and bounded signs at
these starting levels, not every nonlinear/saturated or environmental condition.
Player reward .1 has0 drive contrast but dopamine+.018. In this legacy profile,
hormonal tone is regulatory and all hormone value weights are0.

The separate `BiologicalValueProfile` uses -1/7 for hunger/fatigue/fear/pain/
loneliness/thermal change, +1/7 for ATP and body reserve, zero for curiosity,
reproductive and unassigned extensions; fresh injury has its own nonduplicated
coefficient1. These inherited valuation coefficients are distinct from chemical
emitter gains, sensory inputs and neural weights. Their signs make conceptual
sense; this fixed physiology audit does not establish optimal relative magnitudes
or learned grounded action choice. BIO-018 requires retaining that distinction.

## Cadence, units and separate limitations

- Graph time is integer simulation ticks. Decay uses elapsed ticks (bounded64);
  emitters use inherited cadence crossings (all1 here). Body organ cadences are
  1/6/12 ticks, metabolism6, development60, reproduction120. Body signals enter
  chemistry after the body update; no second endocrine simulation is invoked.
  The `fast_hormone_ticks` field does not gate this graph independently; emitter
  cadences are authoritative here. Default values agree, but changing that field
  alone would not slow the graph.
- Regulatory baselines and emitter gains are developmental-expression gated;
  material ATP baseline is not. Expression follows maturation/.11, capped1,
  and development updates every60 age ticks. The newborn's first59 ticks do not
  have the juvenile's full-strength energy-deficit emitters. Initialization at
  arbitrary age derives expression directly, without chemical history.
- World movement uses2 units/second divided by20; peer contact uses .25/20,
  producing the already observed.0125 dose. These event amounts are integrated
  inputs, not rates to multiply by20 or divide by20 again. Persistent body energy
  deficit is a state sampled once per chemical crossing, not an action event.
- One-shot social dose is not multiplied by catch-up crossings. Both the
  one-tick and six-tick graph controls apply exactly-.0075 loneliness contrast.
- **Batching is not sequentially equivalent.** At fixed body, six sequential
  graph steps yield H/F/ATP `.45284986/.33523324/.61476666`; one six-tick call
  yields `.47409999/.34939998/.60060000`. Decay is integrated first, then persistent
  release is multiplied by crossings without intermediate decay. This is a
  demonstrated scheduling/approximation limitation relevant to AOA-TIME and
  BIO-010/011, not the cause of the one-tick fixture drift. No approximation error
  bound is supplied by this experiment. Do not claim arbitrary cadence equivalence.
- ATP is typed Material but has baseline relaxation plus signed emitters; the
  existing material-baseline tests explicitly preserve this behavior. Its value
  is used as availability and regulation, not a literal conserved ATP inventory.
  The nutrient-to-ATP reaction is stoichiometric. This audit does not certify all
  energy bookkeeping against BIO-011 or justify rewriting saved material pools.

## Source map, checks and disposition

Controlling architecture: sections5,7,9,11.1–11.4; especially AOA-TIME-001/002,
BIO-003/006/007/011/015/016/018, DEV-001/002 and PERSIST-001/002.
At the exact base:

- `biochemistry.rs:695` initializes body/development/graph; `:805` owns multirate
  body/development updates; `:152` owns organ-event shares/upkeep; `:540` derives
  developmental expression.
- `biochemical_graph.rs:1075` defines the reference recipe; `:1318` initializes
  baselines; `:1374` advances decay/emission/reaction; `:1470` derives all drive
  projections; `:1760` aggregates receptors; `:1798/:1828` distinguish persistent
  body loci from one-shot events; `:914/:995` hold opt-in waking/play recipes.
- `chemistry.rs:17/:122` define independent valuation and baseline drive ABI.
  `evolutionary_genetics.rs:1139–1151` compiles chromosome modifiers into the graph.
- `alife_world/headless.rs:71–76` defines20Hz and physical contact dose; registered
  commands advance canonical biology and finalization skips an already advanced
  record. The six earlier physical outcomes independently verified this path.

Finite audit:1/1 passed. Formatting passed. Strict targeted Clippy passed after
one identity-conversion lint correction. Compile-only checks passed. The whole
core suite, world suite, GPU, retained brain and actual behavior were not run.
No claims extend to them.

Disposition: retain raw previous evidence; do not normalize away transient data,
replace inherited baselines, or patch signs that already agree with the equations.
For a future weight/breed evaluation, bind the exact inherited recipe and actual
physiological history, and distinguish sustained reserve-driven pressure from
action-conditioned effects. Catch-up equivalence needs a separately scoped
contract decision before a solver change. No production patch is justified by
the one-tick anomaly itself.

The original [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/)
paper remains methodological context for bounded source-guided iteration. It is
not evidence for A-Life biology, and none of these core calculations is a neural
rollout or a weight-editing success.
