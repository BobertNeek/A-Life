# Opt-in joint-observed action membership

This experimental memory API preserves one **joint observed consequence** and
its authenticated executed action/target membership. It is not a new learner,
a counterfactual action-effect model, or an action-specific reward label.

`MemoryBank` and `MemorySidecarState` expose
`observe_sealed_patch_with_joint_membership` and
`recall_frame_with_joint_membership`. Default admission/recall is unchanged.
No live CPU/game policy entrypoint opts into these methods in this change.

## Capture and retention

Capture validates the sealed patch and resolves each known executed channel to
its actual selected primitive, commanded target and perceptual candidate query.
It stores one exact event: member queries and typed execution facts, event and
query times, sequence/bundle/causal digests, the pre-merge **shared** family
vector (valence, joint success, danger, energy), and the original decision
confidence. That confidence is not causal certainty. Global neural eligibility,
shared physiology and ordinary scalar episode values are not rewritten.

Shared valence, pain and energy can include ambient hazards and player
praise/scold/pet/slap. Membership means participation in that observed joint
interval, not that a participating object/action caused those effects. The
success coordinate remains whole-bundle success even if Ingest succeeded.
The separately typed channel receipt retains its distinct execution result.

Ordinary admission, capacity, merge and eviction still apply. An episode keeps
its latest known exact joint event as a unit, separate from its averaged scalar
values/confidence. Later legacy/unknown observations cannot invent or update an
event. This is bounded retention, not a complete action history. One event is
not copied into independent per-channel episodes. Repeated sealed sequences are
rejected; duplicate event identities across retained records are rejected on
cold validation, including conflicting copies.

## Explicit recall and context

An exact-member index scopes organism, profile/schema/ABI, query version,
tracked target, action family and primitive. Within that namespace, up to 64
records are searched in actual event-sequence order (not record-ID order).
Existing sensory/state-query similarity and threshold are used, with at most
four events, each counted once. Their own confidence and event age determine
confidence; merged observation counts and merged confidence are not borrowed.

A matching joint-event estimate **replaces**, rather than adds to, that
candidate's ordinary family estimate in existing MemoryContext lanes. Target
latent recall and present sensory features are unchanged. If no eligible event
exists, ordinary family recall remains. Sources are explicitly reported as
joint memberships; ordinary scalar-search counters still describe the baseline
lookup. The existing decoder weights and memory cap are unchanged.

This first API is exact-target only. Consumed Food disappears; a new object ID
does not inherit a joint-event association here. No category generalization or
physical food-choice improvement is established. A future consumer must qualify
ordinary sensory-based category transfer separately.

## Versions, cache and cost

Enriched records and portable bank assets use schema 3. Explicit recall receipts
also use schema 3. Legacy records without events retain their exact older wire
and digest behavior; missing provenance is unknown. Old bank/asset admission
rejects schema 3 rather than silently dropping the event. Prepared and finalized
recall are nonserialized runtime types. No enabled collected-row import path is
introduced; later live integration needs an explicit compatible admission path.
A standalone editable diagnostic JSON receipt is not authenticated merely by
being deserializable.

Event bytes and validated decoded values form a private immutable pair. Trusted
internal export may reuse validation only if exported bytes exactly match the
checked original. Mutable external portable input is parsed and checked anew.
Digests link retained provenance; they are not a signature proving an arbitrary
recomputed payload came from the original World. Independent provenance still
requires the retained sealed patch/checkpoint.

Explicit lookup work is included once in `execution_evidence_work`, covering
member-index lookup, accessed records, member comparisons, actual ranking
comparisons and selected projections. `joint_membership_admission_work` reports
additional logical encoding/checked-decode blocks, member validation and actual
index maintenance operations, including shifts/comparisons. These are declared
work units, not measured CPU instructions or isolated physiological debits.
No default bank without events pays this new component. Capture, persistence
and validation are not free. A future live caller must admit and charge both
returned components through ordinary cognitive work, with rollback and budget
controls; this API-only change does not pretend to have done so.

Tests distinguish actual sealed World execution, storage-only synthetic
fixtures, actual subsequent queries, and a saved-context post-consumption probe.
That last probe does not claim consumed food remains visible. Neither a changed
MemoryContext nor a fast-weight response alone proves improved learned behavior.

Ordinary `MemorySidecarState::restore_portable` rejects schema-3 enriched banks. Explicit research restoration uses `restore_portable_with_joint_membership`; its caller owns work admission/accounting. The legacy founder-clone operation rejects enriched banks rather than rebinding authenticated event identity or discarding membership. Existing schema-2 restoration remains unchanged.
