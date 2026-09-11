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
| Luna max / N013 | Bound optional patch history | codex/recovered-n013-20260910 | Correction requested | Preserve every required replay patch and latest consumers | None | 2026-09-11 01:01Z |
| Luna max / N002 | Organ-local cognitive debit | codex/recovered-n002-20260910 | Running | Implement accepted packet | None | 2026-09-11 01:03Z |

| Luna max / N019 | Reach and ownership | codex/recovered-n019-20260910 | Setup pending | Implement from corrected N018 base | None | 2026-09-11 01:03Z |

## Handoffs and requests

| From | To | Request | Evidence or files | Status | Updated (UTC) |
| --- | --- | --- | --- | --- | --- |
| _No active handoffs._ |  |  |  |  |  |

## Decisions and notices

| Date (UTC) | Agent | Decision or notice | Related files |
| --- | --- | --- | --- |
| 2026-09-11 | Bob | Supervisor alone updates this board and integrates accepted commits. No Cargo/build/GPU commands. Preserve founder n512_candidate_live.rs WIP. | .worktrees/initial-founder-training-20260907/docs/superpowers/plans/2026-09-10-recovered-review/README.md |

## Completed work

| Date (UTC) | Agent / task | Result | Commit or evidence |
| --- | --- | --- | --- |
| 2026-09-11 | Bob review / N001 | Integrated after full manual diff and caller review; static check only; Rust unrun | 6e3dd8ed |
| 2026-09-11 | Bob review / N018 | Corrected failed test assumption before accepting primitive mapping; static only | 9da8091e + c3e61cfc |
