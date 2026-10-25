# Fresh foundation-checkpoint assessment

`compare_foundation_probes.py` defaults to `subject_n2048_policy_weights_v1`.
That existing contract excludes only the subject's weights and their derived
asset/genetics digests. Training stage and promotion provenance remain controls.
A mismatch there is expected when comparing checkpoints with different histories.

For newly captured fresh foundation pilots, explicitly select
`--assessment-scope fresh_n2048_foundation_checkpoint_v1`, along with the existing
predeclared control/candidate policy SHA-256 and executable SHA-256 arguments.
This requires `pilot.json`'s separate `initial_checkpoint_assessment_identity`;
there is no fallback to a historical or narrow identity. The full canonical world
identity and full original policy bytes, asset digest and SHA-256 remain audit
evidence. No canonical/archive/checkpoint identity or admission rule is changed.

The new world projection additionally excludes only the subject candidate's
`manifest.training_stage` and `manifest.promotion_receipt`. The projected genetics
content reference is rederived in its own domain; its ownership, revision, schema
and causal tick remain bound. All other manifest fields (including weight count,
capacity, sensor, layout, codebook, decoders, route, plasticity and address map),
nonpolicy genome, physical phenotype, physiology, lifecycle, world, tracking,
speech, other organisms and all brain/memory state-graph references remain bound.
Nothing removes private state or topology from an arbitrary saved creature.

This boundary follows [fresh world/body construction](../crates/alife_world/src/new_game.rs),
[physical expression](../crates/alife_core/src/evolutionary_genetics.rs), and
[exact candidate loading](../crates/alife_core/src/phenotype/construction.rs).
The [pilot](../crates/alife_game_app/src/foundation_training.rs) emits the identity
before runtime admission. The [runtime](../crates/alife_game_app/src/gpu_live_runtime.rs)
compiles the exact candidate without a restored checkpoint, creates a fresh
resident, memory and topology, and inserts a new neural brain. Stage/promotion
validate provenance but do not initialize behavioral parameters. A CPU-only
metadata falsification test compares the actual upload fields and fresh neural
mutable words with identical weights but changed stage/promotion; original
phenotype fingerprints intentionally differ and remain intact.

The selected comparison also binds the first sensory context, physiology,
candidate identities (scores are outputs), sampler and scenario provenance. For
the new scope, it binds available first prior input and provider configuration;
missing first prior evidence fails closed. Prior `decision_inputs` is a 256-entry
ring buffer: pilots with more than 256 decisions can lose the first receipt and
therefore cannot match under this scope. No missing receipt is reconstructed.
This limit does not affect the reported 64-decision case. Later prior requests
and counters are outcomes. This is initial-input matching, not a certification that later inputs
or private states remain equal as the policies act and learn.

Supported lessons are `grab_food`, `eat_held_food`, and (only under the new scope)
`feeding`. Feeding uses its captured `food_position` and `initial_food_distance`,
first target/ownership context and full world geometry; it does not require an
unrelated manipulation setup. Newly captured Feeding also requires its explicit
`food_need_initialization`: `newborn_v1` or `ordinary_healthy_aging_v1`, the subject,
and actual before/prepared physiology and ticks. The fresh scope validates and
binds this entire receipt in its setup component. Missing historical condition
evidence fails closed. Aged and newborn scenes are deliberately different inputs;
this tool must not certify them as matched. The original weights-only v1 and its
historical receipts are unchanged. RelocatedFood remains outside this comparison
scope. Missing captured controls fail closed.

The reported older/newer Feeding observations (13 decisions to eat versus 64
without pickup) remain two within-case observations with limited paired evidence.
Their intended weights/history differences do not alone invalidate that evidence.
They have no new native assessment identity, and no output life manifest/runtime
checkpoint directly certifies full initial private-state equivalence. They are
not newly matched by this tool and do not establish general causal forgetting.
No historical receipt is reinterpreted, and no new performance measurement is
implied by CPU contract tests.
