# S11 Next Stage Roadmap

This roadmap is not an implementation plan. It is a handoff menu for the user
after S11. Do not create S12 automatically, do not create G25, do not create
P37, and do not start a new phase without explicit user instruction.

## If The User Chooses External Playtest First

- Use `docs/productization/S10_EXTERNAL_TESTER_CHECKLIST.md`.
- Keep local evidence under `target/playtest_evidence/S11/` or another
  user-approved untracked evidence directory.
- Collect tester machine details, command results, screenshots, and usability
  notes.
- Triage failures as bugs or product gaps before any tag decision.

## If The User Chooses Alpha Tag Review

- Rerun full validation on the exact main SHA to be tagged.
- Confirm no generated artifacts, logs, captures, or large tensors are tracked.
- Confirm release wording still says alpha and does not overclaim GPU or
  graphics readiness.
- Ask for explicit approval before creating or pushing the tag.

## If The User Chooses More Product Work

Possible future themes, each requiring a new explicit user instruction:

- Graphical UX stabilization and usability polish.
- Bug fixes found by external playtest.
- GPU hardware evidence and performance reporting.
- Installer or distribution automation.
- Balance and ecosystem iteration based on playtest feedback.

These are backlog themes only. They are not a hidden continuation chain.

## If The User Chooses No Action

- Leave main validated and pushed.
- Keep the current state classified as an alpha / external playtest candidate.
- Next plan remains None.

## Guardrails

- Do not create S12 automatically.
- Do not create G25.
- Do not create P37.
- Do not tag a release without explicit approval.
- Do not turn CPU fallback into a GPU performance claim.
- Do not turn manual graphical evidence into a broad release claim.
