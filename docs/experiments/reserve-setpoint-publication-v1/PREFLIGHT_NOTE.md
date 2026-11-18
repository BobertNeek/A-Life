# Frozen-log whitespace clarification

Before compatibility execution, the full staged diff check reported12 diagnostics:
seven trailing-whitespace lines and five blank-at-EOF findings, across five
unchanged historical logs. The plan's initial description of twelve whitespace
lines in two rustfmt logs was too narrow; its first two-file exclusion still
reported the four other historical test-log EOF findings. No source issue was
suppressed and no preserved bytes were changed.

The exact five-file exception set is:

- `docs/experiments/biological-initialization-drift-v1/fmt-initial.log`
- `docs/experiments/biological-initialization-drift-v1/run.log`
- `docs/experiments/reserve-pressure-setpoint-v1/run.log`
- `docs/experiments/reserve-pressure-setpoint-v1/validation.log`
- `docs/experiments/reserve-pressure-setpoint-v1/composition-addendum/run.log`

All five are byte-verified against the sealed source. The initial checks are
retained as warning/failure, not passed.

The parent subsequently requested the repository's existing captured-log
convention: package-local `.gitattributes` containing `*.log -whitespace` in only
the two imported experiment directories. This preserves every historical byte
and makes ordinary full diff checks meaningful without excluding source, tests
or prose. No global attribute or code whitespace exemption is introduced. The
qualification reports the final ordinary full diff check separately, along with
explicit attribute scope and original-file hash verification. This does not
alter the48-test scope or any executable code.
