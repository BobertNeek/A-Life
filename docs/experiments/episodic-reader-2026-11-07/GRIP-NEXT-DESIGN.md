# Next hypothesis: grounded grip context

Status: read-only design, not implemented or tested. References use frozen
612c42015cdc894e80ebdee5d765c9f5a0d5bfa0. The relevant code is unchanged at
combined phase/geometry head 0d3a2ecf.

## What exists

Release is physically available. Grab action 211 is Contact/Interact for every
admitted object. Selecting it on one's already-held target detaches that target
and records a Released grip transfer. Selecting it on an unheld target acquires
it. NO_MANIPULATION does not release. See candidate_enumerator.rs:32–73,
319–345 and headless.rs:5560–5572, 5597–5647.

Deliberate Grab possession is protected from automatic replacement, whereas
activity-held Eat/Play objects can be placed before switching targets. This is
documented behavior, not demonstrated as a physical defect by the pilot. See
headless/hands.rs:30–45, 98–149, 169–173 and docs/REFERENCE.md:68–85. Movement
before manipulation is intentional; simultaneously fleeing and eating a held
item is not by itself a bug.

## What is aliased

Acquisition and release share one Contact candidate, the same 24 object
features, and one episodic family namespace. Contact lane 18 combines touching
and being carried. Lanes 19/20 are observer linear/angular speed. None is a
candidate-bound possession observation. See grounding.rs:102–147 and
grounded_sensing.rs:394–428.

The ObjectSlots pilot also lacks a global grip bit. Tactile lane 2 is populated
only in the Terrain vision branch (headless.rs:3835–3880, 4110). Carried pose,
contact, relative movement and history can support indirect inference, but a
touching unheld object can resemble a held one at rest. Terrain's global bit
still cannot identify which of two contacting candidates is held.

All six per-object actions receive the slot's same features. Perception checks
that equality (perception.rs:1004–1010), so silently repurposing a lane or
inserting a host-written release preference would violate the current contract.

The v12 negative Contact fear/pain readout can suppress both acquiring and
releasing a remembered bad object. Reversing that sign globally can encourage
acquisition. An additive heldness bias alone does not express all four desired
cases under otherwise matched competition: acquire benign/unheld, avoid
harmful/unheld, retain benign/held, release harmful/held. The policy needs an
interaction or separately parameterized operations.

## Candidate design to test separately

1. Add a typed, candidate-bound observation that this target is constrained by
   this organism's grip. Derive it from actual body ownership, not hazard,
   nutrition, object labels or identifier arithmetic. Preserve confidence and
   frame binding.
2. In a new opt-in profile/ABI, distinguish close-grip and open-grip operations.
   Replace the held target's toggle candidate with explicit ReleaseGrip, rather
   than adding a duplicate. Keep Manipulation as the channel initially.
3. Give operations distinct neural readout parameters or a compiled
   grip-context interaction. Do not have the host choose desirability.
4. Scope family expectations by operation where needed. Merely assigning two
   Contact action IDs is insufficient: current memory identity treats action
   IDs specially only for Other (memory/candidate_index.rs:147–161, 646–674).
5. A stale release must fail or do nothing, never acquire a now-unheld target.
   Reuse actual detach/transfer mechanics. Expose genuine grip/touch continuity
   under occlusion without inventing unseen appearance or chemistry.

Keep all V1 lane meanings, family numbers, defaults and checkpoints unchanged.
Do not shift existing memory lanes 24/32 or cognitive lanes 36–53. A ninth
family is more invasive than a versioned Contact operation subtype. Every new
coordinate needs explicit fresh-candidate admission or a separately qualified
migration; this review justifies no acquired-life migration.

This makes the behavior representable, not solved. V12's strong useful signal
was Ingest-family value. A Release candidate does not automatically receive
that expectation, and target pain recall can miss a particular episode. Any
cross-operation memory route must be explicit and neural, or release outcomes
must be learned through ordinary experience. No hidden transfer of negative
Ingest value into a host release score is acceptable.

## Required controls

- Four held/unheld × benign/remembered-harmful contexts with matched physiology,
  appearance, reach and current pain zero; no-memory and neutral controls.
- Hold A beside touching B; swap side, appearance and identity namespaces. Only
  the actually constrained target receives heldness/release semantics.
- Ablate heldness and acquired aversive context separately. Constant dropping
  is not a successful learned-release circuit.
- Separate acquisition, release, failed release and consumption outcomes in
  memory without destroying legitimate target-level generalization.
- Wrong owner, player-held, missing effector, consumed/stale target, blocked
  placement, repeated release, activity-held and deliberate-held cases.
- Gaze/occlusion/crowding tests, exact channel/target receipts, unchanged legacy
  continuation and rejected cross-ABI restoration.
- CPU/GPU framed inputs, scores, choices, gradients, eligibility, updates and
  restored continuation. Actual GPU execution is necessary for parity claims.
- Only then, fixed-budget fresh lives reporting safe-food throughput, grip
  duration, release/reacquisition, harmful exposure and safety stops. Include
  held-out sides/identities and no-memory/no-grip-context arms.

The controlling architecture supports grounded body proprioception and grip
affordances (§6–7), simultaneous body-neutral intentions (§22), and versioned
discoverable embodiment ports (§23). It does not imply a poison-aware drop
reflex.
