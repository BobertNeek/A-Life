# Inspect an archived success bank before training

Build the read-only native inspector with:

```powershell
cargo build --release -p alife_game_app --features foundation-training --bin inspect_archived_success
```

Run `inspect_archived_success SEALED_CYCLE MANIFEST` against the exact trained
cycle intended for continuation. It checks objective2, the exported actor and
optimizer handoff, founder identity, full graph compatibility, original source
digests, actor schema/config/mask and Adam payloads, value parameter shape and
compiled feature compatibility, bounded replay records, and genuine selected Grab/Eat outcomes through
the same typed readers used by archived rehearsal. Duplicate records, false
labels, incompatible sources and incomplete handoffs are errors.

For a selected sealed source at policy P, inspection uses the next continuation
policy P+1, matching training's checked policy increment. Archived episodes E<=P
can pass the ordering guard; future episodes E>P and an overflowing P+1 are
errors. Each record retains its original episode policy and frozen parent actor
binding, and the inspection receipt still reports the selected source policy P.

Inspection reads artifacts and compiles phenotype metadata. It creates no GPU
session, computes no new neural output, performs no optimizer updates, and
changes no historical reward or checkpoint bytes. Its receipt reports the
manifest digest, selected source policy, founder, and checked examples by action.

Set `ALIFE_ARCHIVE_INSPECT_PROGRESS=1` to report validation progress on stderr.
The CLI uses a bounded 32 MiB worker stack for native record decoding on Windows.
Use the release profile for repeated inspection: world and phenotype
construction dominate the debug build's cost. On October 13, the same private
40-example bank took 367.7 seconds in debug and 51.6 seconds in release, with
identical native results and unchanged input hashes. The optimized build also
passed all eight archive checks. This measures CPU inspection, not neural
execution or training throughput. Include validation cost in the replay budget;
the first optimized build has a separate one-time compilation cost.

The integration check accepts private, already-sealed data and retains its
tamper fixtures separately:

```powershell
python scripts/check_archived_success_inspector.py --binary INSPECTOR --source SEALED_CYCLE --manifest MANIFEST --output NEW_CHECK_DIRECTORY
```

It requires genuine Grab and meal examples, then checks rejection of bad schema,
duplicates, altered record digests, a false meal label, another founder, an
incomplete handoff, an actor/export mismatch, empty actor first moments, empty
value parameters, and a well-shaped value head with an incompatible feature count.
Protected inputs must retain
their original hashes.

This prepares an archive for the existing `--archived-success-manifest` and
`--archived-success-epochs` training options. The training path still repeats
native validation and accounts for its actual actor updates. Inspection alone
does not prove learning, Eating retention, transfer, or promotion readiness;
those require the established frozen behavioral tests after actual training.

This preserves causal event credit (`AOA-LEARN-002`, `AOA-LEARN-003`) and bounded
selection of relevant replay (`AOA-SLEEP-005`). It does not implement organism
sleep or redefine the controlling architecture.

## Read the original GPU scores

For passive training diagnosis, build and run the feature-gated example:

```powershell
cargo build -p alife_game_app --features foundation-training --example inspect_saved_policy
target/debug/examples/inspect_saved_policy SEALED_CYCLE ROW
```

It reads one original replay row, checks the sealed cycle, compressed record
digest, row identity, initial asset binding and score layout, then exports the
captured GPU logits alongside candidate action IDs, targets, sampling settings
and masks. It opens no GPU session, computes no neural output and changes no
inputs. Its bounded 32 MiB decoding stack supports Windows.

This is a passive score export, not the full graph and actor handoff validation
above. Score margins across different recorded worlds are descriptive training
evidence. They do not establish frozen behavior, retention or promotion.
