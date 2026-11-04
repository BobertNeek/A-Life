# Current to fresh native CPU assessment

`evaluate_cpu_current` admits a native Current file and evaluates its exact policy
head in a new prepared CPU life. It performs no training, source adoption, seed
reservation or saved-personal-life restoration. The original Current retains its
full actor/value optimizer state, seed union and private asset closure.

```text
cargo build --release --locked --offline -p alife_game_app --features cpu-training --bin evaluate_cpu_current
cargo test --release --locked --offline -p alife_game_app --features cpu-training --lib current_adapter
cargo test --release --locked --offline -p alife_game_app --features cpu-training --lib synthetic_current_gpu_origin
cargo test --release --locked --offline -p alife_training --features cpu-backend --test cpu_backend rebind
```

Admission uses `read_current_training_checkpoint`, including its native private
closure validator and exact same-individual tag checks, then typed handoff
admission. The actual embedded founder asset must agree with the saved actor and
compiler. The existing native rebinding checks are reused through a read-only
validator: graph, sensor profile, routing, decoders, dynamics, plasticity, mask,
all weight bits and actual actor state must be compatible with the requested
fresh body. The candidate CPU constructor rejects delayed projections. No actor
or critic is constructed for candidate inference; no Adam is fabricated or reset.
Training origin and original identity stay in the private source proof.

Use private immutable inputs and an absent private output outside the checkout.
The Current request has this exact shape (replace placeholders; SHA256 is the
original file's raw-byte hash, not its reserialized typed representation):

```json
{
  "current": "ABSOLUTE_PRIVATE_CURRENT_COPY",
  "expected_current_sha256": "RAW_CURRENT_SHA256",
  "seed_history": "ABSOLUTE_PRIVATE_ASSESSMENT_DECLARATION",
  "output": "ABSENT_ABSOLUTE_PRIVATE_OUTPUT",
  "founder_seed": 539363617,
  "seed": 202611044101,
  "decisions": 128,
  "temperature": 1.0,
  "lesson": "obstacle_navigation",
  "food_need_condition": "newborn_v1",
  "known_exclusions": true
}
```

The separate assessment declaration is not a fabricated historical ledger:

```json
{
  "schema_version": 1,
  "kind": "current-cpu-assessment-seeds",
  "source_current_sha256": "RAW_CURRENT_SHA256",
  "historical_freshness": "unknown",
  "excluded_ranges": [[1, 100]],
  "assessment_seeds": [202611044101, 202611044102, 202611044103, 202611044104, 202611044105, 202611044106]
}
```

The numerical case seeds above are examples for qualification. Check them against
both parents' complete known exclusions; do not claim global novelty or substitute
them into an existing assessment receipt or archived score comparison.

**Replace the example exclusion range with the complete declared union of both
parents' known consumed, reserved, interrupted and assessment exclusions.** It
must be sorted, disjoint and coalesced, include every range in the admitted
Current (including its pending allocation), and exclude none of the proposed
matrix seeds. The adapter rejects missing Current ranges, undeclared/zero seeds,
duplicate declarations, collisions and attempts to upgrade historical freshness.
It cannot independently discover exclusions or missing evidence absent from its
inputs; extra union ranges are explicitly declared, not newly verified history.
Do not classify all Current exclusions as consumed training. Their kinds were
coalesced in that format. A seed used in two declared physiology cases is one
matrix seed, not two independent samples. Retain assessment outputs/exclusions
privately before any later training or seed selection.

```text
evaluate_cpu_current --spec PRIVATE_CURRENT_REQUEST.json
evaluate_cpu_current --verify PRIVATE_OUTPUT
evaluate_cpu_current --founder-spec PRIVATE_NATIVE_CPU_FOUNDER_REQUEST.json
evaluate_cpu_current --compare PRIVATE_LEFT_OUTPUT PRIVATE_RIGHT_OUTPUT
```

The native-founder mode uses the existing native CPU request and schema2 receipt.
Both entry points share the unchanged care life runner and initial comparison
fields, allowing one executable identity for both parents/candidates. The new
Current mode has a separate schema1 receipt and source proof; it cannot masquerade
as a native CPU training source. Verification dispatches by exact receipt domain,
with no fallback after verification failure. The existing production comparator
is unchanged. The combined comparator also requires full comparison equality,
including executable SHA256; exit0 is not a substitute for `matched:true`.

Use the [eight-case handoff](cpu-care-comparison-handoff.md) for exact scene,
physiology, 128+1 timing, teacher/prior absence, private learning, sleep and safety
semantics. Fresh body seed is an explicit benchmark control; the historical body
seed remains separately recorded. Incompatible source-to-body graphs fail closed.
A valid Current's lifetime/fast weights, memories, eligibility, inactive bank,
private sampler, body age and sleep journal are preserved in its sealed source,
not transferred or added to the policy head. Results measure fresh-founder
capabilities, not all abilities acquired by the saved living individual.

Native source/history/executable pins and raw artifact inventory bind the receipt.
Cold restore/recapture must be exact and decision/teacher/prior boundaries must
verify. Failed prefixes and existing lives are preserved; no auto-restart is
supported. Retain the original executable at its pinned path with all inputs.

Qualification fixtures exercise actual disposable CPU-training output, its full
Adam, a native fresh-cohort Current wrapper/private closure, and full versus bare
CPU parity in held-food, aged Feeding and aged RelocatedFood. A separate synthetic
origin-tag fixture exercises GPU-origin metadata without GPU execution. These
are not the user's saved PC checkpoint and do not demonstrate its compatibility.
Before comparative claims, the PC owner must admit that actual immutable save,
verify all weights/graph/frozen coordinates and preserved full state, and repeat
actual-parent parity. In particular, full native same-individual GPU private-life
restoration is not part of this fresh-life adapter's qualification.

A Windows/Linux fixture fingerprint alone does not establish runtime equivalence.
Require the actual parent on both runtimes, all eight native initial controls and
recurrent/lifetime/sleep evidence, or run both untouched parents and candidates
under one qualified executable. Do not weaken the executable identity comparison
or adopt a candidate on unmatched or missing skill coverage.

Source artifact hashes are recorded in
[`fixtures/current-cpu-assessment-source-sha256.json`](fixtures/current-cpu-assessment-source-sha256.json).

## Actual-parent full-state parity

The full control accepts the **same standalone Current**, seed declaration and
case controls as the bare entry point. It restores the actual saved actor and
critic checkpoints with their full Adam state, applies only the checked actor
binding metadata for the explicitly new body, then holds both heads read-only.
The critic is carried unchanged and unused for action inference. Original weights,
moments, per-coordinate integer ages, global steps, configuration and stage mask
are preserved. The original private life and sampler remain in the immutable
source Current; they are not translated into this fresh CPU life.

Prepare two requests differing **only in their absent output directories**. Keep
an immutable copy of the exact executable used by these receipts before rebuilding.

```text
evaluate_cpu_current --spec PRIVATE_BARE_REQUEST.json
evaluate_cpu_current --full-spec PRIVATE_FULL_REQUEST.json
evaluate_cpu_current --verify PRIVATE_BARE_OUTPUT
evaluate_cpu_current --verify PRIVATE_FULL_OUTPUT
evaluate_cpu_current --parity PRIVATE_BARE_OUTPUT PRIVATE_FULL_OUTPUT
```

`--parity` fails with a nonzero status unless it verifies one bare Current receipt
and one full Current receipt, the exact same Current source/request controls,
identical executable and native initial controls, byte-identical actual first
inputs and action traces, and byte-identical complete fresh private checkpoints.
Both paths also cold restore/recapture their private checkpoints through native
validation. The full receipt additionally binds `current-full-actor-preservation.json`:
actual full actor hashes before/after collection must equal the source actor after
only its permitted phenotype/source binding changes; the actual carried critic
must equal its saved checkpoint before/after. No trained head payload is written
into the assessment output and no training-source receipt is fabricated.

This is a policy-head/full-optimizer parity control in new CPU lives. It is not a
saved GPU individual-to-CPU private-state conversion, GPU numerical parity proof,
or proof that all of the saved individual's personal skills are in its founder.
Run it against the actual immutable PC copy before interpreting comparative scores.
A native CPU directory and a cloud checkpoint transfer are unnecessary for this
Current-parent control. Different source, case, binary or dose is rejected, as are
restarts and altered preservation evidence. Failed prefixes remain preserved.

The strict seed gate remains unchanged: if a standard comparison seed was already
used for a historical assessment, keep that exclusion. Either use new paired cases
for both policies or establish an explicit separately recorded reassessment scope;
do not omit historical assessment seeds from the complete exclusion union or
claim they are fresh. Consumed/reserved/interrupted and unclassified Current
exclusions may never be recast as assessment-only to force admission.
