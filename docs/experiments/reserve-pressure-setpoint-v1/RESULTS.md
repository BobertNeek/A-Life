# Protected reserve-setpoint candidate

**An inherited .8 reserve target removes the legacy chemistry's persistent
near-full-reserve emergency while retaining monotone scarcity and low-reserve
urgency.** This is a measured opt-in core physiology candidate, not a new default,
learned optimum, autonomous policy or demonstrated world behavior.

The frozen grid passed once: **3,626 core advances**, 246,568 biochemical work
units, zero world intervals/neural emissions. A separate validation/roundtrip
test passed with zero advances. No failures or variants were discarded, no
parameter was retuned after outcomes, and the closed baseline audit was not rerun.

## Source, candidate and default protection

Execution source: `f3602c25929b099c1a04c10aa2fb7ea4a87eef79`, on sealed baseline
`9b41dd90bb44a43ab95cdd788c6e58fd2d194ee8`, ordinary-main ancestor
`550f79ad1f765132c634e7a272da56b521b57ec0`. The final result commit changes only
documentation/evidence after that exact executed source. See [FROZEN_PLAN.md](FROZEN_PLAN.md)
and [outcomes.jsonl](outcomes.jsonl),61 records extracted from [run.log](run.log).

The existing analogue emitter computes `max(source - threshold, 0)`. With
`source = 1 - body.reserve`, threshold `1 - .8` implements
`max(.8 - body.reserve, 0)`. This is an explicit proposed birth/genetic prior,
chosen prospectively near the founder's .7925 reserve, not fitted to this grid.

Only the three EnergyDeficit emitter thresholds change on **both** homologs:
target species1/Hunger,2/Fatigue,7/ATP. The float threshold is
`.19999998807907104`, obtained by `1.0_f32 - .8_f32`. Gains remain +.18,+.12,-.12.
All species/receptor IDs, declaration order, cadence, baselines, retention,
developmental floors, graph counts, neuroemitters and value weights are unchanged.
There is no new sensor, drive ABI, action reward or global EnergyDeficit meaning.

Production code addition:33 lines exposing the existing inherited emitter
threshold through `BiochemicalGraphChromosome::with_emitter_threshold` and its
private compiled-graph setter. No default constructor calls it. The test's
`candidate` helper is the explicit opt-in recipe; [CANDIDATE.json](CANDIDATE.json)
records the exact parameter edit. It is not installed into a live creature.

Validation verifies that undoing exactly these six allele threshold fields
recovers the original whole-genome serialization; constructing another default
founder remains byte-for-byte equivalent. The graph's other structural arrays
and value profile match exactly. Nonfinite/out-of-range thresholds and an invalid
index fail validation. A single changed homolog expresses the average threshold,
and both genotype and fresh candidate-state roundtrips preserve their contents.
No saved acquired state was read, rewritten or reinitialized.

## Matched fixed-reserve results

These graph controls keep the body reserve constant, isolating chemistry. They
are not claims that a real body can remain at that reserve without food/upkeep.
Each row is128 sequential steps from the same original chemical baselines,
not a steady-state initialization. Triples are hunger/fatigue/ATP availability.
All other8 drive readings are identical across every matched pair.

| Reserve | Awake legacy at128 | Awake candidate at128 | Candidate analytical limit |
|---:|---|---|---|
|1.0|.250000/.200000/.750000|.250000/.200000/.750000|.25/.20/.75|
|.9|.697579/.498386/.451614|.250000/.200000/.750000|.25/.20/.75|
|.8|1/.796772/.153228|.250000/.200000/.750000|.25/.20/.75|
|.7925|1/.819152/.130849|.283568/.222379/.727622|.28375/.2225/.7275|
|.65|1/1/0|.921369/.647580/.302421|.925/.65/.30|
|.4|1/1/0|1/1/0|1/1/0|
|.1|1/1/0|1/1/0|1/1/0|

The candidate produces **exactly zero extra drive change at or above .8 reserve**
with no action or sleep input. It neither forces constant activity nor adds a
basal arousal/awake source to make the graph look active. Below target, pressure
increases monotonically and severe scarcity still reaches hunger1/fatigue1/ATP0.
Saturation remains at .4; it was not tuned away.

For the separate prescribed recovery input, each graph step receives sleep dose.1:

| Reserve | Recovering legacy at128 | Recovering candidate at128 |
|---:|---|---|
|1.0|.25/0/1|.25/0/1|
|.9|.697579/0/1|.25/0/1|
|.8|1/0/.899193|.25/0/1|
|.7925|1/0/.876814|.283568/0/1|
|.65|1/0/.451614|.921369/0/1|
|.4|1/.249731/.03|1/0/.302421|
|.1|1/.93/.03|1/.548117/.03|

Recovery remains effective, but does not erase hunger. It cannot restore high
ATP under the mild dose at severe scarcity. The remaining ATP floor .03 is an
inherited ordering effect, not a newly invented survival reserve: EnergyDeficit
clips concentration before SleepRecovery adds `.3 * .1`.

Let P be `max(target-E,0)` and S the imposed per-step sleep dose. The analytical
limits, including inherited per-emitter clipping, are:

- Hunger: `clamp(.25 + .18*P/.04, 0, 1)`.
- Fatigue: `clamp(.20 + (.12*P - .70*S)/.04, 0, 1-.70*S)`.
- ATP: `clamp(.75 + (-.12*P + .30*S)/.04, .30*S, 1)`.

Every step was checked against the scalar recurrence, and every128-step endpoint
was within.006 of its analytical limit. This tolerance covers finite convergence,
not measurement uncertainty or a fit. Full raw11-lane arrays, first step,
endpoints and candidate-minus-control contrasts are preserved in outcomes.jsonl.

## Body feedback and retained limitations

The two16-step core-integrating traces start with the untouched fresh juvenile
body, advance8 awake/no-event steps then8 imposed recovery-dose.1 steps, and
never reset or clamp reserve.

| Point | Legacy reserve; H/F/ATP | Candidate reserve; H/F/ATP |
|---|---|---|
|Initial|.792500; .25/.20/.75|.792500; .25/.20/.75|
|First awake interval|.792067; .287428/.224952/.725048|.792067; .251428/.200952/.749048|
|After8 awake|.788803; .512833/.375222/.574778|.788498; .262242/.208161/.741839|
|After8 subsequent recovery|.785113; .707154/.017201/.654188|.783942; .276627/0/.941206|

Candidate first raw changes are hunger+.0014279783, fatigue+.0009519905,
ATP-.0009520054, versus legacy+.0374279916/+.0249519944/-.0249519944.
This is a real change to an explicitly inherited parameter, not drift subtraction
or changed observation baselines. Both recipes retain the original fresh values.

**Higher ATP signal increases inherited organ upkeep.** The candidate therefore
spends slightly more body reserve by step16 (.001170516 lower than control),
while both bodies lose reserve during sleep. This is not free body energy and
not proof of a longer lifetime. Downstream coupling must be assessed in any
later integrated study rather than calling the improved ATP reading universally
better.

**Full sleep dose retains a known limitation.** In the six single-step dose1
controls, low reserve.1 still raises ATP from.75 to.942 in legacy and.966 in
candidate; fatigue clips to0. At reserves1 and.65, ATP clips to1 in both.
Those measurements are retained. The equations imply repeated full-dose sleep
can saturate ATP even at low reserve; this longer full-dose trace was not run.
The candidate does not fix the legacy direct sleep-to-ATP emitter, distinguish
sleep fatigue from food fatigue, or certify ATP as a conserved material pool.
Existing opt-in waking/recovery biology is a separate recipe, not silently
composed into this candidate.

At expression0, both .8/.1 reserve controls preserve the newborn's material ATP
.75 and zero regulatory drives. The threshold does not bypass development or
create low-age chemical urgency where the inherited emission gate is closed.
That unchanged behavior is a limit, not evidence of good neonatal survival.

There are no policy/head weights, sensory priors, reward scalars or need memories
in this patch. BIO-018 still applies: a drive is state, not a selected action or
a fabricated learned benefit. The reserve target is proposed physiology; whether
the creature learns appropriate grounded actions remains untested.

## Checks and disposition

- Zero-advance threshold validation/roundtrip test:1/1 passed.
- Frozen ignored grid:1/1 passed, one invocation,3,626/3,626 advance calls.
- Work:19 species,1 reaction,22 emitters,26 receptors per call; no neural emitters.
- Formatting, strict targeted Clippy and diff checks passed on the executed code.
- Whole core suite, world behavior, device/GPU, neural selection, lived cohort,
  persistence of an acquired brain and interaction with other lineage recipes:
  **not run**.

Recommendation: retain this as the protected, opt-in inherited reserve-setpoint
candidate. The controlled results support replacing pressure relative to a full
reserve with an explicit reserve target for this recipe. They do not establish
.8 as optimal, authorize a default/genome migration, or qualify the unresolved
sleep/ATP and longer-term body feedback. No further run or variant was consumed.
