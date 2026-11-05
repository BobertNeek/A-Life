# Disposable retained interpolation fixture

`alife_training::cpu::retained_interpolation_experiment` prepares an experimental
two-parent convex interpolation of canonical genetic coordinates. It asserts no
common ancestor and does not use the founder merge API's caller-supplied ancestry
descriptors. A higher policy number does not establish a better parent.

This is an inference-only preparation harness. It archives the PC Current
byte-for-byte and accepts either a second Current or an explicitly tagged native
CPU parent file archive. Both complete parent states retain their actor/value
Adam histories, integer ages, seed histories and private life. Expected source
pins must come from the owners' verified copies. Callers cannot supply a detached
retained projection. Earlier disposable input encodings are replaced by schema3;
opaque native bytes use lossless base64 in the experiment, without changing any
parent bytes. Accepted native checkpoint formats are unchanged.

`derive_retained_projection_from_current(bytes, expected_sha256)` extracts the
PC projection directly from the pinned Current's own required native asset refs.
It checks one individual, saved tick/phenotype/compiler identity, a sealed outcome
boundary, unique canonical manifest paths/IDs, exact closure membership and every
declared asset's byte length/digest. The six selected raw assets (phenotype,
compiler inputs, topology, exact cognition, lifetime and fast banks) receive
separate SHA256 receipts. Exact cognition must agree with the live topology's
structural/dendritic state; both bank selectors/generations must match the source.
The native fixed-slot compiler and canonical-to-live checks then validate the
acquired layout and mutable-bank representability. No caller-supplied projection
or inferred path can replace this chain. The complete original assets stay
unchanged inside the parent archive.

This closes projection source binding. It does **not** establish full native
private recovery, validate every unrelated private asset's semantics, execute a
GPU or qualify the action path. The synthetic fixture uses explicitly labeled
placeholder assets for unrelated runtime state; native full-closure admission
must still be performed by the owner through the existing supported loader.

Only trainable genetic coordinates are interpolated; frozen values must agree
bit-for-bit and masks, curriculum, evaluation and complete neural execution
contracts must match. The checked `N2048LivePolicyTransition` changes weights and
derived identities, then the native fixed-slot compiler reconstructs the PC's
acquired topology. The canonical-to-live permutation, decoder starts/offsets,
all added-edge bits and the complete V11 structural/dendritic state must remain
exact. Both fast and lifetime banks, their active selector and generation and the
learned predictor are copied unchanged; only phenotype identity and corresponding
bank digests are rebound in the disposable view. The acquired edge remains private,
outside actor/Adam coordinates. Unsupported added-edge mutable banks fail closed
under the current CPU retained projection contract; no state is dropped.

The candidate contains no actor optimizer, critic, accepted training handoff or
Current checkpoint. The overlay sequence is not an optimizer step. Parent Adam
histories remain untouched in their archives; candidate optimizer semantics are
unresolved. `validate()` cold-reconstructs every field from the archived inputs
and rejects modified candidates/receipts. This is not a signed authenticity proof.
There is no parent mutation, world dispatch or promotion API. Native CPU archive
admission writes only a bounded owned scratch copy, then removes that copy.
It does not install personal memories, chemistry, activations or eligibility into
a new life. These remain in the complete parent archives.

The source is isolated from PR #256's active assessment branch. It depends on its
experimental retained-layout APIs at `eeb6ff016b1110888759bd3953d38d34074ab27c`.
It adds isolated modules and one test target and does not change assessment code.

Run the focused synthetic CPU checks:

```sh
cargo test --release --locked --offline -p alife_training --features cpu-backend --test retained_interpolation_experiment
cargo test --release --locked --offline -p alife_training --features cpu-backend --test cpu_collector retained_
cargo test --release --locked --offline -p alife_core --test live_policy_overlay
```

These checks cover both active banks, nonzero retained overlays, one native
acquired edge, shifted decoder coordinates, complete parent Adam/critic archives
with ages beyond float integer precision, frozen/graph rejection, unsupported
added-bank rejection, exact interpolation endpoints and cold tamper rejection.
The source-binding cases reject hash-pin substitution, detached payloads, changed
bytes/sizes, missing/orphan/ambiguous assets, wrong requiredness or refs, compiler
substitution, mismatched active banks and stale exact-cognition identity/structure.
They do not test real PC/cloud brains, native GPU parity, embodied skills or
whether the parents improve one another. No unavailable checkpoint is read.
Before real-parent use, the executing owner must:

- Materialize only the specifically approved copies, preserve original bytes and
  use receipt SHA256 pins rather than deriving replacement expected pins from
  arbitrary supplied artifacts.
- Admit both full native sources using the supported loader. Where needed, run
  admission/preparation inside the existing source-bound body-rounding proof
  scope; this adapter does not relax phenotype comparison or alter saved bits.
- Require a sealed PC GPU private life with current topology/exact-cognition refs,
  no pending outcome credit or retained-learning transaction, and all selected
  identities/banks consistent with its actual global policy.
- Qualify the retained action path and both untouched parents on one compatible
  evaluator before comparing a disposable candidate on matched fresh lives.
  A bare founder evaluator would omit the retained overlay and is insufficient.
- Check the fraction-zero PC control through the same identity rebind and fresh
  body path before testing interpolation. Fraction one still carries the PC's
  private overlay, so it is a candidate variant and cannot stand in for the
  untouched cloud-parent control.

## Native CPU archive and preparation command

A native CPU output with `training-handoff.json` and a preceding private collector
does not need, and is never converted into, a Current. `inspect_native_cpu_archive`
and `capture_native_cpu_archive` snapshot every regular file under the sealed,
extracted native source root. Symlinks, ambiguous/private checkpoint variants,
noncanonical paths, more than8192 files or more than512MiB are refused. Every file,
including private collector, original optimizer files and lineage/seed evidence,
stays byte-exact in the embedded archive. Admission uses the existing
`read_cpu_founder_source`, including native output pins, complete typed heads and
existing ancestry verification. No new ancestry is supplied or invented.

The private collector is cold-restored with `collection-policy.alife-foundation`,
`collection-compiler-inputs.json` and `collection-actor-checkpoint.json`, and must
recapture exactly. The original initial critic is retained and validated. The
collector policy version and trained global policy version remain separate in
the receipt. No action or optimizer step is executed during admission; this is
not a claim of resumed training. Unknown historical freshness remains unknown.

The file-tree SHA256 is a domain-separated, length-prefixed hash of sorted UTF8
relative paths and their exact raw bytes. It is distinct from the original ZIP
or other transport hash. The executing owner keeps the approved original archive
unchanged, verifies its receipt hash/length and supported extraction independently,
then records the native root's inspected tree pin. This adapter does not download,
unpack or claim to authenticate a transport archive against its tree by itself.

```sh
cargo build --release --locked --offline -p alife_training --features cpu-backend --bin prepare_retained_interpolation
prepare_retained_interpolation inspect-cpu CPU_NATIVE_ROOT
prepare_retained_interpolation prepare REQUEST.json
prepare_retained_interpolation verify NEW_PRIVATE_EXPERIMENT
```

Place the request outside both preserved source trees:

```json
{
  "pc_current": "PC_COPY.alife-checkpoint",
  "expected_pc_current_sha256": "OWNER_VERIFIED_PC_SHA256",
  "cloud_native_cpu_root": "CPU_NATIVE_ROOT",
  "expected_cloud_tree_sha256": "OWNER_RECORDED_NATIVE_TREE_SHA256",
  "cloud_fraction": 0.5,
  "output": "NEW_PRIVATE_EXPERIMENT",
  "body_rounding_proof": "SOURCE_BOUND_PC_BODY_PROOF.json"
}
```

Omit `body_rounding_proof` if native PC admission needs no compatibility scope.
When provided, it uses the existing envelope with `source_current_sha256` and
`proof`; the source pin must match and the lexical proof must actually be used.
The strict production comparator stays unchanged.

## Zero-update recovery and optimizer initialization probe

The `audit_retained_bridge` CPU example accepts
`--probe-quarter-recovery REQUEST.json` for a previously measured 25%-cloud
experiment. This bounded diagnostic cold-validates both original parents and
the prepared experiment, restores a saved measured initial state, checks its
zero fast/lifetime banks and retained topology/predictor, and replays exactly
one decision. The original body-rounding proof stays explicitly scoped.

The request contains `prepared`, `initial_trial`, `pc_current`,
`expected_pc_sha256`, `cloud_native_root`, `expected_cloud_tree_sha256`, and a
new isolated `output` directory. All paths refer to local, owner-verified copies.
No materialization or network operation occurs. The output includes a reset
projection and recovery copies; these are private assessment/debugging artifacts.
The prepared experiment's original retained projection has **not** itself been
reset. The actual measured initial checkpoint supplies that separate state.

Three zero-update constructor probes use the candidate genetic weights:

- Fresh native actor Adam and a fresh zero critic, with the admitted parent
  trainable mask and the native default Adam configuration.
- A disposable PC optimizer initialization, retaining its complete mask,
  configuration, moments, integer ages and global step, plus its complete critic.
- The equivalent explicitly selected cloud optimizer/critic initialization.

For a donor initialization, only the disposable actor's weight vector and the
two derived phenotype/source identities are replaced. Native checkpoint
validation must accept it, every optimizer bit must still match that donor, and
the unmodified donor's exact `rebind_for_next_cohort` route must reject the
changed genetic weights without mutating itself. This is an explicit optimizer
transfer fork, not exact parameter continuation. No optimizer states are averaged.
The three recovered decisions and private world states must agree before any
global gradient update; this does not qualify their later learning behavior or
the accuracy of a transferred critic.

The probe never applies gradients, changes a critic's last-updated policy marker,
exports candidate actor/critic files, manufactures a training handoff, or promotes
a candidate. It verifies source immutability afterward. Its saved assessment seed
and assessment-acquired private state must not become training input.

The low-level CPU constructors and residual training kernels support an explicit
fresh/donor initialization. The ordinary exact-continuation CLI does not import
two-parent forks. The separate experimental route below preserves that distinction.
Full original parent archives remain rollback points. CPU recovery does not
establish hardware GPU parity.

## Explicit PC-optimizer training fork

`train_retained_fork_cpu`, behind `cpu-training`, runs one bounded native actor
and residual-critic update from a 25%-cloud experiment. Its required optimizer
choice is `complete_pc_donor_optimizer_transfer`. Both complete source archives
remain embedded byte-exact in the preserved experiment. This is a new experimental
fork, not exact continuation, an averaged optimizer, an accepted Current, or a
promotion. It never enters the development fixture route.

```sh
cargo build --release --locked --offline -p alife_game_app --features cpu-training --bin train_retained_fork_cpu
train_retained_fork_cpu run REQUEST.json
train_retained_fork_cpu verify FORK EXPECTED_MANIFEST_SHA256 EXPECTED_EXPERIMENT_SHA256
```

The request contains the prepared experiment path and its expected compressed
SHA256; the PC Current path/SHA256; the cloud native root/tree SHA256 and preserved
transport archive path/SHA256; an explicit `CheckpointSeedsV1` `seed_history`
path/SHA256; consecutive `seed` and `recovery_seed`; `decisions` in 1..=128;
`optimizer_initialization`; and a new isolated `output`. The expected-hash fields
are named `expected_experiment_sha256`, `expected_pc_sha256`,
`expected_cloud_tree_sha256`, `expected_cloud_archive_sha256`, and
`expected_seed_history_sha256`. Source paths are `prepared`, `pc_current`,
`cloud_native_root`, `cloud_archive`, and `seed_history`.

Full native parent admission, source pins, all PC exclusions, cloud seed evidence
and authenticated lineage-frame training exclusions precede dispatch. Assessment
reservations are excluded from training too. Freshness must remain `unknown`.
The owned seed state is persisted before dispatch; an existing output is never
restarted or overwritten automatically. A technical failure keeps its private
collector and captured prefix where available. Collection runs its requested
decision horizon unless natural death or an actual technical limit occurs;
injury/health is observed without censoring. Teacher and semantic prior are
explicitly disabled while real collection rows retain their normal training loss.

The donor actor's complete mask, Adam configuration, moments, per-coordinate u32
ages and global step are retained, and its complete critic is copied unchanged.
Only the disposable actor's genetic weights and derived identities change at
initialization. Both fast/lifetime physical banks reset to +0 once at fork start;
their original selector/generation, acquired edge/layout, dendrites and predictor
are retained. After collection, ordinary CPU waking/sleep changes are preserved.

The CPU owns one active canonical fast and lifetime vector. Newborn export maps
those current values into the current native sparse layout and duplicates each
active vector into both exported newborn banks. This explicitly represents
newborn inheritance, not GPU ping-pong-bank continuation. The complete old-bank
inheritance evidence and personal state remain separately in the collection
checkpoint; no inactive GPU history is invented or silently deleted. Acquired
edges remain outside actor/Adam coordinates. Native weight-only rebinding checks
all layout metadata and signed-zero alpha bits before installing new genetic
values. Unsupported acquired state fails rather than falling back to bare policy.

`training-handoff.json` is emitted only after one real native update and strict
native head admission, with the critic's actual update marker. It describes the
updated fresh-cohort global head. `retained-neural.json` carries the separate
learned newborn projection. `collection/` retains the preceding collector under
its original collection asset and full initial optimizer. Consumers must admit
the complete pinned `retained-fork.json` bundle; using the head alone would drop
acquired state. No ordinary single-parent `cpu-cycle.json` receipt is fabricated.

Cold verification checks the complete artifact closure, source-bound initialization,
seed history, original collection recovery, current learned projection, and native
handoff. It recomputes the single native PPO/critic update on a disposable clone
and requires the saved full optimizer/head to agree. A second, separately reserved
training-validation world cold-restores the updated head and retained state and
reproduces its next physical transition. This is recovery/update evidence, not a
new matched skill battery, GPU parity, or a claim that transferred Adam/critic
history is well calibrated for the interpolated policy.

Subsequent disposable episodes may supply `previous_fork` with `directory` and
`manifest_sha256`, plus an explicit `lesson` and `food_need_condition`. They
cold-verify that predecessor before loading its complete current optimizer,
critic and retained projection. There is no repeated somatic reset or renewed
optimizer donation. Predecessor exclusions must remain included. Each new
manifest pins its predecessor; preserve the entire local chain for recovery.
Verification walks at most32 predecessors and reproduces every native update.

The `retention REQUEST.json` command compares a pinned `baseline` fork and
`candidate` fork against the actual PC/cloud global parents with native fresh
neural state. Its request declares the expected experiment/source pins, a pinned
assessment-only reservation, 1..=20 seeds, and a new output. Every arm uses the
same128 scored decisions plus bootstrap, physical body seed, temperature1,
teacher/prior-off protocol and ordinary private plasticity. Shared optimizer
states stay frozen. Natural deaths, injuries and incomplete cases remain in
results. Both training and retention consumers verify an8GiB free-storage floor
plus a per-job reserve before dispatch; storage admission never deletes sources.

Preparation refuses an existing or overlapping output and rechecks source bytes.
It writes `experiment.json.zst` with both complete parent snapshots, the candidate
foundation, compiler inputs and retained-neural projection, plus an inference-only
receipt. Verification cold-rebuilds the experiment and checks every candidate
export against it. Outputs contain private model state: keep them outside version
control. They contain no candidate actor, critic, training handoff or Current export.

The native assessment owner owns co-location and the actual candidate/control
dispatch. The preparation CLI does not measure skills. Synthetic tests use the
supported CPU writer and verify identical candidate genetics and retained state
against a separate matched-head fixture, exact fraction-zero control, CLI recovery,
full optimizer/archive retention, corruption rejection and parent immutability.
Optimizer/adoption semantics remain unresolved; fixture success implies no model
comparison or promotion.
