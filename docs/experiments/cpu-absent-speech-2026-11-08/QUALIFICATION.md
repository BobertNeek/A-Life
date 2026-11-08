# CPU absent-speech parity repair

A fixed opt-in Nano drive experiment exposed a CPU/native discrepancy. Its
phenotype has no speech-payload decoder. When a sampled joint choice included
Vocal, the CPU helper demanded 1,024 speech rows and returned InvalidDecisionEvidence.
The production WGSL decoder instead emits no speech payload for an absent head.

The CPU helper now returns None only when both the speech plan and all SpeechPayload
rows are absent. Existing vector-shape guards still run first. Partial/unbound
present heads are not treated as absent, and the existing present-head path is
unchanged. No token, replacement action or successful utterance is invented.

Four fresh fixed regressions cover:

- Absent-head no-payload return with unchanged founder checkpoint and malformed
  input-length rejection.
- The actual selected Vocal channel and primitive remain selected. The ordinary
  world records its missing-payload failure and emits no utterance/token.
- Present-head shape and finite-value guards remain enforced.
- Partial, unbound and malformed present decoders fail admission.

## Independent ordinary-main qualification

This small patch was extracted from the opt-in head-5 experiment and applied to
ordinary-main-equivalent tree 0a34d588488c09f9a7fd4d8bc6115be61bdcd750. No experimental
head, recipe, Nano host-admission or biological-drive change is included. The
main-compatible test uses an existing action-credit recipe and existing world APIs.

Official Rust 1.99.1, offline Cargo, jobs 1, incremental compilation and dev/test
debug symbols disabled. Source mtimes were refreshed before building against a
warm target; logs name the actual ordinary-main checkout for all compiled crates.

- cpu_absent_speech: 4 passed, no failures or ignores.
- Full cpu_collector target: 25 passed, no failures, one existing manual benchmark
  ignored. Includes checkpoint/restore, failed-step rollback, exact seals and
  inherited readout retention regressions.
- Strict training lib/tests Clippy with cpu-backend: passed.
- Workspace formatting: passed.

These are CPU/helper/world fixtures, not a GPU-device numerical parity run or
new default-brain admission. No existing private creature/checkpoint is an input,
no acquired state or wire format changes, and no optimizer update is added. The
failed initial drive batch remains a partial failed attempt; its later fixed-plan
revision is a separate, negative policy result, not evidence of learned improvement.
