# Closed held-out transfer: seven validated lives, one unknown

The first hosted run is **partial, not a completed eight-life replication**.
All eight assignments were attempted. Seven outcomes passed the strict public
reporting checks; the last reader-on outcome did not. No life was replaced or
rerun, no parameters were adjusted, and no private payload was retrieved to
rescue the result. The native-study workflow is now explicitly disabled. This
candidate remains research-only, draft and unpromoted.

## Exact execution and source

- [PR #300](https://github.com/BobertNeek/A-Life/pull/300), initial head
  `5e6784b95c7a6d68829201ff28ceec17c3b0e244`.
- [Run 42407565303](https://github.com/BobertNeek/A-Life/actions/runs/42407565303).
  [Integration job 128508455947](https://github.com/BobertNeek/A-Life/actions/runs/42407565303/job/128508455947)
  passed; [frozen-study job 128510042586](https://github.com/BobertNeek/A-Life/actions/runs/42407565303/job/128510042586)
  failed at `report/sanitizer_schema`. Always-run sanitized finalization and
  the exact seven-file artifact publication succeeded.
- Hosted Rust 1.99.1, native CPU, dev profile, one Cargo job, incremental disabled,
  debug symbols disabled, assertions retained. No local Cargo/native run was
  added for this replication.
- Integration qualification exercised the opt-in core identity, CPU checkpoint
  and assay contracts, GPU upload/WGSL validation, and focused strict Clippy.
  The 38 Python tests passed. These are not GPU-device execution or gameplay.
- The actual lives ran the separately hash-bound frozen 612c source reconstructed
  from f4cdbee plus the declared episode-only extension. They did **not** run
  the current integration world's body-clearance correction. The receipt binds
  476 inputs, source patches, plan, exact integration head and executable
  `9874c56962d7aa15e6c4f873f0c7fb9895a20484b877c898d1e02402e8ffe2b1`.

The fixed genes, sleep, safety thresholds, world and 256-tick ceiling were not
tuned on these new seeds. Construction seed remained 2026110901. The two assets
are the original reader-off/on bytes, differing only at the 40 Ingest family
value readout weights. The new seeds vary fresh body/world/sampling identity;
second/fourth scenes mirror food z positions. This is not a sample of new
genetic brains, breeds or new learned synaptic solutions.

## Measured partial outcomes

Rows below are validated whole trajectories. A is harmful food; B is safe.
The reader is the inherited +32 coefficient on acquired Ingest family value.

| Episode | Reader off: A/B, ticks | Reader on: A/B, ticks | On: final grip / failed B attempts while holding A |
| --- | --- | --- | --- |
| 2026111201 | 3/3,85 | 1/6,256 | A / 92 |
| 2026111202, mirrored | 3/3,69 | 1/12,256 | A / 6 |
| 2026111203 | 3/5,89 | 1/11,256 | A / 33 |
| 2026111204, mirrored | 3/3,108 | **Unknown** | **Unknown** |

All four validated off arms stopped at the vitality safeguard. The safeguard
checks boundaries after a step: final injury was about 0.283, beyond its 0.25
threshold, not a promise that injury never crosses 0.25. All three validated on
arms reached the fixed tick budget, with final injury about 0.092 and health
about 0.908. Endpoints differ in duration; do not call them matched-time survival
estimates or claim the missing fourth on arm succeeded.

The three complete pairs show less repeated harmful feeding with the fixed
reader. After first harm, the on arms still had 209/196/213 A-contact decision
opportunities, so their single A meal is not explained by no subsequent contact.
But all ended holding A, and their 92/6/33 attempts to ingest B while holding A
failed. Release selections did occur during these lives, so this is persistent
or recurrent possession interference, not evidence that release was impossible.
Those concurrent ownership/failure aggregates do not establish that possession
was the sole cause of every failed attempt. These outcomes do not solve feeding,
reliable self-care or long-term survival.

Detached, fixed-state family-input ablations changed the sampled choice to
Ingest(A) on 17 logged reader-on rows, all after harm; 11 were while A was unheld.
The public reporter validates actual and ablated sampling against each actual
frame/counter. This supports a local causal contribution from acquired episodic
recall under the inherited reader. It is not a counterfactual future rollout,
proof that Adam or synaptic weights learned avoidance, or a reversal test.

For each of the seven validated lives the recorded founder/optimizer equality,
checkpoint byte roundtrip and full next-checkpoint continuation flags passed.
Every reported meal passed the explicit known-Consumed provenance gate. These
are assay checks of fresh disposable lives; they do not establish retention of
unrelated skills in existing creatures. Independent review checked the allowed
public data/source and did not reopen private checkpoint payloads.

## Why the eighth result stays unknown

The checks list eight attempted assignments but only seven observed/completed
ones. Seed 2026111204 reader-on failed the strict reporting stage. No validated
behavior, meal counts, safeguard outcome or preservation checks are available
for it. `failed_cases: []` covers the seven observed cases, not an eighth success.

Source control flow calls the reporter after the native command returns zero.
That narrows the process location but does not validate the biological result.
The final seven-life publication's `completion_marker_present: false` describes
the last valid report, not necessarily the final private directory.

The exact assertion is deliberately absent from allowed logs. Source review
identified several possible gates: per-meal representative/Consumed provenance,
sampling/counter checks, schema limits, and the final-only comparison of summary
objects after Rust JSON reserialization. Float-sensitive equality is a plausible
edge, not an established diagnosis. Correct evidence rejection is also possible.
The available sanitized outputs cannot distinguish them. We did not relax a
gate, certify the missing outcome, fetch private files or repeat exposure.

## Preserved public evidence

[run-42407565303](run-42407565303/) contains exactly the seven allowlisted public
files. Four JSON files and SHA256SUMS were extracted from fixed sanitized log
sentinels. Report.md was regenerated solely from that sanitized summary and
source-only.patch copied from the declared episode patch; both matched the
uploaded hashes exactly. Full public schema, source/head/binary/plan binding,
assignment accounting and all six content hashes passed again locally without
opening private inputs. No raw journals, assets, executable, diagnostics,
checkpoints, neural banks or optimizer banks are bundled.

Key hashes:

- summary.json: `bba3686a7617433d6fa9f74dc2140efdf35dea73b64c67788518a26ac9377a5f`
- causal-rows.json: `dcddfbb8d9c33afcb9b994627fcf4a9f4648d20f18f21282334d884d0895b010`
- checks.json: `df4ff691f91b33392c6877fc21649fc75b7f3985bd040b4ce155cdfd7b856b6f`
- build-receipt.json: `1021bef4c2d3f9e35560a40dfa0c488793e3950de3a9e778deaa13e0e5c15d8a`

An independent read-only review verified the exact public bytes/hashes, all
476 source bindings, three paired comparisons, opportunity/grip counts and 17
detached switches. It found no inconsistency in the narrow partial account.
It could not determine the missing assertion or inspect private state.

## Lessons for the next design and Astra

1. Keep a protected baseline and meaningful version history. Directly written
   recurrent weights needed several revisions, not an untested one-shot claim.
2. Macro-lobe routes are insufficient: prove actual encoder-to-action paths,
   sign contracts, cadence and operating points before rollout. Preserve frozen
   identities; do not reinterpret old acquired weights by index.
3. Episodic outcome phase matters. Preparatory contact must not overwrite an
   actual meal's consequence. Legacy Unknown memories must remain reversible
   by new genuine consumption without fabricated provenance.
4. Stronger recall can reduce harmful repetition while leaving a motor/world
   bottleneck. Measure opportunities, actual grip, execution and consequences,
   not only food counts or logits. ObjectSlots has no explicit heldness bit.
5. The [grounded grip proposal](../GRIP-NEXT-DESIGN.md) remains unimplemented.
   Test candidate-bound proprioception and explicit acquisition/release semantics
   with their own identity and preservation controls. Do not add a hidden
   poison-aware drop reflex to fit this assay.
6. Future reporting should expose a bounded rejection category/case identifier
   without raw data, and bind completion entries to original summary-byte hashes
   rather than float-sensitive reserialization. Such instrumentation would help
   diagnosis; it would not retrospectively validate this missing result. Any new
   experiment needs a fresh, reviewed budget, not a blind rerun of this study.
