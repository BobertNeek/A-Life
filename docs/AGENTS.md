# docs/ AGENTS.md

This subtree controls the project specification, architecture decisions,
handoff prompts, and future-compatibility notes.

Authoritative files:

- `master_spec.md` is the controlling engineering specification.
- `architecture_decisions.md` records non-negotiable ADRs.
- `schooling_and_teacher_architecture.md` controls teacher and schooling boundaries.
- `future_research_compatibility.md` is non-requirements future scope.
- `codex_handoff_prompt.md` is an operational prompt, not a replacement for the spec.

Rules:

- Do not introduce Unity, C#, or HLSL production requirements.
- Do not describe `Standard2048` as the global brain shape; it is only a reference tier.
- Keep internal SLM and external teacher LLM boundaries separate.
- Keep Graphify and DOX as developer tooling, not runtime dependencies.
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for
  legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented
  causal and performance gates.
- When docs change architecture, update `architecture_decisions.md` or explain why no ADR change is needed.
