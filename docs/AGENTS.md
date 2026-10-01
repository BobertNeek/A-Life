# Documentation instructions

Root [AGENTS.md](../AGENTS.md) applies. This subtree contains the controlling
architecture, current implementation guides, plans, and dated evidence.
Keep those roles explicit.

- `architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative A-Life architecture. Use the stable `AOA-*` IDs in
  `architecture/requirement_registry.csv` for compliance reports.
- Change that architecture only through an explicitly authorized, versioned
  revision. Update derived guides when their described state changes.
- `../README.md` is the entry point. `VISION.md`, `STATUS.md`, `ROADMAP.md`,
  `DEVELOPMENT.md`, and `EVIDENCE.md` describe the project and its evidence.
  `ARCHITECTURE.md` and `REFERENCE.md` describe the implementation. None defines
  a second architecture authority.
- Put codebase compliance in dated, source-bound reports, not the controlling
  architecture. Historical evidence certifies only its original source.
- Keep world enumeration score-free and archive-before-GPU-insertion/retirement
  ordering explicit in affected docs.
- Keep teacher input perception-only. The private local SLM cannot select actions,
  targets, rewards, or weights.
- Keep genetic inheritance separate from lifetime weights, memories, learned
  language, teacher-private state, and SLM-authored state.
- Label missing or incomplete evidence `Unknown` or `Blocked`. UI registration,
  screenshots, synthetic clicks, tables, and green unit tests alone do not prove
  causal gameplay.
- Replace superseded work orders with their useful outcomes and unresolved
  questions. Git history retains the original documents.
