# Legacy GPU test prerequisites — 2026-10-04

Source: main `482948ec116bad505cee0225cad74c1ab110acb7`, plus this focused test-only change.

## Correction

The six active-battery, five Era 1 (including learning), and one evolutionary
hardening GPU success tests unwrap constructors that deliberately return `CanonicalBiologyUnavailable` before GPU
initialization. Mark these twelve tests explicitly ignored until canonical
organism biology and sealed receptor-gated learning are integrated. Keep their
assertions and bodies intact for that future integration.

The production blocker remains unchanged. The active-challenge world-spec test,
the Era 1 request identity/world-family contract test, and the headless guard
covering all three legacy evaluators remain enabled. This does not enable
legacy evaluation, training, or promotion.

## Evidence and limits

Before the change, the selected active-battery, Era 1, learning and evolution
success tests all failed immediately with the expected `CanonicalBiologyUnavailable` error;
none reached GPU initialization. After the change, the four safe integration
test targets with `gpu-tests` enabled pass five CPU/contract tests and report
eleven explicit ignores. The selected blocked evolution test reports the twelfth
ignore; its separate GPU foundation regression is not run. The default-feature
headless legacy guard also passes.
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

The controlling architecture is unchanged. This work touches only four
training integration test files and this dated report; it has no file overlap
with the contact, newborn, GeneForge, or island/art changes.
