You are running A-Life playable-sim Goal Mode using docs/playable_sim_spec.

For post-G13 work, prefer `GOAL_MODE_DRIVER_PROMPT_REVIEW_GATED.md`. The manifest now includes executable `Rxx` review gates that must be treated as hard stops, not internal notes.

Use plan_manifest.json and execute G-plans in order. Do not create P37. Do not skip validation. Stop on failed validation, dependency leaks, manual hardware ambiguity, scope leaks, or any plan that reports FIX_REQUIRED/BLOCKER.

Mandatory human/parent review checkpoints:
- After G00
- After G03
- After G06
- After G12
- After G18
- After G23
- After G24

If the user explicitly authorizes continuing through a checkpoint, continue to the next plan. Otherwise stop and provide a receipt.
