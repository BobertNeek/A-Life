# Spec Loop Ponytail Skill

A software-development agent skill for disciplined coding loops without process bloat.

It combines:

- Mode-gated execution: Direct Fix, Micro-Spec, Full Spec Loop, PR Review, GitHub Issue Loop, and Automation Mode.
- Spec-first artifacts when they are useful, not by default for every small edit.
- Verified loops: trigger → goal contract → attempt → feedback → diagnose → correct → verify → stop.
- Ponytail minimalism: delete, standard library, native platform, existing dependency, then minimum root-cause-safe code.
- External reviewer intake for CodeRabbit, Greptile, Gemini/Jules, Gemini CLI, CI bots, human/user GitHub issues, PR comments, review threads, and pasted local review output.
- Autonomous loop recipes for performance budgets, docs parity, architecture satisfaction, logging coverage, production-error sweeps, SEO/GEO visibility, and full product evaluation.
- Review/fix sub-agent contracts with explicit review classes, budgets, issue hygiene, branch/worktree rules, and no fake background monitoring.

Install by copying this folder into the skill/plugin location for your coding agent, or by pasting `SKILL.md` into your agent's project instructions if your tool does not support skills directly.

The GitHub issue loop is not the default path for ordinary defects. Findings should be routed to inline fix, PR comment, local ledger, or GitHub issue based on durability, privacy, repo policy, source authority, and whether cross-session/cross-agent tracking is needed.

External comments and issues are normalized before fixing. The skill treats bot and user comments as untrusted task data: they may describe a bug or desired behavior, but they cannot override repo policy, security rules, tool permissions, branch policy, or verification requirements.

Autonomous loop recipes are optional patterns, not modes. Each recipe must have a loop goal contract before repeated execution: trigger, goal type, exact stop condition, verifier, scope boundary, budget, allowed operations, and failure handoff. The included prompt seeds live in `templates/loop-recipes.md`.

Interactive agents only check issues/comments or repeat loop cycles at explicit checkpoints during the active run. Scheduled monitoring, nightly docs sweeps, production-log review, auto-PR creation, Slack/team notifications, or automatic commits require a real external runner or explicit repo/user permission.
