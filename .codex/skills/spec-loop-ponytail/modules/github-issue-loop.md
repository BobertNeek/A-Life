# GitHub Issue / Local Queue Loop

Use this module only in Mode 4 or Mode 5, or when the user explicitly asks for issue-backed reviewer/fixer behavior.

## Polling policy

Interactive mode is not background monitoring.

Default interactive polling:

```text
Start of run: 1 poll if issue/comment context matters
After reviewer pass: poll only if reviewer created/updated findings
Before final: 1 poll if issue loop was active
Maximum: 3 polls per active run unless user requests more
```

Mode 5 polling requires an actual runner. Runner config must declare cadence, permissions, timeout, budget, notification route, and failure handling.

## Claim protocol

GitHub issue labels/comments are not atomic locks. Use a best-effort claim protocol:

```text
Claim ID: <agent-or-run>/<branch>/<timestamp>
Add claim comment and label/tag if policy allows.
Immediately re-read issue/queue.
If another valid claim exists, release or mark blocked.
Only one active claim per actor by default.
Claim expires after configured stale timeout or explicit release.
```

Do not claim issues with active Jules/Gemini/CodeRabbit/Greptile/human ownership unless user directs handoff or repo policy says the claim is stale.

## Branch, worktree, and base selection

- If fixing review issue from active feature/PR branch: branch from that feature/PR branch.
- If fixing mainline issue: branch from default branch.
- Use naming such as `fix/issue-<number>-<slug>` or `fix/review-<finding-id>-<slug>`.
- Parallel agents use separate worktrees/branches.
- Before final review, rebase or merge latest base when safe, then rerun checks.
- Do not merge automatically unless repo/user policy permits it.
- Clean worktrees only after final receipt or explicit merge/abandon decision.

## Issue creation/update route

Create GitHub issues only after external intake and public safety gate.

Suggested labels if repo policy allows:

```text
agent-review
agent-fixing
source:coderabbit
source:greptile
source:jules
source:gemini
source:human
source:ci
priority:p0|p1|p2|p3
status:blocked
status:needs-human
status:single-agent-verified
```

If labels do not exist and creating labels is not allowed, use title/body markers instead.

## Close policy

Default: fixer cannot close its own issue. Reviewer closes after checking close criteria.

Single-agent fallback: may close only when:

```text
- user/repo policy allows self-close, or the issue is local ledger only;
- evidence is attached;
- acceptance/close criteria are checked;
- label/status includes single-agent-verified when using GitHub.
```

Otherwise comment with close evidence and request review.

## Local ledger sync

Use `templates/agent-issues.md` or `templates/review-queue.json` when GitHub is unavailable, private/sensitive, or too noisy.

If GitHub access becomes available later:

- Migrate only durable unresolved items.
- Keep transient/private/sensitive items local unless user/repo asks otherwise.
- Mark migrated local items with GitHub URL/issue number.
