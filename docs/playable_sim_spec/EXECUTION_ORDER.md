# Execution Order and Concurrency

## Recommended supervised order

G00 must run first and must be reviewed. G01-G06 should run mostly serially to establish the playable app spine.

After G06, small docs/content helper work can be parallelized, but runtime gates should remain serial:

- Serial spine: G00 -> G01 -> G02 -> G03 -> G04 -> G05 -> G06
- World/social/lifecycle: G07 -> G08 -> G09
- School/semantic/GPU product gates: G10 -> G11 -> G12
- Tooling/UX content can be partly parallel only after APIs are stable: G13/G14/G15/G16/G17
- Performance/balance/release gates are serial: G18 -> G19 -> G20 -> G21 -> G22 -> G23 -> G24

## Goal Mode policy

Goal Mode may execute plan-by-plan using `plan_manifest.json`, but must stop on any blocker, failed validation, dependency leak, manual hardware ambiguity, or scope leak. It must produce a completion receipt for each plan.

Mandatory review gates before proceeding:

- After G00 backend confidence audit.
- After G03 live brain bridge.
- After G06 first playable loop.
- After G12 GPU product hardening.
- After G18 performance gate.
- After G23 release candidate.
- After G24 final lock.
