# Bounded hosted M2 qualification, pretty-list and private-channel-key V4

Status: V4 is a distinct source-only correction, not published, activated,
natively compiled or fixture-qualified. V3 is terminal: run 42578269916 at
c3bf62432ecefd2ec22c8a586013f84dcd4ff548. Both official toolchain setup stages
passed. Boredom compiled, then failed LIST_PROTOCOL while listing training tests.
Reflex failed BUILD with E0624 at recovery_reflex.rs 224:32 and 230:32. Both
completed zero fixtures. Sanitized receipts and attribution are retained in
V3-RESULTS.json. No V3 rerun, marker reuse or branch modification is permitted.

The proven V3 setup adapter is unchanged. The two libtest listing commands now
request `--format pretty`, which emits the required footer. The unique-name,
count/footer parser is unchanged and still rejects footerless terse output.
Reflex now uses one local private comparison-key helper at the two previously
inaccessible core-method calls. It exactly maps all 261 public enum values;
no public core API, cast, channel, guard, physical policy or test-body change.
Boredom Rust, all fixed fixture names/bytes and call bounds stay unchanged.
Only boredom (6) and reflex (22) are eligible; meal/GDS stay disabled.

Earlier V2 run 42575100065 remains terminal with zero builds/cases and unknown
setup failure cause. Its raw stderr was not retained. V1/V2/V3 source packets,
seals and archives remain preserved. Existing checkouts, targets, acquired state
and active studies were not modified. Meal's old bc source in this inherited
packet is historical, not its current qualified ownership-bound V2 host at
b78561488f72dfbd27e69b530f391831628ce0de, and cannot be activated here.

## Reconstruction result

Accepted main is `c3aec16df33861f1316d127736f5f60eea327ede`. The research baseline is
`fbf2a517f93098b5d936878a3d26bbb8d0391c38`, tree
`020d7a4b036d0309a4ffdd762536c36e945f7496`.

An incremental variant patch on main is insufficient. The exact selected library
closure from main to fbf consists of 120 changed paths. Its Git full-index patch
is 1,542,113 bytes, SHA-256
`be1fff0fde16f474c118f9b37ec95b526b52c6f644216b1d7d0c12eee0372a74`.
The original deterministic gzip remains unchanged in V1: 276,792 bytes, SHA-256
`c0bda9c01d63dabdd3782163e381263155fa7c6394bc33ed1e2bad086ef38a7e`.
V2 publishes readable UTF-8 parts instead: 563,200, 563,195 and 415,718 bytes,
each ending at a newline and at most 550 KiB. The runner verifies every part,
concatenates in numbered order without text conversion, and verifies the
original raw length/hash before git apply. This changes transport only; all
variant patch bytes and Rust fixture-list bytes are unchanged.
`closure.json` supplies every before/after byte hash and byte count. The smaller
96-path/213,243-byte compressed measurement omitted explicit example-target
files; the deliverable conservatively includes their source so Cargo target
discovery has the declared paths. No example is selected or run.

The full historical research tree is deliberately not reproduced. The complete
c3→fbf diff includes 372 paths and historical exports which do not belong in this
packet. The source closure is exact for the selected Cargo library targets and
their in-module tests, not a claim that all historical examples/integration
targets are buildable. Integration-test fixture captures and archived example
data are omitted. Missing data for an excluded target never authorizes fetching
it or broadening the build command.

`source-base.json` binds 299 library-source/config/include files. This is a
conservative source superset, including cfg-disabled sources, **not** a claim
that every bound `.rs` is compiled. Actual root dep-info must be a subset. Docs
`.rs` archives are never enumerated as crate source. `#[path]` imports bind the
three source-authored grouped-host recipe helper modules under examples/support.

Required added data includes are the three source-authored deterministic recipe
JSON files: M2 mediation, recovery-group confidence and recovery-group arbitration.
Their hashes are bound individually. Existing main-owned foundation and island
bytes and WGSL dependencies are hash-bound and reused from main, not uploaded.
No private asset, checkpoint, organism, bank, journal, executable or target is
in this source closure.

The meal patch includes
`crates/alife_core/src/memory/testdata/legacy_same_sequence_bank_fbf.json`.
It is the small deterministic source-owned synthetic unit fixture, not an
acquired creature bank. Its exact bytes/hash are in `source-meal.json`. The
different 2,063-byte pre-motor resident integration fixture is not part of this
packet. No opaque binary fixture exception is needed.

## Independent source contexts

- Meal: fbf → qualified core `a5b12af62dab4f167c003e48e4ca37fdda74d0c1` → actual
  host `bc2cc5dc144cf71f1436678aa37bce493dce66f8`. Original published patches retain
  their exact 120,043-byte and 122,483-byte identities.
- Boredom: fbf plus the formatted V2 68,449-byte source overlay with SHA-256
  `7e4f7e94daeb39b68df7761e8e3c7dc0b2e822e7aa882f524391702f0fe73e2f`.
  No meal or GDS code is composed into it. Full/readout-sham and DriveSham remain
  different labels; the extra pair is detached Profile5 Play arithmetic.
- Recovery/reflex world: fbf plus the corrected 74,744-byte five-source-file patch
  with SHA-256 `d83677117de9e27e696e7f8ea7a495381e14f268b1e67fe74b7779998fded6b4`.
  The original 74,202-byte E0624-producing patch remains preserved, SHA-256
  `3bd0e0d55eaa531caaf661b629326d9a622e26c001eb999d4b598d07f66158f5`.
  Only recovery_reflex.rs changes between them: one private match helper and two
  call replacements. The other four source files, including all 22 tests, are
  identical. `reflex-key-v2/` holds the finite source witness and incremental
  patch. World API/evidence candidate only; no M2 host integration or native
  qualification is claimed for this corrected revision.
- GDS fallback only: fbf → qualified GDE1
  `2f2cd592ef7a05d199dd1bc3353afdd7f49bcee7` → GDS runtime
  `2a36370f6734f31d8637f8931dacff0e8e6efbea`. The FULL witness-only comparison
  source is `f98fed57f54c0e76fd513af45be7730f5c54fc02`, whose only changes from
  qualified GDE1 are the identical witness module and its declaration. Runtime
  identity does not use the later plan-only revision f02cd005.

Every patch was reconstructed in memory from exact Git-object bytes and matched
its intended revision/overlay. No temporary index, Git-object write, checkout or
worktree change was required. Patch hashes/sizes are in `patches.json`; source
variant overrides and fixture-list bytes are bound separately.

## Fixed tests and real call bounds

`fixtures.json` names every selected test exactly, its crate, source file hash and
kind. `fixture-bounds.json` enumerates every selection's real source/body/world
bounds. No prefix-run or substring-run is permitted. Test listing is checked
before execution, and the actual run must contain exactly one successful test.

Current first hosted activation is two independent fresh jobs, 28 exact tests:
boredom V2 (6) and world-reflex (22), at most two jobs at once. The 42-case meal
selection below is an INACTIVE historical reference. Current meal qualification
has moved to the separately qualified ownership-bound V2 host. The marker and runner reject meal/GDS activation.

- Meal, INACTIVE reference only: 24 existing synthetic core cases +14 new synthetic bounded-sleep/core
  cases +4 host cases. Core contracts have zero world/body/neural-source calls.
  The four host cases use five fresh 240-tick preparations: 1,200 body ticks,
  at most 21 host attempts, including one intentional failed NaN attempt.
  Successful completed intervals are at most 20. The scripted Rest+Eat control
  uses at most three explicit test interventions plus two warm/cold steps. It is
  labeled scripted registered-world evidence, never an autonomous sampler draw.
  The separate eight-interval sampler case reports bounded scalar coverage; zero
  supplemental meals is allowed but reported as uncovered, not a learning pass.
- Boredom: six exact detached cases, zero world/body ticks, 16 successful actual
  source executions, three malformed/budget attempts rejected before source,
  and 45 synthetic partition-arithmetic controls. This is not learning,
  registered-world Play, hardware GPU or game-performance evidence.

- Recovery/reflex: 22 fixed physical-world controls using existing seed 961060,
  no body warmup, native neural source or M2 host calls. The authored loops bound
  41 fresh template/scenario helper calls (each builds one public template and
  one registered scenario of one or two bodies), plus one ordinary new-game
  admission-rejection control. There are 48 new reflex acting attempts, including
  one repeated-application rejection and one injected late rollback; three old
  sleep-choice successful controls; three old no-composition-guard rejections;
  six body-only calls (one injected rollback); seven explicit clock advances;
  eight raw GRAB setup commands; and nine independent expected-chemistry calls
  on copied before-state. Counts include warm/cold comparison branches and are
  per-test enumerated, not a claim of 48 autonomous decisions. Zero-capacity,
  scheduler-phase and exact opposing-segment controls are explicitly synthetic.
  This cannot qualify a neural host, predictor/memory binding or learned avoidance.

GDS is a disabled reference recipe whose exact six selections have passed locally: four sham cases
plus the identical FULL witness on each source. It has eight 240-tick and one
1,200-tick preparations, 3,120 preparation ticks and at most 40 host attempts,
including one deliberate NaN failure (at most 39 completed). Foreign-mode,
corrupted-provenance and reuse attempts operate on staged diagnostic copies,
not extra lives. Each FULL witness produces exactly five files privately; all
five must compare byte-for-byte, with only a Boolean/hash summary exported.
Do not dispatch this now: local GDS work has already supplied the same
qualification. No automatic fallback or duplicate run exists.

The old 10,000-tick compatibility fixture is excluded. If separately selected
later, its source contains 10,000 age-disabled preparation ticks plus 32 warm
and 32 cold host calls (64 total), not 32 total. No older 164-case whole-core or
whole-library replay is part of this first packet. It does not requalify an
entire old suite.

All grouped host controls preserve the established fixed GRPHOST seed
`0x03475250484F5354`; no new seed, coefficient search, adaptive extension,
autonomous example, exhausted PR300 life or active Full/Sham state is involved.
Missing expected sleep/consumption/continuation events fail the declared case.

## Minimal hosted publication shape

V4 branch: `qualification/m2-fixed-fixtures-v4`, created from the verified V3
PREPARED head `dcace9456304e58e442a7423f934102eac1f4323`, where both prior
marker paths are absent. Do not base it on the activated V3 head. UPDATE.json
contains only changed/new UTF-8 paths and expected old Git blob SHA-1 values.
Do not republish unchanged closure parts or other source variants. The inherited
corpus remains under `docs/experiments/m2-hosted-qualification-v2/` on V4; edits
there on the new branch do not mutate V2/V3 history.

New workflow: `.github/workflows/m2-fixed-fixtures-v4.yml`.
New marker: `docs/experiments/m2-hosted-qualification-v4/ACTIVATE.json`.
New plan/concurrency/work directory identity: `m2-fixed-fixtures-v4`.
The V4 template is inert; no actual activation marker is included. Inherited
V2 and V3 workflows retain their exact bytes and require their respective
branches, so neither can activate on V4. All 15 other inherited workflows are
also branch-ineligible. The prepared remote inventory and blob hashes are
recorded in TRIGGER-AUDIT.json. No PR, old study, secret, write permission,
workflow_dispatch, credential expansion or optional-review wait is introduced.

Parent publishes the minimal UPDATE, then verifies every blob, new prepared
head, marker absence, all workflow filters and V4 run ledger. Only after review
may the parent add one NEW V4 marker at the new path in a single normal commit
push. It binds that prepared head, main, V4 seal, unchanged fixture list and only
boredom/reflex. No REST-ref-creation assumption, workflow-token-trigger
assumption, force push, merge, manual rerun or old marker reuse is allowed.
Reconcile uncertain outcomes before another write. At most two separate jobs,
maximum parallelism two, fail-fast false, no retries. No publication or native
work was performed during this source preparation.

Both setup and native entrypoints retain the existing strict seal/marker,
repo/ref/event/run_attempt, single-parent and single-added-marker admission.
The setup adapter's code is byte-identical to proven V3. Apart from V4 identities,
the ONLY runner edits are the two listing format strings, terse to pretty.
The parser remains byte-identical. Exact listing returns the selected name plus
its count/footer; ignored listing returns the unchanged exact ignored inventory
plus its count/footer. No empty-list, duplicate, count mismatch, unknown ignored
test or footerless fallback has been added. Synthetic tests evaluate the actual
two runner command expressions and bind the pretty choice.

## Standard official toolchain with private setup output

Both V3 setup jobs passed with this adapter. The repository already uses `dtolnay/rust-toolchain@stable` in CI. We verified
its official signed commit `b725eda525362e7cf34af9a7243cac1e3f01a7ec` and
vendored its exact 7,481-byte public action.yml as provenance, hash-bound by the
seal. This adapter follows its audited command:

`rustup toolchain install 1.99.1 --profile minimal --no-self-update --force-non-host`

It is NOT an unmodified composite action invocation. The official action uses
`tee` on arbitrary installer output. This small adapter captures stdout/stderr
privately instead, omits checksum retries, default mutation and rustup bootstrap,
and requests no additional components or targets. Missing platform rustup
fails at the specific discovery stage. V3 proved this setup path works, without establishing the hidden cause of V2 failure.

Installation uses the existing normal hosted HOME and discovered rustup home,
with official Rust download endpoints, before the private native runner. Cargo
home, target and all temp/output paths are private even during setup. No fresh
isolated RUSTUP_HOME is created. Discovery uses `rustup show home`, then
`rustup run 1.99.1 rustc --print sysroot`; the canonical sysroot must be exactly
the 1.99.1 x86_64-unknown-linux-gnu toolchain below that discovered home. Compiler,
cargo and rustdoc must be regular executables in its bin directory. The private
environment then uses that explicit RUSTUP_HOME, a private HOME, and exact RUSTC
and RUSTDOC paths. Compiler/Cargo calls are `rustup run 1.99.1 rustc/cargo`, never
rustc/cargo shims with + syntax. Rustc verbose output must match official release
commit `58cf8f9c35b355199727627d07ae34a4e2846d67`, date 2026-10-13 and the
pinned host/release. Only validated public version/commit/LLVM and binary hashes,
a scalar sysroot byte count, and static setup stages/codes can leave. No raw
installer text, exception message, discovered path or arbitrary version text is
printed. Binary hashes are observations, not independent distribution attestation.

Toolchain resource accounting is explicit: the selected sysroot is separately
measured, reported and monitored at 3 GiB. Controlled checkouts, private Cargo
home, target, logs and setup/native temporary storage retain the 12 GiB monitored
threshold. Other preinstalled toolchains and rustup platform caches are a
separate platform component, not claimed inside the controlled-work bound. The
2 GiB free floor is checked on BOTH controlled-work and normal rustup-home
filesystems, including during installation. Thresholds are polled, not hard
quotas, and may overshoot between polls. Setup timeout is 600 seconds; the job's
55-minute bound still applies across setup and fixture work. No local install
or native invocation occurs while preparing this packet.

Permissions remain `contents: read`; checkout is pinned and credentials are not
persisted. Cargo uses the exact lockfile and well-known crates.io registry. No
new persistent access, source replacement, git dependency or native output
artifact is introduced. The selected library build commands, setup functions, fixture lists, ignored
inventory and call bounds are unchanged. Only the two listing format arguments
and reflex private-key source correction differ, as documented above.

## Output and failure contract

`run_hosted.py` captures all subprocess stdout/stderr into ephemeral private
files, with no `tee`, raw diagnostic echo or failure artifact. A nonzero Cargo
compile returns only up to 12 deduplicated Rust error codes (E followed by four
digits), each with up to three primary path/line/column locations. Paths must
match the exact bound source corpus and are normalized to repository-relative
names. Registry, private, unknown, absolute-outside-source and unsafe paths are
omitted. Rendered diagnostics, messages, snippets, labels, suggestions, child
messages and explanations are never projected. Errors without a code produce
only a fixed Boolean. Malformed Cargo JSON fails with BUILD_PROTOCOL and no
partial diagnostic projection. Stderr is never parsed or printed.

Runtime failures identify only the pinned test and static status. Every ordinary
failure retains the list/count of previously completed pinned tests, including
compile/protocol/resource failures. The failure renderer independently checks
all enums, names, codes, locations and keys before serialization. Diagnostic
truncation is an explicit Boolean, not permission to dump remaining output.
Successful output retains the existing bounded hashes, names and scalar schema. No raw assertions, state, trace vectors,
checkpoint files, journal records, private IDs or variable error strings escape.
Unrecognized list/result protocols fail closed. Exit failure, no test, unexpected
ignored test, unexpected ignored-set member, malformed scalar, byte mismatch,
wrong features, wrong source head or changed recipe all reject the packet.

The exact five existing Linux training microbenchmarks, one core topology
microbenchmark and two world microbenchmarks are allowed only in the **listed/not-run** ignored inventory.
Their full names live in the runner, not a prefix whitelist. None is executed.
The optional FULL witness is a separate explicitly ignored selection requiring
its own future recipe, not a general exception. No hardware claim follows from
compilation, upload, CPU kernels or Naga validation.

Limits: 55-minute workflow job timeout, 50-minute internal elapsed cap,
30-minute cap per compile, 300-second cap per selected test, 12 GiB temporary
hosted-workspace cap and 2 GiB free-space floor. Native stdout+stderr has a 32 MiB
monitored stop threshold per command, checked about every two seconds and again
at completion. It can overshoot between observations; it is not a hard quota.
Workspace/free-space and wall-time checks are likewise monitored thresholds.
No filesystem quota guarantee is claimed.
Sanitized final JSON is capped at 32 KiB. No artifact upload is configured. Failed
native diagnostics stay private and are discarded with the ephemeral runner;
only the static failure code remains. Investigating a failure requires a new
explicit source-bound packet, never dumping raw logs or replaying a life.

V1 stays unchanged. The combined V1 plus named V2 source/report allowance is
8 MiB; there is no target/dependency allocation or deletion. `SYNTHETIC-TESTS.json` records pure synthetic projection, completion, parser,
transport, source/seal and activation checks, with zero subprocess or native
fixture calls. `STATIC-CHECK.json` distinguishes successful
source reconstruction/parser checks from native stages, all of which remain
unrun. The draft runner/workflow is not represented as hosted-validated.
