# A-Life Playable Sim Release Tag Proposal

Status: proposal only. No tag was created by G24.

## Proposed Candidate Name

`playable-sim-rc1`

## Required Preconditions

Before creating a tag, verify:

- G24 validation passed on `main`.
- R24 final review passed on `main`.
- `main` equals `origin/main`.
- Working tree is clean.
- `docs/playable_sim_spec/FINAL_PLAYABLE_SIM_STATUS_REPORT.md` and
  `docs/playable_sim_spec/review_gates/R24_REVIEW_REPORT.md` accurately record
  limitations.
- Manual GPU and graphics evidence are either measured and attached to the
  release notes, or explicitly marked manual/unknown.
- The user explicitly requests tagging.

## Suggested Commands

Replace `<validated-main-sha>` with the exact R24-validated main SHA:

```powershell
git tag -a playable-sim-rc1 <validated-main-sha> -m "A-Life playable sim RC1"
git push origin playable-sim-rc1
```

## Release Notes Baseline

- Supported path: headless CPU playground and deterministic product smoke suite.
- Manual paths: graphical playground and GPU hardware performance.
- Known limitations: see `docs/playable_sim_spec/known_issues.md` and
  `docs/playable_sim_spec/FINAL_PLAYABLE_SIM_STATUS_REPORT.md`.
- Backlog/issues: see `docs/playable_sim_spec/POST_RELEASE_BACKLOG.md`.

## Explicit Non-Action

G24 does not tag, package, sign, publish, or start a new implementation plan.
