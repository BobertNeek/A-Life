# Reserve-threshold composition addendum

Frozen before running these structural tests. Prior final source remains
`98fe445493192bc80bb9cbf0b2e16fc16c7df2bf`; the sealed physiology execution remains
`f3602c25929b099c1a04c10aa2fb7ea4a87eef79`. No prior source/test/results/grid bytes
are edited and the3626-call physiology grid is not repeated.

Four small structural recipe tests, **zero physiology/graph advances**, zero
world intervals, zero neural activity, no private/acquired state or device:

1. Compare reserve-before-waking and waking-before-reserve. Confirm removed
   energy-deficit fatigue and sleep-to-ATP emitters stay removed; ATP gain remains
   -.02, and only surviving EnergyDeficit thresholds change.
2. Compare reserve-before-play and play-before-reserve. The actual API in this
   source is `with_play_stimulation`, not `with_play_drive`. Preserve boredom,
   curiosity, valuation and all non-threshold graph fields.
3. For each fixed recipe order waking→play and play→waking, compare all three
   positions of the reserve modifier. Do not assert that waking and play themselves
   commute or reorder emitter declarations to make them equal.
4. With waking already applied, deliberately change surviving hunger/ATP gains
   to .11/-.013, then apply the threshold. Verify the requested gain values survive.

Each path starts with the existing public seed921412/foundation(10,1,7,N512).
Both homologs receive the same recipe, and the helper explicitly rejects
nonhomologous emitter arrays. It selects only existing non-inverted analogue
EnergyDeficit emitters whose targets map to Hunger/Fatigue/ATP, never appends
missing rows and never copies gains from the reference graph. The reserve target
remains the prospective.8, with threshold `1.0_f32 - 0.8_f32`.

Maximum construction paths:11, across4 tests, one test invocation after format,
compile/lint checks. No numerical tuning, additional variants, persistence load
of a real creature, repeated physiology grid or whole-world suite is authorized
by this addendum. All conclusions are structural/source compatibility only;
composed recipe physiology has not been measured.
