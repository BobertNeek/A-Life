# Grip fixed-fixture qualification V3 continuation

Source-only, separately frozen hosted qualification proposal. No local Rust,
Cargo or fixture execution and no GitHub write or activation were performed.
V5 implementation/runtime overlays, genes and controls are preserved.
The unchanged V1 GPU test addition remains. The V2 serialization correction remains unchanged. V3 changes only f019’s
expected interval-velocity bookkeeping; all other test bodies remain identical.

## Selected work and explicit limits

FIXTURES.json is the complete executable inventory: 28 remaining exact tests in six
explicit Cargo test targets. Every target is compiled with cargo test --no-run,
--locked, -j1 and the frozen official Rust 1.99.1. No example, broad test filter,
--ignored, GPU device, hardware runtime, autonomous study or old-life input.
All binaries are identified from Cargo JSON test artifacts and hash-bound under
a new private target directory. Every exact name must list as one test before
any fixture is executed. Each is then invoked once with --exact and one test
thread in its own fresh working directory. The first failure stops everything;
there is no retry, rerun or repeated fixture body.

Selected coverage includes legacy wire and extension compatibility, exact grip
memory family/target isolation, own-held sensing, primitive Close idempotence,
Release empty/stale safety, forged-source authoritative ABI rejection, typed
ports and their cold continuation, maintenance isolation, coordination features,
the authored fresh CPU host/cold continuation, existing CPU upload/Naga checks,
and the new explicit frame-profile upload rejection test.

The separately ignored sealed-source reconstruction test remains excluded.
No dedicated registered grip transaction rollback/work-limit rejection test was
found. Primitive rejected attempts below are not registered biology rollback
coverage. Integrated cold continuation does not force or prove selection of
Close/Release; the separate primitive fixtures prove those action semantics.

## Exact known execution dose and disclosed construction work

- One fixed fresh CPU-host recipe build; one resident; no prior checkpoint input.
- Twelve newly constructed worlds: eight primitive fixtures, one canonical
  newborn ABI-gating fixture and three constructions inside the host fixture.
  A separate cold-restored continuation branch and transaction scratch clones
  are not counted as additional newly constructed worlds.
- Twenty-five unregistered primitive attempts: 16 successful, nine rejected.
  These do not advance registered biology.
- Three explicit integrated host source forwards, three bundle attempts and
  three committed registered intervals: warm two, cold one. No authored
  rolled-back registered interval attempt. The test asserts one source
  execution per decision and compares cold/warm continuation.
- Eight isolated grip-bank forwards and three maintenance-partition forwards,
  including all warm/cold and finite-difference branches: 3,380 cell updates.
  Integrated grip adds three bank forwards/768 grip-cell updates. These counts
  do not represent all source-network numerical arithmetic.
- Three enumerated fresh N512 scaffold compiler calls in GPU-upload fixtures.
  Additional pure numerical construction/restore/validation inside the CPU-host
  fixture repeatedly rebuilds the frozen source. Its total is not enumerated
  and is not claimed to be zero or included in those three calls. Fixed outer
  bounds are one resident, one recipe, one explicit cold restore and three host
  steps; the same job time/RSS/storage/output guards bound all construction work.

SOURCE-AUDIT.json records source paths, exact hashes, callgraph observations,
primitive-versus-registered distinctions and known gaps. FIXTURES.json contains
per-fixture integer counts and explicitly marks the incomplete compiler total.
The final safe summary distinguishes source-audited attempted/passed dose from
actual fixture-pass/process counts and measured time/RSS/storage/output. A
failed fixture is conservatively charged its entire audited dose, not asserted
to have completed all that work. No private native counters are invented.

## Small additive GPU test, narrowly labeled

TEST-ONLY-DELTA.json and patches/gpu-upload-gate.patch add only
crates/alife_gpu_backend/tests/grip_profile_upload_gate.rs, byte-identical
to V1. The unchanged five-step V5 recovery executes first, followed by that
GPU addition, unchanged f012 correction, and the separate f019 test patch. The manifest has
866 records: 863 original V5 records unchanged, two test-only records corrected,
and the unchanged GPU test addition. Implementation and genes are untouched.

The new test constructs a genuine N512/PrivilegedAffordanceV1 phenotype and
slot, validates its matching legacy frame and successful upload, then constructs
and validates a typed GroundedGripSleepChoiceV1 frame with empty own-grip state.
It asserts LayoutMismatch at the actual public try_from_frame entrypoint.

This is the explicit unconditional frame-profile gate: exact V5 implementation
returns LayoutMismatch for the research profiles before candidate count, frame
validation or any slot read. It is not inferred from an old-slot mismatch.
The test independently validates both frames and the slot, and proves the
legacy control succeeds with that same slot. It does NOT qualify construction
of a new-profile slot, hardware upload or GPU execution. Own-held semantics
are covered separately. The additive fixture invokes one N512 compiler, two
successful legacy upload entry calls (one through validate_against) and one
grip rejection, with zero source forwards, worlds or biology intervals.

## Preserved workflows and two-operation publication

Exact main: c3aec16df33861f1316d127736f5f60eea327ede.
Main tree: 9140d525fe1af16fed6539c3178a1a45e1fb04d0.
Immutable V5 source-reuse head: ff991c39ca4a11f72eb56f015f8b3dab71baff20.
V5 tree: 59a0625744eb64591b98050d4d9e38fd2bb5dac0.

New branch: qualification/m2-grip-fixed-fixtures-v3.
New workflow: .github/workflows/m2-grip-fixed-fixtures-v3.yml.
Packet: docs/experiments/m2-grip-fixed-fixtures-v3.

Preserve every inherited main entry and all 15 workflow blobs/modes/bytes.
The prepared tree adds only this docs packet and the new tightly gated workflow.
Do not publish a crate source change: the test-only overlay is applied only in
the isolated hosted source checkout. Combine every archive member and every
REUSE-MANIFEST.json source blob at its exact destination. Never reuse any old
activation, workflow, runner, cache, binary or checkpoint.

Before writing, verify branch absence, exact main identity, all intended
additions and any uncertain earlier operation outcome. The prepared commit is
a direct child of pinned main. Verify its complete tree and absent ACTIVATE.json,
then create the new branch directly there. Do not create an intermediate ref.
Only after remote prepared-head verification may a separate authorized operation
add ACTIVATE.json as the sole changed path of one direct-child commit.

TRIGGER-AUDIT.json proves all inherited push filters exclude this branch:
eleven main-only, Group50 main/group50-ci/**, Jules codex/**; two have no push
trigger. No inherited create/schedule/workflow_run/repository_dispatch event.
Branch exclusion does not depend on new-branch path-diff behavior. New workflow
job gates independently reject branch creation, deletion, force and rerun.
The runner requires exact repository/ref/event before/after, a sole parent and
sole marker addition, exact activation JSON, sealed packet, and the unchanged
15 inherited workflows plus exactly its own new workflow. All guards precede
native fixture execution.

Do not open a PR, merge, manually dispatch, rename refs or modify old workflows.
Such independent operations can invoke preserved workflows. This audit covers
the proposed branch creation and marker push, not unrelated organization apps.
No denied deletion operation was repeated or routed around.

## Resource and output contract

Same official Rust 1.99.1 compiler identity checks and private fresh Cargo/target
storage; no persisted checkout credentials, source-state upload or shared cache.
Ninety-minute wall, 6 GiB sampled process-session RSS, 12 GiB accounted storage,
2 GiB hosted free-space floor, 3 GiB selected sysroot cap, serial compilation.
RSS sampling is 250 ms, disk at most every two seconds: monitored guards, not
hard instantaneous cgroup quotas. Core dumps disabled; process sessions killed
on all exits. Additional numerical reconstruction is subject to these guards.

Private stdout/stderr are bounded to 32 MiB combined; captured output per command
is additionally capped at 2 MiB. No native assertion, journal, checkpoint, binary
or raw log is emitted or uploaded. The sole final JSON is at most 32 KiB and
contains safe static fixture/target IDs, hashes, counts, measured resource data,
whitelisted failure codes and normalized bound compiler path/code/line only.
Successful test stdout is not requested. No artifact or cache upload step.

Local source staging is capped at 2 MiB with a 4 GiB free-space floor. All V5
and earlier packets remain preserved. Static/synthetic tests are local Python
only. Hosted compile/list/fixture results remain pending until an authorized
activation completes; this preparation is not a behavioral pass.

## V2 correction and prior outcome

V1 run42674144296/job129351631714 compiled all nine binaries, listed all46
selected names, passed f001-f011, and stopped at f012 before worlds or source
forwards. This record is the parent's sanitized result, not raw-log analysis.

The failing source fixture used the restricted grip-metadata DeferredValue
adapter for ordinary known fields too. In particular, its enum adapter does
not accept the externally tagged ObjectSlot map used by visible legacy cases.
The correction keeps serde_json::Value for known fields, uses DeferredValue
only for unknown metadata, and requires an independent eleven-field legacy
Deserialize baseline to accept the same input.

F012-CORRECTION.json binds the exact before/after bytes and sole-file patch.
Seven old fixtures times two metadata positions add fourteen pure baseline
decodes. All46 exact names and all existing doses remain; no additional world,
source-forward, biology, registered rollback or hardware work is introduced.
This is a new corrected qualification, never a rerun of the failed V1 job.

The corrected source is 18131 bytes, SHA256
fd4c1b7764a16c79e1cfc0179d82ce7211fa7dd10560e04d23b24d313874645e.
The independent reference gains only Deserialize; no production deserializer
changes. Frozen V1/V5 and their source inputs remain untouched.

REMOTE-FILES.json is the complete allowed prepared-path inventory. It contains
31 archive source members plus21 exact V5 blob reuses, all mode100644. Only the
separate ACTIVATE.json marker may later be added, after remote prepared-tree
verification and separate authorized activation. Local generation performs
Python synthetic checks only, not native compilation or fixture execution.

The new preparation resolver retains one exact, hash-bound historical Python
bytecode cache in place. PRIOR-V1-PRESERVATION.json identifies its sole allowed
path, size and hash. It is never executed, reused as source, copied or uploaded.
All113 bound historical packet files and eleven separately bound source files
must remain identical, and every other extra/missing path still fails closed.


## V3 continuation and source-derived f019 correction

CONTINUATION.json includes the exact pinned safe V2 receipt and original source
identities for f001-f018. Those18 passes are retained, not rerun. V3 compiles
six needed targets, lists28 exact remaining names, and executes f019-f046 in
their original order. Report new and retained outcomes separately, never as a
single46/46 batch. No dependency/source bytes used by the retained core tests
change: the only V2 delta is inside the world-library cfg(test) f019 body.

V2's safe receipt identifies f019, not a failed assertion line. Exact source
nevertheless proves a sufficient mismatch: Close(second) records its movement
from x=.8 to grip x=.5. The following rejected stale Release correctly preserves
ownership/pose while inherited interval bookkeeping clears that old velocity.
The old full snapshot comparison expected the stale nonzero velocity.

The correction predicts only second's velocity becoming ZERO, explicitly checks
the actor's ZERO velocity, no transfers/touches and zero displacement, and keeps
complete-object comparisons after both stale Releases. It does not discard
ownership, pose, active/consumed flags, constraints or any other object fields.
No production/physics/genes changes, extra world or additional action attempts.

The prior failed fixture's source-audited dose is a conservative upper bound,
not evidence seven operations completed. The new f019 attempt retains its
same planned one world/seven primitive attempts. The fourteen pure baseline
decodes are retained from V2 rather than executed again.

All old packets and the one exact historical cache remain untouched. New local
staging lives only under /tmp/m2-grip-fixed-fixtures-v3, at most2MiB with4GiB
free-space and2GiB available-memory admission. Hosted budgets remain unchanged.
