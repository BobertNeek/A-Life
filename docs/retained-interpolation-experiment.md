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
fresh/donor initialization. The shipped exact-continuation CLI does not provide
a provenance-aware two-parent fork importer or install this retained projection.
A practical training adapter must first admit both source pins, record the
optimizer choice as a new fork, construct a fresh world on independently allocated
training seeds, preserve the acquired layout across weight/identity rebinding,
and emit a native handoff only after genuine consistent actor/value updates.
It must not relabel a parent receipt, forge the critic's policy marker, or route
the candidate through a development fixture. Full original parent archives remain
the rollback points. CPU recovery does not establish hardware GPU parity.

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
