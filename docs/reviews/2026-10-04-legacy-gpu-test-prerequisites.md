# Legacy GPU test prerequisites — 2026-10-04

Source: main `482948ec116bad505cee0225cad74c1ab110acb7`, plus this focused test-only change.

## Correction

The six active-battery GPU success tests and four Era 1 GPU success tests unwrap
constructors that deliberately return `CanonicalBiologyUnavailable` before GPU
initialization. Mark these ten tests explicitly ignored until canonical
organism biology and sealed receptor-gated learning are integrated. Keep their
assertions and bodies intact for that future integration.

The production blocker remains unchanged. The active-challenge world-spec test,
the Era 1 request identity/world-family contract test, and the headless guard
covering all three legacy evaluators remain enabled. This does not enable
legacy evaluation, training, or promotion.

## Evidence and limits

Before the change, the selected active-battery and Era 1 success tests both
failed immediately with the expected `CanonicalBiologyUnavailable` error;
neither reached GPU initialization. After the change, those three integration
test targets with `gpu-tests` enabled pass three CPU/contract tests and report
ten explicit ignores. The default-feature headless legacy guard also passes.
Optional-feature all-targets Cargo check and strict Clippy, formatting, static
core boundaries, and documentation assertions pass.

No GPU execution or training run was performed. Production GPU behavior and
legacy evaluator capability remain **Blocked** pending canonical-biology
integration; compiling or ignoring these tests provides no capability evidence.
This change adds no production runtime work, allocations, or scans. No runtime
performance measurement was needed or made.

## Architecture trace

- AOA-INV-009 / AOA-AUTH-003: preserve the same canonical organism step and its
  existing integration blocker; do not accept independently advancing synthetic
  physiology as evaluator evidence.
- AOA-BIO-015 / AOA-BIO-026: enabling these tests requires organism-owned derived
  drives and measured biological consequences, rather than default deltas.
- AOA-LEARN-002: the prerequisite includes sealing and applying or discarding
  the matching eligibility transaction.

The controlling architecture is unchanged. This work touches only the two
training integration test files and this dated report; it has no file overlap
with the contact, newborn, GeneForge, or island/art changes.
