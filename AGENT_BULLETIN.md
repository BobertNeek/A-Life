# Agent bulletin board

Operational notes for agents and sub-agents working in this repository.

This file coordinates work. It does not override `AGENTS.md`, the controlling
architecture, user instructions, or dated evidence reports.

## How to use this board

1. Read the board before starting work and before handing work to another agent.
2. Claim work in **Active work** with your agent name, exact scope, branch or
   worktree, status, next action, blocker, and a UTC timestamp.
3. Keep one row per task. Update the row when the status, owner, scope, or next
   action changes.
4. Add a handoff under **Handoffs and requests** when another agent must act.
   Include the files, evidence, and decision needed.
5. Move finished work to **Completed work** only after recording the commit or
   other durable evidence.
6. Keep secrets, credentials, large logs, and speculative claims out of this
   file. Link to a repository path or ignored receipt instead.
7. Use `YYYY-MM-DD HH:mmZ` timestamps so entries remain comparable across
   machines and worktrees.

## Active work

| Agent / task | Scope | Branch or worktree | Status | Next action | Blocker | Updated (UTC) |
| --- | --- | --- | --- | --- | --- | --- |
| Bob / recovered review supervisor | N001-N023 repair plans; C001-C005 triage | initial-founder-training-20260907 | Dispatching Luna max in isolated worktrees | Review every worker diff before integration; no Cargo | None | 2026-09-11 01:01Z |
| Luna max / N020 | Carried object motion | codex/recovered-n020-20260910 | Correction requested | Detach at terminal death before retirement | Bob owns routine Git | 2026-09-11 04:17Z |
| Luna max / N014 | Runtime and core evaluation unknown regime | codex/recovered-n014-20260910 | Scope approved | Preserve six bins and derive unknown exposure | Bob owns routine Git | 2026-09-11 04:17Z |
| Luna max / N009 | Neuroemitter accounting | codex/recovered-n009-20260910 | Running | Implement bounded packet | Bob owns routine Git | 2026-09-11 04:17Z |

## Handoffs and requests

| From | To | Request | Evidence or files | Status | Updated (UTC) |
| --- | --- | --- | --- | --- | --- |
| _No active handoffs._ |  |  |  |  |  |

## Decisions and notices

| Date (UTC) | Agent | Decision or notice | Related files |
| --- | --- | --- | --- |
| 2026-09-11 | Bob | Supervisor alone updates this board and integrates accepted commits. No Cargo/build/GPU commands. Preserve founder n512_candidate_live.rs WIP. | .worktrees/initial-founder-training-20260907/docs/superpowers/plans/2026-09-10-recovered-review/README.md |

Routine Git approvals belong to Bob. Workers hand off sandbox-blocked Git operations in their final response. Bob performs scoped Git through permitted supervisor tools; sandbox settings remain unchanged.

## Completed work

| Date (UTC) | Agent / task | Result | Commit or evidence |
| --- | --- | --- | --- |
| 2026-09-11 | Bob review / N001 | Integrated after full manual diff and caller review; static check only; Rust unrun | 6e3dd8ed |
| 2026-09-11 | Bob review / N018 | Corrected failed test assumption before accepting primitive mapping; static only | 9da8091e + c3e61cfc |
| 2026-09-11 | Bob review / N002 | Pro-rata organ debit accepted after full diff and rollback review; static only | c1670704 |
| 2026-09-11 | Bob review / N013 | Bounded replay retention accepted after backend ring/caller/rollback review; static only | f91dd9d7 + b11a1ed9 |
| 2026-09-11 | Bob review / N003 | Elapsed capped organ upkeep accepted; static only | 06307185 |
| 2026-09-11 | Bob review / N019 | Corrected reach/lifetime contact evidence accepted; static only | 61fd9947 + 4e84084a |
| 2026-09-11 | Bob review / N012 | Already fixed by N013; independently traced current path; no duplicate changes | b11a1ed9 |
| 2026-09-11 | Bob review / N017 | Finalized guard and atomic staged observation accepted; static only | d86ebe50 |
| 2026-09-11 | Bob review / N007 | Material-only validation accepted after correcting test ordering; static only | 87a045e7 + 378b447b |
| 2026-09-11 | Bob review / N008 | Generic and contextual chemical ranges accepted after full caller review; static only | 4b908941 |
| 2026-09-11 | Bob review / N010 | Shared eligibility and actor partner scan accepted after correction; static only | 71393bf7 + f9cc324e |
| 2026-09-11 | Bob review / N015 | Removed ungrounded communication inference; source and regression reviewed; static only | 5548be00 |
| 2026-09-11 | Bob review / N004 | Corrected causal regression accepted; static only | e1092f0b + cb97e0fd |
| 2026-09-11 | Bob review / N011 | Bounded readiness and current pair checks accepted; static only | 3e3b1754 |
| 2026-09-11 | Bob review / N016 | Grounded avoidance denominator accepted; static only | c6bc35cd |
| 2026-09-11 | Bob review / N005 | Bounded persistent emission catch-up and one-shot events accepted; static only | 120c71df |
