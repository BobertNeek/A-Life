# A-Life Codex completion plan pack v1

This pack tells Codex how to move the A-Life repository from the current scaffold to a complete implementation with minimal human handholding.

Start with:

- `START_HERE_FOR_USER.md` if you are the human operator.
- `prompts/CODEX_MASTER_PROMPT.md` if you are giving instructions to the main Codex agent.
- `ORDER_AND_CONCURRENCY.md` for branch scheduling.
- `plans/` for individual implementation plans.
- `plan_manifest.json` for machine-readable dependencies and branch names.

The plan pack is intentionally strict. The early plans lock down contracts before runtime features. This prevents the project from drifting into Bevy-specific, GPU-specific, or logging-specific types before the core cognitive model is testable.
