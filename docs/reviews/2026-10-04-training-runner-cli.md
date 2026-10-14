# Diagnostic runner CLI correction — 2026-10-04

Cassidy authorized this repair, committing and pushing. Base:
`482948ec116bad505cee0225cad74c1ab110acb7`. This report concerns command
construction and input validation; GPU evaluation and training are **Unrun**.

## Failure and correction

The dry plan attached `--request-token` to both vocabulary lessons. The
[Rust CLI](../../crates/alife_game_app/src/bin/train_n2048_care.rs) rejects that
flag for vocabulary production. Thus a complete two-arm run would terminate
before finishing its evaluation commands.

[The runner](../../scripts/training_dose_diagnostic.py) now attaches the controlled
request only to vocabulary reception. Production keeps its ordinary scenario
utterance. Evaluation tick counts must be 1–2048, and request tokens must be
1, 2, 9, 13, 15 or 16, matching
[the evaluator](../../crates/alife_game_app/src/foundation_training.rs).
Unsupported inputs fail before creating the output directory. Seeds retain
their positive-u64 and matching-founder requirements.

The two epoch arms, lessons, frozen-file hashes, prior handling, shared
40-minute deadline, fresh-output rule, incomplete-run receipt and promotion
policy are unchanged. No optimizer, learning rate, masks, weights, genetics,
neural policy or runtime persistence interface is changed.

## CPU evidence and cost

`python3 -m unittest scripts/test_training_dose_diagnostic.py` reproduces the
unsupported production flag and acceptance of invalid evaluator inputs before
the fix. Afterward all three tests pass, including 12 valid combinations of the
tick endpoints and supported request pairs, 17 invalid input cases, and the
full 18-command plan. Tests use a nonexecutable dummy binary and omit `--run`;
no trainer child is launched. They check frozen hashes, prior-off plan fields,
empty completion receipts and no promotion.

Python bytecode compilation, strict Ruff 0.16.10 checks for both scripts,
`git diff --check` and all 77 documentation assertions pass. Script imports
are normalized and executable bits match their existing shebangs. The existing
GitHub Rust and host gates do not invoke these Python tests; the focused command
above was run locally.

The added validation is a constant-size membership check before input hashing;
command construction retains its fixed 18 entries. This is source-based cost
evidence, not a measured training or GPU performance result. The command plan
is checked against the production Rust source, without executing a GPU pilot.

Relevant v2.0 requirements: AOA-INV-005 (bounded work), AOA-INV-009 (shared
evaluation semantics), AOA-INV-010 (explicit failure), AOA-TEACH-004 and
AOA-TEACH-008 (grounded requests and evaluation). These tests certify the CLI
contract, not comprehension or learned capability.

No changed file overlaps GeneForge PR3, contact PR4, food PR5 or island/art
files. No PC task, model service, training or graphical workload was run.
