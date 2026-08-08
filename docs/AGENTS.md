# Documentation instructions

This subtree describes the current A-Life project. Keep it concise, source-backed, and explicit about evidence limits.

## Authorities

- `../README.md` is the project entry point.
- `VISION.md` defines the aspiration and non-goals.
- `STATUS.md` records implemented, integrated, player-visible, and proven state.
- `ARCHITECTURE.md` owns the current component and authority map.
- `ROADMAP.md` orders the remaining work and acceptance gates.
- `DEVELOPMENT.md` owns supported developer workflows.
- `EVIDENCE.md` owns receipt rules and durable result interpretation.
- `REFERENCE.md` owns stable ABI, tier, inheritance, teacher, SLM, persistence, and archive invariants.

The Git history is the archive for superseded plans and specifications. Do not recreate a second active authority set.

## Rules

- Keep production neural execution GPU-authoritative WGSL. Do not introduce a live CPU neural shadow, parity gate, or automatic CPU neural fallback.
- Keep world enumeration score-free. The world remains authoritative for legality, targets, action execution, and outcomes.
- Keep teacher input perception-only and keep the private local SLM outside action, reward, target, and weight authority.
- Treat `Standard2048` as a reference profile, not a universal topology.
- Promote only N512, N1024, and N2048. Keep N4096 and larger classes research-only until their own evidence gates pass.
- Keep genetic inheritance separate from lifetime weights, memories, learned language, teacher-private state, and SLM-authored state.
- Require archive-before-GPU-insertion and archive-before-retirement ordering.
- Label missing or incomplete evidence `Unknown` or `Blocked`.
- Do not infer causal gameplay from UI registration, screenshots, synthetic clicks, tables, or green unit tests alone.
- When architecture changes, update `ARCHITECTURE.md`, `REFERENCE.md`, and the affected status or evidence statement in the same change.
