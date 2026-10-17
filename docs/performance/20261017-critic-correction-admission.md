# Critic correction package admission — 2026-10-17

This lane admits a critic-only derivative of an existing sealed training
checkpoint. It does not qualify behavior, claim accepted PPO work, or authorize
activation. Fresh held-out evaluation and resumed-PPO stability remain external
adoption gates.

`critic-correction.json` schema 1 identifies the preserved source, its complete
SHA-256 file map, the replacement value checkpoint, the recovery manifest and
bounded qualification evidence. Admission requires the original source's native
sealed-handoff validation, valid actor/critic state, unchanged policy and feature
identity, and 1–5000 additional critic optimizer steps. Actor weights, moments,
ages, all other source files and the original `cycle.json` remain byte-identical.
The actual critic age comes from the decoded replacement checkpoint. No cycle
receipt is rewritten to pretend those critic steps were PPO updates.

The recovery source and evidence paths must still resolve to their exact hashed
bytes. Missing dependencies, unsafe relative file references, symlinks, extra or
missing files, reset claims, nested correction sources and changed inputs fail
explicitly. File count, depth, byte size and evidence count are bounded. Ordinary
sealed checkpoints without the marker retain their existing admission contract.
Traversal rejects duplicate normalized names and literal backslashes in file
components, avoiding collisions between a Unix filename and a Windows separator.
Archived-rehearsal entrypoints retain their ordinary strict handoff and currently
reject corrected sources; this admission lane does not claim that support.

This implements an explicit training-checkpoint migration receipt under
AOA-PERSIST-002 and AOA-PERSIST-003, preserving the unmodified state and causal
identity required by AOA-PERSIST-001 and AOA-PERSIST-004. Invalid ownership,
metadata and nonfinite optimizer state are rejected under AOA-FAIL-001.
The referenced preserved source remains the rollback under AOA-FAIL-004.

CPU regressions cover ordinary and corrected admission, continuation with the
actual critic age, unchanged source bytes, actor/moment and receipt mutations,
incorrect ages/counts, malformed critic state, preservation/evidence hash failures,
unsafe paths, missing/extra files and an unreceipted replacement critic. No GPU,
neural execution, optimization or learner process is needed for these checks.
