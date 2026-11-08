# Completed matched-world comparison: fresh 23xx v5

## Question and design

Could the creature's chosen safe-food actions succeed if the world allowed an
occupied hand to replace its item atomically? This bounded CPU development
comparison tested the **bundled atomic-replacement and item-grounding world
change**, not a new brain or a separately isolated swap coefficient.

The [fixed protocol and source archive](../fresh-matched-23xx-v5/README.md)
records two seeds, 2026112301 and 2026112302, each in old and new worlds, using
one unchanged inherited v12 asset. Its SHA-256 is
`6edcd6dd1dc5bfedf8f7b73bf605bb480e36c0ba022a316840a6f0bd87629632`.
Case order was s1 old, s1 new, s2 new, s2 old. Each received exactly 600
world-only preparation advances followed by 256 primary ticks. Paired initial
identities matched. Common instrumentation and the inherited genes were fixed;
ordinary private plasticity remained active and could diverge after trajectories
diverged. This is not a claim that all brain weights stayed identical forever.

Four cases were attempted, returned and validated, with zero rejected, unknown,
unstarted or retried cases. All reached the horizon. Total exposure was exactly
2,400 preparation advances and 1,024 primary steps. All 1,024 actual decision
replay checks were equal. No sleeping ticks occurred. The safeguards were health
below 0.70, injury above 0.25, death or a third harmful meal. There were no
replacement lives, horizon extensions or retunes.

## Results

| Seed | World | Safe meals | Harmful meals | Held-H / safe-EAT attempts | Such failures | EAT H-to-S swaps |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 2026112301 | old | 10 | 1 | 54 | 54 | 0 |
| 2026112301 | new | 14 | 1 | 1 | 0 | 1 |
| 2026112302 | old | 7 | 1 | 72 | 68 | 0 |
| 2026112302 | new | 13 | 1 | 5 | 0 | 5 |

Safe meals increased by four and six in the two paired seeds. The frozen
`h_to_s_atomic_swaps` metric counts selected-EAT swaps only. Across **all** grip
receipts, the new worlds had one and seven atomic swaps. Seed 2 additionally
had a pickup H-to-S swap at tick 133 and a reverse S-to-H EAT swap at tick 8;
total all-action H-to-S swaps were one and six. Old worlds had none.

Minimum health was approximately 0.9053, maximum injury approximately 0.0947,
and final health ranged from 0.90796 to 0.90835. All four remained alive.
Physical meals, exact Ingest episode coverage and exact-key recall are separate
measurements in `summary.json`, not interchangeable learning outcomes.

## Direct mechanism evidence and limits

Seed 1's exported scalar rows match through tick 167. At tick 168 both arms
held harmful food deliberately and selected the same safe EAT candidate, with
matching recall source 151, reach, effector and alive facts. Old world:
`MissingAffordance`, no transfer, harmful item retained. New world: `Touch`,
ordered `PlacedH` then `AcquiredS`. Seed 2 first diverged at tick 4: old world
placed the prior item only, while new world placed it and acquired the selected
item in the same action. See `PHYSICAL-SWAP-EVIDENCE.json` and the original rows.

These observations support a bounded bundled-world effect on actual brain-chosen
actions and improved safe-food access. They do not establish new biological
policy learning, durable aversion, population generalization, death/survival
improvement or GPU performance. Two seed pairs sharing one inherited asset are
not independent brain replications. Exported scalar equality does not prove full
unexported-state equality. After divergence, same-tick actions are not matched
counterfactual choices; generic failures do not establish a hold-rule cause for
every old-world failure.

## Body state, meal coverage and next policy question

Final energy increased from 0.487968 to 0.530196 for seed 1 and from 0.461188
to 0.520480 for seed 2. Hunger was not logged and is not inferred from energy.
Every physical meal matched an active exact Ingest episode; staged coverage was
zero. `FAILURE-AND-MEAL-COVERAGE.json` retains the exact body values and events.
All 54 and 68 old-world held-H / safe-EAT failures had `MissingAffordance`,
with alive, effector, contact-reach and grasp-reach facts true, while execution
contact was `none`. These facts narrow the context, not prove each failure's
cause. The first-divergence receipt above remains the strongest matched example.

The physical replacement obstruction is demonstrated. Offline inspection of the
existing traces also found later H acquisitions after each case's first harmful
meal: old/new counts were 7/12 for seed 1 and 5/8 for seed 2. Of 32 total,
31 used Grab/Contact and one used EAT preparation; none consumed another harmful
meal. Concurrent H/EAT candidates recalled the harmful meal with family valence
about -0.291 and confidence 0.791–0.934. Grab's own family values ranged from
zero to approximately -0.000268. Target memory confidence and source counts were
present, but target valence and target-source identity were not exported. This
is **not** evidence that Grab received no harm signal.

More acquisitions alone are not worse behavior: post-harm H-held decision
boundaries fell from 103 to 46 and from 117 to 53 in the new worlds. Changed
release opportunities and holding durations matter. The evidence motivates
investigating cross-family routing and use of adverse outcomes, not declaring
a confirmed brain defect, causal credit-assignment failure or learned aversion.
The exact source-safe analysis, detailed notes and analysis script are in
`harmful-food-regrabs/`; no new simulation or replay was needed. The script
retains its original source paths and requires those public/source inputs to be
materialized at those paths or adapted in a separate working copy. Its recorded
paths are provenance, not additional runtime state bundled here.

## Final design judgment: preserve useful handling

The [source trace and optional future design](NEXT-BRAIN-DESIGN.md) rejects the
strong claim that harmful experience has no route into Grab. The frozen v12
constructor already supplies summed target fear/pain-delta readouts of -8/-32
for Contact and -16/-64 for Ingest, plus +32 on Ingest family value. The target
channels can carry cross-family consequences; their actual values were not
exported here. No neural defect or causal scoring attribution is established.

H is harmful to ingest, but handling it can be harmless or useful for carrying,
relocation or disposal. **No policy change is recommended solely because of
re-grabs.** Preserve the accepted behavior unless a real handling cost is shown.
The optional qualified explicit Close/Release reader-on/off contrast changes 16
weights; it remains a future mechanistic candidate, not an activated study or a
recommended default. Any future evaluation must preserve harmless handling,
food relocation, task-conditioned release and unrelated-target behavior, and
include unrelated-pain controls. Withhold promotion if it only suppresses useful
manipulation or reduces acquisition counts without practical net benefit.

`NEXT-BRAIN-DESIGN.md` is a curated copy retaining the supplied technical design.
Its source document SHA-256 was
`706618053de4b9f54b08442ab242082466f904c93f3264cc918602971b7ef5d4`;
the curated file's checksum is in this directory's `SHA256SUMS`.

## Failures and chronology

This is the complete four-case study, not a partial-prefix recovery. All validated
rows were retained; no summary-only fallback was used. The entire earlier 22xx
allocation remains closed without retry and without a validated comparison. Its
original failure event and tick remain unknown. Later successful instrumentation
and this fresh comparison do not repair or erase that historical uncertainty.

The fixed protocol/source snapshot was published after execution had begun.
It is source provenance, not publicly timestamped preregistration. The archive
base is PR 301 head `6f7b492ea81cd749a0d08511f220282ed06b20f1`; that documentation
commit is not a study executable. `AS-RUN.json` records the exact source trees,
patch, plan, executable and package identities used by the study.

## Recovery and validation

`matched-world-v5-public-scalars.tar.gz` preserves exactly these eight original
public files, byte-for-byte: `SHA256SUMS`, `checks.json`,
`common-audit-kernel.patch`, `report.md`, `rows.json`, `source-bindings.json`,
`summary.json`, and `world-factor.patch`. Their uncompressed total is 7,362,409
bytes, below the fixed 16 MiB public cap. The deterministic USTAR/gzip bundle is
396,675 bytes, SHA-256
`faa894785e9f00be0cd7c0b967561b26017dc5711c9dce45155aef031e92de75`.

`BUNDLE-MANIFEST.json` pins every member, order and normalized metadata. Verify
this directory with `sha256sum -c SHA256SUMS`, then extract the bundle into an
empty directory and run its own `sha256sum -c SHA256SUMS`. Extraction is data
recovery, not permission or instructions to execute a new study.

Archive preparation verified the exact eight-member allowlist, hashes and
metadata; strict finite scalar schemas and row semantics for all four cases;
recomputed summary metrics; paired initial identities; and source-text-only
patch contents. `AS-RUN.json` and `PHYSICAL-SWAP-EVIDENCE.json` are original,
byte-exact copies. Small summary and source bindings are duplicated outside the
bundle for convenient review. Original compressed report wording about an
unchanged foundation refers to the inherited asset, not frozen private plasticity.

No private runtime state, checkpoints, neural banks, raw stderr, credentials,
permission metadata, executables or assets are included. No native compilation,
native test, new creature life or study rerun was performed for this archive.
The added re-acquisition analysis was reproduced byte-for-byte with its inspected
source/scalar-only Python script. Failure events, meal events, episode coverage
and final body values were checked against the admitted scalar rows. These later
derived files do not change any original eight-file evidence bytes or the bundle.
Earlier evidence directories remain byte-for-byte unchanged.
