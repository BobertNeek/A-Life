# Productization Acceptance Tests

The S-phase is complete only if S11 can classify the build honestly.

## Minimum graphical player path

- A persistent A-Life window opens on local graphics hardware.
- The window stays open until user close or smoke timeout.
- A tiny world is visible.
- A creature is visible.
- Food/resource/hazard markers are visible.
- Backend/fallback status is visible.
- Player/tester can pause, step, resume/run, select a creature, and read an inspector.
- Save/load UX works or is explicitly blocked.
- The app exits cleanly.

## Minimum headless fallback path

- Existing P35 playground `run-all` passes.
- Release candidate smoke passes.
- Product QA smoke passes.
- Package smoke passes.
- Long-run balance smoke passes.
- Full validation passes.

## Evidence standard

- CLI-only success is not graphical evidence.
- Dry-run success is not graphical evidence.
- CPU fallback is not GPU performance evidence.
- Manual evidence must identify hardware/environment.
- Screenshots/logs should be stored under `target/playtest_evidence` and not tracked by default.

## Release decision standard

S11 must classify final state as one of:

- release candidate,
- alpha/prototype,
- not ready / blockers remain.

It must not tag a release without explicit user approval.
