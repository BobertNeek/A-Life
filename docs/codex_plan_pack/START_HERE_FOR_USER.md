# Start here - instructions for you

This zip is designed to reduce your handholding. Put it where Codex can read it, then make Codex follow the plan pack rather than improvising.

## Recommended setup

1. Unzip this pack into the repository under `docs/codex_plan_pack/`.
2. Commit it as a planning-only commit, or keep it uncommitted and point Codex at the path. Committing is cleaner because every Codex branch can read the same plan files.
3. Start with the prompt in `prompts/CODEX_MASTER_PROMPT.md`.
4. Tell Codex to execute P00 first.
5. After P04 is complete, use concurrent subagents for the branchable Group 1 plans listed in `ORDER_AND_CONCURRENCY.md`.

## Single-agent mode

Use this when you want one Codex instance to work through the project sequentially. Paste `prompts/CODEX_MASTER_PROMPT.md` and add:

> Begin with P00. Continue through the next unblocked plan after each successful completion. Stop only when a plan requires a branch merge that has not happened, a validation command fails and cannot be repaired locally, or the pack tells you to wait for concurrent branches.

This is simpler but slower.

## Multi-agent mode

Use this when you want faster progress.

1. Run P00-P04 serially first.
2. Create separate Codex sessions/branches for P05, P06, P07, P08, and P09.
3. In each branch, paste `prompts/CODEX_SUBAGENT_PROMPT.md` plus the relevant plan file.
4. Merge those branches into an integration branch.
5. Run P10 and P11 on the integration branch.
6. Split again where `ORDER_AND_CONCURRENCY.md` says it is safe.

## The prompt to give Codex

Use `prompts/CODEX_MASTER_PROMPT.md` for the main agent. Use `prompts/CODEX_SUBAGENT_PROMPT.md` for branch workers.

## What to require from Codex after each plan

Codex must produce a completion receipt with:

- Plan ID and branch name.
- Files changed.
- Public structs/APIs added or changed.
- Tests added or changed.
- Commands run and results.
- Invariants checked.
- Deviations from the plan.
- Known limitations.
- Exact next plan(s).

Do not accept vague summaries like "implemented the feature". Make it name files and tests.

## What not to let Codex do

Do not let it jump to Bevy, Avian, GPU shaders, D2NWG, SLM, or visual playground work before the core contracts and CPU reference loop are deterministic. That would recreate the same technical debt the specs are trying to avoid.

Do not let it put Bevy/wgpu types into `alife_core` for convenience. That is the fastest way to make the project hard to test and hard to port.

Do not let it use `ExperiencePatch` as a zero-copy log struct. Runtime cognition and packed logs have different requirements.
