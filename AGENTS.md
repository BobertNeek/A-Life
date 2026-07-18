# AGENTS.md - A-Life Root Instructions

Read `docs/master_spec.md` and `docs/architecture_decisions.md` before edits.

This file controls repository-wide agent behavior. Child `AGENTS.md` files in
`docs/` and each crate add local rules for that subtree. When rules conflict,
prefer the more specific local file unless it violates the project-wide
architecture decisions below.

Non-negotiable rules:

- Rust + Bevy + wgpu/WebGPU + WGSL only.
- No Unity.
- No HLSL production shaders.
- No fixed global 2048-neuron brain assumption.
- `Standard2048` is a reference tier only.
- Use scalable brain classes and sparse class-bucketed storage.
- Internal SLM is a private subconscious semantic prior.
- External teacher LLM teaches through ordinary perception.
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for
  legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented
  causal and performance gates.
- N2048 is the first trained foundation; N4096 remains research-only.
- Language token IDs are stable logical codes, never neuron indices or packed
  GPU offsets.
- Player, creature, and teacher speech enters as spatial perception; neural
  `Vocalize` payload selection remains GPU-authoritative.
- Archive every creature before GPU insertion and archive death before GPU
  retirement.
- Keep docs and local AGENTS.md files updated after meaningful changes.
- Prefer Graphify queries for architecture questions when installed and a graph
  exists.
- On Windows, never assume `bash` means Git Bash. Plain `bash` may invoke WSL
  and fail if WSL virtualization is unavailable. Use `scripts/check.ps1`,
  `scripts/check_core_boundaries.ps1`, and `scripts/docs_check.ps1`, or the
  explicit Git Bash path.
- Future post-R24 productization work should use `docs/productization_s_plans/`
  and must not create P37/G25/S12 automatically.

## graphify

Graphify is optional project tooling. It is installed project-scoped through
`.codex/` when available and may write an ignored `graphify-out/` directory.
Do not make Graphify a prerequisite for `cargo build`, `cargo check`, or
`cargo test`.

When the user types `/graphify`, invoke the `skill` tool with `skill: "graphify"` before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `scripts/graphify.sh update` or `graphify update .` to keep the graph current when Graphify is installed.
