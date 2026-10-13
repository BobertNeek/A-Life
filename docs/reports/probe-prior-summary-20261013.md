# Compact probe prior diagnostics — 2026-10-13

Target base: `codex/swappable-priors-20261010` at
`5d5d296784581fe530d999e0b2b78a8d66f984b9`. Both night scripts and their
existing tests match the audited PR38 head `791df127fec5ba1f242e353906a7f5f6ea5bed75`
byte-for-byte; the reporting repair does not depend on PR38's replay changes.

The Grab probe, its held-food retention probe, and the held-food night probe
computed `semantic_prior_diagnostics` locally but omitted it from their compact
assessment summaries. All three now retain the existing five-field diagnostic
shape: failure policy, warnings, decision frames, frames with a prior, and frames
without a prior. Existing behavioral outcome, directory, and decision fields are
unchanged. The original `pilot.json` remains untouched.

Coverage counts come only from the optional prior receipt. Missing or malformed
counts serialize as `null` (Unknown), rather than invented zero coverage. Valid
nonnegative counts, including measured zero, remain unchanged. Episode decisions
do not stand in for unknown prior counts. Warnings remain separate from behavioral
outcomes under `log_and_continue_without_prior`; missing or unavailable priors do
not reject an otherwise valid probe or waive genuine selectable-action evidence.

`PriorDecisionInput` declares organism, sequence, and tick as `u64` wire values.
The diagnostic validates those types and ranges before hashing decision
identities. Malformed list/dict identities previously raised `TypeError`; invalid
identities now produce the existing identity warning and leave behavior intact.
Valid identity values, including zero and the maximum `u64`, remain unchanged.

Validation:

```text
python -m unittest discover -s scripts -p 'test_probe_prior_summaries.py' -v
python -m unittest discover -s scripts -p 'test*food_night.py' -v
```

Nine focused CPU tests cover 117 validator/fixture combinations across all three
summary paths. All passed, along with the existing 28 night-runner tests. The
initial seven-test suite failed on the source base with 48 missing-summary-key
errors before the repair. Eight original list/dict identity failures were also
reproduced across both scripts. No learner, GPU, or provider was launched. Full Cargo checks and live
training/provider delivery are not established by these receipt fixtures.

The existing CI and fast-gate workflows did not run Python night-runner tests.
CI now includes a dedicated `Probe prior receipt diagnostics` job that discovers
the new focused suite and both existing night-runner suites with the commands
above. A passing Rust/host fast-gate check alone does not certify this regression.
