# Changelog

## v1.5 — Compact modular skill entrypoint

Changed:

- Replaced the large single-file `SKILL.md` with a compact root entrypoint.
- Moved detailed rules into targeted `modules/` files.
- Added an explicit compact loading protocol: do not dump/read every module/template at startup.
- Preserved the v1.4 feature set while reducing always-loaded instruction size.
- Added a README explanation for the Codex terminal truncation problem.

Why:

- v1.4's `SKILL.md` was large enough that Codex often reported terminal output truncation when re-reading it.
- The skill should guide execution, not consume the run budget by making the agent re-read a 58 KB contract.

New module layout:

```text
modules/modes-and-workflow.md
modules/loop-goals-and-recipes.md
modules/review-and-subagents.md
modules/external-intake.md
modules/github-issue-loop.md
modules/command-classifier.md
modules/verification-and-finalization.md
```

Compatibility:

- Keeps v1.4 modes, loop recipes, review classes, external reviewer intake, GitHub/local issue loop, command classifier, artifact lint, and final receipt.
- Existing templates remain available under `templates/`.

## v1.4 — Bounded autonomous loop recipes

Added loop goal contracts, seven loop recipes, prompt seeds, day-zero guardrails, judge-loop rubrics, context/cost budgets, and progress logging templates.

## v1.3 — External reviewer/user intake

Added vendor-neutral intake for CodeRabbit, Greptile, Gemini/Jules, Gemini CLI/GitHub Actions, CI bots, human issues/comments, and pasted review output.

## v1.2 — Mode gating and issue hygiene

Added Direct Fix, Micro-Spec, Full Spec Loop, PR Review Mode, GitHub Issue Loop, and Automation Mode; made GitHub issues an escalation route rather than default.
