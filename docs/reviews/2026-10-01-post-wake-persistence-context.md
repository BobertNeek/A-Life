# Post-wake persistence and gameplay continuity — 2026-10-01

The readiness fixture now separates gameplay continuity from exact external-hint
receipt identity. Production persistence and prior preparation are unchanged.
This follows the user's 13:07:35Z instruction to prioritize a stable playable game
and retained learning over harmless diagnostic discrepancies. A new hardware run
is still **Unrun**; the previous exact-diagnostic failure remains recorded.

## Reported Windows failure

The coordinator supplied operational source
`a6ef9adcf3e4518d4626767fbee61265064ebdb1`, ending at
`2026-10-01T08:14:50.2112810Z`, exit 101, wall time 34.7820451 seconds.
Executable SHA256:
`908B98C22FB28AD11DEADD0E8EE11A997E61C101B937D1450990846F84678A50`.
The command selected only
`gpu_live_runtime::checkpoint_manifest_pruning_tests::readiness_resume_preserves_external_actors_toys_and_private_prior`
with `--exact --test-threads=1 --nocapture`, `ALIFE_SLM_PRIOR=on`, and the explicit
transferred current N2048 founder. The receipt did not separately record argv;
these invocation details came from the coordinator's read-only excerpt.

Neural, world, biology, private-prior, memory, topology, and sleep restoration
checks passed before the first-response assertion. The operational correction
compared all durable private-prior fields while accounting for the documented
reset/resubmission of in-flight service work.

At tick 25, sequence 9, restored input had no semantic context; uninterrupted
input had zero-confidence hints with codes 1, 3, and 14, each salience 0.3.
These changed perception, query, and decision source/canonical digests. Selected
action remained `Ingest`, `ActionId(210)`, candidate 3, logit 4.6564636.
The coordinator found both complete debug dumps identical after replacing only
the semantic context and derived/source digests; retrieval-context digests matched.
Later world, memory, and neural assertions were never reached.

This locates the observed mismatch at external-input availability. Asynchronous
reply timing and cache availability are not identical across reopen, even at zero
gain. The evidence does not distinguish those two causes, prove lost acquired
state, or establish that later learning is broken. It is not by itself a gameplay
or training blocker under the user's revised priority.

## Focused test-policy repair

The separate branch `codex/cloud-post-wake-persistence-20261001` starts at
`2bad5e02b6bbf125a10822d3c7effd7debc96d8f`; the published CI branch remains frozen.
The Windows fixture correction for in-flight prior work is applied unchanged.
All existing exact checks before the response remain, including acquired neural
state, organism biology, world state, memory, topology, and sleep.

After the first response, the fixture requires valid sealed patches and identical
ordinary perception, retrieval context, organism/tick/sequence, selected action,
motor bundle, neural decision payload, outcome, cognitive work, and substantive
cognitive/prediction payloads. Only zero-confidence hints with no active flags or standalone salience, and
the associated source identities, are tolerated by that comparison. Nonzero hint changes and
ordinary world differences still fail. Each patch must remain internally valid;
no stored state or digest is edited to make this comparison pass.

World state must still match. Both memory sidecars must validate, retain the same
organism set, record counts, and latest durable sequence. Learning failures are
rejected and a resumed learning receipt is required. Both lifetime and fast weight
banks, their active bank/generation, and the last learning replay identity must
match exactly; unresolved eligibility is rejected. This preserves strong acquired-
learning protection without requiring all newly generated memory metadata to be
identical under unequal asynchronous hints.

`readiness-post-wake.json` records exact patch, memory, and full-neural equality
as separate diagnostic results after the material checks pass.
`ALIFE_PERSISTENCE_EXACT_DIAGNOSTICS=1` additionally runs the original strict
post-response assertions. These remain explicit diagnostics, and are not silently
reported as passes when their values differ.

The comparison helper is test-only. No fixed service, runtime injection field,
network synchronization, format migration, or legacy compatibility was added.
Production tick preparation and all source files hashed into the provider identity
are unchanged. Founder assets, existing saves, and training implementation are
untouched. Scope: AOA-PERSIST-001/002/004 and nonauthoritative input AOA-SLM-004.

## Runtime check priorities

| Check | Material protection and placement |
| --- | --- |
| GPU arena offsets, buffer lengths, slot/organism identity, live generation, dispatch caps, and finite values | Protect bounds, valid dispatch, ownership and stability; retain cheap changing-state checks before GPU use. Immutable class/layout inputs can be fully checked once at admission. |
| Current save schema, required neural/biology/dependency state, content-addressed assets, and allocation limits | Protect malformed input, corruption, wrong-individual restore, and lost learning; retain at decode/load/admission and publication boundaries. These are not arbitrary per-tick diagnostics. |
| World legality and eligibility/outcome ownership and replay sequence | Protect invalid actions, duplicate credit, wrong-creature learning, and unresolved transactions; retain per transaction. |
| Repeated full draft/frame/candidate/query digest recomputation and validation of already validated immutable payloads | Candidate for consolidation at one authenticated preparation boundary, reusing typed immutable results; do not remove changing bounds/identity checks with them. The existing shared-frame optimization already does this inside candidate loops. No further release-path removal is claimed here. |
| Exact resumed receipt, metadata, and full-snapshot equality under unequal inactive external hints | Useful explicit diagnostics; already outside the game hot path. This repair moves their readiness verdict into opt-in exact diagnostic mode while retaining gameplay/learning checks. |

The static review identifies candidates, not measured release performance gains.
Broad hot-path removal was not needed to repair this test boundary and was not
performed. Corruption protection and material learning failures remain failures.

## Validation and remaining evidence

Final CPU regression coverage reproduces the actual zero-gain digest discrepancy
and rejects meaningful prior/world changes. A bounded affected-app CPU library
run during the investigation passed 211 tests; its two earlier fixed-input tests
were subsequently replaced when the user revised the tradeoff, so that total is
not a final-source receipt. The 209 pre-existing test bodies and production path
are unchanged. Final focused and profile compile/lint receipts are kept under
`/workspace/A-Life/target/artifacts/cloud-post-wake-persistence-2bad5e0/`.
Final checks: two CPU regressions passed; strict Clippy for the explicit GPU
test profile (`--lib --tests`, `-D warnings`) passed without executing its test
bodies; formatting, static boundaries, whitespace, and all 77 documentation
assertions passed. CPU builds used two jobs, profile lint one job, with incremental
and dev/test debug output disabled. The final CPU check observed 9.34 GiB total
cgroup memory and at least 16.12 GiB disk free; neither resource guard fired.

The final patch is retained locally. The coordinator reported that automatic
approval review rejected another GitHub source publication pending explicit
destination disclosure/approval; the coordinator's user permission request is
pending. This worker will not push, upload, or use an alternate publication path.

The full N2048 learning/recovery/gameplay-persistence outcome is still unrun on
this repair. At an allowed time, rerun the existing readiness test on a suitable
physical adapter; use exact diagnostic mode only when investigating receipt
identity. No desktop interaction, GPU execution, or training launch occurred here.
