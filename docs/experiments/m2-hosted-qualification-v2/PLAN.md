# Bounded hosted M2 qualification, readable-transport V2

Status: prepared locally, not published, not dispatched, no native compilation or
fixture execution. Existing checkouts, targets, acquired state and active studies
were not modified. This is a fixed disposable fixture packet, not another life
study. GDS has passed its six exact local selections. Meal qualification is active
locally. Both are **ineligible** for the first hosted activation; their source
plans are retained only as inactive reference/fallback context.

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
- Recovery/reflex world: fbf plus the 74,202-byte five-source-file patch with
  SHA-256 `3bd0e0d55eaa531caaf661b629326d9a622e26c001eb999d4b598d07f66158f5`.
  World API/evidence candidate only; M2 host integration remains absent.
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
selection below is an INACTIVE reference because that same local qualification
is now active. The marker and runner reject meal/GDS activation.

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

Draft branch: `qualification/m2-fixed-fixtures-v2`, based on accepted main. Add
only `.github/workflows/m2-fixed-fixtures-v2.yml` plus this packet under
`docs/experiments/m2-hosted-qualification-v2/`. No production source on that
branch changes. Reconstruct separate disposable source trees inside hosted jobs.

Publish the workflow/scripts/source bindings first with ACTIVATE.json absent.
That preparation is inert: the workflow requires BOTH the exact qualification
branch and a change to the single ACTIVATE.json path. A REST-created branch is
not assumed to emit the needed push event. After checking the prepared remote
head and existing run ledger, add one frozen marker in one normal commit push
through the existing authorized publication connection. Do not use a workflow
GITHUB_TOKEN, create new credentials or expand permissions. The marker contains
the prepared head, main, seal hash, fixture-list hash and the two exact eligible variant
names (boredom and reflex). Its generated template is inert until the publisher supplies the verified
prepared head and commits it at the activation path.

The native runner independently requires an existing-branch push (not creation,
deletion or force), exact repo/branch, run_attempt=1, one parent equal to the
marker prepared head, and exactly one added file: ACTIVATE.json. It verifies the
marker fields and sealed source before compiling. The workflow fetches two
commits for that check. Later marker edits/deletions and manual reruns cannot
silently repeat the tests. Do not delete/re-add the marker or create a new
activation commit to get around a failed/uncertain run. Reconcile the run ledger
and outcome first. No absolute server-side once-only guarantee is claimed by a
read-only workflow.

The fixed eligible matrix is boredom V2 and world-reflex, maximum parallelism two,
fail-fast false, no retries. The immutable activation head, prepared head, marker
hash and source binding are recorded in the sanitized result. There is no
pull_request or workflow_dispatch trigger. No activation marker or native run
was published during preparation.

TRIGGER-AUDIT.json records every existing workflow at accepted main and the
source hashes checked. None of their push filters matches this non-codex branch:
they target main or codex/**. Marker publication therefore matches only the new
workflow at this reviewed snapshot. Recheck that inventory at the prepared head
before the single activation push, and verify the resulting exact-head run rather
than assuming delivery or retrying blindly. Official GitHub documentation links
are included in the audit; they confirm push on nondefault workflow commits,
combined branch/path filtering and workflow-token recursion suppression.

Do not open a draft PR merely to start qualification: existing unfiltered CI
would also run and Jules review uses a repository secret and write permissions.
This non-codex branch avoids that push trigger. Any later PR is separate and
must account for those existing automations. No waiting for an optional review
is required for the source work or this bounded packet.

Permissions are `contents: read` only. Checkout action is pinned, credentials
are not persisted, no custom secrets are supplied, no persistent access is
created, and no action uploads native outputs. Fresh official Rust 1.99.1 is
installed with minimal profile. Cargo uses the exact lockfile, ordinary crates.io
checksums and registry; there are no git dependencies or source replacements.
The six local crates are core, world, GPU backend, runtime, storage and training.
The current activation builds only training/world `--lib` harnesses; the inactive
meal recipe would additionally build core; cpu-backend/receipt-lineage are
enabled as required, gpu-tests is never enabled. No cache reuse is claimed.

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
