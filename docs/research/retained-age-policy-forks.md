# Explicit retained-state age-policy research forks

`HeadlessWorld::fork_without_age_death_for_research()` and
`CpuCanonicalHost::fork_without_age_death_for_research(&founder)` create a new
counterfactual copy of an existing living simulation. They do not mutate the
source. This is an experimenter-selected world policy, not learning, a genetic
change, rehabilitation, or evidence that the original individual survived.

The copy changes only the existing persisted `disable_age_death` flag. Ages,
world time, injury, organ reserves, chemistry, memories, neural state, RNG,
source assets, genotype and calibrated policy remain unchanged. The host API
authenticates its founder, checks the complete checkpoint with only that flag
normalized, and validates native cold admission. The caller retains the exact
founder/optimizer checkpoint. No biological interval or action is advanced by
creating the copy.

Both APIs reject an already-enabled policy, empty population, dead records,
and records that are already terminal under the original age policy. They
cannot resurrect a dead checkpoint or rescue a still-marked-alive record that
has already reached its terminal age, health or energy boundary. Biological
aging continues in the copy. Health and energy death remain active; disabling
age death does not heal or provide resources.

The existing tick-zero New Game setter and all defaults remain unchanged.
There is no schema migration: the flag already belongs to the saved world
format and is already supported by the existing New Game option. Legacy false
checkpoints retain their old execution and death semantics. A research fork
must be labeled separately from its finite-age source and any completed death
outcome. Keep both branches, account for prior elapsed time and costs, and do
not report the fork as an uninterrupted continuation of a dead endpoint.

This API does not select actions, request sleep, supply rewards, move objects,
or add food. Any separate experimental care must use authenticated public
world operations, be declared independently and retain its actual costs and
outcomes. CPU qualification is not full-game or GPU execution coverage.
