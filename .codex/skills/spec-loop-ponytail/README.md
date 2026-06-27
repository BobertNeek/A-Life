# Spec Loop Ponytail v1.5

This is a compact, modular Codex-style skill for bounded software implementation loops.

It combines:

- Spec-driven execution when the work actually needs artifacts.
- Ponytail-style minimalism: smallest root-cause-safe diff, not process bloat.
- Explicit trigger/goal/termination contracts for autonomous loops.
- Review classes and sub-agent contracts.
- External reviewer/user intake for CodeRabbit, Greptile, Gemini/Jules, Gemini CLI, CI bots, and human GitHub issues/comments.
- GitHub/local issue-backed reviewer/fixer loops when durable tracking is useful.

## Why v1.5 exists

v1.4 kept most rules in a single `SKILL.md` file of roughly 58 KB / 875 lines. In Codex, that often caused terminal-output truncation when the agent reloaded the skill with commands like `Get-Content` or `cat`.

v1.5 fixes that by making `SKILL.md` a compact entrypoint. Detailed rules live in `modules/` and are loaded only when needed. Templates remain in `templates/` and should be read only when creating the corresponding artifact.

## Loading rule

Always load only `SKILL.md` first. Then load one or two targeted modules by task need:

```text
modules/modes-and-workflow.md
modules/loop-goals-and-recipes.md
modules/review-and-subagents.md
modules/external-intake.md
modules/github-issue-loop.md
modules/command-classifier.md
modules/verification-and-finalization.md
```

Do not dump all modules/templates into terminal output at startup.

## Recommended install shape

```text
.codex/skills/spec-loop-ponytail/
  SKILL.md
  README.md
  CHANGELOG.md
  modules/
  templates/
```

## Operational summary

Default to the lightest mode:

```text
Mode 0: Direct Fix
Mode 1: Micro-Spec
Mode 2: Full Spec Loop
Mode 3: PR Review Mode
Mode 4: GitHub Issue Loop
Mode 5: Automation Mode
```

Issue tracking is not default. Route findings through:

```text
inline fix -> PR comment -> local ledger -> GitHub issue
```

Autonomous loops require a loop-goal contract:

```text
trigger + goal type + verifier + termination + scope + budget + failure handoff
```

No fake background work: true periodic behavior requires a real Mode 5 runner.
