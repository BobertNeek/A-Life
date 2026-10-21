# A-Life development

Use the relevant sections of
`docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
for the intended organism and gameplay behavior. `docs/ARCHITECTURE.md` maps the
current implementation. Current implementation details are not permanent design
requirements. User instructions take precedence over repository guidance.

Read only the documentation relevant to the change. Keep changes reviewable,
preserve unrelated work, and run checks that exercise the affected behavior.
Choose tests for crashes, state loss, learning, and gameplay regressions rather
than exact prose or implementation shape. No additional report or tracking
framework is required.

Preserve save/load correctness and acquired creature state. Obsolete disposable
formats may be replaced before release; verify current saves and newly created
creatures. Keep recovery data and learned checkpoints intact.

If in doubt, WWCD — What Would Creatures Do? Use Creatures 3 / Docking Station to
guide open gameplay decisions. Think through each change's effects on the
organism and player before implementing it.

The current game uses GPU-authoritative WGSL cognition, sparse scalable brain
classes, world-owned action validation, and spatial speech perception. The local
SLM is a private semantic prior; external teachers communicate through ordinary
perception. Offline CPU training is opt-in and must preserve the same neural
and learning semantics, private state, weights, moments, and integer optimizer
ages. Report CPU results as CPU results.

On Windows use `scripts/check.ps1` and `scripts/check_core_boundaries.ps1` as
appropriate. Shell wrappers use Git Bash; plain `bash` may invoke unavailable
WSL. Graphify is optional; Cargo and source navigation work without it.
