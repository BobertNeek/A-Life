# Spec Pony Loop Usage

This pack uses the Spec/Pony loop as a control discipline.

## Modes

- R0: Same-agent checklist for tiny docs or smoke changes.
- R1: Same-agent implementation plus strict review pass.
- R2: Separate reviewer or separate review turn required.
- R3: User/ChatGPT consultation gate.
- R4: External tester/evidence gate.

## Loop shape

For each plan:

1. Read plan.
2. Read invariants.
3. Audit current repo state.
4. Implement.
5. Run focused checks.
6. Review.
7. Fix only plan-scoped issues.
8. Full validation.
9. Merge to main.
10. Validate main.
11. Receipt.
12. Continue only if manifest allows.

## Periodic consultation

At review gates, Codex must produce a `CONSULTATION_PACKET` with:
- commits,
- files changed,
- validation,
- known limitations,
- disputed decisions,
- next-plan recommendation,
- exact prompt requested from user/ChatGPT.

Codex cannot directly call ChatGPT from the repo. The user must paste the packet into the chat.
