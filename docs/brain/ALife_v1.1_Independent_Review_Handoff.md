# A-Life v1.1 independent architecture review handoff

## Review state

- Mode: repair-only source closure.
- Controlling spec: `docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md`.
- Base: `19b4c0af272f41cb640e9cfd0ee93582408c6dc6`.
- Branch: `codex/v11-repair-only-intelligent-animal`.
- Source deficiencies: D01-D27 marked complete in `ALife_v1.1_Repair_Ledger.md`.
- Validation state: behavioral and performance claims remain untested.

## Ordered repair stack

| Commit | Repair |
| --- | --- |
| `66bf689c` | bounded sparse structural learning and dendritic allocation |
| `0ec6d911` | one shared production causal cognition transaction |
| `863f468c` | grounded peripheral/focal attention and canonical interoception |
| `a72014f7` | active target-specific concept/gap neural context |
| `16fbf112`, `514f95c6` | stable interaction-capable grounded prediction used before decision |
| `8dfc94cc` | six-channel factorized motor selection and eligibility |
| `70954f9f` | measured joint outcome, true RPE, and learning semantics |
| `5677fd8e` | atomic biological sleep and consolidation |
| `6347fc80` | evolvable cognition policy and configurable cognitive economics |
| `e29ad762` | exact acquired-state persistence and explicit founder projection |
| `49c888cc`, `07e8f8e8` | anti-drift source instructions and player-facing topology wording |

## Manual review questions

1. Do gameplay and Era1 actually call the same causal cognition transaction?
2. Is the canonical world organism record the sole biological authority?
3. Does focal attention obtain richer grounded data under phenotype bounds?
4. Are concepts and unresolved gaps target-specific and causally active before decision?
5. Does prediction use stable meanings, categorical actions, interaction capacity, and predecision consequences?
6. Do all six motor channels retain their own commands while sharing one physical outcome and one honest modulator?
7. Are homeostatic, reward, value, RPE, pain, injury, novelty, residual, and social fields measured and semantically distinct?
8. Is synaptogenesis sparse, bounded, event-nominated, and computationally active?
9. Do dendritic branches implement real nonlinear conjunctions and bounded allocation?
10. Is sleep one atomic CPU/GPU/memory/predictor/topology/structural transaction on biological cadence?
11. Are architecture policies heritable while acquired state is excluded, and is cognitive work independent of hardware?
12. Does exact save/load preserve every acquired state or reject incompatibility before mutation?
13. Did any repair introduce hidden host policy, forced action, fabricated reward, privileged sensing, or an accidental N2048 rule?

## Verification performed in this repair run

Only formatting, diff checks, targeted affected-crate `cargo check`, and one final workspace compile are authorized. No `cargo test`, GPU scenario, EI1 screen/corpus, ablation, benchmark, profile, soak, evolution run, or full journey belongs in this handoff.

Final compile: `cargo check --workspace --message-format=short` passed in 2 minutes 47 seconds. It emitted warnings in older world, GPU ABI, curated-founder, and game-app paths. The warnings are listed as review concerns rather than hidden or treated as behavioral failures.

## Reviewer inputs

- `ALife_v1.1_Repair_Ledger.md`
- `ALife_v1.1_ABI_Persistence_Change_Manifest.md`
- `ALife_v1.1_Stimulus_to_Response_Trace.md`
- `ALife_v1.1_Unresolved_Source_Defects.md`
- `docs/reviews/A-Life-Full-Project-Review-2026-08-08.html`

The next action is one independent source-level architecture review. Do not begin behavioral validation until that review returns a verdict and any source findings are repaired.
