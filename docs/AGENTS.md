# Documentation instructions

This subtree contains the controlling v2.0 architecture, current implementation
documentation, dated evidence, and historical design lineage. Keep each role
explicit.

## Controlling authority

- `architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative A-Life architecture.
- `architecture/requirement_registry.csv` is the stable `AOA-*` registry used
  by dated compliance reports.
- Preserve the v2.0 categories exactly: LOCKED GOAL, LOCKED CAPABILITY, LOCKED
  INVARIANT, LOCKED INTERFACE, REFERENCE MECHANISM, TUNABLE DEFAULT, DEFERRED
  CAPABILITY, and RESEARCH. Apply the conflict precedence in the controlling
  document.
- Earlier architecture specifications are superseded historical material.
  They may explain lineage but cannot override v2.0.

## Derived and historical documents

- `../README.md` is the project entry point.
- `VISION.md`, `STATUS.md`, `ROADMAP.md`, `DEVELOPMENT.md`, and `EVIDENCE.md`
  are non-normative project and evidence documents.
- `ARCHITECTURE.md` and `REFERENCE.md` describe the current implementation.
  They are non-normative and cannot define the target.
- The v1.1 brain specification, compliance matrix, and recovery documents are
  retained only as clearly marked history.
- Current codebase compliance belongs in dated reports using `AOA-*` IDs. Do
  not embed pass/fail status in the controlling architecture.
- Do not create a second active architecture authority set.

## Current implementation guardrails

The rules below protect the current code during ordinary changes. Rust types,
Bevy entities, processor placement, GPU layouts, constants, adapters, brain
classes, N2048 assumptions, tests, and fixtures remain implementation choices
unless v2.0 explicitly locks their semantics.

- Keep production neural execution GPU-authoritative WGSL. Do not introduce a live CPU neural shadow, parity gate, or automatic CPU neural fallback.
- Keep world enumeration score-free. The world remains authoritative for legality, targets, action execution, and outcomes.
- Keep teacher input perception-only and keep the private local SLM outside action, reward, target, and weight authority.
- Treat `Standard2048` as a reference profile, not a universal topology.
- Promote only N512, N1024, and N2048. Keep N4096 and larger classes research-only until their own evidence gates pass.
- Keep genetic inheritance separate from lifetime weights, memories, learned language, teacher-private state, and SLM-authored state.
- Require archive-before-GPU-insertion and archive-before-retirement ordering.
- Label missing or incomplete evidence `Unknown` or `Blocked`.
- Do not infer causal gameplay from UI registration, screenshots, synthetic clicks, tables, or green unit tests alone.
- Change the controlling architecture only through an explicitly authorized,
  versioned architecture revision. Update derived implementation, status, or
  evidence documents when their described state changes.
