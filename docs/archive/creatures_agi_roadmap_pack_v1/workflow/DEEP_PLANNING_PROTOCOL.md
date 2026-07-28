# Deep Planning Protocol

Use this before changing code.

1. Restate objective in one sentence.
2. Identify current baseline and relevant files.
3. Identify invariants and forbidden scope.
4. Identify smallest working slice.
5. Identify failure modes.
6. Choose workflow mode:
   - Direct Fix: one obvious small fix.
   - Micro-Spec: one narrow feature or doc/control surface.
   - Full Spec Loop: architecture or multi-crate feature.
   - Review Gate: audit and stop.
7. Create branch from clean main.
8. Implement smallest slice first.
9. Run focused tests.
10. Iterate only on failing evidence.
11. Run full validation.
12. Self-review against plan.
13. Merge only after review passes.
14. Validate main again.
15. Push and output receipt.

## Ponytail rule

The smallest verified diff wins. Do not add architecture just because the roadmap is large.

## Stop rules

Stop for user consultation when:
- review gate says FIX_REQUIRED/BLOCKER,
- architecture choice is ambiguous,
- validation would require weakening tests,
- hardware evidence is unavailable but necessary,
- task would create a new phase not in manifest,
- Codex is tempted to create S12/G25/P37.
