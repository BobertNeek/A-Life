# Local hands, visible steering, and care assessment — 2026-10-19

Non-normative implementation report for the patch based on `5d0a8c3f` on
`feat/local-hands-and-visible-steering-20261019`. The controlling v2.0 architecture
is unchanged. Source paths below bind each claim to this implementation; software
checks and the final publication receipt supply verification for the actual head.

| Requirements | Implementation | Scope and evidence boundary |
| --- | --- | --- |
| AOA-WORLD-001/002/003; AOA-MOTOR-002 | `crates/alife_world/src/headless/hands.rs`, `headless.rs` | Shared legal Eat/Play preparation, protected deliberate carrying, bounded safe placement, explicit transfer receipts. World authority validates every attempted transition; no candidate scores or target preference are added. |
| AOA-SENSE-001/002; AOA-MOTOR-005 | `crates/alife_world/src/headless/local_steering.rs` | Fresh canonical calibrated visual admission authorizes one bounded local clearance step. Nine physical ranges and the observed bearing enter the selector; no hidden target pose, opening search, or route is available there. |
| AOA-EMB-001/003; AOA-TIME-001/003 | `headless.rs`, `persistence.rs` | Ordinary body/terrain/grip validation, measured movement and effort, persisted grip provenance and net-progress reference. Registered transaction rollback covers all new world state. |
| AOA-TEACH-007; AOA-OBS-002 | `gpu_live_runtime.rs`, `gpu_live_runtime/staged_tick.rs`, `foundation_training.rs`, `foundation_training_cycle.rs` | App-only measured manipulation receipts bind organism/ticks/sequence and selected primitive/target. Preparation cannot create completed Play labels, stimulation, nourishment, or activity praise. Eat completion retains actual consumption evidence. No core experience or packed neural ABI changes. |

## Physical behavior

Mobile Play targets require preparation before activation; fixed toys stay in
place. Eat pickup and later consumption remain separate selected Eat intervals.
An activity switch validates the new target before attempting safe placement.
An unavailable placement or protected deliberate carry leaves possession intact.
Legacy unknown possession defaults to protected carry. Repeated drop/regrab does
not supply activity stimulation or investigation novelty. Reachable non-food Eat
attempts retain ordinary physical contact and hazard consequences.

Play narration and activity praise bind matching completed target activation to
positive measured investigation or play stimulation from that same channel.
A novelty-1.0 activation supplies investigation instead of familiar-toy play
stimulation; its physical completion can be narrated without inventing boredom
relief. The registered motor-receipt regression covers both activation cases and
pickup-only preparation. Scripted Era1 peer demonstrations supply their own
known transformed endpoint; learner movement still obeys fresh visibility
admission, and demonstration success/digest validation is unchanged.

Opaque hidden destinations disable automatic detours. Learned intermediate
movement can restore sight; target occlusion pauses assistance on the next
physical decision. The finite detour allowance replenishes only after meaningful
improvement beyond the best observed distance, which survives reload. Local
regressions cover body-width slits, low obstacles, transparent solids, hidden-goal
and distant-geometry invariance, loss of sight, carry clearance, and oscillation.
The actuator does not choose destinations, exploration, or hidden routes.

Canonical world signature schema 8 binds the new authoritative state. Subject
policy-comparison identity also binds the current physical schema and does not
exclude grip provenance or steering state. Optional save fields default to false
or absent for legacy worlds; invalid possession/controller states are rejected.
This changes current-source identities without rewriting historical evidence.

## Care assessment

`EatReachableFood` reuses the existing physical unheld-food arrangement through a
separate care setup field. It does not create `GrabFood` experiment acquisitions
or its +0.25 credit. Care demonstrations choose Eat for both physical pickup and
later consumption; the obsolete teacher-private Grab substitution is removed.
Explicit Grab remains an independently chosen carrying action and a distinct
historical experiment mode.

Evaluation-only silent controls suppress initial and repeated spatial requests.
Prompted and silent receipts are separate conditions; absence of a demonstrator
alone does not establish silence. Feeding's prompted cue is evaluation-only,
leaving its existing training mix unchanged. Exact learner/target
`FoundationFoodOutcome` nourishment evidence defines care completion; blocked
actions are reported separately and do not erase a genuine meal. Language
production is not a prerequisite for the care lesson.

The [care-first plan](../../training/care-first-next-run-2026-10-19.md) records
held care, reachable unheld care, visible approach, and need-driven choices before
hidden intermediate routes. Training and learning evaluations remain held.
Software regressions do not establish policy competence, founder promotion,
familiarity, durable personal memory, or exact terminal-individual recovery.
Foundation/actor/value Adam, accepted packages, original qualification, seeds,
and the continuity audit remain unchanged; the final receipt records their hashes.
