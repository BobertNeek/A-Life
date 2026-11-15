# Fresh inherited grip proprioception (V5)

`CognitiveArchitectureGenomeParameters::with_grip_proprioception_v5()` creates a
new inherited identity. It preserves the inherited motor layout and explicit
memory-candidate bound. It does not migrate existing genomes, acquired state or
checkpoints. Old architecture versions omit the field and preserve their codec,
digest and sensing behavior. Older readers reject architecture5.

For an explicitly enabled `GroundedObjectSlotsV1` creature, existing tactile
lane2 (flat sensory index34) reports global occupancy of its own grip. The world
scans all authoritative objects, reduces `carried_by == observer`, and applies
Touch gain once. Other creatures' possession and mere contact are not self-grip.
No hidden object class, food state or candidate identity is exposed. This is not
candidate-in-grip information. Terrain profiles already populate this lane and
remain unchanged. Candidate contact and motion features remain unchanged.

An optional `GripProprioceptionV1` frame receipt binds the held result, actual
visited-object count and gain into the perception digest. Rebinding/finalization
retains it; malformed or inconsistent receipts are rejected. Its logical work is
N+3: N occupancy reductions, one output materialization, multiply and clamp.
CPU and gameplay receipt collectors add this once to `focal_target_ops`. This is
an explicit accounting tariff, not a runtime-speed claim. Ordinary neural work
and body metabolism still apply independently.

The signal uses the normal sensor encoder and biochemical receptor effects.
It does not mask actions, force holding, or select a target. Existing zero neural
connections may prevent useful behavioral use; sensor availability is not an
efficacy claim or a learned adaptation. Genetically seeded connectivity and
acquired updates must be reported separately. Authenticated harm memories and
all existing Ingest projection/support rules are unchanged.
