# S00 Next Stage Recommendation

Recommendation: **B. graphical stabilization phase**.

## Evidence Basis

The current repository is strong on the supported headless CPU path:

- Full validation passed with Windows-safe wrappers.
- 38 product/dev-facing CLI commands passed.
- The P35 full headless playground suite passed.
- The release-candidate and product-QA smokes passed.
- The GPU runtime smoke produced an honest CPU fallback report, not a false GPU performance claim.

The current graphical path is not yet a normal playable game:

- `scripts/run_graphical_playground.ps1` ran successfully, but it executed `visible-world-smoke` and exited.
- No persistent game window remained for Computer Use or manual interaction.
- Camera movement, creature selection, inspector overlay, pause/step/run, save/load UI, school/semantic panels, and GPU/status UI could not be tested through a graphical app surface.
- Computer Use itself was unavailable in this run due a plugin/runtime export error, but the shell and screenshot evidence still show that the current graphical launcher is a smoke command rather than an interactive game window.

## Why Not Release/Tag Now

Do not recommend release/tag for a normal player-facing game yet. The validated scope remains headless CPU playground plus deterministic product smoke suite. A normal player does not currently get a persistent visual world, camera controls, menus, or direct interaction loop from the graphical launcher.

## Recommended Next Stage

Run a focused graphical stabilization phase with these goals:

1. Make `scripts/run_graphical_playground.ps1` launch a persistent graphical app window.
2. Show a visible world with at least one creature, food/resource object, and basic status surface.
3. Add or expose player controls for camera movement, creature selection, pause, step, resume, and clean exit.
4. Expose inspector/debug, save/load, school/semantic, and GPU fallback/status surfaces as visible UI or explicitly defer them.
5. Repeat this S00 playtest after the graphical path can remain open long enough for screenshot and interaction evidence.

## Secondary Follow-Ups

- Fix or update the Computer Use plugin/runtime integration if future automated UI playtests should depend on it.
- Keep GPU performance in manual/unknown status until measured with hardware flags and an explicit report.
- Keep headless CPU validation as the correctness oracle while graphical stabilization proceeds.

## Not Recommended

- Do not create G25 or P37.
- Do not tag a release based on this evidence.
- Do not treat CLI smoke success as proof of normal-player graphical playability.
