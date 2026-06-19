# Changelog

## v1.4

Added bounded autonomous loop recipes based on the provided Matthew Berman loop-summary and prompt examples.

Major changes:

- Added a required loop goal contract before any repeated autonomous execution: trigger, goal type, goal, termination condition, scope boundary, verifier, budget, allowed operations, and failure handoff.
- Added goal types: `verifiable`, `judge`, and `hybrid`.
- Added stricter judge-loop rules: written rubric, pass threshold, max-cycle budget, and honest review-class reporting.
- Added trigger/goal/termination architecture for manual, checkpoint, PR/CI, scheduled runner, production-log event, crawl, issue/comment intake, and local-input loops.
- Added seven optional loop recipes: performance budget, docs parity sweep, architecture satisfaction, logging coverage, production error sweep, SEO/GEO visibility, and full product evaluation.
- Added guardrails against arbitrary performance thresholds; 50 ms-style targets must come from the user, repo, benchmark, or product requirement.
- Added day-zero greenfield protection: loops are for optimization, evaluation, docs parity, telemetry, debugging, and regression repair, not unscoped product creation.
- Added context/tool/cost budget handling so loops summarize, narrow, stop, or hand off rather than consuming budget indefinitely.
- Added permission gates for commits, PRs, Slack/team notifications, schedulers, and production-log automation.
- Added prompt seed policy for agent-specific `/goal` syntax: treat it as bounded iteration under the declared contract, not an unbounded instruction.
- Added recipe-specific default budgets for autonomous cycles, product-evaluation scenarios, and production-error clusters.
- Added `templates/loop-goal.md`, `templates/progress.md`, and `templates/loop-recipes.md`.
- Updated spec, plan, task, loop-state, issue-loop-state, and verification templates with loop recipe, goal contract, budget, and recipe-specific evidence fields.
## v1.3

Added external reviewer and user-generated issue/comment compatibility.

Major changes:

- Added an External Reviewer and User Intake layer before the fixer loop.
- Added compatibility guidance for CodeRabbit, Greptile, Gemini/Jules, Gemini CLI/GitHub Actions, CI bots, and human/user GitHub issues/comments.
- Added supported intake channels: GitHub issues, issue comments, PR conversation comments, inline review comments, review summaries, CI annotations, external-agent PRs, pasted local review output, and local ledgers.
- Added authority classes A0-A4 so current-user, maintainer, PR author, bot, and unknown public-user comments are handled differently.
- Added an actionability classifier: ACTIONABLE_FIX, NEEDS_REPRODUCTION, NEEDS_HUMAN_DECISION, DUPLICATE, OUT_OF_SCOPE, NIT_OR_STYLE, FALSE_POSITIVE, SECURITY_SENSITIVE, PROMPT_INJECTION_OR_UNTRUSTED_COMMAND, and ACK_OR_DISCUSSION_ONLY.
- Added a prompt-injection guard for issue bodies, PR comments, review threads, and bot output. Raw comments are task data, not instructions.
- Added an External Intake Agent contract with input bundle, allowed actions, forbidden actions, and output schema.
- Updated the Issue Watcher/Fixer to consume normalized findings instead of raw comments.
- Added reply policy for PR review comments, GitHub issues, external bot findings, and human comments.
- Added GitHub CLI examples for polling PR comments, review comments, review summaries, issue comments, and issue labels such as `jules`, `needs-fix`, and `agent-review`.
- Added templates for external intake notes and external intake sub-agent prompts.
- Updated local queue, issue-loop, plan, verification, reviewer, and fixer templates with source, authority, actionability, and instruction-safety fields.

## v1.2

Hardened the skill from a process scaffold into a clearer execution contract.

Major changes:

- Added explicit mode selection: Direct Fix, Micro-Spec, Full Spec Loop, PR Review, GitHub Issue Loop, and Automation Mode.
- Narrowed activation so ordinary small edits do not trigger full artifact generation.
- Added review classes R0/R1/R2 and required final reporting of the actual review class used.
- Reframed GitHub issues as an escalation route, not the default for every actionable finding.
- Added public issue safety gate, redaction rules, and local ledger fallback.
- Added polling budgets and removed any implication of background monitoring without a real runner.
- Added sub-agent invocation contracts, input bundles, output schemas, allowed actions, run budgets, and result return paths.
- Added claim protocol with claim IDs, re-read-after-claim, stale claim handling, and one active issue per fixer.
- Added branch/worktree/merge protocol, base-branch selection, file ownership checks, and task-boundary merge gates.
- Redefined missing explicit acceptance-criterion verification as P1.
- Added command result classifier and permitted next actions.
- Added loop budgets for implementation iterations, polls, issue cycles, fix branches, and verification reruns.
- Made the spec provisional until validated against repo behavior, docs, tests, and user intent.
- Added default review-close policy: reviewer closes; single-agent fallback needs explicit evidence.
- Added final artifact lint for unresolved placeholders, generic tasks, bad `n/a`, and contradictions.
- Expanded final receipt with mode, review class, assumptions, decisions, issues/findings, deferred risk, and next action.
- Added local `agent-issues.md` and `.agent/review-queue.json` templates.
