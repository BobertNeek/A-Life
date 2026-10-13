# Inspect an archived success bank before training

Build the read-only native inspector with:

```powershell
cargo build -p alife_game_app --features foundation-training --bin inspect_archived_success
```

Run `inspect_archived_success SEALED_CYCLE MANIFEST` against the exact trained
cycle intended for continuation. It checks objective2, the exported actor and
optimizer handoff, founder identity, full graph compatibility, original source
digests, bounded replay records, and genuine selected Grab/Eat outcomes through
the same typed readers used by archived rehearsal. Duplicate records, false
labels, incompatible sources and incomplete handoffs are errors.

Inspection reads artifacts and compiles phenotype metadata. It creates no GPU
session, computes no new neural output, performs no optimizer updates, and
changes no historical reward or checkpoint bytes. Its receipt reports the
manifest digest, selected source policy, founder, and checked examples by action.

Set `ALIFE_ARCHIVE_INSPECT_PROGRESS=1` to report validation progress on stderr.
The CLI uses a bounded 32 MiB worker stack for native record decoding on Windows.
Full graph reconstruction can take several minutes for a large bank; include
that validation cost in the replay budget.

The integration check accepts private, already-sealed data and retains its
tamper fixtures separately:

```powershell
python scripts/check_archived_success_inspector.py --binary INSPECTOR --source SEALED_CYCLE --manifest MANIFEST --output NEW_CHECK_DIRECTORY
```

It requires genuine Grab and meal examples, then checks rejection of bad schema,
duplicates, altered record digests, a false meal label, another founder, an
incomplete handoff, and an actor/export mismatch. Protected inputs must retain
their original hashes.

This prepares an archive for the existing `--archived-success-manifest` and
`--archived-success-epochs` training options. The training path still repeats
native validation and accounts for its actual actor updates. Inspection alone
does not prove learning, Eating retention, transfer, or promotion readiness;
those require the established frozen behavioral tests after actual training.

This preserves causal event credit (`AOA-LEARN-002`, `AOA-LEARN-003`) and bounded
selection of relevant replay (`AOA-SLEEP-005`). It does not implement organism
sleep or redefine the controlling architecture.
