# Standalone observed-need evidence V1: boundaries

This module is an opt-in research component, not a MemoryBank replacement, learned policy, default runtime attachment or decoder port. All existing MemoryBank fields, wire records, digests and readers remain unchanged. It does not import old candidate records or reconstruct missing per-need history. The only production edits are a new module/export surface and moving the already present serde_json workspace dependency from core dev-dependencies to normal dependencies.

## Evidence and identity

One validated sealed source interval yields at most one representative observed-target association. The evidence retains all eleven signed normalized drive changes (`after - before`), with no uniform reward/deficit interpretation. Missing measured physiology and unsupported/unobserved targets yield Unknown; a real measured zero remains a measured observation. Targetless Rest/Idle are intentionally outside this first target-associated component, not examples of absent bodily consequences.

The exact query binds organism, sensor profile, action ID/kind/family, observed tracked target plus world entity, and a normalized context digest. It hashes a dedicated typed projection of the original validated frame, omitting only four enclosing clock fields. It never reconstructs a fictitious earlier frame. Body pose, need/hormone values, sensory/social/language observations, target/candidate context and retrieval values remain exact. This deliberately does NOT qualify useful generalization across changing need or position; changed context normally cannot recall the same source. An additional, separately tested similarity/query contract would be needed for ordinary policy use.

The store retains bounded serialized sealed source patches and revalidates/rederives keys/deltas on restore. That supports stronger internal consistency than detached scalar records, at higher storage cost. A sealed/validated patch is a consistency contract, not cryptographic proof of an external world's authenticity. The runtime producer must remain the authorized canonical observation path. Tests explicitly distinguish synthetic contract fixtures from actual registered-world measurements.

No isolated physiological causality is inferred for any channel. A joint interval stays one association with its source sequence/bundle provenance and command count. Physical confirmation of a channel does not isolate which chemical change that channel caused. Opposite-sign observations are retained independently, not merged into a synthetic zero. There is no automatic pooling or fuzzy transfer.

## Bounded lifecycle

Hard caps:32 records,128KiB encoded source,1MiB complete encoded store,32 inspected records and8 returned observations per query. Admission rejects oversized sources before deep cloning. Restore rejects oversized input before parsing and validates every retained source. Oldest-first pruning handles count and encoded-byte pressure. The monotonic sequence/interval watermark survives pruning and Unknown observations, rejecting duplicated/reordered source observations without unbounded tombstones. This intentionally requires ordered delivery; out-of-order replay is not silently repaired. Restoring a deliberately older external snapshot cannot prove knowledge of observations after that snapshot.

JSON is the explicit bounded persistence API; no permissive generic Deserialize is exposed for the store. The envelope integrity digest detects accidental mutation, while source validation remains required even if that digest is recomputed. Raw stores/patches remain local and are excluded from shared artifacts. Safe reports contain byte counts, hashes and scalar contrasts only.

## Before an appended decoder can consume it

A future explicit recipe/plan must separately admit an appended need-consequence port and its source schema, lane order, observation availability, confidence, source count and association semantics. It must leave old decoder coordinates and old Unknown behavior unchanged. A loneliness-specific reader can interpret negative observed delta as relief under positive current pressure, with worsening adverse and Unknown contributing no evidence. ATP, curiosity, reproduction and reserved lanes must not inherit that sign indiscriminately.

Persistence must bind the optional store to organism, recipe/profile, source schema and checkpoint lifecycle; old saves must remain without it, with no invented migration. CPU and GPU must agree on query packing, Unknown/zero distinction, ordering, precision, clipping, scoring, gradient/eligibility and learning credit. Appended costs/rows require honest budget accounting. Exact-context retrieval is currently too strict for general policy transfer; similarity/generalization would be an additional finite component with aliasing, changed-need/position, target/family and adverse-outcome controls. No new acted allocation or default activation is part of this implementation.

## Separate next-query proposal if exact matching fails in the next world frame

Do not loosen the evidence/provenance key. A second explicitly versioned retrieval mode could keep hard organism/profile/action-family/primitive/tracked-target/entity equality and bound source age, inspected records and returned records. Its matching projection would explicitly distinguish stable observed target attributes (shape/material/color/chemical and available sensor confidence) from expected-to-change query variables (current need/hormones, actor pose and target bearing/distance). Those changing variables remain stored in the source and available for future conditional estimation; they are not fabricated or erased from evidence.

The narrow first test would be a newly observed source queried in the actual immediately following frame, at most one tick old, with the same tracked target/family and unchanged stable observed attributes. It must still reject unrelated target/family/profile, materially changed observed attributes, stale source and unavailable confidence. Both relief and worsening observations remain separately available, with no preselected favorable exemplar and no inference from peer category. Unknown and measured zero remain distinct. This is an explicit invariance hypothesis, not an established similarity metric or authorization to treat arbitrary contexts as identical. Broader sensor-noise tolerances, pressure-dependent weighting and target generalization require separately frozen controls.

No such alternate matching API is implemented in V1. The actual next-world query test is intentionally allowed to demonstrate its current usability limit rather than resetting the creature to make a match.


## Host ownership and population-cost boundary

This is an opt-in research evidence prototype, initially suitable for draft review, not gameplay/default promotion. OrganismId is only scoped within its world/life; it is not globally unique. Runtime attachment must separately bind world/life identity, inherited genome identity, recipe/profile and checkpoint lineage, including rollback/replay ownership. Retaining a source genome ID inside each validated patch does not replace that outer host boundary. No CPU collector, GPU runtime, save envelope or gameplay factory attaches this store today.

Actual fixed observations cost15,949–16,241bytes each and17,701–18,027bytes for a one-record envelope. A1MiB per-store cap is a proof boundary, not a scalable population-memory claim. Many simultaneous actors would require a separately budgeted compact witness design, allocation accounting and profiling. No learned/private source JSON or complete banks are published as assets.
