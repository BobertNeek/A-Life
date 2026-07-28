# Human / ChatGPT Consultation Protocol

Codex cannot directly consult ChatGPT from inside the repository. It must stop and emit a consultation packet.

## When to consult

- Every CAR review gate.
- Any R3/R4 plan.
- Any architecture decision that touches `alife_core`.
- Any decision to reduce CPU shadow checks.
- Any release/tag decision.
- Any missing external human evidence.
- Any failed validation that requires more than local repair.

## Consultation packet format

```text
CONSULTATION_PACKET
Current main:
Plan/gate:
Branch:
Commits:
Files changed:
What changed:
Validation:
User-facing evidence:
Known limitations:
Findings:
Decision needed:
Recommended next prompt:
```

The user should paste this packet into ChatGPT, ask for review, then paste the answer back to Codex.
