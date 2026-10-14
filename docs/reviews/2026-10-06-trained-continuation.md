# Established N2048 continuation

The established `cycle-0527-recovery` retains the trained actor at policy 528,
actor Adam step 1696 and value Adam step 1056. Its omitted biological objective
version decodes as the historical version 1. Previously, resuming that receipt
under objective 2 discarded both optimizer/value checkpoints and started their
state over. Ordinary resume now rejects that mismatch before creating output or
world/GPU resources.

An explicit `--resume-cycle` transition may name
`--preserve-objective-state-from 1`. It loads the same sealed actor and value
checkpoints, retains their weights, moments, per-coordinate ages, masks and
optimizer progress, and records `objective_transition_from: 1` in the new
objective-2 receipt. It does not edit or relabel the source receipt. Wrong,
unsupported or unnecessary version declarations fail. Adaptation-only inputs
and imitation warmups cannot use this transition to manufacture saved state.

The native trainer compares its actual restored actor checkpoint with the
saved checkpoint before training. After uploading the saved value head for
frozen predictions, it compares that GPU checkpoint before applying an update.
These checks include the complete serialized state, rather than step counts
alone. Source phenotype/sensor/decoder admission and normal checkpoint validation
still apply; the transition flag bypasses none of them.

This is an offline training-state transition, not full individual continuity.
The cohort constructor still creates a new organism/world. The established
personal save is tick 88, whereas the cycle ended at tick 345; it does not supply
a terminal personal checkpoint coupled to the final actor/value seal. Its
historical sensor/decoder ABI also requires a genuine state-preserving transfer
before the current terrain lesson can use it. The nightly launch remains
**Blocked**. No diagnostic branch, fresh baseline, source checkpoint rewrite,
training run or promotion is authorized by this implementation.

Relevant architecture contracts are AOA-PERSIST-002 (no silent state
substitution), AOA-PERSIST-003 (auditable migrations), AOA-BRAIN-007 (distinct
acquired state) and AOA-INV-011 (one causal individual sequence).

Validation: focused CPU tests cover legacy rejection before output/assets,
version declarations and complete actor/value file preservation during explicit
decode. CI also compiles the feature-gated training CLI. Actual GPU restoration,
state-preserving ABI transfer, personal-state continuation and behavioral
retention require separate guarded hardware evidence. No local build or GPU
check is admitted during active interactive use.
