# A-Life agent instructions

For changes to organism behavior, cognitive contracts, or architecture, consult the relevant sections of
`docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`.
Use `docs/architecture/requirement_registry.csv` to trace requirements.

These instructions apply repository-wide. Child `AGENTS.md` files add local
rules. Prefer the more specific rule unless it conflicts with v2.0.

## Architecture authority

- A-Life v2.0 is the single normative architecture. Earlier specifications,
  recovery designs, plans, and compliance matrices are historical material.
- Preserve the v2.0 categories: LOCKED GOAL, LOCKED CAPABILITY, LOCKED
  INVARIANT, LOCKED INTERFACE, REFERENCE MECHANISM, TUNABLE DEFAULT, DEFERRED
  CAPABILITY, and RESEARCH. Follow its conflict precedence.
- Current types, engines, GPU layouts, brain sizes, constants, and other
  implementation choices are not architecture unless v2.0 explicitly locks
  their semantics.
- Record compliance in dated reports citing stable `AOA-*` requirement IDs,
  not in the controlling architecture.
- Report gaps between implementation and v2.0. Do not redefine the target to
  match the code or start an unrequested repair.

## Working style

Use the global working preferences. Read only documentation relevant to the touched behavior. Update affected docs and run the relevant existing checks. Preserve unrelated work.

If in doubt, WWCD — What Would Creatures Do? Use Creatures 3 / Docking Station to guide open gameplay decisions.

## Current implementation guardrails

These constraints protect the current implementation. They do not amend v2.0.
Changes to them require an authorized task that covers the affected behavior.

- No HLSL production shaders.
- No fixed global 2048-neuron assumption. `Standard2048` is a reference tier.
  Use scalable brain classes and sparse class-bucketed storage.
- The internal SLM is a private subconscious semantic prior. External teachers
  teach through ordinary perception.
- Production neural execution is GPU-authoritative WGSL. Do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Promote only N512, N1024, and N2048 until larger tiers pass their documented
  causal and performance gates.
- Player, creature, and teacher speech enters as spatial perception. Neural
  `Vocalize` payload selection remains GPU-authoritative.

## Windows commands

Use `scripts/check.ps1`, `scripts/check_core_boundaries.ps1`, and
`scripts/docs_check.ps1` for the relevant checks. If calling Bash directly,
use the explicit Git Bash path. Plain `bash` may invoke unavailable WSL.

## Graphify

Graphify is optional and must not be a prerequisite for Cargo commands.
On Windows, use `scripts/graphify.ps1` to find the installed executable even
when it is absent from PATH.

- Run `scripts/graphify.ps1 update --no-cluster` to build or refresh the code
  graph. This performs structural extraction without an LLM.
- Once `graphify-out/graph.json` exists, use
  `scripts/graphify.ps1 query "<question>"`, `path "<A>" "<B>"`, or
  `explain "<concept>"` for focused questions.
- Use `graphify-out/wiki/index.md` for broad navigation when available. Read
  `graphify-out/GRAPH_REPORT.md` only when focused queries are insufficient.
- Generated files under `graphify-out/` are ignored. Check source files
  directly when graph results are stale or incomplete.
- Refresh an existing graph when this task relies on graph results that the changes made stale.
- When the user requests `/graphify`, follow the available Graphify skill.
