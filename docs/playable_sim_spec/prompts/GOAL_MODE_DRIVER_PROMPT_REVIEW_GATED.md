You are running A-Life playable-sim Goal Mode using docs/playable_sim_spec.

Use plan_manifest.json as the execution chain. Review gates with IDs starting with `R` are executable plans and hard stops, not advisory notes.

Rules:
- Do not create P37.
- Do not skip validation.
- Do not continue past an R gate without explicit user authorization after the R gate receipt.
- Stop on failed validation, dependency leaks, manual hardware ambiguity, scope leaks, or any plan that reports FIX_REQUIRED/BLOCKER.
- Use Windows PowerShell validation wrappers on Windows. Do not run plain `bash scripts/check.sh` on Windows.

Current hard review gates:
- R13 after G13 and before G14. This retrospectively audits G01-G13 and the missed G03/G06/G12 checkpoints.
- R18 after G18 and before G19.
- R23 after G23 and before G24.
- R24 after G24. This is the final playable-sim roadmap lock review and has no next implementation plan by default.

When the next manifest item is an R gate:
1. Execute only that review gate.
2. Produce its required review receipt.
3. Stop even if the verdict is PASS.
4. State the next implementation plan only as gated future work requiring explicit user authorization.

When the next manifest item is a G plan:
1. Execute only that G plan.
2. Validate as required by the plan.
3. Produce its required completion receipt.
4. Follow the manifest next-chain, but stop if the next item is an R gate unless the user explicitly instructs you to run that review gate.
